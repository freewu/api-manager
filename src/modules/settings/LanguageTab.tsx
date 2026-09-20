import { setLanguage } from "../../commands";
import { setLang, useT } from "../../i18n";
import { SettingsSection } from "./Section";
import { SettingsTabProps } from "./types";

/** 语言：界面语言切换（即时生效 + 持久化 + 联动托盘） */
export function LanguageTab({ settings, patch }: SettingsTabProps) {
  const t = useT();

  // 切换界面语言：即时生效 + 持久化 + 联动托盘
  const switchLang = (l: "zh" | "zh-tw" | "en") => {
    if (settings.language === l) return;
    setLang(l);
    patch({ language: l });
    setLanguage(l).catch(() => {});
  };

  return (
    <SettingsSection id="language" titleKey="settings.nav.language">
      <div className="settings-row">
        <span className="settings-label">{t("settings.languageTip")}</span>
        <div className="settings-options">
          <label
            className={`settings-option ${settings.language === "zh" ? "active" : ""}`}
          >
            <input
              type="radio"
              name="language"
              checked={settings.language === "zh"}
              onChange={() => switchLang("zh")}
            />
            {t("settings.lang.zh")}
          </label>
          <label
            className={`settings-option ${settings.language === "zh-tw" ? "active" : ""}`}
          >
            <input
              type="radio"
              name="language"
              checked={settings.language === "zh-tw"}
              onChange={() => switchLang("zh-tw")}
            />
            {t("settings.lang.zhTw")}
          </label>
          <label
            className={`settings-option ${settings.language === "en" ? "active" : ""}`}
          >
            <input
              type="radio"
              name="language"
              checked={settings.language === "en"}
              onChange={() => switchLang("en")}
            />
            {t("settings.lang.en")}
          </label>
        </div>
      </div>
      <div className="settings-desc">{t("settings.langDesc")}</div>
    </SettingsSection>
  );
}
