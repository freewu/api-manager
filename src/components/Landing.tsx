import { useT } from "../i18n";
import { UpdateInfo, defaultSettings } from "../types";
import { AppToolbar } from "./AppToolbar";
import logoUrl from "../assets/logo.png";

/** 未打开工作目录时的落地页（开始页）：选择目录 / 最近打开 / 目录结构说明 */
interface LandingProps {
  version: string;
  updateInfo: UpdateInfo | null;
  recent: string[];
  recentLimit: number;
  onPickWorkspace: () => void;
  onOpenRecent: (ws: string) => void;
  onOpenUpdate: () => void;
}

export function Landing({
  version,
  updateInfo,
  recent,
  recentLimit,
  onPickWorkspace,
  onOpenRecent,
  onOpenUpdate,
}: LandingProps) {
  const t = useT();
  return (
    <div className="app">
      <AppToolbar
        workspace={null}
        version={version}
        updateInfo={updateInfo}
        recent={recent}
        recentLimit={recentLimit}
        envs={{ active: "", environments: [] }}
        mock={{ running: false, routeCount: 0 }}
        settings={defaultSettings()}
        onPickWorkspace={onPickWorkspace}
        onOpenRecent={onOpenRecent}
        onSwitchEnv={() => {}}
        onOpenEnvValue={() => {}}
        onOpenEnvModal={() => {}}
        onToggleMock={() => {}}
        onRefresh={() => {}}
        onOpenUpdate={onOpenUpdate}
      />
      <div className="landing">
        <img className="landing-logo" src={logoUrl} alt="API Manager" />
        <h1>API Manager</h1>
        <p>{t("start.subtitle")}</p>
        <button className="btn primary" style={{ fontSize: 14, padding: "10px 24px" }} onClick={onPickWorkspace}>
          {t("start.chooseDir")}
        </button>
        {recent.length > 0 && (
          <div className="recent-workspaces">
            <div className="recent-title">{t("start.recent")}</div>
            {recent.slice(0, recentLimit).map((p) => (
              <button key={p} className="recent-item" title={p} onClick={() => onOpenRecent(p)}>
                📁 {p}
              </button>
            ))}
          </div>
        )}
        <div className="file-tree-note">
          {t("start.structureTitle")}：
          <br />├── __info.json &nbsp;// {t("start.treeInfo")}
          <br />├── {t("start.treeGroup")}/
          <br />│&nbsp;&nbsp;├── __info.json &nbsp;// {t("start.treeGroupDesc")}
          <br />│&nbsp;&nbsp;└── {t("start.treeApi")} &nbsp;// {t("start.treeApiDesc")}
        </div>
        <div className="hint">{t("start.hint")}</div>
      </div>
    </div>
  );
}
