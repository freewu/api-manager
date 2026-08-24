import { useState } from "react";
import { GenLogItem, openPath } from "../commands";
import { useT } from "../i18n";

function fmtMs(ms: number): string {
  if (ms < 1000) return `${ms} ms`;
  return `${(ms / 1000).toFixed(2)} s`;
}

interface Props {
  detail: GenLogItem | null;
  /** 重新生成：用记录配置重新打开数据生成弹窗 */
  onRegen: (rec: GenLogItem) => void;
}

/** 数据生成记录详情（右侧，布局与请求历史类似） */
export function GenLogsDetail({ detail, onRegen }: Props) {
  const t = useT();
  const [dirErr, setDirErr] = useState("");
  const openDir = async () => {
    setDirErr("");
    try {
      await openPath(detail!.dir);
    } catch (e) {
      setDirErr(String(e));
    }
  };
  if (!detail) {
    return <div className="genlogs-empty genlogs-detail-empty">{t("objects.genLogsEmpty")}</div>;
  }
  return (
    <div className="genlogs-detail">
      <div className="genlogs-detail-head">
        <span className="genlogs-badge">{detail.format.toUpperCase()}</span>
        <span className="genlogs-detail-name">{detail.object_name}</span>
        <span className="genlogs-detail-file">{detail.file}</span>
        <span style={{ flex: 1 }} />
        <button className="btn small" onClick={() => void openDir()}>
          📂 {t("objects.genLogsOpenDir")}
        </button>
        <button className="btn small primary" onClick={() => onRegen(detail)}>
          🔁 {t("objects.genLogsRegen")}
        </button>
      </div>
      {dirErr && <div className="genlogs-direrr">{t("objects.genLogsOpenDirFail", { err: dirErr })}</div>}
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
            <th>{t("objects.genDataDesc")}</th>
            <th>{t("objects.genDataMock")}</th>
            <th>{t("objects.genDataEnabled")}</th>
          </tr>
        </thead>
        <tbody>
          {detail.props.map((p, i) => (
            <tr key={i} className={p.enabled ? "" : "gen-row-off"}>
              <td>{p.key}</td>
              <td>{p.kind}</td>
              <td className="genlogs-desc">{p.desc || "—"}</td>
              <td>{p.mock || "—"}</td>
              <td>{p.enabled ? "✓" : "—"}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
