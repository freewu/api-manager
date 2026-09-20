import { useEffect, useState } from "react";
import { useT } from "../../i18n";
import { SettingsSection } from "./Section";
import { Switch } from "./Switch";
import { SettingsTabProps } from "./types";

interface Props extends SettingsTabProps {
  /** 当前工作区名称（根 __info.json 的 name，无则取目录名） */
  workspaceName: string;
  /** 保存工作区名称（写入根 __info.json） */
  onSaveWorkspaceName: (name: string) => Promise<void>;
}

/** 工作目录：名称、最近记录上限、分组默认展开状态 */
export function WorkspaceTab({ settings, patch, workspaceName, onSaveWorkspaceName }: Props) {
  const t = useT();
  const [wsName, setWsName] = useState(workspaceName);
  // 保存工作区名称后（props 更新）同步本地输入框
  useEffect(() => setWsName(workspaceName), [workspaceName]);

  return (
    <SettingsSection id="workspace" titleKey="settings.nav.workspace">
      <div className="settings-row settings-port-row">
        <span className="settings-label">{t("settings.wsName")}</span>
        <input
          className="settings-port-input settings-ws-input"
          value={wsName}
          placeholder={t("settings.wsName")}
          onChange={(e) => setWsName(e.target.value)}
          onBlur={() => {
            // 失焦即保存（值未变化或为空时跳过）
            const n = wsName.trim();
            if (!n || n === workspaceName) return;
            void onSaveWorkspaceName(wsName);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter") (e.target as HTMLInputElement).blur();
          }}
        />
      </div>
      <div className="settings-desc">{t("settings.wsDesc")}</div>
      <div className="settings-row settings-port-row">
        <span className="settings-label">{t("settings.recentLimit")}</span>
        <input
          className="settings-port-input"
          type="number"
          min={3}
          value={settings.recentLimit}
          onChange={(e) =>
            patch({ recentLimit: Math.max(3, Number(e.target.value.replace(/\D/g, "")) || 3) })
          }
        />
        <span className="settings-min-hint">{t("settings.recentLimitMin")}</span>
      </div>
      <div className="settings-desc">{t("settings.recentLimitDesc")}</div>
      <div className="settings-feature">
        <div className="settings-feature-head">
          <span className="settings-feature-name">{t("settings.groupStateTip")}</span>
          <Switch
            checked={settings.defaultFolderState === "expanded"}
            onChange={(v) => patch({ defaultFolderState: v ? "expanded" : "collapsed" })}
          />
        </div>
        <div className="settings-feature-desc">{t("settings.groupStateDesc")}</div>
      </div>
    </SettingsSection>
  );
}
