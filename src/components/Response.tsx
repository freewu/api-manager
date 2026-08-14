import { useEffect, useMemo, useState } from "react";
import { HttpResult } from "../types";

interface Props {
  result: HttpResult | null;
  sending: boolean;
}

type View = "raw" | "html" | "json" | "xml";
const VIEWS: View[] = ["raw", "html", "json", "xml"];

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

/** 对 HTML / XML 做简单语法高亮，返回 HTML 片段 */
export function highlightMarkup(text: string): string {
  const esc = escapeHtml(text);
  return esc.replace(
    /(&lt;!--[\s\S]*?--&gt;)|(&lt;\/?\??[\w:.-]+)|(\/?&gt;)|([\w:.-]+=)("[^"]*"|'[^']*')/g,
    (m, comment: string | undefined, tag: string | undefined, tagEnd: string | undefined, attr: string | undefined, val: string | undefined) => {
      if (comment) return `<span class="xml-comment">${comment}</span>`;
      if (tag) return `<span class="xml-tag">${tag}</span>`;
      if (tagEnd) return `<span class="xml-tag">${tagEnd}</span>`;
      if (attr) return `<span class="xml-attr">${attr}</span><span class="xml-value">${val}</span>`;
      return m;
    }
  );
}

/** 根据 Content-Type 与内容自动判断默认视图 */
function detectView(body: string, contentType: string | undefined): View {
  const ct = (contentType || "").toLowerCase();
  if (ct.includes("json") || ct.includes("javascript")) return "json";
  if (ct.includes("html")) return "html";
  if (ct.includes("xml")) return "xml";
  const t = body.trim();
  if (!t) return "raw";
  if (t.startsWith("{") || t.startsWith("[")) return "json";
  if (t.startsWith("<")) return /<!doctype|<html|<head|<body/i.test(t) ? "html" : "xml";
  return "raw";
}

export function Response({ result, sending }: Props) {
  const [tab, setTab] = useState<"body" | "headers">("body");
  const [view, setView] = useState<View>("raw");

  const contentType = useMemo(
    () => result?.headers.find(([k]) => k.toLowerCase() === "content-type")?.[1],
    [result]
  );

  // 新响应到达时自动选择视图
  useEffect(() => {
    if (result) setView(detectView(result.body, contentType));
  }, [result, contentType]);

  const bodyEl = useMemo(() => {
    if (!result) return null;
    switch (view) {
      case "raw":
        return <div className="json-view raw-view">{result.body || ""}</div>;
      case "html":
      case "xml":
        return (
          <div
            className="json-view"
            dangerouslySetInnerHTML={{ __html: highlightMarkup(result.body) }}
          />
        );
      case "json":
        return (
          <div className="json-view" dangerouslySetInnerHTML={{ __html: highlightJson(result.body) }} />
        );
    }
  }, [result, view]);

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
              <>
                {result.body !== "" && (
                  <div className="view-bar">
                    {VIEWS.map((v) => (
                      <div
                        key={v}
                        className={`view-chip ${view === v ? "active" : ""}`}
                        onClick={() => setView(v)}
                      >
                        {v.toUpperCase()}
                      </div>
                    ))}
                    {contentType && (
                      <span className="view-ct" title="响应 Content-Type">
                        {contentType.split(";")[0]}
                      </span>
                    )}
                  </div>
                )}
                {result.body ? (
                  bodyEl
                ) : (
                  <div className="response-empty">
                    <span>（空响应体）</span>
                  </div>
                )}
              </>
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
