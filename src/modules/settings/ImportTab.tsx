import { REQUIRED_IMPORT_FORMATS } from "../../types";
import { useT } from "../../i18n";
import { FormatIcon } from "../layout/FormatSelect";
import { IMPORT_FORMATS } from "./constants";
import { SettingsSection } from "./Section";
import { Switch } from "./Switch";
import { SettingsTabProps } from "./types";

/** 导入：总开关、各格式可用性 */
export function ImportTab({ settings, patch }: SettingsTabProps) {
  const t = useT();

  return (
    <SettingsSection id="import" titleKey="settings.nav.import">
      <div className="settings-feature">
        <div className="settings-row settings-toggle-row">
          <span className="settings-label">{t("settings.importEnabled")}</span>
          <Switch
            checked={settings.importEnabled !== false}
            onChange={(v) => patch({ importEnabled: v })}
          />
        </div>
        <div className="settings-desc">{t("settings.importTypesDesc")}</div>
        <div className="settings-format-list">
          {IMPORT_FORMATS.map((f) => {
            const required = REQUIRED_IMPORT_FORMATS.includes(f.value);
            return (
              <div className="settings-format-row" key={f.value}>
                <span className="settings-format-name">
                  <FormatIcon value={f.value} className="settings-format-icon" />
                  {t(f.labelKey)}
                </span>
                {required ? (
                  <span className="settings-required">{t("settings.required")}</span>
                ) : (
                  <Switch
                    checked={settings.importTypes[f.value] !== false}
                    onChange={(v) =>
                      patch({
                        importTypes: { ...settings.importTypes, [f.value]: v },
                      })
                    }
                  />
                )}
              </div>
            );
          })}
        </div>
      </div>
    </SettingsSection>
  );
}
