import { useEffect, useRef } from "react";
import { ShortcutAction, isEditableTarget, isShortcutRecording, matchAction } from "../utils/shortcuts";

/**
 * 全局快捷键：监听 keydown，按用户配置执行对应动作。
 * - 焦点在输入框 / 可编辑区域（含 Vditor）时不触发，保留原生编辑快捷键
 * - 录制快捷键期间挂起（见 utils/shortcuts 的 setShortcutRecording）
 */
export function useShortcuts(
  shortcuts: Record<ShortcutAction, string>,
  handlers: Record<ShortcutAction, () => void>
) {
  const shortcutsRef = useRef(shortcuts);
  const handlersRef = useRef(handlers);
  shortcutsRef.current = shortcuts;
  handlersRef.current = handlers;

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.defaultPrevented || e.repeat) return;
      if (isShortcutRecording()) return;
      if (isEditableTarget(e.target)) return;
      const action = matchAction(e, shortcutsRef.current);
      if (!action) return;
      const run = handlersRef.current[action];
      if (!run) return;
      // 拦截浏览器 / WebView 默认行为（Ctrl+A 全选、Ctrl+S 保存、Ctrl+F 查找等）
      e.preventDefault();
      e.stopPropagation();
      run();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}
