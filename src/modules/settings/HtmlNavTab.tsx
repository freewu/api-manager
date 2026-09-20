import { useT } from "../../i18n";
import { SettingsSection } from "./Section";
import { SettingsTabProps } from "./types";

/** HTML 导出文档导航（关闭 / 左侧 / 右侧） */
export function HtmlNavTab({ settings, patch }: SettingsTabProps) {
  const t = useT();

  return (
    <SettingsSection id="export-extra" titleKey="settings.htmlNavTitle">
      <div className="settings-feature">
        <div className="settings-row">
          <span className="settings-label">{t("settings.htmlNav")}</span>
          <div className="settings-options">
            <label
              className={`settings-option ${settings.htmlNav === "off" ? "active" : ""}`}
            >
              <input
                type="radio"
                name="htmlNav"
                checked={settings.htmlNav === "off"}
                onChange={() => patch({ htmlNav: "off" })}
              />
              {t("settings.htmlNavOff")}
            </label>
            <label
              className={`settings-option ${settings.htmlNav === "left" ? "active" : ""}`}
            >
              <input
                type="radio"
                name="htmlNav"
                checked={settings.htmlNav === "left"}
                onChange={() => patch({ htmlNav: "left" })}
              />
              {t("settings.htmlNavLeft")}
            </label>
            <label
              className={`settings-option ${settings.htmlNav === "right" ? "active" : ""}`}
            >
              <input
                type="radio"
                name="htmlNav"
                checked={settings.htmlNav === "right"}
                onChange={() => patch({ htmlNav: "right" })}
              />
              {t("settings.htmlNavRight")}
            </label>
          </div>
        </div>
        <div className="settings-desc">{t("settings.htmlNavHint")}</div>
      </div>
    </SettingsSection>
  );
}
