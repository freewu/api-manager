import { useT } from "../../i18n";
import { SettingsSection } from "./Section";
import { Switch } from "./Switch";
import { SettingsTabProps } from "./types";

interface Props extends SettingsTabProps {
  /** 工作目录版本控制类型（有值时才会渲染本分区） */
  vcs: "git" | "svn";
}

/** 同步：工作目录版本控制的远程同步开关（仅 git / svn 工作目录显示） */
export function SyncTab({ settings, patch, vcs }: Props) {
  const t = useT();

  return (
    <SettingsSection id="sync" titleKey="settings.nav.sync">
      <div className="settings-feature">
        <div className="settings-feature-head">
          <span className="settings-feature-name">
            {t("settings.syncRemote", { vcs: vcs === "git" ? "Git" : "SVN" })}
          </span>
          <Switch
            checked={settings.syncRemote}
            onChange={(v) => patch({ syncRemote: v })}
          />
        </div>
        <div className="settings-feature-desc">{t("settings.syncRemoteDesc", { vcs: vcs === "git" ? ".git" : ".svn" })}</div>
      </div>
    </SettingsSection>
  );
}
