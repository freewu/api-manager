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

/** 历史记录的协议类型（GraphQL 历史记录的 method 为 POST，并入 HTTP） */
function protoOf(r: HistorySummary): string {
  if (r.method === "WS") return "websocket";
  if (r.method === "SIO") return "socketio";
  return "http";
}

/** 高级搜索可选的协议类型（历史记录可区分：WebSocket / Socket.IO / HTTP） */
const HIST_PROTOCOL_OPTIONS = [
  { id: "http", label: "HTTP" },
  { id: "websocket", label: "WebSocket" },
  { id: "socketio", label: "Socket.IO" },
] as const;

/** 高级搜索可选的 Method（WS / SIO 记录无 Method） */
const HIST_METHOD_OPTIONS = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

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
  /** URL 模糊查询 */
  const [filter, setFilter] = useState("");
  /** 高级搜索：是否展开过滤面板 */
  const [advOpen, setAdvOpen] = useState(false);
  /** 高级搜索：按协议类型多选过滤（空数组 = 不过滤） */
  const [protocolFilters, setProtocolFilters] = useState<string[]>([]);
  /** 高级搜索：按 Method 多选过滤（空数组 = 不过滤） */
  const [methodFilters, setMethodFilters] = useState<string[]>([]);

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
  // URL 模糊查询 + 协议类型 / Method 高级搜索过滤
  const groups = useMemo(() => {
    const q = filter.trim().toLowerCase();
    const filterUuid =
      diffIds.length >= 1 ? records.find((x) => x.id === diffIds[0])?.apiUuid : undefined;
    const map = new Map<string, HistorySummary[]>();
    for (const r of records) {
      if (filterUuid && r.apiUuid !== filterUuid) continue;
      if (q && !r.url.toLowerCase().includes(q)) continue;
      if (protocolFilters.length > 0 && !protocolFilters.includes(protoOf(r))) continue;
      if (methodFilters.length > 0 && !methodFilters.includes(r.method)) continue;
      const day = fmtDay(r.time);
      if (!map.has(day)) map.set(day, []);
      map.get(day)!.push(r);
    }
    return [...map.entries()];
  }, [records, diffIds, filter, protocolFilters, methodFilters]);

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
        <span style={{ flex: 1 }} />        {diffMode ? (
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
      {/* URL 模糊搜索 + 高级搜索（协议类型 / Method） */}
      <div className="history-search-row">
        <div className="search-box">
          <span className="icon">🔍</span>
          <input
            placeholder={t("history.search")}
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            spellCheck={false}
          />
          {filter && (
            <button
              className="search-clear"
              onClick={() => setFilter("")}
              title={t("common.clear")}
              aria-label={t("common.clear")}
            >
              <svg viewBox="0 0 24 24" width="13" height="13" fill="currentColor" aria-hidden="true">
                <path d="M19 6.41 17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z" />
              </svg>
            </button>
          )}
          <button
            className={`search-adv-toggle${advOpen ? " on" : ""}`}
            onClick={() => setAdvOpen((s) => !s)}
            title={t("sidebar.advSearch")}
            aria-label={t("sidebar.advSearch")}
          >
            <svg viewBox="0 0 24 24" width="13" height="13" fill="currentColor" aria-hidden="true">
              <path d="M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58a.49.49 0 0 0 .12-.61l-1.92-3.32a.49.49 0 0 0-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54a.484.484 0 0 0-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58a.49.49 0 0 0-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z" />
            </svg>
          </button>
        </div>
      </div>
      {advOpen && (
        <div className="adv-search">
          <div className="adv-search-head">
            <span className="adv-search-head-title">{t("sidebar.advSearch")}</span>
            {(filter !== "" || protocolFilters.length > 0 || methodFilters.length > 0) && (
              <button
                className="adv-clear"
                onClick={() => {
                  setFilter("");
                  setProtocolFilters([]);
                  setMethodFilters([]);
                }}
              >
                {t("history.clearFilters")}
              </button>
            )}
          </div>
          <div className="adv-search-title">{t("sidebar.advProtocolType")}</div>
          <div className="adv-methods">
            {HIST_PROTOCOL_OPTIONS.map((p) => {
              const on = protocolFilters.includes(p.id);
              return (
                <label key={p.id} className={`adv-method${on ? " on" : ""}`}>
                  <input
                    type="checkbox"
                    checked={on}
                    onChange={() =>
                      setProtocolFilters((prev) =>
                        on ? prev.filter((x) => x !== p.id) : [...prev, p.id]
                      )
                    }
                  />
                  {p.label}
                </label>
              );
            })}
          </div>
          <div className="adv-search-title">{t("sidebar.advMethodType")}</div>
          <div className="adv-methods">
            {HIST_METHOD_OPTIONS.map((m) => {
              const on = methodFilters.includes(m);
              return (
                <label key={m} className={`adv-method${on ? " on" : ""}`}>
                  <input
                    type="checkbox"
                    checked={on}
                    onChange={() =>
                      setMethodFilters((prev) =>
                        on ? prev.filter((x) => x !== m) : [...prev, m]
                      )
                    }
                  />
                  {m}
                </label>
              );
            })}
          </div>
        </div>
      )}
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
