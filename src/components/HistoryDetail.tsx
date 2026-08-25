import { HistoryDetail as HistoryDetailType } from "../commands";
import { highlightJson } from "./Response";
import { useT } from "../i18n";

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

/** 右侧请求历史详情（请求 Headers/Body + 响应 Headers/Body；WS/SIO 记录按握手阶段/通讯阶段拆分展示） */
export function HistoryDetail({
  detail,
  loading,
}: {
  detail: HistoryDetailType | null;
  loading: boolean;
}) {
  const t = useT();
  if (loading) return <div className="history-empty">{t("history.loading")}</div>;
  if (!detail) {
    return <div className="history-empty">{t("historyDetail.emptyHint")}</div>;
  }
  const isWs = detail.method === "WS" || detail.method === "SIO";
  const connOk = detail.ok && !detail.error;
  return (
    <div className="history-detail">
      <div className="history-detail-head">
        <span className={`node-method ${methodClass(detail.method)}`}>{detail.method}</span>
        <span className="history-detail-url" title={detail.url}>
          {detail.url}
        </span>
        {detail.ok ? (
          <span className={`status-badge ${statusClass(detail.status)}`}>
            {detail.status || detail.method} {detail.statusText}
          </span>
        ) : (
          <span className="status-badge status-5xx">{t("resp.failed")}</span>
        )}
        <span className="resp-meta">
          <span>
            <span className="label">{t("resp.time")} </span>
            <b>{detail.timeMs} ms</b>
          </span>
          {detail.size > 0 && (
            <span>
              <span className="label">{t("resp.size")} </span>
              <b>{(detail.size / 1024).toFixed(2)} KB</b>
            </span>
          )}
        </span>
      </div>
      {isWs ? (
        <div className="history-detail-sections">
          <div className="history-section">
            <div className="history-section-title">
              {t("historyDetail.handshake")}
            </div>
            <div className="history-section-row">
              <span className="history-section-row-label">{t("historyDetail.connUrl")}</span>
              <span className="history-section-row-value" title={detail.url}>
                {detail.url}
              </span>
            </div>
            <div className="history-section-row">
              <span className="history-section-row-label">{t("historyDetail.connStatus")}</span>
              <span
                className={`history-section-row-value ${connOk ? "ws-ok" : "ws-bad"}`}
              >
                {connOk ? t("historyDetail.connOk") : t("historyDetail.connFail")}
              </span>
            </div>
            <div className="history-section-row">
              <span className="history-section-row-label">{t("historyDetail.roundTrip")}</span>
              <span className="history-section-row-value">{detail.timeMs} ms</span>
            </div>
            {detail.error && (
              <div className="history-section-row">
                <span className="history-section-row-label">{t("historyDetail.error")}</span>
                <span className="history-section-row-value ws-bad">{detail.error}</span>
              </div>
            )}
          </div>
          <div className="history-section">
            <div className="history-section-title">{t("historyDetail.comm")}</div>
            <div className="history-section-sub">{t("historyDetail.sendMsg")}</div>
            {detail.reqBody ? (
              <pre className="history-pre">{detail.reqBody}</pre>
            ) : (
              <div className="history-section-empty">{t("historyDetail.none")}</div>
            )}
            <div className="history-section-sub">{t("historyDetail.recvMsg")}</div>
            {detail.respBody ? (
              <pre
                className="history-pre"
                dangerouslySetInnerHTML={{ __html: highlightJson(prettyBody(detail.respBody)) }}
              />
            ) : (
              <div className="history-section-empty">{t("historyDetail.none")}</div>
            )}
          </div>
        </div>
      ) : (
      <div className="history-detail-sections">
        <div className="history-section">
          <div className="history-section-title">{t("historyDetail.reqHeaders")}</div>
          {detail.reqHeaders.length === 0 ? (
            <div className="history-section-empty">{t("historyDetail.none")}</div>
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
          <div className="history-section-title">{t("historyDetail.reqBody")}</div>
          {detail.reqBody ? (
            <pre className="history-pre">{detail.reqBody}</pre>
          ) : (
            <div className="history-section-empty">{t("historyDetail.none")}</div>
          )}
        </div>
        {detail.error && (
          <div className="history-section">
            <div className="history-section-title">{t("historyDetail.error")}</div>
            <div className="error-banner">{detail.error}</div>
          </div>
        )}
        <div className="history-section">
          <div className="history-section-title">{t("historyDetail.respHeaders")}</div>
          {detail.respHeaders.length === 0 ? (
            <div className="history-section-empty">{t("historyDetail.none")}</div>
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
          <div className="history-section-title">{t("historyDetail.respBody")}</div>
          <pre
            className="history-pre"
            dangerouslySetInnerHTML={{ __html: highlightJson(prettyBody(detail.respBody)) }}
          />
        </div>
      </div>
      )}
    </div>
  );
}
