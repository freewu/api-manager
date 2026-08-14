import { useMemo, useState } from "react";
import { HistoryDay, HistorySummary } from "../commands";

function fmtTime(secs: number): string {
  const d = new Date(secs * 1000);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
}

function fmtDay(secs: number): string {
  const d = new Date(secs * 1000);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(
    d.getDate()
  ).padStart(2, "0")}`;
}

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

interface Props {
  records: HistorySummary[];
  days: HistoryDay[];
  loading: boolean;
  hasMore: boolean;
  selectedId: string | null;
  totalCount: number;
  onSelect: (id: string) => void;
  onLoadMore: () => void;
  onReload: () => void;
  onClear: () => void;
}

/** 左侧请求历史列表（替代接口树），按天分组、懒加载 */
export function HistoryList({
  records,
  days,
  loading,
  hasMore,
  selectedId,
  totalCount,
  onSelect,
  onLoadMore,
  onReload,
  onClear,
}: Props) {
  const [confirmClear, setConfirmClear] = useState(false);

  // 已加载的记录按天分组（记录本身已按时间倒序）
  const groups = useMemo(() => {
    const map = new Map<string, HistorySummary[]>();
    for (const r of records) {
      const day = fmtDay(r.time);
      if (!map.has(day)) map.set(day, []);
      map.get(day)!.push(r);
    }
    return [...map.entries()];
  }, [records]);

  const dayCount = useMemo(() => new Map(days.map((d) => [d.day, d.count])), [days]);

  return (
    <div className="history-list-side">
      <div className="history-side-toolbar">
        <span className="history-side-count">
          {totalCount > 0 ? `共 ${totalCount} 条` : "暂无记录"}
        </span>
        <span style={{ flex: 1 }} />
        <button
          className="icon-btn"
          onClick={onReload}
          disabled={loading}
          title="刷新历史"
          aria-label="刷新历史"
        >
          <svg viewBox="0 0 24 24" width="15" height="15" fill="currentColor" aria-hidden="true">
            <path d="M17.65 6.35A7.95 7.95 0 0 0 12 4a8 8 0 1 0 7.73 10h-2.08A6 6 0 1 1 12 6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z" />
          </svg>
        </button>
        <button
          className={`icon-btn ${confirmClear ? "danger" : ""}`}
          onClick={() => {
            if (confirmClear) {
              onClear();
              setConfirmClear(false);
            } else {
              setConfirmClear(true);
              window.setTimeout(() => setConfirmClear(false), 2500);
            }
          }}
          title={confirmClear ? "再次点击确认清空全部历史" : "清空全部请求历史"}
          aria-label="清空全部请求历史"
        >
          <svg viewBox="0 0 24 24" width="15" height="15" fill="currentColor" aria-hidden="true">
            <path d="M6 19a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z" />
          </svg>
        </button>
      </div>
      <div className="history-list-scroll">
        {records.length === 0 && !loading && (
          <div className="history-empty">暂无请求记录，发送请求后自动保存</div>
        )}
        {groups.map(([day, list]) => (
          <div key={day}>
            <div className="history-day">
              {day}
              {dayCount.has(day) && (
                <span className="history-day-count">{dayCount.get(day)} 条</span>
              )}
            </div>
            {list.map((r) => (
              <div
                key={r.id}
                className={`history-item ${selectedId === r.id ? "active" : ""}`}
                onClick={() => onSelect(r.id)}
                title={`${r.method} ${r.url}`}
              >
                <span className="history-time">{fmtTime(r.time)}</span>
                <span className={`node-method ${methodClass(r.method)}`}>{r.method}</span>
                {r.status > 0 ? (
                  <span className={`history-status ${statusClass(r.status)}`}>{r.status}</span>
                ) : (
                  <span className="history-status status-5xx">ERR</span>
                )}
                <span className="history-url">{r.url}</span>
              </div>
            ))}
          </div>
        ))}
        {loading && <div className="history-empty">加载中…</div>}
        {!hasMore && records.length > 0 && (
          <div className="history-empty">已全部加载（{records.length} 条）</div>
        )}
        {hasMore && !loading && (
          <button className="history-load-more" onClick={onLoadMore} disabled={loading}>
            加载更多（已加载 {records.length} 条）
          </button>
        )}
      </div>
    </div>
  );
}
