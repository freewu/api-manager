/**
 * 全局快捷键：动作定义、按键归一化 / 匹配 / 展示。
 *
 * 存储格式为归一化字符串，修饰键固定顺序 ctrl+alt+shift，例如：
 *   "ctrl+a"（Ctrl+A）、"ctrl+shift+e"（Ctrl+Shift+E）、"f5"（F5）
 * 空字符串表示未绑定；macOS 上的 Command 键按 Ctrl 处理（同一条配置两端通用）。
 */

/** 可自定义的快捷键动作 */
export type ShortcutAction =
  | "viewApi"
  | "viewObjects"
  | "viewHistory"
  | "viewGenlogs"
  | "viewFavorites"
  | "openExport"
  | "openImport"
  | "openSettings"
  | "openEnv";

export interface ShortcutDef {
  action: ShortcutAction;
  /** 动作名称 i18n key */
  labelKey: string;
}

/** 全部可配置动作（顺序 = 设置页展示顺序，也是按键冲突时的优先顺序） */
export const SHORTCUT_DEFS: ShortcutDef[] = [
  { action: "viewApi", labelKey: "settings.shortcuts.viewApi" },
  { action: "viewObjects", labelKey: "settings.shortcuts.viewObjects" },
  { action: "viewHistory", labelKey: "settings.shortcuts.viewHistory" },
  { action: "viewGenlogs", labelKey: "settings.shortcuts.viewGenlogs" },
  { action: "viewFavorites", labelKey: "settings.shortcuts.viewFavorites" },
  { action: "openExport", labelKey: "settings.shortcuts.openExport" },
  { action: "openImport", labelKey: "settings.shortcuts.openImport" },
  { action: "openSettings", labelKey: "settings.shortcuts.openSettings" },
  { action: "openEnv", labelKey: "settings.shortcuts.openEnv" },
];

/** 默认快捷键（与 Rust 侧 default_shortcuts 保持一致） */
export const DEFAULT_SHORTCUTS: Record<ShortcutAction, string> = {
  viewApi: "ctrl+a",
  viewObjects: "ctrl+o",
  viewHistory: "ctrl+h",
  viewGenlogs: "ctrl+g",
  viewFavorites: "ctrl+f",
  openExport: "ctrl+e",
  openImport: "ctrl+i",
  openSettings: "ctrl+s",
  openEnv: "ctrl+m",
};

/** 支持的特殊键（event.key 小写 → 存储键名） */
const NAMED_KEYS = [
  "escape",
  "enter",
  "tab",
  "backspace",
  "delete",
  "insert",
  "home",
  "end",
  "pageup",
  "pagedown",
  "arrowup",
  "arrowdown",
  "arrowleft",
  "arrowright",
];

/** 展示用键名（存储键名 → 界面文案） */
const KEY_LABELS: Record<string, string> = {
  ctrl: "Ctrl",
  alt: "Alt",
  shift: "Shift",
  meta: "Meta",
  space: "Space",
  escape: "Esc",
  enter: "Enter",
  tab: "Tab",
  backspace: "Backspace",
  delete: "Delete",
  insert: "Insert",
  home: "Home",
  end: "End",
  pageup: "PageUp",
  pagedown: "PageDown",
  arrowup: "↑",
  arrowdown: "↓",
  arrowleft: "←",
  arrowright: "→",
};

