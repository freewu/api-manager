import { useT } from "../../i18n";
import { SettingsSection } from "./Section";
import { Switch } from "./Switch";
import { SettingsTabProps } from "./types";

/** 版本：接口版本管理开关 */
export function VersionTab({ settings, patch }: SettingsTabProps) {
  const t = useT();

  return (
    <SettingsSection id="version" titleKey="settings.nav.version">
      <div className="settings-feature">
        <div className="settings-feature-head">
          <span className="settings-feature-name">{t("settings.enableVersion")}</span>
          <Switch
            checked={settings.enableVersion}
            onChange={(v) => patch({ enableVersion: v })}
          />
        </div>
        <div className="settings-feature-desc">{t("settings.enableVersionDesc")}</div>
      </div>
    </SettingsSection>
  );
}
