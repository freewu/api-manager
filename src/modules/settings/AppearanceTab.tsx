import { useT } from "../../i18n";
import { MODES } from "./constants";
import { SettingsSection } from "./Section";
import { SettingsTabProps } from "./types";

/** 外观：主题模式（跟随系统 / 深色 / 浅色）+ 预览 */
export function AppearanceTab({ settings, patch }: SettingsTabProps) {
  const t = useT();

  return (
    <SettingsSection id="appearance" titleKey="settings.nav.appearance">
      <div className="settings-row">
        <span className="settings-label">{t("settings.displayMode")}</span>
        <div className="settings-options">
          {MODES.map((m) => (
            <label
              key={m.value}
              className={`settings-option ${settings.displayMode === m.value ? "active" : ""}`}
            >
              <input
                type="radio"
                name="displayMode"
                checked={settings.displayMode === m.value}
                onChange={() => patch({ displayMode: m.value })}
              />
              {t(m.labelKey)}
            </label>
          ))}
        </div>
      </div>

      <div className="settings-preview">
        <div className="settings-preview-title">{t("settings.preview")}</div>
        <div className="settings-preview-row">
          <span className="preview-dot preview-dot-folder">📁</span>
          <span className="preview-text">{t("settings.previewUserMgmt")}</span>
        </div>
        <div className="settings-preview-row">
          <span className="preview-dot preview-dot-api">🌐</span>
          <span className="preview-text">{t("settings.previewCreateUser")}</span>
          <span className="preview-method">GET</span>
        </div>
        <div className="settings-preview-row">
          <span className="preview-dot preview-dot-api">🌐</span>
          <span className="preview-text">{t("settings.previewOrders")}</span>
          <span className="preview-method">POST</span>
        </div>
      </div>
    </SettingsSection>
  );
}