/** 归一化单键（event.key → 存储键名）；不能作为快捷键的键返回 null（如单独的 Ctrl/Shift/Dead） */
function normalizeKey(key: string): string | null {
  if (!key) return null;
  if (key === " ") return "space";
  if (key === "Esc") return "escape";
  if (key.length === 1) {
    // 字母 / 数字 / 符号：统一小写（Shift 由修饰键表达）
    return /^[a-z0-9`\-=[\]\\;',./]$/i.test(key) ? key.toLowerCase() : null;
  }
  const k = key.toLowerCase();
  if (/^f\d{1,2}$/.test(k)) return k; // F1 ~ F12
  return NAMED_KEYS.includes(k) ? k : null;
}

/** 从键盘事件生成归一化快捷键；必须带 Ctrl/Alt/Command（F1~F12 除外），否则返回 null */
export function shortcutFromEvent(e: KeyboardEvent): string | null {
  const key = normalizeKey(e.key);
  if (!key) return null;
  const mods: string[] = [];
  if (e.ctrlKey || e.metaKey) mods.push("ctrl");
  if (e.altKey) mods.push("alt");
  if (e.shiftKey) mods.push("shift");
  if (!mods.length && !/^f\d{1,2}$/.test(key)) return null;
  return [...mods, key].join("+");
}

/** 事件是否命中某条快捷键（修饰键需完全一致） */
export function matchShortcut(e: KeyboardEvent, spec: string): boolean {
  if (!spec) return false;
  const parts = spec.split("+");
  const key = parts[parts.length - 1];
  if (normalizeKey(e.key) !== key) return false;
  const ctrl = parts.includes("ctrl");
  if (ctrl !== (e.ctrlKey || e.metaKey)) return false;
  if (parts.includes("alt") !== e.altKey) return false;
  if (parts.includes("shift") !== e.shiftKey) return false;
  return true;
}

/** 事件命中的动作（多条冲突时取 SHORTCUT_DEFS 中靠前的），无命中返回 null */
export function matchAction(
  e: KeyboardEvent,
  shortcuts: Record<ShortcutAction, string>
): ShortcutAction | null {
  for (const d of SHORTCUT_DEFS) {
    const spec = shortcuts[d.action];
    if (spec && matchShortcut(e, spec)) return d.action;
  }
  return null;
}

/** 展示用文案：ctrl+shift+a → Ctrl + Shift + A */
export function formatShortcut(spec: string): string {
  if (!spec) return "";
  return spec.split("+").map(keyLabel).join(" + ");
}

/** 单个键的展示文案（字母 / 数字大写，F1~F12 大写，特殊键取键名表） */
function keyLabel(part: string): string {
  const label = KEY_LABELS[part];
  if (label) return label;
  if (part.length === 1 || /^f\d{1,2}$/.test(part)) return part.toUpperCase();
  return part;
}

/** 合并默认值：旧配置缺少的动作、非字符串值一律回退默认（"" 表示用户主动解绑，保留） */
export function normalizeShortcuts(raw: unknown): Record<ShortcutAction, string> {
  const src = (raw ?? {}) as Record<string, unknown>;
  const out = {} as Record<ShortcutAction, string>;
  for (const d of SHORTCUT_DEFS) {
    const v = src[d.action];
    out[d.action] = typeof v === "string" ? v.trim().toLowerCase() : DEFAULT_SHORTCUTS[d.action];
  }
  return out;
}

/** 存在按键冲突的动作集合（同一按键被多个动作使用） */
export function conflictActions(shortcuts: Record<ShortcutAction, string>): Set<ShortcutAction> {
  const used = new Map<string, ShortcutAction[]>();
  for (const d of SHORTCUT_DEFS) {
    const spec = shortcuts[d.action];
    if (!spec) continue;
    used.set(spec, [...(used.get(spec) ?? []), d.action]);
  }
  const out = new Set<ShortcutAction>();
  for (const actions of used.values()) {
    if (actions.length > 1) actions.forEach((a) => out.add(a));
  }
  return out;
}

/** 录制状态标记：录制快捷键期间挂起全局快捷键，避免录制过程触发动作 */
let recording = false;
export const setShortcutRecording = (v: boolean) => {
  recording = v;
};
export const isShortcutRecording = () => recording;

/** 焦点在输入类元素（含 Vditor 等可编辑区域）上时返回 true，此时不触发全局快捷键 */
export function isEditableTarget(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  if (!el || !el.tagName) return false;
  const tag = el.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
  if (el.isContentEditable) return true;
  return !!el.closest?.("[contenteditable='true']");
}
