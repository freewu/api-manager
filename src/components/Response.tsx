import { useMemo, useState } from "react";
import { HttpResult } from "../types";

interface Props {
  result: HttpResult | null;
  sending: boolean;
}

function statusClass(status: number) {
  if (status >= 200 && status < 300) return "status-2xx";
  if (status >= 300 && status < 400) return "status-3xx";
  if (status >= 400 && status < 500) return "status-4xx";
  if (status >= 500) return "status-5xx";
  return "";
}

function escapeHtml(s: string) {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

/** 对 JSON 做简单语法高亮，返回 HTML 片段 */
export function highlightJson(text: string): string {
  try {
    const parsed = JSON.parse(text);
    const pretty = JSON.stringify(parsed, null, 2);
    return pretty
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(
        /("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false)\b|\bnull\b|-?\d+(?:\.\d*)?(?:[eE][+-]?\d+)?)/g,
        (match) => {
          let cls = "json-number";
          if (/^"/.test(match)) {
            cls = /:$/.test(match) ? "json-key" : "json-string";
          } else if (/true|false/.test(match)) {
            cls = "json-boolean";
          } else if (/null/.test(match)) {
            cls = "json-null";
          }
          return `<span class="${cls}">${match}</span>`;
        }
      );
  } catch {
    return escapeHtml(text);
  }
}

export function Response({ result, sending }: Props) {
  const [tab, setTab] = useState<"body" | "headers">("body");

  const bodyHtml = useMemo(() => (result ? highlightJson(result.body) : ""), [result]);

  if (sending && !result) {
    return (
      <div className="response">
        <div className="response-body">
          <div className="response-empty">
            <span className="big">⏳</span>
            <span>请求发送中…</span>
          </div>
        </div>
      </div>
    );
  }

  if (!result) {
    return (
      <div className="response">
        <div className="response-head" style={{ color: "var(--text-faint)" }}>
          响应
        </div>
        <div className="response-body">
          <div className="response-empty">
            <span className="big">📡</span>
            <span>点击「发送」查看响应结果</span>
          </div>
        </div>
      </div>
    );
  }

  const kb = (result.size / 1024).toFixed(2);

  return (
    <div className="response">
      <div className="response-head">
        {result.ok ? (
          <>
            <span className={`status-badge ${statusClass(result.status)}`}>
              {result.status} {result.statusText}
            </span>
            <span className="resp-meta">
              <span>
                <span className="label">耗时 </span>
                <b>{result.timeMs} ms</b>
              </span>
              <span>
                <span className="label">大小 </span>
                <b>{kb} KB</b>
              </span>
            </span>
          </>
        ) : (
          <span className="status-badge status-5xx">请求失败</span>
        )}
      </div>
      <div className="response-body">
        {result.error ? (
          <div className="error-banner">{result.error}</div>
        ) : (
          <>
            <div className="resp-tabs">
              <div
                className={`resp-tab ${tab === "body" ? "active" : ""}`}
                onClick={() => setTab("body")}
              >
                Body
              </div>
              <div
                className={`resp-tab ${tab === "headers" ? "active" : ""}`}
                onClick={() => setTab("headers")}
              >
                Headers ({result.headers.length})
              </div>
            </div>
            {tab === "body" && (
              <div className="json-view" dangerouslySetInnerHTML={{ __html: bodyHtml }} />
            )}
            {tab === "headers" && (
              <table className="resp-headers-table">
                <tbody>
                  {result.headers.map(([k, v], i) => (
                    <tr key={i}>
                      <td>{k}</td>
                      <td>{v}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </>
        )}
      </div>
    </div>
  );
}
