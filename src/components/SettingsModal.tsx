import { useEffect, useState } from "react";
import { AppSettings, ExportFormat, ImportFormat, REQUIRED_EXPORT_FORMATS, REQUIRED_IMPORT_FORMATS } from "../types";
import { Modal } from "./Modal";
import { openExternal, setLanguage } from "../commands";
import { CODE_LANGS, CodeLang } from "../utils/codegen";
import { FormatIcon, FormatSelect } from "./FormatSelect";
import { LangSelect } from "./LangSelect";
import { KeyValueEditor } from "./KeyValueEditor";
import { setLang, useT } from "../i18n";
import logoUrl from "../assets/logo.png";

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

const MODES = [
  { value: "dark", labelKey: "settings.mode.dark" },
  { value: "light", labelKey: "settings.mode.light" },
  { value: "system", labelKey: "settings.mode.system" },
] as const;

const PROJECT_URL = "https://github.com/freewu/api-manager";
const ISSUE_URL = "https://github.com/freewu/api-manager/issues/new";

/** 导入格式列表（含可开关的） */
const IMPORT_FORMATS: { value: ImportFormat; labelKey: string }[] = [
  { value: "postman", labelKey: "export.postman" },
  { value: "openapi", labelKey: "export.openapi" },
  { value: "markdown", labelKey: "export.markdown" },
  { value: "apifox", labelKey: "export.apifox" },
  { value: "apipost", labelKey: "export.apipost" },
];

/** 导出格式列表（含可开关的） */
const EXPORT_FORMATS: { value: ExportFormat; labelKey: string }[] = [
  { value: "postman", labelKey: "export.postman" },
  { value: "openapi", labelKey: "export.openapi" },
  { value: "docsify", labelKey: "export.docsify" },
  { value: "markdown", labelKey: "export.markdown" },
  { value: "html", labelKey: "export.html" },
  { value: "apifox", labelKey: "export.apifox" },
  { value: "apipost", labelKey: "export.apipost" },
];

/** 左侧导航（目录）项：点击滚动到对应分区 */
const NAV = [
  { id: "workspace", icon: "📁", titleKey: "settings.nav.workspace", descKey: "settings.nav.workspaceDesc" },
  { id: "language", icon: "🌐", titleKey: "settings.nav.language", descKey: "settings.nav.languageDesc" },
  { id: "appearance", icon: "🎨", titleKey: "settings.nav.appearance", descKey: "settings.nav.appearanceDesc" },
  { id: "version", icon: "📦", titleKey: "settings.nav.version", descKey: "settings.nav.versionDesc" },
  { id: "mock", icon: "🛡️", titleKey: "settings.nav.mock", descKey: "settings.nav.mockDesc" },
  { id: "codegen", icon: "💻", titleKey: "settings.nav.codegen", descKey: "settings.nav.codegenDesc" },
  { id: "export", icon: "📤", titleKey: "settings.nav.export", descKey: "settings.nav.exportDesc" },
  { id: "import", icon: "📥", titleKey: "settings.nav.import", descKey: "settings.nav.importDesc" },
  { id: "headers", icon: "🧾", titleKey: "settings.nav.headers", descKey: "settings.nav.headersDesc" },
  { id: "sync", icon: "🔄", titleKey: "settings.nav.sync", descKey: "settings.nav.syncDesc" },
  { id: "about", icon: "ℹ️", titleKey: "settings.nav.about", descKey: "settings.nav.aboutDesc" },
] as const;

function Switch({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      type="button"
      className={`switch ${checked ? "on" : ""}`}
      onClick={() => onChange(!checked)}
    />
  );
}

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

