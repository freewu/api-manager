import { HistoryDetail as HistoryDetailType } from "../commands";
import { highlightJson } from "./Response";

function statusClass(status: number) {
  if (status >= 200 && status < 300) return "status-2xx";
  if (status >= 300 && status < 400) return "status-3xx";
  if (status >= 400 && status < 500) return "status-4xx";
  if (status >= 500) return "status-5xx";
  return "";
}

function methodClass(method: string) {
  return `method-${method.toLowerCase()}`;
}

function prettyBody(text: string): string {
  try {
    return JSON.stringify(JSON.parse(text), null, 2);
  } catch {
    return text;
  }
}

/** 右侧请求历史详情（请求 Headers/Body + 响应 Headers/Body） */
export function HistoryDetail({
  detail,
  loading,
}: {
  detail: HistoryDetailType | null;
  loading: boolean;
}) {
  if (loading) return <div className="history-empty">加载中…</div>;
  if (!detail) {
    return <div className="history-empty">点击左侧记录查看请求与响应详情</div>;
  }
  return (
    <div className="history-detail">
      <div className="history-detail-head">
        <span className={`node-method ${methodClass(detail.method)}`}>{detail.method}</span>
        <span className="history-detail-url" title={detail.url}>
          {detail.url}
        </span>
        {detail.ok ? (
          <span className={`status-badge ${statusClass(detail.status)}`}>
            {detail.status} {detail.statusText}
          </span>
        ) : (
          <span className="status-badge status-5xx">请求失败</span>
        )}
        <span className="resp-meta">
          <span>
            <span className="label">耗时 </span>
            <b>{detail.timeMs} ms</b>
          </span>
          {detail.size > 0 && (
            <span>
              <span className="label">大小 </span>
              <b>{(detail.size / 1024).toFixed(2)} KB</b>
            </span>
          )}
        </span>
      </div>
      <div className="history-detail-sections">
        <div className="history-section">
          <div className="history-section-title">请求 Headers</div>
          {detail.reqHeaders.length === 0 ? (
            <div className="history-section-empty">（无）</div>
          ) : (
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
        </div>
        <div className="history-section">
          <div className="history-section-title">请求 Body</div>
          {detail.reqBody ? (
            <pre className="history-pre">{detail.reqBody}</pre>
          ) : (
            <div className="history-section-empty">（无）</div>
          )}
        </div>
        {detail.error && (
          <div className="history-section">
            <div className="history-section-title">错误信息</div>
            <div className="error-banner">{detail.error}</div>
          </div>
        )}
        <div className="history-section">
          <div className="history-section-title">响应 Headers</div>
          {detail.respHeaders.length === 0 ? (
            <div className="history-section-empty">（无）</div>
          ) : (
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
        </div>
        <div className="history-section">
          <div className="history-section-title">响应 Body</div>
          <pre
            className="history-pre"
            dangerouslySetInnerHTML={{ __html: highlightJson(prettyBody(detail.respBody)) }}
          />
        </div>
      </div>
    </div>
  );
}
