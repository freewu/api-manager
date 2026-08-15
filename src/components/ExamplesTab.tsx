import { useEffect, useState } from "react";
import { ApiFile, ExampleFile, ExampleSummary } from "../types";
import { deleteExample, listExamples, readExample } from "../commands";
import { highlightJson } from "./Response";

interface Props {
  /** 当前接口 uuid（.examples/<uuid>/ 目录） */
  uuid: string;
  /** 当前接口（「应用到当前接口」需要） */
  api: ApiFile;
  /** 修改接口（「应用到当前接口」回调） */
  onChange: (api: ApiFile) => void;
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
  const t = text.trim();
  if (!t) return <div className="examples-empty">（空）</div>;
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

export function ExamplesTab({ uuid, api, onChange }: Props) {
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
          请求示例{" "}
          <span className="help">保存在工作区 .examples/{uuid || "…"}/ 目录，同名示例会覆盖</span>
        </span>
        <div className="examples-actions">
          <span className="examples-count">{list.length} 个</span>
          <button type="button" className="btn small" onClick={() => void load()} disabled={loading}>
            🔄 刷新
          </button>
        </div>
      </div>

      {error && <div className="error-banner">{error}</div>}

      {loading ? (
        <div className="examples-empty">加载中…</div>
      ) : list.length === 0 ? (
        <div className="examples-empty">
          <span className="big">🧪</span>
          <span>暂无示例。发送请求后，在响应区点击「保存为示例」即可生成</span>
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
                  title="删除示例"
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
                      title="将 Header / Path / Query / Body 参数填充到当前接口"
                      onClick={() => apply(detail)}
                    >
                      ⬇ 应用到当前接口
                    </button>
                  </div>
                  <div className="examples-section">
                    <div className="examples-detail-title">Header</div>
                    <KVTable rows={detail.reqHeaders} empty="（无请求头）" />
                  </div>
                  <div className="examples-section">
                    <div className="examples-detail-title">Path</div>
                    <KVTable rows={detail.reqPath} empty="（无路径参数）" />
                  </div>
                  <div className="examples-section">
                    <div className="examples-detail-title">Query</div>
                    <KVTable rows={detail.reqQuery} empty="（无 Query 参数）" />
                  </div>
                  <div className="examples-section">
                    <div className="examples-detail-title">Body</div>
                    <BodyView text={detail.reqBody || ""} />
                  </div>
                  <div className="examples-section">
                    <div className="examples-detail-title">
                      响应{" "}
                      <span className="examples-detail-meta">
                        {detail.error
                          ? "请求失败"
                          : `${detail.status} ${detail.statusText} · ${detail.timeMs} ms · ${(
                              detail.size / 1024
                            ).toFixed(2)} KB`}
                      </span>
                    </div>
                    {detail.error ? (
                      <div className="error-banner">{detail.error}</div>
                    ) : (
                      <>
                        <KVTable rows={detail.respHeaders} empty="（无响应头）" />
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
