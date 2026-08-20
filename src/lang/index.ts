/**
 * 轻量 i18n：简体中文 / 繁體中文 / English 三语字典 + 语言状态（跟随设置持久化）
 * 用法：t("key") / t("key", { name })；组件内用 useT() / useLang() 响应切换
 */
import { useSyncExternalStore } from "react";

export type Lang = "zh" | "zh-tw" | "en";

let lang: Lang = "zh";
const listeners = new Set<() => void>();

export function getLang(): Lang {
  return lang;
}

export function setLang(l: Lang) {
  if (l === lang) return;
  lang = l;
  listeners.forEach((f) => f());
}

export function useLang(): Lang {
  return useSyncExternalStore(
    (cb) => {
      listeners.add(cb);
      return () => listeners.delete(cb);
    },
    () => lang
  );
}

/** 响应语言切换的翻译函数（组件内使用，触发重渲染） */
export function useT() {
  useLang();
  return t;
}

import { ZH } from "./zh";
import { TW } from "./zh-tw";
import { EN } from "./en";

const DICT: Record<Lang, Record<string, string>> = { zh: ZH, "zh-tw": TW, en: EN };

/** 翻译：t("key", {name: "xx"})，缺失键时原样返回 key */
export function t(key: string, params?: Record<string, string | number>): string {
  const dict = DICT[lang] || ZH;
  let s = dict[key] ?? ZH[key] ?? key;
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      s = s.replaceAll(`{${k}}`, String(v));
    }
  }
  return s;
}
