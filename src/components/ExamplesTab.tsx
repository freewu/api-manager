import { useEffect, useState } from "react";
import { ExampleFile, ExampleSummary } from "../types";
import { deleteExample, listExamples, readExample } from "../commands";
import { highlightJson, statusClass } from "./Response";

interface Props {
  /** 当前接口 uuid（.examples/<uuid>/ 目录） */
  uuid: string;
}

function fmtTime(t: number): string {
  return new Date(t * 1000).toLocaleString();
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

export function ExamplesTab({ uuid }: Props) {
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

  return (
    <div className="examples-root">
      <div className="examples-head">
        <span className="examples-title">
          请求示例 <span className="help">保存在工作区 .examples/{uuid || "…"}/ 目录，同名示例会覆盖</span>
        </span>
        <div className="examples-actions">
          <span className="examples-count">{list.length} 个</span>
          <button type="button" className="btn mini" onClick={() => void load()} disabled={loading}>
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
                <span className={`status-badge ${statusClass(s.status)}`}>{s.status}</span>
                <span className="examples-item-name" title={s.name}>
                  {s.name}
                </span>
                <span className="examples-item-meta">
                  {s.method} {s.url}
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
                  <div className="examples-detail-col">
                    <div className="examples-detail-title">请求</div>
                    <div className="examples-line">
                      <b>{detail.method}</b> {detail.url}
                    </div>
                    {detail.reqHeaders.length > 0 && (
                      <table className="resp-headers-table">
                        <tbody>
                          {detail.reqHeaders.map(([k, v], i) => (
                            <tr key={i}>
                              <td>{k}</td>
                              <td>{v}</td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    )}
                    <BodyView text={detail.reqBody || ""} />
                  </div>
                  <div className="examples-detail-col">
                    <div className="examples-detail-title">
                      响应{" "}
                      <span className="examples-detail-meta">
                        {detail.timeMs} ms · {(detail.size / 1024).toFixed(2)} KB
                      </span>
                    </div>
                    {detail.error ? (
                      <div className="error-banner">{detail.error}</div>
                    ) : (
                      <>
                        {detail.respHeaders.length > 0 && (
                          <table className="resp-headers-table">
                            <tbody>
                              {detail.respHeaders.map(([k, v], i) => (
                                <tr key={i}>
                                  <td>{k}</td>
                                  <td>{v}</td>
                                </tr>
                              ))}
                            </tbody>
                          </table>
                        )}
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
