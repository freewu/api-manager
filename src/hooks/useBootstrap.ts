import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getAppVersion, getRecentWorkspaces, mockStatus } from "../commands";
import { MockStatus, UpdateInfo } from "../types";
import { setLang } from "../i18n";
import { normalizeLang } from "./useSettings";

/**
 * 应用启动：版本号、最近打开目录、托盘事件监听、Mock 状态、主题跟随。
 */
export function useBootstrap(opts: {
  displayMode: string; // 来自设置（dark / light / system）
  onLanguageChanged: (lang: "zh" | "zh-tw" | "en") => void;
  onUpdateAvailable: (info: UpdateInfo) => void;
  onOpenEnvEditor: () => void;
  onMockChanged: (s: MockStatus) => void;
}) {
  const [version, setVersion] = useState("");
  const [recent, setRecent] = useState<string[]>([]);
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [showUpdateModal, setShowUpdateModal] = useState(false);

  // 应用显示模式（深色 / 浅色 / 跟随系统）
  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const mode =
        opts.displayMode === "system"
          ? mq.matches
            ? "dark"
            : "light"
          : opts.displayMode;
      document.documentElement.setAttribute("data-theme", mode);
    };
    apply();
    mq.addEventListener("change", apply);
    return () => mq.removeEventListener("change", apply);
  }, [opts.displayMode]);

  useEffect(() => {
    (async () => {
      try {
        const v = await getAppVersion();
        setVersion(v);
      } catch {
        /* noop */
      }
      getRecentWorkspaces()
        .then(setRecent)
        .catch(() => {});
      mockStatus().then(opts.onMockChanged).catch(() => {});
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 窗口标题带上版本号
  useEffect(() => {
    if (!version) return;
    const title = `API Manager v${version}`;
    document.title = title;
    getCurrentWindow().setTitle(title).catch(() => {});
  }, [version]);

  // 托盘菜单切换语言后，前端同步刷新文案与设置状态
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      unlisten = await listen("language-changed", (e) => {
        const lang = normalizeLang(e.payload);
        setLang(lang);
        opts.onLanguageChanged(lang);
      });
    })();
    return () => unlisten?.();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 托盘检查更新发现新版本后，弹出更新提醒弹窗
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      unlisten = await listen("update-available", (e) => {
        const info = e.payload as UpdateInfo;
        setUpdateInfo(info);
        opts.onUpdateAvailable(info);
      });
    })();
    return () => unlisten?.();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 托盘菜单点击「环境变量」-> 打开环境变量编辑器
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      unlisten = await listen("open-env-editor", () => opts.onOpenEnvEditor());
    })();
    return () => unlisten?.();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 托盘 Mock 服务启动/停止后，主页面联动刷新状态并提示
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      unlisten = await listen("mock-status-changed", async () => {
        try {
          const s = await mockStatus();
          opts.onMockChanged(s);
        } catch {
          /* noop */
        }
      });
    })();
    return () => unlisten?.();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return { version, recent, setRecent, updateInfo, showUpdateModal, setShowUpdateModal };
}
