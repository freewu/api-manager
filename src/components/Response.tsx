import { useEffect, useMemo, useState } from "react";
import { HttpResult } from "../types";
import { useT } from "../i18n";

interface Props {
  result: HttpResult | null;
  sending: boolean;
  /** 点击「保存为示例」并确认名称后回调（App 负责写入 .examples 目录） */
  onSaveExample?: (name: string) => void;
  /** WebSocket 响应无 HTTP 状态码：为 true 时不展示状态码 */
  hideStatus?: boolean;
}

type View = "auto" | "raw" | "html" | "xml" | "json" | "text";
const VIEWS: { value: View; label: string }[] = [
  { value: "raw", label: "RAW" },
  { value: "html", label: "HTML" },
  { value: "xml", label: "XML" },
  { value: "json", label: "JSON" },
  { value: "text", label: "TEXT" },
  { value: "auto", label: "" },
];

export function statusClass(status: number) {
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

/** 根据 Content-Type 与内容自动判断实际视图 */
function detectView(body: string, contentType: string | undefined): View {
  const ct = (contentType || "").toLowerCase();
  if (ct.includes("json") || ct.includes("javascript")) return "json";
  if (ct.includes("html")) return "html";
  if (ct.includes("xml")) return "xml";
  const t = body.trim();
  if (!t) return "text";
  if (t.startsWith("{") || t.startsWith("[")) return "json";
  if (t.startsWith("<")) return /<!doctype|<html|<head|<body/i.test(t) ? "html" : "xml";
  return "text";
}

function prettyJson(text: string): string {
  try {
    return JSON.stringify(JSON.parse(text), null, 2);
  } catch {
    return text;
  }
}

async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    try {
      const ta = document.createElement("textarea");
      ta.value = text;
      ta.style.position = "fixed";
      ta.style.opacity = "0";
      document.body.appendChild(ta);
      ta.select();
      const ok = document.execCommand("copy");
      document.body.removeChild(ta);
      return ok;
    } catch {
      return false;
    }
  }
}

export function Response({ result, sending, onSaveExample, hideStatus }: Props) {
  const t = useT();
  const [tab, setTab] = useState<"body" | "headers">("body");
  const [view, setView] = useState<View>("auto");
  const [copied, setCopied] = useState(false);
  const [saveOpen, setSaveOpen] = useState(false);
  const [saveName, setSaveName] = useState("");

  const contentType = useMemo(
    () => result?.headers.find(([k]) => k.toLowerCase() === "content-type")?.[1],
    [result]
  );

  // 新响应到达时回到自动选择
  useEffect(() => {
    setView("auto");
  }, [result]);

  const actualView: View = useMemo(() => {
    if (!result) return "text";
    return view === "auto" ? detectView(result.body, contentType) : view;
  }, [result, view, contentType]);

  const bodyEl = useMemo(() => {
    if (!result) return null;
    switch (actualView) {
      case "raw":
      case "text":
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
          <div
            className="json-view"
            dangerouslySetInnerHTML={{ __html: highlightJson(prettyJson(result.body)) }}
          />
        );
      case "auto":
        return null;
    }
  }, [result, actualView]);

  const handleCopy = async () => {
    if (!result) return;
    const text = actualView === "json" ? prettyJson(result.body) : result.body;
    const ok = await copyText(text);
    if (ok) {
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    }
  };

  if (sending && !result) {
    return (
      <div className="response">
        <div className="response-body">
          <div className="response-empty">
            <span className="big">⏳</span>
            <span>{t("resp.sending")}</span>
          </div>
        </div>
      </div>
    );
  }

  if (!result) {
    return (
      <div className="response">
        <div className="response-head" style={{ color: "var(--text-faint)" }}>
          {t("resp.response")}
        </div>
        <div className="response-body">
          <div className="response-empty">
            <span className="big">📡</span>
            <span>{t("resp.hint")}</span>
          </div>
        </div>
      </div>
    );
  }

  const kb = (result.size / 1024).toFixed(2);

  return (
    <div className="response">
      <div className="response-head">
        {hideStatus ? (
          <>
            <span className="status-badge status-2xx">{t("resp.wsConnected")}</span>
            <span className="resp-meta">
              <span>
                <span className="label">{t("resp.time")} </span>
                <b>{result.timeMs} ms</b>
              </span>
              <span>
                <span className="label">{t("resp.size")} </span>
                <b>{kb} KB</b>
              </span>
            </span>
          </>
        ) : result.ok ? (
          <>
            <span className={`status-badge ${statusClass(result.status)}`}>
              {result.status} {result.statusText}
            </span>
            <span className="resp-meta">
              <span>
                <span className="label">{t("resp.time")} </span>
                <b>{result.timeMs} ms</b>
              </span>
              <span>
                <span className="label">{t("resp.size")} </span>
                <b>{kb} KB</b>
              </span>
            </span>
          </>
        ) : (
          <span className="status-badge status-5xx">{t("resp.failed")}</span>
        )}
        {onSaveExample && (
          <div className="resp-save-example">
            {saveOpen ? (
              <>
                <input
                  className="resp-save-input"
                  autoFocus
                  placeholder={t("resp.exampleName")}
                  value={saveName}
                  onChange={(e) => setSaveName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && saveName.trim()) {
                      onSaveExample(saveName.trim());
                      setSaveOpen(false);
                      setSaveName("");
                    }
                    if (e.key === "Escape") {
                      setSaveOpen(false);
                      setSaveName("");
                    }
                  }}
                />
                <button
                  type="button"
                  className="btn small primary"
                  disabled={!saveName.trim()}
                  onClick={() => {
                    if (saveName.trim()) {
                      onSaveExample(saveName.trim());
                      setSaveOpen(false);
                      setSaveName("");
                    }
                  }}
                >
                  {t("common.save")}
                </button>
                <button type="button" className="btn small" onClick={() => setSaveOpen(false)}>
                  {t("common.cancel")}
                </button>
              </>
            ) : (
              <button type="button" className="btn small" onClick={() => setSaveOpen(true)}>
                💾 {t("resp.saveExample")}
              </button>
            )}
          </div>
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
                    <select
                      className="view-select"
                      value={view}
                      onChange={(e) => setView(e.target.value as View)}
                      title={t("resp.viewFormat")}
                    >
                      {VIEWS.map((v) => (
                        <option key={v.value} value={v.value}>
                          {v.value === "auto" ? t("resp.auto") : v.label}
                        </option>
                      ))}
                    </select>
                    <button
                      type="button"
                      className={`view-copy ${copied ? "ok" : ""}`}
                      onClick={() => void handleCopy()}
                      title={t("resp.copyResult")}
                    >
                      {copied ? t("resp.copied") : "📋 " + t("common.copy")}
                    </button>
                    {contentType && (
                      <span className="view-ct" title={t("resp.contentType")}>
                        {contentType.split(";")[0]}
                      </span>
                    )}
                  </div>
                )}
                {result.body ? (
                  bodyEl
                ) : (
                  <div className="response-empty">
                    <span>{t("resp.emptyBody")}</span>
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
