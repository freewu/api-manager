import { GenLogItem } from "../commands";
import { useT } from "../i18n";

function fmtMs(ms: number): string {
  if (ms < 1000) return `${ms} ms`;
  return `${(ms / 1000).toFixed(2)} s`;
}

/** 文件大小人性化展示：B / KB / MB / GB */
export function fmtSize(bytes: number): string {
  if (!bytes || bytes < 0) return "0 B";
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let v = bytes / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v >= 100 ? Math.round(v) : v.toFixed(1)} ${units[i]}`;
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
    <div
      className="genlogs-list-view"
      onContextMenu={(e) => {
        // 数据生成记录左侧禁止右键
        const target = e.target as HTMLElement;
        if (target.tagName === "INPUT" || target.tagName === "TEXTAREA") return;
        e.preventDefault();
      }}
    >
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
                {l.time_str} · {l.count.toLocaleString()} 条 · {fmtMs(l.elapsed_ms)}
                {l.file_size ? ` · ${fmtSize(l.file_size)}` : ""}
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
