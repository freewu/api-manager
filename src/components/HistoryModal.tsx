import { useCallback, useEffect, useMemo, useState } from "react";
import {
  HistoryDay,
  HistoryDetail,
  HistorySummary,
  historyClear,
  historyDays,
  historyDetail,
  historyRecords,
} from "../commands";
import { Modal } from "./Modal";
import { highlightJson } from "./Response";

const PAGE = 100;

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

function prettyBody(text: string): string {
  try {
    return JSON.stringify(JSON.parse(text), null, 2);
  } catch {
    return text;
  }
}

export function HistoryModal({ onClose }: { onClose: () => void }) {
  const [records, setRecords] = useState<HistorySummary[]>([]);
  const [days, setDays] = useState<HistoryDay[]>([]);
  const [offset, setOffset] = useState(0);
  const [loading, setLoading] = useState(false);
  const [hasMore, setHasMore] = useState(true);
  const [selected, setSelected] = useState<string | null>(null);
  const [detail, setDetail] = useState<HistoryDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [confirmClear, setConfirmClear] = useState(false);

  const loadPage = useCallback(async (start: number) => {
    setLoading(true);
    try {
      const list = await historyRecords(start, PAGE);
      setRecords((prev) => (start === 0 ? list : [...prev, ...list]));
      setOffset(start + list.length);
      setHasMore(list.length === PAGE);
    } catch (e) {
      console.error(e);
      setHasMore(false);
    } finally {
      setLoading(false);
    }
  }, []);

  const reload = useCallback(() => {
    void loadPage(0);
    historyDays().then(setDays).catch(() => {});
  }, [loadPage]);

  useEffect(() => {
    reload();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const select = async (id: string) => {
    setSelected(id);
    setDetailLoading(true);
    try {
      setDetail(await historyDetail(id));
    } catch (e) {
      console.error(e);
      setDetail(null);
    } finally {
      setDetailLoading(false);
    }
  };

  const handleClear = async () => {
    try {
      await historyClear();
      setRecords([]);
      setDays([]);
      setHasMore(false);
      setOffset(0);
      setDetail(null);
      setSelected(null);
    } catch (e) {
      console.error(e);
    }
    setConfirmClear(false);
  };

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
  const totalCount = days.reduce((s, d) => s + d.count, 0);

  return (
    <Modal
      title={`请求历史${totalCount > 0 ? `（共 ${totalCount} 条）` : ""}`}
      onClose={onClose}
      className="modal-history"
      footer={
        <>
          {hasMore && (
            <button className="btn" onClick={() => loadPage(offset)} disabled={loading}>
              {loading ? "加载中…" : `加载更多（已加载 ${records.length} 条）`}
            </button>
          )}
          <div style={{ flex: 1 }} />
          <button
            className="btn"
            onClick={reload}
            title="重新加载历史"
            disabled={loading}
          >
            🔄
          </button>
          <button
            className={`btn ${confirmClear ? "danger" : ""}`}
            onClick={() => {
              if (confirmClear) void handleClear();
              else {
                setConfirmClear(true);
                window.setTimeout(() => setConfirmClear(false), 2500);
              }
            }}
            title="清空全部请求历史"
          >
            {confirmClear ? "确认清空？" : "🗑 清空"}
          </button>
        </>
      }
    >
      <div className="history-body">
        <div className="history-list">
          {records.length === 0 && !loading && (
            <div className="history-empty">暂无请求记录</div>
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
                  className={`history-item ${selected === r.id ? "active" : ""}`}
                  onClick={() => void select(r.id)}
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
        </div>
        <div className="history-detail">
          {detailLoading && <div className="history-empty">加载中…</div>}
          {!detailLoading && !detail && (
            <div className="history-empty">点击左侧记录查看请求与响应详情</div>
          )}
          {!detailLoading && detail && (
            <>
              <div className="history-detail-head">
                <span className={`node-method ${methodClass(detail.method)}`}>
                  {detail.method}
                </span>
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
                    dangerouslySetInnerHTML={{
                      __html: highlightJson(prettyBody(detail.respBody)),
                    }}
                  />
                </div>
              </div>
            </>
          )}
        </div>
      </div>
    </Modal>
  );
}
