import { useEffect, useRef, useState } from "react";
import { useT } from "../../i18n";
import {
  DEFAULT_SHORTCUTS,
  SHORTCUT_DEFS,
  ShortcutAction,
  conflictActions,
  formatShortcut,
  setShortcutRecording,
  shortcutFromEvent,
} from "../../utils/shortcuts";
import { SettingsSection } from "./Section";
import { SettingsTabProps } from "./types";

/** 纯修饰键：按下时保持录制状态，等待真正的按键 */
const MODIFIER_KEYS = ["Control", "Shift", "Alt", "Meta", "AltGraph", "CapsLock"];

/**
 * 快捷键：录制 / 清除 / 恢复默认。
 * 点击按键按钮进入录制状态（按钮获得焦点），按下组合键即保存；
 * Esc 取消录制，Backspace / Delete 清除绑定。
 */
export function ShortcutsTab({ settings, patch }: SettingsTabProps) {
  const t = useT();
  /** 正在录制的动作（null = 未录制） */
  const [recording, setRecording] = useState<ShortcutAction | null>(null);
  /** 上一次按键不合法（缺少修饰键） */
  const [invalid, setInvalid] = useState(false);
  const btnRef = useRef<HTMLButtonElement | null>(null);

  const shortcuts = settings.shortcuts;
  const conflicts = conflictActions(shortcuts);

  // 录制中挂起全局快捷键；进入录制时聚焦按钮，保证按键事件有目标
  useEffect(() => {
    setShortcutRecording(recording !== null);
    if (recording) btnRef.current?.focus();
    return () => setShortcutRecording(false);
  }, [recording]);

  const setShortcut = (action: ShortcutAction, key: string) => {
    patch({ shortcuts: { ...shortcuts, [action]: key } });
  };

  const onKeyDown = (e: React.KeyboardEvent, action: ShortcutAction) => {
    // 阻止冒泡，避免被全局快捷键 / 弹窗 Esc 处理拦截
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      setRecording(null);
      setInvalid(false);
      return;
    }
    if (e.key === "Backspace" || e.key === "Delete") {
      setRecording(null);
      setInvalid(false);
      setShortcut(action, "");
      return;
    }
    if (MODIFIER_KEYS.includes(e.key)) return; // 只按了修饰键：继续等待
    const key = shortcutFromEvent(e.nativeEvent);
    if (!key) {
      setInvalid(true);
      return;
    }
    setInvalid(false);
    setRecording(null);
    setShortcut(action, key);
  };

  return (
    <SettingsSection id="shortcuts" titleKey="settings.nav.shortcuts">
      <div className="settings-desc">{t("settings.shortcutsHint")}</div>
      <div className="settings-shortcut-list">
        {SHORTCUT_DEFS.map((d) => {
          const spec = shortcuts[d.action];
          const isRecording = recording === d.action;
          const conflict = conflicts.has(d.action);
          return (
            <div className="settings-shortcut-row" key={d.action}>
              <span className="settings-shortcut-name">{t(d.labelKey)}</span>
              <div className="settings-shortcut-keys">
                {conflict && !isRecording && (
                  <span className="settings-shortcut-warn" title={t("settings.shortcutsDuplicate")}>
                    ⚠
                  </span>
                )}
                <button
                  type="button"
                  ref={isRecording ? btnRef : undefined}
                  data-shortcut-recording={isRecording ? "1" : undefined}
                  className={`settings-shortcut-key${isRecording ? " recording" : ""}${
                    conflict ? " conflict" : ""
                  }`}
                  onClick={() => {
                    setInvalid(false);
                    setRecording(isRecording ? null : d.action);
                  }}
                  onKeyDown={(e) => isRecording && onKeyDown(e, d.action)}
                  onBlur={() => isRecording && setRecording(null)}
                  title={t("settings.shortcutsRecordTip")}
                >
                  {isRecording
                    ? t("settings.shortcutsRecording")
                    : formatShortcut(spec) || t("settings.shortcutsUnbound")}
                </button>
                <button
                  type="button"
                  className="settings-shortcut-reset"
                  disabled={spec === DEFAULT_SHORTCUTS[d.action]}
                  onClick={() => {
                    setRecording(null);
                    setShortcut(d.action, DEFAULT_SHORTCUTS[d.action]);
                  }}
                  title={t("settings.shortcutsReset")}
                >
                  ↺
                </button>
              </div>
            </div>
          );
        })}
      </div>
      <div className="settings-shortcut-foot">
        <span className="settings-desc">
          {t("settings.shortcutsFooter")}
          {invalid && <span className="settings-shortcut-error">{t("settings.shortcutsInvalid")}</span>}
        </span>
        <button
          type="button"
          className="settings-shortcut-reset-all"
          onClick={() => {
            setRecording(null);
            setInvalid(false);
            patch({ shortcuts: { ...DEFAULT_SHORTCUTS } });
          }}
        >
          {t("settings.shortcutsResetAll")}
        </button>
      </div>
    </SettingsSection>
  );
}
