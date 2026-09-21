import { useState } from "react";
import { openExternal } from "../../commands";
import { useT } from "../../i18n";
import logoUrl from "../../assets/logo.png";
import { ISSUE_URL, JUSTFILE_URL, PROJECT_URL } from "./constants";
import { DEP_GROUPS } from "./deps";
import { badgeUrl, CORE_STACK, groupBadges, type StackBadge } from "./badges";
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

/** 单个技术栈徽章（shields.io）：离线 / 加载失败时自动降级为文字标签 */
function StackBadgeItem({ badge }: { badge: StackBadge }) {
  const [failed, setFailed] = useState(false);
  const alt = `${badge.label} ${badge.version}`;
  return (
    <button
      type="button"
      className={failed ? "about-dep" : "about-dep about-dep--badge"}
      title={`${alt} ↗`}
      onClick={() => {
        void openExternal(badge.url);
      }}
    >
      {failed ? (
        <>
          <span className="about-dep-name">{badge.label}</span>
          <span className="about-dep-version">{badge.version}</span>
        </>
      ) : (
        <img
          className="about-dep-badge"
          src={badgeUrl(badge)}
          alt={alt}
          loading="lazy"
          draggable={false}
          onError={() => {
            setFailed(true);
          }}
        />
      )}
    </button>
  );
}

/** 关于：应用信息、版本号、项目地址 / 反馈地址、技术栈徽章、开源组件 */
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
        <LinkRow icon="🛠️" title={t("settings.justfileTitle")} desc={t("settings.justfileDesc")} url={JUSTFILE_URL} />
      </div>
      <div className="about-deps">
        <div className="about-deps-head">
          <span className="about-deps-title">{t("settings.aboutDeps")}</span>
          <span className="about-deps-hint">{t("settings.aboutDepsHint")}</span>
        </div>
        <div className="about-deps-group about-deps-core">
          <div className="about-deps-group-title">{t("settings.aboutCoreStack")}</div>
          <div className="about-deps-list">
            {CORE_STACK.map((badge) => (
              <StackBadgeItem badge={badge} key={badge.label} />
            ))}
          </div>
        </div>
        {DEP_GROUPS.map((group) => (
          <div className="about-deps-group" key={group.titleKey}>
            <div className="about-deps-group-title">
              {t(group.titleKey)}
              <span className="about-deps-count">{group.deps.length}</span>
            </div>
            <div className="about-deps-list">
              {groupBadges(group).map((badge) => (
                <StackBadgeItem badge={badge} key={badge.label} />
              ))}
            </div>
          </div>
        ))}
      </div>
    </SettingsSection>
  );
}
