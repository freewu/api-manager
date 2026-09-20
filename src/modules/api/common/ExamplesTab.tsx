import { useEffect, useMemo, useRef, useState } from "react";
import { ApiFile, ExampleFile, ExampleSummary, isNetProtocol } from "../../../types";
import {
  bytesToHex,
  encodeValue,
  hexToBytes,
  parsePacket,
  type PacketFieldResult,
} from "../../../utils/packet";
import { PacketViewTable } from "./PacketViewTable";
import {
  deleteExample,
  exportExamplesHttp,
  listExamples,
  readExample,
  renameExample,
} from "../../../commands";
import { highlightJson } from "../http/Response";
import { CopyBtn } from "./CopyBtn";
import { useT } from "../../../i18n";

interface Props {
  /** 当前接口 uuid（.examples/<uuid>/ 目录） */
  uuid: string;
  /** 当前接口（「应用到当前接口」需要） */
  api: ApiFile;
  /** 修改接口（「应用到当前接口」回调） */
  onChange: (api: ApiFile) => void;
  /** 示例数量变化回调（供父级页签角标显示） */
  onCountChange?: (count: number) => void;
}

function fmtTime(t: number): string {
  return new Date(t * 1000).toLocaleString();
}

/** 键值表（Header / Path / Query / 响应头） */
function KVTable({ rows, empty }: { rows: [string, string][]; empty: string }) {
  if (!rows.length) return <div className="examples-empty">{empty}</div>;
  return (
    <table className="resp-headers-table">
      <tbody>
        {rows.map(([k, v], i) => (
          <tr key={i}>
            <td>{k}</td>
            <td>{v}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/** 请求/响应体展示：JSON 高亮，其余原文 */
function BodyView({ text }: { text: string }) {
  const T = useT();
  const t = text.trim();
  if (!t) return <div className="examples-empty">{T("examples.emptyBody")}</div>;
  const isJson = t.startsWith("{") || t.startsWith("[");
  if (isJson) {
    try {
      const pretty = JSON.stringify(JSON.parse(t), null, 2);
      return (
        <div
          className="examples-body json-view"
          dangerouslySetInnerHTML={{ __html: highlightJson(pretty) }}
        />
      );
    } catch {
      /* 非 JSON，走原文 */
    }
  }
  return <pre className="examples-body examples-pre">{text}</pre>;
}

/** 解析出的字节 → 可直接回填的字段值：能原样编码回相同字节时用文本，否则用 0x… */
function bytesToFieldValue(r: PacketFieldResult): string {
  const hex = r.hex.replace(/ /g, "");
  if (!hex) return "";
  if (r.text.trim() && bytesToHex(encodeValue(r.text)).replace(/ /g, "") === hex) return r.text;
  return "0x" + hex;
}

/**
 * TCP / UDP 示例详情：与 HTTP 示例不同，不展示 Header / Path / Query / Body，
 * 而是展示「请求报文（按封包定义）」与「响应报文（按解包定义）」的报文字节与字段解析。
 */
function NetExampleDetail({
  detail,
  api,
  onApply,
}: {
  detail: ExampleFile;
  api: ApiFile;
  onApply: () => void;
}) {
  const t = useT();
  // 优先使用示例保存时的字段定义，缺失时回退到当前接口的定义
  const pack = detail.pack?.length ? detail.pack : api.pack || [];
  const unpack = detail.unpack?.length ? detail.unpack : api.unpack || [];
  const reqBytes = useMemo(() => hexToBytes(detail.reqBody || ""), [detail.reqBody]);
  const respBytes = useMemo(() => hexToBytes(detail.respBody || ""), [detail.respBody]);
  const reqRows = useMemo(
    () => (pack.length && reqBytes.length ? parsePacket(pack, reqBytes) : []),
    [pack, reqBytes]
  );
  const respRows = useMemo(
    () => (unpack.length && respBytes.length ? parsePacket(unpack, respBytes) : []),
    [unpack, respBytes]
  );
  return (
    <div className="examples-detail">
      <div className="examples-request-line">
        <b>{(detail.protocol || detail.method || "").toUpperCase()}</b> {detail.url}
        <button
          type="button"
          className="btn small primary examples-apply"
          title={t("examples.netApplyTip")}
          onClick={onApply}
        >
          ⬇ {t("examples.apply")}
        </button>
      </div>
      <div className="examples-section">
        <div className="examples-detail-title">
          {t("examples.netReqPacket")}{" "}
          <span className="examples-detail-meta">
            {reqBytes.length > 0 ? `${reqBytes.length} ${t("net.byte")}` : ""}
          </span>
          {reqBytes.length > 0 && (
            <CopyBtn className="copy-inline" text={detail.reqBody || ""} title={t("net.copyPreviewTip")} />
          )}
        </div>
        {reqBytes.length === 0 ? (
          <div className="examples-empty">{t("examples.netNoPacket")}</div>
        ) : (
          <>
            <code className="packet-hex">{detail.reqBody}</code>
            {reqRows.length > 0 && <PacketViewTable fields={pack} rows={reqRows} />}
          </>
        )}
      </div>
      <div className="examples-section">
        <div className="examples-detail-title">
          {t("examples.netRespPacket")}{" "}
          <span className="examples-detail-meta">
            {detail.error
              ? t("examples.failed")
              : `${detail.timeMs} ms · ${detail.size} ${t("net.byte")}`}
          </span>
          {!detail.error && respBytes.length > 0 && (
            <CopyBtn className="copy-inline" text={detail.respBody || ""} title={t("net.copyPreviewTip")} />
          )}
        </div>
        {detail.error ? (
          <div className="error-banner">{detail.error}</div>
        ) : respBytes.length === 0 ? (
          <div className="examples-empty">{t("net.noResponse")}</div>
        ) : (
          <>
            <code className="packet-hex">{detail.respBody}</code>
            {respRows.length > 0 && <PacketViewTable fields={unpack} rows={respRows} />}
          </>
        )}
      </div>
    </div>
  );
}

export function ExamplesTab({ uuid, api, onChange, onCountChange }: Props) {
  const t = useT();
  const [list, setList] = useState<ExampleSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [expanded, setExpanded] = useState<string | null>(null);
  const [detail, setDetail] = useState<ExampleFile | null>(null);
  const [error, setError] = useState("");

  const load = async () => {
    if (!uuid) return;
    setLoading(true);
    setError("");
    try {
      setList(await listExamples(uuid));
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  // 数量上报：加载完成后同步到父级（页签角标）
  useEffect(() => {
    onCountChange?.(list.length);
  }, [list, onCountChange]);

  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [uuid]);

  const toggle = async (sum: ExampleSummary) => {
    if (expanded === sum.file) {
      setExpanded(null);
      setDetail(null);
      return;
    }
    try {
      const d = await readExample(uuid, sum.file);
      setDetail(d);
      setExpanded(sum.file);
    } catch (e) {
      setError(String(e));
    }
  };

  const remove = async (sum: ExampleSummary) => {
    try {
      await deleteExample(uuid, sum.file);
      if (expanded === sum.file) {
        setExpanded(null);
        setDetail(null);
      }
      await load();
    } catch (e) {
      setError(String(e));
    }
  };

  // ---- 重命名示例：行首名称变为输入框，Enter/失焦提交，Esc 取消 ----
  const [renaming, setRenaming] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  // 与 renaming 同步的 ref：避免 Enter 提交后紧随的 blur 重复提交
  const renamingRef = useRef<string | null>(null);
  const startRename = (sum: ExampleSummary) => {
    renamingRef.current = sum.file;
    setRenaming(sum.file);
    setEditName(sum.name);
  };
  const cancelRename = () => {
    renamingRef.current = null;
    setRenaming(null);
  };
  const commitRename = async () => {
    const file = renamingRef.current;
    if (!file) return; // 已提交过 / 已取消
    renamingRef.current = null;
    setRenaming(null);
    const name = editName.trim();
    if (!name) {
      setError(t("examples.nameEmpty"));
      return;
    }
    try {
      const newFile = await renameExample(uuid, file, name);
      // 展开中的项改名后保持展开：按新文件名重新读取详情
      if (expanded === file) {
        try {
          setDetail(await readExample(uuid, newFile));
          setExpanded(newFile);
        } catch {
          setExpanded(null);
          setDetail(null);
        }
      }
      await load();
    } catch (e) {
      setError(String(e));
    }
  };

  // ---- 一次性导出全部示例为一个 .http 文件（仅 HTTP 接口；保存框取消时不提示） ----
  const isHttp = api.protocol === "http";
  /** TCP / UDP 接口（示例详情展示报文字节与字段解析） */
  const isNet = isNetProtocol(api.protocol);
  const [exported, setExported] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);
  const exportAllHttp = async () => {
    setExporting(true);
    try {
      const saved = await exportExamplesHttp(uuid, api.name);
      if (saved) {
        setExported(saved);
        window.setTimeout(() => setExported((p) => (p === saved ? null : p)), 6000);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setExporting(false);
    }
  };

  // 把示例的 Header / Path / Query / Body 应用到当前接口（URL 保持当前接口的占位符形式）
  // 把示例的 Header / Path / Query / Body 应用到当前接口（URL 保持当前接口的占位符形式）
  const apply = (d: ExampleFile) => {
    const kv = (rows: [string, string][]) =>
      rows.map(([key, value]) => ({ key, value, enabled: true, description: "" }));
    const body = { ...api.body };
    if (d.reqBody !== undefined && d.reqBody !== null) {
      const t = d.reqBody.trim();
      body.mode = t.startsWith("{") || t.startsWith("[") ? "json" : "raw";
      body.raw = d.reqBody;
    }
    onChange({
      ...api,
      headers: kv(d.reqHeaders),
      params: kv(d.reqPath),
      query: kv(d.reqQuery),
      body,
    });
  };

  /** TCP / UDP：把示例报文的封包字段值应用回当前接口（按保存时的封包定义解析字节） */
  const applyNet = (d: ExampleFile) => {
    const defs = d.pack?.length ? d.pack : api.pack || [];
    const bytes = hexToBytes(d.reqBody || "");
    if (!defs.length || !bytes.length) return;
    const rows = parsePacket(defs, bytes);
    onChange({
      ...api,
      pack: (api.pack || []).map((f, i) => {
        // 字段名一致才回填，避免保存后字段顺序变化导致值错位
        const r = rows[i];
        if (!r || (defs[i]?.key || "") !== (f.key || "")) return f;
        const value = bytesToFieldValue(r);
        return value ? { ...f, value } : f;
      }),
    });
  };

  return (
    <div className="examples-root">
      <div className="examples-head">
        <span className="examples-title">
          {t("examples.title")}{" "}
          <span className="help">{t("examples.savedHint", { uuid: uuid || "…" })}</span>
        </span>
        <div className="examples-actions">
          <span className="examples-count">{list.length} {t("examples.count")}</span>
          {isHttp && list.length > 0 && (
            <button
              type="button"
              className="btn small"
              title={t("examples.exportAllTip")}
              onClick={() => void exportAllHttp()}
              disabled={loading || exporting}
            >
              ⬇ {t("examples.exportAll")}
            </button>
          )}
          <button type="button" className="btn small" onClick={() => void load()} disabled={loading}>
            🔄 {t("common.refresh")}
          </button>
        </div>
      </div>

      {error && <div className="error-banner">{error}</div>}
      {exported && (
        <div className="examples-notice">{t("examples.exportedTo", { path: exported })}</div>
      )}

      {loading ? (
        <div className="examples-empty">{t("examples.loading")}</div>
      ) : list.length === 0 ? (
        <div className="examples-empty">
          <span className="big">🧪</span>
          <span>{t("examples.empty")}</span>
        </div>
      ) : (
        <div className="examples-list">
          {list.map((s) => (
            <div key={s.file} className={`examples-item ${expanded === s.file ? "open" : ""}`}>
              <div className="examples-item-head" onClick={() => void toggle(s)}>
                {renaming === s.file ? (
                  <input
                    className="examples-rename-input"
                    autoFocus
                    value={editName}
                    spellCheck={false}
                    onClick={(e) => e.stopPropagation()}
                    onChange={(e) => setEditName(e.target.value)}
                    onKeyDown={(e) => {
                      e.stopPropagation();
                      if (e.key === "Enter") void commitRename();
                      else if (e.key === "Escape") cancelRename();
                    }}
                    onBlur={() => void commitRename()}
                  />
                ) : (
                  <span className="examples-item-name" title={s.name}>
                    {s.name}
                  </span>
                )}
                <span className="examples-item-time">{fmtTime(s.time)}</span>
                <button
                  type="button"
                  className="examples-icon"
                  title={t("examples.rename")}
                  onClick={(e) => {
                    e.stopPropagation();
                    startRename(s);
                  }}
                >
                  ✎
                </button>
                <button
                  type="button"
                  className="examples-delete"
                  title={t("examples.delete")}
                  onClick={(e) => {
                    e.stopPropagation();
                    void remove(s);
                  }}
                >
                  🗑
                </button>
              </div>
              {expanded === s.file && detail && isNet && (
                <NetExampleDetail detail={detail} api={api} onApply={() => applyNet(detail)} />
              )}
              {expanded === s.file && detail && !isNet && (
                <div className="examples-detail">
                  <div className="examples-request-line">
                    <b>{detail.method}</b> {detail.url}
                    <button
                      type="button"
                      className="btn small primary examples-apply"
                      title={t("examples.applyTip")}
                      onClick={() => apply(detail)}
                    >
                      ⬇ {t("examples.apply")}
                    </button>
                  </div>
                  <div className="examples-section">
                    <div className="examples-detail-title">Header</div>
                    <KVTable rows={detail.reqHeaders} empty={t("examples.noHeaders")} />
                  </div>
                  <div className="examples-section">
                    <div className="examples-detail-title">Path</div>
                    <KVTable rows={detail.reqPath} empty={t("examples.noPath")} />
                  </div>
                  <div className="examples-section">
                    <div className="examples-detail-title">Query</div>
                    <KVTable rows={detail.reqQuery} empty={t("examples.noQuery")} />
                  </div>
                  <div className="examples-section">
                    <div className="examples-detail-title">Body</div>
                    <BodyView text={detail.reqBody || ""} />
                  </div>
                  <div className="examples-section">
                    <div className="examples-detail-title">
                      {t("examples.response")}{" "}
                      <span className="examples-detail-meta">
                        {detail.error
                          ? t("examples.failed")
                          : `${detail.status || detail.method} ${detail.statusText} · ${detail.timeMs} ms · ${(
                              detail.size / 1024
                            ).toFixed(2)} KB`}
                      </span>
                    </div>
                    {detail.error ? (
                      <div className="error-banner">{detail.error}</div>
                    ) : (
                      <>
                        <KVTable rows={detail.respHeaders} empty={t("examples.noRespHeaders")} />
                        <BodyView text={detail.respBody} />
                      </>
                    )}
                  </div>
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