export function SettingsModal({ settings, appVersion, vcs, workspaceName, onSaveWorkspaceName, onClose, onSave }: Props) {
  const t = useT();
  const [active, setActive] = useState<string>("appearance");
  const [wsName, setWsName] = useState(workspaceName);
  // 保存工作区名称后（props 更新）同步本地输入框
  useEffect(() => setWsName(workspaceName), [workspaceName]);
  const patch = (p: Partial<AppSettings>) => onSave({ ...settings, ...p });

  // 切换界面语言：即时生效 + 持久化 + 联动托盘
  const switchLang = (l: "zh" | "zh-tw" | "en") => {
    if (settings.language === l) return;
    setLang(l);
    patch({ language: l });
    setLanguage(l).catch(() => {});
  };

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
          <section id="settings-workspace" className="settings-section">
            <div className="settings-panel-title">{t("settings.nav.workspace")}</div>
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
          </section>

          <section id="settings-language" className="settings-section">
            <div className="settings-panel-title">{t("settings.nav.language")}</div>
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
          </section>

          <section id="settings-appearance" className="settings-section">
            <div className="settings-panel-title">{t("settings.nav.appearance")}</div>
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
          </section>

          <section id="settings-version" className="settings-section">
            <div className="settings-panel-title">{t("settings.nav.version")}</div>
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
          </section>

          <section id="settings-mock" className="settings-section">
            <div className="settings-panel-title">{t("settings.nav.mock")}</div>
            <div className="settings-feature">
              <div className="settings-feature-head">
                <span className="settings-feature-name">{t("settings.enableMock")}</span>
                <Switch
                  checked={settings.enableMock}
                  onChange={(v) => patch({ enableMock: v })}
                />
              </div>
              <div className="settings-feature-desc">{t("settings.enableMockDesc")}</div>
              <div className="settings-row settings-port-row">
                <span className="settings-label">{t("settings.mockPort")}</span>
                <input
                  className="settings-port-input"
                  type="number"
                  min={1}
                  max={65535}
                  value={settings.mockPort || 5050}
                  onChange={(e) =>
                    patch({
                      mockPort: Number(e.target.value.replace(/\D/g, "")) || 0,
                    })
                  }
                />
              </div>
            </div>
          </section>

          <section id="settings-codegen" className="settings-section">
            <div className="settings-panel-title">{t("settings.nav.codegen")}</div>
            <div className="settings-feature">
              <div className="settings-feature-head">
                <span className="settings-feature-name">{t("settings.enableCodegen")}</span>
                <Switch
                  checked={settings.enableCodegen}
                  onChange={(v) => patch({ enableCodegen: v })}
                />
              </div>
              <div className="settings-feature-desc">{t("settings.enableCodegenDesc")}</div>
              {settings.enableCodegen && (
                <div className="settings-row settings-port-row">
                  <span className="settings-label">{t("settings.codegenLang")}</span>
                  <LangSelect
                    value={settings.codegenLang as CodeLang}
                    options={CODE_LANGS}
                    onChange={(v) => patch({ codegenLang: v })}
                  />
                </div>
              )}
            </div>
          </section>

          <section id="settings-export" className="settings-section">
            <div className="settings-panel-title">{t("settings.nav.export")}</div>
            <div className="settings-feature">
              <div className="settings-row settings-port-row">
                <span className="settings-label">{t("settings.exportFormat")}</span>
                <FormatSelect
                  className="settings-export-format"
                  value={settings.exportFormat}
                  options={EXPORT_FORMATS.map((f) => ({ value: f.value, label: t(f.labelKey) }))}
                  onChange={(v) => patch({ exportFormat: v })}
                />
                <span className="settings-desc-inline">{t("settings.exportFormatHint")}</span>
              </div>
              <div className="settings-desc">{t("settings.exportTypesDesc")}</div>
              <div className="settings-format-list">
                {EXPORT_FORMATS.map((f) => {
                  const required = REQUIRED_EXPORT_FORMATS.includes(f.value);
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
                          checked={settings.exportTypes[f.value] !== false}
                          onChange={(v) =>
                            patch({
                              exportTypes: { ...settings.exportTypes, [f.value]: v },
                            })
                          }
                        />
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          </section>

          <section id="settings-import" className="settings-section">
            <div className="settings-panel-title">{t("settings.nav.import")}</div>
            <div className="settings-feature">
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
          </section>

          <section id="settings-export-extra" className="settings-section">
            <div className="settings-panel-title">{t("settings.htmlNavTitle")}</div>
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
          </section>

          <section id="settings-headers" className="settings-section">
            <div className="settings-panel-title">{t("settings.nav.headers")}</div>
            <div className="settings-feature">
              <div className="settings-feature-head">
                <span className="settings-feature-name">{t("settings.enableDefaultHeaders")}</span>
                <Switch
                  checked={settings.enableDefaultHeaders}
                  onChange={(v) => patch({ enableDefaultHeaders: v })}
                />
              </div>
              <div className="settings-feature-desc">{t("settings.enableDefaultHeadersDesc")}</div>
              {settings.enableDefaultHeaders && (
                <div className="settings-kv-wrap">
                  <KeyValueEditor
                    rows={settings.defaultHeaders}
                    onChange={(rows) => patch({ defaultHeaders: rows })}
                    makeRow={() => ({ enabled: true, key: "", value: "", description: "" })}
                  />
                </div>
              )}
            </div>
          </section>

          {vcs && (
            <section id="settings-sync" className="settings-section">
              <div className="settings-panel-title">{t("settings.nav.sync")}</div>
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
            </section>
          )}

          <section id="settings-about" className="settings-section">
            <div className="settings-panel-title">{t("settings.nav.about")}</div>
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
          </section>
        </div>
      </div>
    </Modal>
  );
}
