import { useState } from "react";
import { AppSettings } from "../types";
import { Modal } from "./Modal";

interface Props {
  settings: AppSettings;
  onClose: () => void;
  onSave: (s: AppSettings) => void;
}

export function SettingsModal({ settings, onClose, onSave }: Props) {
  const [draft, setDraft] = useState<AppSettings>({ ...settings });

  return (
    <Modal
      title="⚙ 设置"
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
      <div className="settings-group">
        <div className="settings-title">显示</div>
        <div className="settings-row">
          <span className="settings-label">显示模式</span>
          <div className="settings-options">
            <label className={`settings-option ${draft.displayMode === "dark" ? "active" : ""}`}>
              <input
                type="radio"
                name="displayMode"
                checked={draft.displayMode === "dark"}
                onChange={() => setDraft({ ...draft, displayMode: "dark" })}
              />
              🌙 深色
            </label>
            <label className={`settings-option ${draft.displayMode === "light" ? "active" : ""}`}>
              <input
                type="radio"
                name="displayMode"
                checked={draft.displayMode === "light"}
                onChange={() => setDraft({ ...draft, displayMode: "light" })}
              />
              ☀️ 浅色
            </label>
          </div>
        </div>
      </div>

      <div className="settings-group">
        <div className="settings-title">功能</div>
        <label className="settings-row check">
          <input
            type="checkbox"
            checked={draft.enableVersion}
            onChange={(e) => setDraft({ ...draft, enableVersion: e.target.checked })}
          />
          <span>接口版本管理</span>
          <span className="settings-desc">在主页面显示「保存」按钮与右键「查看版本信息」</span>
        </label>
        <label className="settings-row check">
          <input
            type="checkbox"
            checked={draft.enableMock}
            onChange={(e) => setDraft({ ...draft, enableMock: e.target.checked })}
          />
          <span>Mock 服务</span>
          <span className="settings-desc">在主页面显示 Mock 开关与端口</span>
        </label>
      </div>
    </Modal>
  );
}
