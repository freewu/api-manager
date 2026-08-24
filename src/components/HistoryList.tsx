import { useEffect, useMemo, useState } from "react";
import { HistoryDay, HistorySummary } from "../commands";
import { useT } from "../i18n";

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

function ymd(d: Date): string {
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
  /** Diff 比对模式 */
  diffMode: boolean;
  diffIds: string[];
  diffError: string;
  onToggleDiffMode: (on: boolean) => void;
  onToggleDiffSelect: (r: HistorySummary) => void;
  onStartDiff: () => void;
}

/** 左侧请求历史列表（替代接口树），按天分组、懒加载；支持 Diff 比对 */
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
  diffMode,
  diffIds,
  diffError,
  onToggleDiffMode,
  onToggleDiffSelect,
  onStartDiff,
}: Props) {
  const t = useT();
  const [confirmClear, setConfirmClear] = useState(false);

  // 比对模式下按 ESC 退出比对
  useEffect(() => {
    if (!diffMode) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onToggleDiffMode(false);
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [diffMode, onToggleDiffMode]);

  // 已加载的记录按天分组（记录本身已按时间倒序）
  // 比对模式下勾选第 1 条后（含选满 2 条）：非同一接口（apiUuid 不同）的记录全部隐藏，只保留参与比对接口的记录
  const groups = useMemo(() => {
    const filterUuid =
      diffIds.length >= 1 ? records.find((x) => x.id === diffIds[0])?.apiUuid : undefined;
    const map = new Map<string, HistorySummary[]>();
    for (const r of records) {
      if (filterUuid && r.apiUuid !== filterUuid) continue;
      const day = fmtDay(r.time);
      if (!map.has(day)) map.set(day, []);
      map.get(day)!.push(r);
    }
    return [...map.entries()];
  }, [records, diffIds]);

  const dayCount = useMemo(() => new Map(days.map((d) => [d.day, d.count])), [days]);

  /** 今天 / 昨天 / 前天 用文字提示替换日期 */
  const dayLabel = (day: string) => {
    const now = new Date();
    const today = ymd(now);
    const y = new Date(now);
    y.setDate(now.getDate() - 1);
    const by = new Date(now);
    by.setDate(now.getDate() - 2);
    if (day === today) return t("history.today");
    if (day === ymd(y)) return t("history.yesterday");
    if (day === ymd(by)) return t("history.beforeYesterday");
    return day;
  };

  return (
    <div
      className="history-list-side"
      onContextMenu={(e) => e.preventDefault()}
    >
      <div className="history-side-toolbar">
        <span className="history-side-count">
          {totalCount > 0 ? t("history.total", { count: totalCount }) : t("history.empty")}
        </span>
        <span style={{ flex: 1 }} />
        {diffMode ? (
          <>
            <button
              className={`icon-btn ${diffIds.length === 2 ? "accent" : ""}`}
              onClick={onStartDiff}
              disabled={diffIds.length !== 2}
              title={
                diffIds.length === 2 ? t("history.diffStart") : t("history.diffNeedTwo")
              }
              aria-label={t("history.diffStart")}
            >
              <svg viewBox="0 0 24 24" width="15" height="15" fill="currentColor" aria-hidden="true">
                <path d="M9.01 14H2v2h7.01v3L13 15l-3.99-4v3zm5.98-1v-3H22V8h-7.01V5L11 9l3.99 4z" />
              </svg>
            </button>
            <button
              className="icon-btn"
              onClick={() => onToggleDiffMode(false)}
              title={t("history.diffCancel")}
              aria-label={t("history.diffCancel")}
            >
              <svg viewBox="0 0 24 24" width="15" height="15" fill="currentColor" aria-hidden="true">
                <path d="M19 6.41 17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z" />
              </svg>
            </button>
          </>
        ) : (
          <button
            className="icon-btn"
            onClick={onReload}
            disabled={loading}
            title={t("history.refresh")}
            aria-label={t("history.refresh")}
          >
            <svg viewBox="0 0 24 24" width="15" height="15" fill="currentColor" aria-hidden="true">
              <path d="M17.65 6.35A7.95 7.95 0 0 0 12 4a8 8 0 1 0 7.73 10h-2.08A6 6 0 1 1 12 6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z" />
            </svg>
          </button>
        )}
        {!diffMode && (
          <button
            className="icon-btn"
            onClick={() => onToggleDiffMode(true)}
            title={t("history.diffMode")}
            aria-label={t("history.diffMode")}
          >
            <svg viewBox="0 0 24 24" width="15" height="15" fill="currentColor" aria-hidden="true">
              <path d="M9.01 14H2v2h7.01v3L13 15l-3.99-4v3zm5.98-1v-3H22V8h-7.01V5L11 9l3.99 4z" />
            </svg>
          </button>
        )}
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
          title={confirmClear ? t("history.clearConfirm") : t("history.clear")}
          aria-label={t("history.clear")}
        >
          <svg viewBox="0 0 24 24" width="15" height="15" fill="currentColor" aria-hidden="true">
            <path d="M6 19a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z" />
          </svg>
        </button>
      </div>
      {diffMode && (
        <div className={`history-diff-hint${diffError ? " error" : ""}`}>
          {diffError ? t(diffError) : t("history.diffHint", { n: diffIds.length })}
        </div>
      )}
      <div className="history-list-scroll">
        {records.length === 0 && !loading && (
          <div className="history-empty">{t("history.emptyHint")}</div>
        )}
        {groups.map(([day, list]) => (
          <div key={day}>
            <div className="history-day" title={day}>
              {dayLabel(day)}
              {dayCount.has(day) && (
                <span className="history-day-count">{dayCount.get(day)} {t("history.items")}</span>
              )}
            </div>
            {list.map((r) => {
              const checked = diffIds.includes(r.id);
              const sel = selectedId === r.id;
              return (
                <div
                  key={r.id}
                  className={`history-item ${sel ? "active" : ""} ${checked ? "diff-checked" : ""}`}
                  onClick={() => (diffMode ? onToggleDiffSelect(r) : onSelect(r.id))}
                  title={`${r.method} ${r.url}`}
                >
                  {diffMode && (
                    <span
                      className={`history-check${checked ? " on" : ""}`}
                      aria-hidden="true"
                    >
                      {checked ? "✓" : ""}
                    </span>
                  )}
                  <span className="history-time">{fmtTime(r.time)}</span>
                  {r.method === "WS" || r.method === "SIO" ? (
                    // 实时请求（WebSocket / Socket.IO）：只显示协议名称
                    <span className={`node-method proto-${r.method.toLowerCase()}`}>
                      {r.method === "WS" ? "WebSocket" : "Socket.IO"}
                    </span>
                  ) : (
                    // HTTP 请求：显示 method + 状态码
                    <>
                      <span className={`node-method ${methodClass(r.method)}`}>{r.method}</span>
                      {r.status > 0 ? (
                        <span className={`history-status ${statusClass(r.status)}`}>{r.status}</span>
                      ) : (
                        <span className="history-status status-5xx">ERR</span>
                      )}
                    </>
                  )}
                  <span className="history-url">{r.url}</span>
                </div>
              );
            })}
          </div>
        ))}
        {loading && <div className="history-empty">{t("history.loading")}</div>}
        {!hasMore && records.length > 0 && (
          <div className="history-empty">{t("history.allLoaded", { count: records.length })}</div>
        )}
        {hasMore && !loading && (
          <button className="history-load-more" onClick={onLoadMore} disabled={loading}>
            {t("history.loadMore", { count: records.length })}
          </button>
        )}
      </div>
    </div>
  );
}
