import { GenLogItem } from "../commands";
import { useT } from "../i18n";

function fmtMs(ms: number): string {
  if (ms < 1000) return `${ms} ms`;
  return `${(ms / 1000).toFixed(2)} s`;
}

interface Props {
  records: GenLogItem[];
  loading: boolean;
  selectedId: string | null;
  onSelect: (id: string) => void;
  onReload: () => void;
}

/** 数据生成记录列表（侧边栏，布局与请求历史类似） */
export function GenLogsList({ records, loading, selectedId, onSelect, onReload }: Props) {
  const t = useT();
  return (
    <div className="genlogs-list-view">
      <div className="genlogs-list-head">
        <span className="genlogs-list-count">{t("objects.genLogsCount", { count: records.length })}</span>
        <span style={{ flex: 1 }} />
        <button className="icon-btn" onClick={onReload} title={t("history.reload")} aria-label={t("history.reload")}>
          <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor" aria-hidden="true">
            <path d="M17.65 6.35A7.96 7.96 0 0 0 12 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08A5.99 5.99 0 0 1 12 18c-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z" />
          </svg>
        </button>
      </div>
      <div className="genlogs-list-scroll">
        {loading ? (
          <div className="genlogs-empty">{t("history.loading")}</div>
        ) : records.length === 0 ? (
          <div className="genlogs-empty">{t("objects.genLogsEmpty")}</div>
        ) : (
          records.map((l) => (
            <div
              key={l.file}
              className={`genlogs-item${selectedId === l.file ? " active" : ""}`}
              onClick={() => onSelect(l.file)}
            >
              <div className="genlogs-item-title">
                <span className="genlogs-badge">{l.format.toUpperCase()}</span>
                {l.object_name}
              </div>
              <div className="genlogs-item-sub">
                {l.time_str} · {l.count} 条 · {fmtMs(l.elapsed_ms)}
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
