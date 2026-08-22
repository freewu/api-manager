import { useCallback, useMemo, useState } from "react";
import { loadSettings, saveSettings } from "../commands";
import { AppSettings, defaultSettings } from "../types";
import { setLang } from "../i18n";

/** 归一化语言值：兼容旧配置 "" / "zh" / "en" 与新值 "zh-tw" */
export const normalizeLang = (v: unknown): "zh" | "zh-tw" | "en" => {
  const s = String(v || "").toLowerCase().replace(/_/g, "-");
  if (s === "en") return "en";
  if (s === "zh-tw" || s === "zh-hant" || s === "zh-cht" || s === "tw" || s === "cht") return "zh-tw";
  return "zh";
};

/**
 * 应用设置：启动时从后端加载（与默认值合并，保证新字段存在），
 * 修改时即时持久化（无需点保存）。
 */
export function useSettings(onError: (err: string) => void) {
  const [settings, setSettings] = useState<AppSettings>(defaultSettings());

  /** 初始化加载设置（旧配置可能缺少 importTypes/exportTypes 等开关，合并默认值） */
  const load = useCallback(async () => {
    try {
      const s = await loadSettings();
      const def = defaultSettings();
      setSettings({
        ...def,
        ...s,
        importTypes: { ...def.importTypes, ...(s.importTypes || {}) },
        exportTypes: { ...def.exportTypes, ...(s.exportTypes || {}) },
      });
      setLang(normalizeLang(s.language));
    } catch {
      /* 设置读取失败时使用默认值 */
    }
  }, []);

  /** 修改设置：本地立即生效 + 后端持久化 */
  const save = useCallback(
    async (s: AppSettings) => {
      setSettings(s);
      try {
        await saveSettings(s);
      } catch (e) {
        onError(String(e));
      }
    },
    [onError]
  );

  const recentLimit = useMemo(() => Math.max(3, settings.recentLimit || 5), [settings.recentLimit]);

  return {
    settings,
    setSettings: save,
    /** 仅本地更新（不持久化），供语言切换等场景使用 */
    setSettingsRaw: setSettings,
    load,
    recentLimit,
  };
}
