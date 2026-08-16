import { useEffect, useState } from "react";
import { AppSettings } from "../types";
import { Modal } from "./Modal";
import { openExternal } from "../commands";
import { CODE_LANGS } from "../utils/codegen";
import { KeyValueEditor } from "./KeyValueEditor";
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
  { value: "dark", label: "🌙 深色" },
  { value: "light", label: "☀️ 浅色" },
  { value: "system", label: "🖥 跟随系统" },
] as const;

const PROJECT_URL = "https://github.com/freewu/api-manager";
const ISSUE_URL = "https://github.com/freewu/api-manager/issues/new";

/** 左侧导航（目录）项：点击滚动到对应分区 */
const NAV = [
  { id: "workspace", icon: "📁", title: "工作区", desc: "名称 · 路径" },
  { id: "appearance", icon: "🎨", title: "外观", desc: "显示模式 · 预览" },
  { id: "version", icon: "📦", title: "接口版本", desc: "版本快照开关" },
  { id: "mock", icon: "🛡️", title: "Mock 服务", desc: "本地 Mock · 端口" },
  { id: "codegen", icon: "💻", title: "代码生成", desc: "20 种语言请求代码" },
  { id: "export", icon: "📤", title: "导出", desc: "默认导出格式" },
  { id: "headers", icon: "🧾", title: "默认 Header", desc: "新接口自动附带请求头" },
  { id: "sync", icon: "🔄", title: "同步远程", desc: "Git / SVN 远程同步" },
  { id: "about", icon: "ℹ️", title: "关于", desc: "版本与项目信息" },
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
  const [active, setActive] = useState<string>("appearance");
  const [wsName, setWsName] = useState(workspaceName);
  // 保存工作区名称后（props 更新）同步本地输入框
  useEffect(() => setWsName(workspaceName), [workspaceName]);
  const [wsSaving, setWsSaving] = useState(false);
  const patch = (p: Partial<AppSettings>) => onSave({ ...settings, ...p });

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
      title="设置"
      onClose={onClose}
      className="modal-settings"
      footer={<span className="settings-auto-hint">⚡ 修改即时生效，无需保存</span>}
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
                <span className="settings-nav-title">{n.title}</span>
                <span className="settings-nav-desc">{n.desc}</span>
              </span>
            </div>
          ))}
        </div>

        <div className="settings-panel" id="settings-panel">
          <section id="settings-workspace" className="settings-section">
            <div className="settings-panel-title">工作区</div>
            <div className="settings-row settings-port-row">
              <span className="settings-label">工作区名称</span>
              <input
                className="settings-port-input settings-ws-input"
                value={wsName}
                placeholder="工作区名称"
                onChange={(e) => setWsName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void onSaveWorkspaceName(wsName).finally(() => setWsSaving(false));
                }}
              />
              <button
                className="btn"
                disabled={wsSaving || !wsName.trim() || wsName.trim() === workspaceName}
                onClick={() => {
                  setWsSaving(true);
                  void onSaveWorkspaceName(wsName).finally(() => setWsSaving(false));
                }}
              >
                {wsSaving ? "保存中…" : "保存名称"}
              </button>
            </div>
            <div className="settings-desc">
              工作区名称即根目录 __info.json 的 name，显示于应用各处；不设置时使用目录名
            </div>
          </section>

          <section id="settings-appearance" className="settings-section">
            <div className="settings-panel-title">外观</div>
            <div className="settings-row">
              <span className="settings-label">显示模式</span>
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
                    {m.label}
                  </label>
                ))}
              </div>
            </div>
            <div className="settings-desc">深色 / 浅色 / 跟随系统（Windows 主题自动切换）</div>

            <div className="settings-preview">
              <div className="settings-preview-title">预览</div>
              <div className="settings-preview-row">
                <span className="preview-dot preview-dot-folder">📁</span>
                <span className="preview-text">用户管理</span>
              </div>
              <div className="settings-preview-row">
                <span className="preview-dot preview-dot-api">🌐</span>
                <span className="preview-text">创建用户</span>
                <span className="preview-method">GET</span>
              </div>
              <div className="settings-preview-row">
                <span className="preview-dot preview-dot-api">🌐</span>
                <span className="preview-text">获取订单列表</span>
                <span className="preview-method">POST</span>
              </div>
            </div>
          </section>

          <section id="settings-version" className="settings-section">
            <div className="settings-panel-title">接口版本</div>
            <div className="settings-feature">
              <div className="settings-feature-head">
                <span className="settings-feature-name">启用接口版本</span>
                <Switch
                  checked={settings.enableVersion}
                  onChange={(v) => patch({ enableVersion: v })}
                />
              </div>
              <div className="settings-feature-desc">
                在主页面显示「保存」按钮与右键「查看版本信息」
              </div>
            </div>
          </section>

          <section id="settings-mock" className="settings-section">
            <div className="settings-panel-title">Mock 服务</div>
            <div className="settings-feature">
              <div className="settings-feature-head">
                <span className="settings-feature-name">启用 Mock 服务</span>
                <Switch
                  checked={settings.enableMock}
                  onChange={(v) => patch({ enableMock: v })}
                />
              </div>
              <div className="settings-feature-desc">在主页面显示 Mock 开关与端口</div>
              <div className="settings-row settings-port-row">
                <span className="settings-label">Mock 端口</span>
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
                <span className="settings-desc-inline">默认 5050</span>
              </div>
            </div>
          </section>

          <section id="settings-codegen" className="settings-section">
            <div className="settings-panel-title">代码生成</div>
            <div className="settings-feature">
              <div className="settings-feature-head">
                <span className="settings-feature-name">启用代码生成</span>
                <Switch
                  checked={settings.enableCodegen}
                  onChange={(v) => patch({ enableCodegen: v })}
                />
              </div>
              <div className="settings-feature-desc">
                在编辑区显示「生成代码」页签，支持 20 种语言一键生成请求代码
              </div>
              {settings.enableCodegen && (
                <div className="settings-row settings-port-row">
                  <span className="settings-label">默认开发语言</span>
                  <select
                    className="settings-port-input codegen-lang-select"
                    value={settings.codegenLang}
                    onChange={(e) => patch({ codegenLang: e.target.value })}
                  >
                    {CODE_LANGS.map((l) => (
                      <option key={l.value} value={l.value}>
                        {l.label}
                      </option>
                    ))}
                  </select>
                  <span className="settings-desc-inline">页签默认语言</span>
                </div>
              )}
            </div>
          </section>

          <section id="settings-export" className="settings-section">
            <div className="settings-panel-title">导出</div>
            <div className="settings-feature">
              <div className="settings-row settings-port-row">
                <span className="settings-label">默认导出格式</span>
                <select
                  className="settings-port-input codegen-lang-select"
                  value={settings.exportFormat}
                  onChange={(e) => patch({ exportFormat: e.target.value as typeof settings.exportFormat })}
                >
                  <option value="postman">Postman Collection（.json）</option>
                  <option value="openapi">OpenAPI 3.0（.json）</option>
                  <option value="docsify">Docsify 文档（.md 目录）</option>
                </select>
                <span className="settings-desc-inline">导出弹窗默认选中格式</span>
              </div>
            </div>
          </section>

          <section id="settings-headers" className="settings-section">
            <div className="settings-panel-title">默认 Header</div>
            <div className="settings-feature">
              <div className="settings-feature-head">
                <span className="settings-feature-name">启用默认 Header</span>
                <Switch
                  checked={settings.enableDefaultHeaders}
                  onChange={(v) => patch({ enableDefaultHeaders: v })}
                />
              </div>
              <div className="settings-feature-desc">
                新增接口时自动附带以下请求头（勾选开关可临时停用某一条）
              </div>
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
              <div className="settings-panel-title">同步远程</div>
              <div className="settings-feature">
                <div className="settings-feature-head">
                  <span className="settings-feature-name">
                    同步远程（{vcs === "git" ? "Git" : "SVN"}）
                  </span>
                  <Switch
                    checked={settings.syncRemote}
                    onChange={(v) => patch({ syncRemote: v })}
                  />
                </div>
                <div className="settings-feature-desc">
                  已检测到工作目录 {vcs === "git" ? ".git" : ".svn"}。开启后「同步」「提交并 Push 远程」会与远程仓库交互（git pull / push、svn update / commit）；关闭则仅本地提交。
                </div>
              </div>
            </section>
          )}

          <section id="settings-about" className="settings-section">
            <div className="settings-panel-title">关于</div>
            <div className="about-app">
              <div className="about-logo">
                <img src={logoUrl} alt="API Manager" style={{ width: 34, height: 34, objectFit: "contain" }} />
              </div>
              <div className="about-app-info">
                <div className="about-app-name">API Manager</div>
                <div className="about-app-desc">API 接口文档 · 测试 · Mock 工具</div>
                <div className="about-version">版本 v{appVersion || "0.1.0"}</div>
              </div>
            </div>
            <div className="about-links">
              <LinkRow icon="📦" title="项目地址" desc={PROJECT_URL} url={PROJECT_URL} />
              <LinkRow icon="🐛" title="提交 Issue" desc="反馈问题、建议新功能" url={ISSUE_URL} />
            </div>
            <div className="about-footnote">
              接口、目录与 Mock 数据保存在本地工作区文件，版本快照位于 .version 目录。
            </div>
          </section>
        </div>
      </div>
    </Modal>
  );
}
