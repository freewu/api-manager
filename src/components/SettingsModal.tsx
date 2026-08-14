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
  const [draft, setDraft] = useState<AppSettings>({ ...settings });
  const [tab, setTab] = useState<"appearance" | "features">("appearance");

  return (
    <Modal
      title="设置"
      onClose={onClose}
      className="modal-settings"
      footer={
        <>
          <button className="btn" onClick={onClose}>
            取消
          </button>
          <button className="btn primary" onClick={() => onSave(draft)}>
            保存
          </button>
        </>
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
                      className={`settings-option ${draft.displayMode === m.value ? "active" : ""}`}
                    >
                      <input
                        type="radio"
                        name="displayMode"
                        checked={draft.displayMode === m.value}
                        onChange={() => setDraft({ ...draft, displayMode: m.value })}
                      />
                      {m.label}
                    </label>
                  ))}
                </div>
              </div>
              <div className="settings-desc">
                深色 / 浅色 / 跟随系统（Windows 主题自动切换）
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
                    checked={draft.enableVersion}
                    onChange={(v) => setDraft({ ...draft, enableVersion: v })}
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
                    checked={draft.enableMock}
                    onChange={(v) => setDraft({ ...draft, enableMock: v })}
                  />
                </div>
                <div className="settings-feature-desc">
                  在主页面显示 Mock 开关与端口
                </div>
              </div>
            </>
          )}
        </div>
      </div>
    </Modal>
  );
}
