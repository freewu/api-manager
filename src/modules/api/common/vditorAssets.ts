/**
 * Vditor 本地资源：Vditor 默认从 CDN（unpkg）拉取 Lute 解析器、提示文案、图标与内容主题，
 * 桌面端离线时会 404 并导致编辑器初始化中断，因此统一改为打包内资源：
 *   - Lute（Markdown 解析器）：以 ?url 形式打包，通过 Vditor 的 `_lutePath` 指定
 *   - 内容主题（light / dark）：以 ?url 形式打包，通过 `preview.theme.path + current` 指定
 *   - 提示文案：动态 import 官方语言包（副作用脚本，执行后挂到 window.VditorI18n）
 *   - 工具栏图标：动态 import 官方图标精灵（副作用脚本，向 body 注入 svg symbol）
 */
import type Vditor from "vditor";
import type { Lang } from "../../../i18n";

import darkThemeUrl from "vditor/dist/css/content-theme/dark.css?url";
import lightThemeUrl from "vditor/dist/css/content-theme/light.css?url";
import luteUrl from "vditor/dist/js/lute/lute.min.js?url";

/** Lute 解析器脚本地址（Vditor 的 _lutePath） */
export const LUTE_PATH = luteUrl;

/** Vditor 语言标识（对应 dist/js/i18n/<lang>.js） */
type VditorLang = "zh_CN" | "zh_TW" | "en_US";
const VEDITOR_LANG: Record<Lang, VditorLang> = {
  zh: "zh_CN",
  "zh-tw": "zh_TW",
  en: "en_US",
};

type VditorOptions = NonNullable<ConstructorParameters<typeof Vditor>[1]>;

/** 由打包后的资源 URL 拆出 Vditor 需要的目录与主题名（Vditor 会拼 `${path}/${current}.css`） */
function contentTheme(url: string): { path: string; current: string } {
  const i = url.lastIndexOf("/");
  return { path: url.slice(0, i), current: url.slice(i + 1).replace(/\.css$/, "") };
}

/** 内容主题（浅色 / 深色）在打包后的位置 */
export const CONTENT_THEME = {
  light: contentTheme(lightThemeUrl),
  dark: contentTheme(darkThemeUrl),
};

/** 已加载的提示文案缓存（key 为 Vditor 语言标识） */
const tipsCache: Record<string, VditorOptions["i18n"]> = {};

/**
 * 加载 Vditor 提示文案。
 * 显式传入 `i18n` 时 Vditor 不再请求 CDN 语言包，改用这里的官方语言包对象。
 */
export async function loadTips(lang: Lang): Promise<VditorOptions["i18n"]> {
  const id = VEDITOR_LANG[lang];
  if (!tipsCache[id]) {
    if (id === "zh_TW") await import("vditor/dist/js/i18n/zh_TW.js");
    else if (id === "en_US") await import("vditor/dist/js/i18n/en_US.js");
    else await import("vditor/dist/js/i18n/zh_CN.js");
    tipsCache[id] = (window as unknown as { VditorI18n: VditorOptions["i18n"] }).VditorI18n;
  }
  return tipsCache[id];
}

let iconsLoaded = false;

/**
 * 加载工具栏图标精灵。
 * 图标本身由打包内脚本注入（向 body 插入 svg symbol），同时插入同 id 的占位 script 标签，
 * 让 Vditor 跳过它自己的 CDN 请求：addScript / addScriptSync 在发现同 id 元素时直接返回。
 */
export async function loadIcons(): Promise<void> {
  if (iconsLoaded) return;
  iconsLoaded = true;
  const stub = document.createElement("script");
  stub.id = "vditorIconScript";
  document.head.appendChild(stub);
  await import("vditor/dist/js/icons/ant.js");
}

/** Vditor 语言标识（与 loadTips 对应，供 options.lang 使用） */
export function vditorLang(lang: Lang): VditorLang {
  return VEDITOR_LANG[lang];
}
