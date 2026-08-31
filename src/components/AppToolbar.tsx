import { useEffect, useRef, useState } from "react";
import { useT } from "../i18n";
import { AppSettings, EnvStore, MockStatus, UpdateInfo } from "../types";
import logoUrl from "../assets/logo.png";

/**
 * 顶栏：工作目录切换 / 最近目录 / 环境切换 / Mock 开关 / 刷新。
 * workspace 为 null 时渲染落地页（开始页）的简约顶栏。
 */
interface AppToolbarProps {
  workspace: string | null;
  version: string;
  updateInfo: UpdateInfo | null;
  recent: string[];
  recentLimit: number;
  envs: EnvStore;
  mock: MockStatus;
  settings: AppSettings;
  onPickWorkspace: () => void;
  onOpenRecent: (ws: string) => void;
  onSwitchEnv: (name: string) => void;
  onOpenEnvValue: () => void;
  onOpenEnvModal: () => void;
  onToggleMock: () => void;
  onRefresh: () => void;
  onOpenUpdate: () => void;
  onToast: (msg: string) => void;
}

export function AppToolbar({
  workspace,
  version,
  updateInfo,
  recent,
  recentLimit,
  envs,
  mock,
  settings,
  onPickWorkspace,
  onOpenRecent,
  onSwitchEnv,
  onOpenEnvValue,
  onOpenEnvModal,
  onToggleMock,
  onRefresh,
  onOpenUpdate,
  onToast,
}: AppToolbarProps) {
  const t = useT();
  const [showRecent, setShowRecent] = useState(false);
  const recentBtnRef = useRef<HTMLDivElement | null>(null);

  // 点击「最近目录」按钮外部时关闭下拉
  useEffect(() => {
    const closeRecent = (e: MouseEvent) => {
      if (recentBtnRef.current && !recentBtnRef.current.contains(e.target as Node)) {
        setShowRecent(false);
      }
    };
    document.addEventListener("mousedown", closeRecent);
    return () => document.removeEventListener("mousedown", closeRecent);
  }, []);

  const activeEnv = envs.environments.find((e) => e.name === envs.active);

  return (
    <div className="toolbar">
      <div className="logo">
        <img className="logo-img" src={logoUrl} alt="API Manager" />
        <span>API Manager</span>
        {updateInfo?.hasUpdate && (
          <button className="logo-update-badge" title={t("update.badgeTip")} onClick={onOpenUpdate}>
            {t("update.badge")}
          </button>
        )}
      </div>
      {!workspace ? (
        <>
          <div className="toolbar-spacer" />
          <span style={{ color: "var(--text-faint)", fontSize: 12 }}>
            {t("start.tagline")} · v{version}
          </span>
        </>
      ) : (
        <>
          <div className="workspace-chip" title={t("toolbar.workspaceTip")} onClick={onPickWorkspace}>
            📁 {workspace}
          </div>
          <div className="toolbar-spacer" />
          <div className="env-box">
            <div className="recent-btn-wrap" ref={recentBtnRef}>
              <button
                className={`btn ${showRecent ? "active" : ""}`}
                title={t("toolbar.recent")}
                onClick={() => setShowRecent((s) => !s)}
              >
                🕘
              </button>
              {showRecent && (
                <div className="recent-dropdown">
                  <div className="recent-dropdown-title">{t("toolbar.recentTitle")}</div>
                  {recent.length === 0 ? (
                    <div className="recent-dropdown-empty">{t("toolbar.recentEmpty")}</div>
                  ) : (
                    recent.slice(0, recentLimit).map((p) => (
                      <button
                        key={p}
                        className="recent-dropdown-item"
                        title={p}
                        onClick={() => {
                          setShowRecent(false);
                          onOpenRecent(p);
                        }}
                      >
                        📁 {p}
                      </button>
                    ))
                  )}
                </div>
              )}
            </div>
            <span style={{ fontSize: 12, color: "var(--text-dim)" }}>{t("toolbar.env")}</span>
            <select
              className="env-select"
              value={envs.active || ""}
              onChange={(e) => onSwitchEnv(e.target.value)}
              title={t("toolbar.envTip")}
            >
              <option value="">{t("toolbar.noEnv")}</option>
              {envs.environments.map((e) => (
                <option key={e.name} value={e.name}>
                  {e.name}
                </option>
              ))}
            </select>
            <button
              className="btn"
              disabled={!activeEnv}
              title={activeEnv ? t("toolbar.envManageTip", { name: activeEnv.name }) : t("toolbar.envPickTip")}
              onClick={onOpenEnvValue}
            >
              📋
            </button>
            <button className="btn" title={t("toolbar.envsTip")} onClick={onOpenEnvModal}>
              🌐
            </button>
          </div>
          {settings.enableMock && (
            <div className="mock-box">
              <span style={{ fontSize: 12, color: "var(--text-dim)" }}>
                {t("toolbar.mockLabel", { port: settings.mockPort })}
              </span>
              <button
                className={`switch ${mock.running ? "on" : ""}`}
                onClick={onToggleMock}
                title={t("toolbar.mockToggleTip", { port: settings.mockPort })}
              />
              <span className="mock-status">
                {mock.running ? t("toolbar.mockRunning", { count: mock.routeCount }) : t("toolbar.mockStopped")}
              </span>
              {mock.running && mock.url && (
                <button
                  className="btn mock-copy-btn"
                  onClick={() => {
                    // 默认复制 /mock-list 接口地址（列出所有 mock 路由）
                    void navigator.clipboard
                      .writeText(`${mock.url}/mock-list`)
                      .then(() => onToast(t("toolbar.mockAddrCopied")))
                      .catch(() => onToast(t("toolbar.mockAddrCopyFailed")));
                  }}
                  title={t("toolbar.mockAddrTip", { url: mock.url })}
                >
                  📋
                </button>
              )}
            </div>
          )}
          <button className="btn" onClick={onRefresh} title={t("toolbar.refresh")}>
            🔄
          </button>
        </>
      )}
    </div>
  );
}
