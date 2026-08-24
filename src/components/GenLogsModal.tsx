import { useEffect, useMemo, useState } from "react";
import { GenLogItem, listGenLogs } from "../commands";
import { Modal } from "./Modal";

interface Props {
  onClose: () => void;
  t: (key: string, vars?: Record<string, string | number>) => string;
}

function fmtMs(ms: number): string {
  if (ms < 1000) return `${ms} ms`;
  const s = (ms / 1000).toFixed(2);
  return `${s} s`;
}

/** 数据生成记录管理：左侧记录列表 + 右侧提交数据详情（布局与请求历史类似） */
export default function GenLogsModal({ onClose, t }: Props) {
  const [logs, setLogs] = useState<GenLogItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [selected, setSelected] = useState<string | null>(null);

  const reload = async () => {
    setLoading(true);
    try {
      const items = await listGenLogs();
      setLogs(items);
      setSelected((s) => (s && items.some((i) => i.file === s) ? s : items[0]?.file ?? null));
    } catch {
      setLogs([]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void reload();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const detail = useMemo(() => logs.find((l) => l.file === selected) ?? null, [logs, selected]);

  return (
    <Modal
      title={`📄 ${t("sidebar.genLogs")}`}
      onClose={onClose}
      className="modal-xwide genlogs-modal"
      footer={
        <button className="btn" onClick={() => void reload()}>
          🔄 {t("history.reload")}
        </button>
      }
    >
      <div className="genlogs-wrap">
        <div className="genlogs-list">
          {loading ? (
            <div className="genlogs-empty">{t("history.loading")}</div>
          ) : logs.length === 0 ? (
            <div className="genlogs-empty">{t("objects.genLogsEmpty")}</div>
          ) : (
            logs.map((l) => (
              <div
                key={l.file}
                className={`genlogs-item${selected === l.file ? " active" : ""}`}
                onClick={() => setSelected(l.file)}
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
        <div className="genlogs-detail">
          {detail ? (
            <>
              <div className="genlogs-detail-head">
                <span className="genlogs-badge">{detail.format.toUpperCase()}</span>
                <span className="genlogs-detail-name">{detail.object_name}</span>
                <span className="genlogs-detail-file">{detail.file}</span>
              </div>
              <div className="genlogs-info">
                <div className="genlogs-info-row">
                  <label>{t("objects.genLogsTime")}</label>
                  <span>{detail.time_str}</span>
                </div>
                <div className="genlogs-info-row">
                  <label>{t("objects.genLogsTable")}</label>
                  <span>{detail.table}</span>
                </div>
                <div className="genlogs-info-row">
                  <label>{t("objects.genDataCount")}</label>
                  <span>{detail.count}</span>
                </div>
                <div className="genlogs-info-row">
                  <label>{t("objects.genLogsElapsed")}</label>
                  <span>{fmtMs(detail.elapsed_ms)}</span>
                </div>
                <div className="genlogs-info-row">
                  <label>{t("objects.genLogsFile")}</label>
                  <span className="genlogs-path">{detail.file}</span>
                </div>
                <div className="genlogs-info-row">
                  <label>{t("objects.genDataDir")}</label>
                  <span className="genlogs-path">{detail.dir}</span>
                </div>
              </div>
              <div className="genlogs-props-title">{t("objects.genLogsSubmit")}</div>
              <table className="genlogs-props">
                <thead>
                  <tr>
                    <th>{t("objects.genDataKey")}</th>
                    <th>{t("objects.genDataKind")}</th>
                    <th>{t("objects.genDataMock")}</th>
                    <th>{t("objects.genDataEnabled")}</th>
                  </tr>
                </thead>
                <tbody>
                  {detail.props.map((p, i) => (
                    <tr key={i} className={p.enabled ? "" : "gen-row-off"}>
                      <td>{p.key}</td>
                      <td>{p.kind}</td>
                      <td>{p.mock || "—"}</td>
                      <td>{p.enabled ? "✓" : "—"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </>
          ) : (
            <div className="genlogs-empty">{t("objects.genLogsEmpty")}</div>
          )}
        </div>
      </div>
    </Modal>
  );
}
