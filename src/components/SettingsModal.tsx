import { useState } from "react";
import { AppSettings } from "../types";
import { Modal } from "./Modal";

interface Props {
  settings: AppSettings;
  onClose: () => void;
  onSave: (s: AppSettings) => void;
}

const MODES = [
  { value: "dark", label: "🌙 深色" },
  { value: "light", label: "☀️ 浅色" },
  { value: "system", label: "🖥 跟随系统" },
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

export function SettingsModal({ settings, onClose, onSave }: Props) {
  const [tab, setTab] = useState<"appearance" | "features">("appearance");

  const patch = (p: Partial<AppSettings>) => onSave({ ...settings, ...p });

  return (
    <Modal
      title="设置"
      onClose={onClose}
      className="modal-settings"
      footer={
        <span className="settings-auto-hint">⚡ 修改即时生效，无需保存</span>
      }
    >
      <div className="settings-layout">
        <div className="settings-nav">
          <div
            className={`settings-nav-item ${tab === "appearance" ? "active" : ""}`}
            onClick={() => setTab("appearance")}
          >
            🎨 外观
          </div>
          <div
            className={`settings-nav-item ${tab === "features" ? "active" : ""}`}
            onClick={() => setTab("features")}
          >
            ⚡ 功能
          </div>
        </div>

        <div className="settings-panel">
          {tab === "appearance" && (
            <>
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
              <div className="settings-desc">
                深色 / 浅色 / 跟随系统（Windows 主题自动切换）
              </div>

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
            </>
          )}

          {tab === "features" && (
            <>
              <div className="settings-panel-title">功能</div>
              <div className="settings-feature">
                <div className="settings-feature-head">
                  <span className="settings-feature-name">接口版本管理</span>
                  <Switch
                    checked={settings.enableVersion}
                    onChange={(v) => patch({ enableVersion: v })}
                  />
                </div>
                <div className="settings-feature-desc">
                  在主页面显示「保存」按钮与右键「查看版本信息」
                </div>
              </div>
              <div className="settings-feature">
                <div className="settings-feature-head">
                  <span className="settings-feature-name">Mock 服务</span>
                  <Switch
                    checked={settings.enableMock}
                    onChange={(v) => patch({ enableMock: v })}
                  />
                </div>
                <div className="settings-feature-desc">
                  在主页面显示 Mock 开关与端口
                </div>
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

              <div className="settings-about">
                <div className="settings-about-title">关于</div>
                <div className="settings-about-item">API Manager 接口管理工具</div>
                <div className="settings-about-item">接口、目录与 Mock 数据保存在本地工作区</div>
              </div>
            </>
          )}
        </div>
      </div>
    </Modal>
  );
}
