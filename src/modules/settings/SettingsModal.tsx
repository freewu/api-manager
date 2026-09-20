import { useEffect, useState } from "react";
import { AppSettings } from "../../types";
import { useT } from "../../i18n";
import { Modal } from "../layout/Modal";
import MockEditorModal from "../api/common/MockEditorModal";
import { AboutTab } from "./AboutTab";
import { AppearanceTab } from "./AppearanceTab";
import { CodegenTab } from "./CodegenTab";
import { NAV } from "./constants";
import { ExportTab } from "./ExportTab";
import { HeadersTab } from "./HeadersTab";
import { HtmlNavTab } from "./HtmlNavTab";
import { ImportTab } from "./ImportTab";
import { LanguageTab } from "./LanguageTab";
import { MockTab } from "./MockTab";
import { SyncTab } from "./SyncTab";
import { useCustomMocks } from "./useCustomMocks";
import { VersionTab } from "./VersionTab";
import { WorkspaceTab } from "./WorkspaceTab";

interface Props {
  settings: AppSettings;
  appVersion: string;
  /** 工作目录版本控制类型（.git / .svn），为空时不显示「同步远程」设置 */
  vcs?: "git" | "svn" | null;
  /** 当前工作区名称（根 __info.json 的 name，无则取目录名） */
  workspaceName: string;
  /** 保存工作区名称（写入根 __info.json） */
  onSaveWorkspaceName: (name: string) => Promise<void>;
  onClose: () => void;
  onSave: (s: AppSettings) => void;
}

/** 设置弹窗：左侧目录导航 + 右侧各 Tab 分区（左侧点击滚动、滚动联动高亮） */
export function SettingsModal({ settings, appVersion, vcs, workspaceName, onSaveWorkspaceName, onClose, onSave }: Props) {
  const t = useT();
  const [active, setActive] = useState<string>("appearance");
  const mocks = useCustomMocks();

  const patch = (p: Partial<AppSettings>) => onSave({ ...settings, ...p });

  // 点击导航 -> 平滑滚动到对应分区
  const scrollTo = (id: string) => {
    setActive(id);
    document.getElementById("settings-" + id)?.scrollIntoView({ behavior: "smooth", block: "start" });
  };

  // 滚动监听：视口顶部附近的分区高亮对应导航项（scroll-spy）
  useEffect(() => {
    const panel = document.getElementById("settings-panel");
    if (!panel) return;
    const ids = NAV.filter((n) => n.id !== "sync" || vcs).map((n) => "settings-" + n.id);
    const obs = new IntersectionObserver(
      (entries) => {
        for (const e of entries) {
          if (e.isIntersecting) setActive(e.target.id.replace("settings-", ""));
        }
      },
      { root: panel, rootMargin: "-5% 0px -85% 0px" }
    );
    for (const id of ids) {
      const el = document.getElementById(id);
      if (el) obs.observe(el);
    }
    return () => obs.disconnect();
  }, [vcs]);

  const navItems = NAV.filter((n) => n.id !== "sync" || vcs);

  return (
    <Modal
      title={t("settings.title")}
      onClose={onClose}
      className="modal-settings"
      noContextMenu
      footer={<span className="settings-auto-hint">⚡ {t("settings.autoHint")}</span>}
    >
      <div className="settings-layout">
        <div className="settings-nav">
          {navItems.map((n) => (
            <div
              key={n.id}
              className={`settings-nav-item ${active === n.id ? "active" : ""}`}
              onClick={() => scrollTo(n.id)}
            >
              <span className="settings-nav-icon">{n.icon}</span>
              <span className="settings-nav-text">
                <span className="settings-nav-title">{t(n.titleKey)}</span>
                <span className="settings-nav-desc">{t(n.descKey)}</span>
              </span>
            </div>
          ))}
        </div>

        <div className="settings-panel" id="settings-panel">
          <WorkspaceTab
            settings={settings}
            patch={patch}
            workspaceName={workspaceName}
            onSaveWorkspaceName={onSaveWorkspaceName}
          />
          <LanguageTab settings={settings} patch={patch} />
          <AppearanceTab settings={settings} patch={patch} />
          <VersionTab settings={settings} patch={patch} />
          <MockTab
            settings={settings}
            patch={patch}
            customMocks={mocks.customMocks}
            onAdd={() => mocks.setEditor({ initial: null })}
            onEdit={(m) => mocks.setEditor({ initial: m })}
            onToggle={mocks.toggle}
            onDelete={mocks.remove}
          />
          <CodegenTab settings={settings} patch={patch} />
          <ExportTab settings={settings} patch={patch} />
          <ImportTab settings={settings} patch={patch} />
          <HtmlNavTab settings={settings} patch={patch} />
          <HeadersTab settings={settings} patch={patch} />
          {vcs && <SyncTab settings={settings} patch={patch} vcs={vcs} />}
          <AboutTab appVersion={appVersion} />
        </div>
      </div>
      {mocks.editor && (
        <MockEditorModal
          initial={mocks.editor.initial}
          existingNames={mocks.customMocks.map((m) => m.name)}
          onSave={mocks.save}
          onClose={() => mocks.setEditor(null)}
          t={t}
        />
      )}
    </Modal>
  );
}
