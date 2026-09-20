import { openExternal } from "../../commands";
import { useT } from "../../i18n";
import logoUrl from "../../assets/logo.png";
import { ISSUE_URL, PROJECT_URL } from "./constants";
import { SettingsSection } from "./Section";

interface Props {
  appVersion: string;
}

/** 关于页里的外链行 */
function LinkRow({ icon, title, desc, url }: { icon: string; title: string; desc: string; url: string }) {
  return (
    <button
      type="button"
      className="about-link"
      onClick={() => {
        void openExternal(url);
      }}
    >
      <span className="about-link-icon">{icon}</span>
      <span className="about-link-body">
        <span className="about-link-title">{title}</span>
        <span className="about-link-desc">{desc}</span>
      </span>
      <span className="about-link-arrow">↗</span>
    </button>
  );
}

/** 关于：应用信息、版本号、项目地址 / 反馈地址 */
export function AboutTab({ appVersion }: Props) {
  const t = useT();

  return (
    <SettingsSection id="about" titleKey="settings.nav.about">
      <div className="about-app">
        <div className="about-logo">
          <img src={logoUrl} alt="API Manager" style={{ width: 34, height: 34, objectFit: "contain" }} />
        </div>
        <div className="about-app-info">
          <div className="about-app-name">API Manager</div>
          <div className="about-app-desc">{t("settings.aboutDesc")}</div>
          <div className="about-version">v{appVersion || "0.1.0"}</div>
        </div>
      </div>
      <div className="about-links">
        <LinkRow icon="📦" title={t("settings.projectUrl")} desc={PROJECT_URL} url={PROJECT_URL} />
        <LinkRow icon="🐛" title={t("settings.issueUrl")} desc={t("settings.issueUrlDesc")} url={ISSUE_URL} />
      </div>
    </SettingsSection>
  );
}
