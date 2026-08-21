import { useEffect, useState } from "react";
import { ApiFile, ExampleFile, ExampleSummary } from "../types";
import { deleteExample, listExamples, readExample } from "../commands";
import { highlightJson } from "./Response";
import { useT } from "../i18n";

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

  return (
    <div className="examples-root">
      <div className="examples-head">
        <span className="examples-title">
          {t("examples.title")}{" "}
          <span className="help">{t("examples.savedHint", { uuid: uuid || "…" })}</span>
        </span>
        <div className="examples-actions">
          <span className="examples-count">{list.length} {t("examples.count")}</span>
          <button type="button" className="btn small" onClick={() => void load()} disabled={loading}>
            🔄 {t("common.refresh")}
          </button>
        </div>
      </div>

      {error && <div className="error-banner">{error}</div>}

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
                <span className="examples-item-name" title={s.name}>
                  {s.name}
                </span>
                <span className="examples-item-time">{fmtTime(s.time)}</span>
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
              {expanded === s.file && detail && (
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
