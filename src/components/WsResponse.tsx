import { useEffect, useRef } from "react";
import { WsLogEntry } from "../types";
import { useT } from "../i18n";

interface Props {
  /** 已连接 */
  connected: boolean;
  /** 正在连接 */
  connecting: boolean;
  /** WebSocket 交互记录（发送 / 接收 / 连接事件 / 错误） */
  entries: WsLogEntry[];
  onDisconnect: () => void;
}

function fmtTime(ms: number) {
  const d = new Date(ms);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
}

/** 展示 WebSocket 接口的实时交互记录（连接、发送、接收、错误） */
export function WsResponse({ connected, connecting, entries, onDisconnect }: Props) {
  const t = useT();
  const logRef = useRef<HTMLDivElement | null>(null);

  // 新消息自动滚动到底部
  useEffect(() => {
    if (logRef.current) logRef.current.scrollTop = logRef.current.scrollHeight;
  }, [entries.length]);

  return (
    <div className="response ws-response">
      <div className="response-head">
        <span className={`status-badge ${connected ? "status-2xx" : connecting ? "status-3xx" : ""}`}>
          {connected ? t("resp.wsConnected") : connecting ? t("resp.wsConnecting") : t("resp.wsEvent")}
        </span>
        {connected && (
          <button type="button" className="btn small" onClick={onDisconnect}>
            {t("resp.wsDisconnect")}
          </button>
        )}
      </div>
      <div className="response-body">
        <div className="resp-tabs">
          <div className="resp-tab active">WebSocket</div>
        </div>
        {entries.length === 0 ? (
          <div className="response-empty">
            <span>{t("resp.wsEmpty")}</span>
          </div>
        ) : (
          <div className="ws-log" ref={logRef}>
            {entries.map((e, i) => (
              <div key={i} className={`ws-log-item ws-${e.dir}`}>
                <span className="ws-log-time">{fmtTime(e.time)}</span>
                <span className={`ws-log-badge ws-badge-${e.dir}`}>
                  {e.dir === "sent" ? t("resp.wsSend") : e.dir === "recv" ? t("resp.wsReceive") : t("resp.wsEvent")}
                </span>
                <pre className="ws-log-text">{e.text || "␀"}</pre>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
