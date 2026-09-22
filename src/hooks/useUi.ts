import { useCallback, useRef, useState } from "react";

/** 顶栏高度（与 styles/layout.css 的 .toolbar 保持一致），用于把「窗体高度」换算到内容区 */
const TOOLBAR_H = 46;
/** 响应面板默认占窗体高度的比例（2/5） */
const RESPONSE_WINDOW_RATIO = 0.4;
/** 内容区高度不可用时的兜底编辑器占比（响应约占内容区 2/5） */
const FALLBACK_EDITOR_RATIO = 0.6;
/** 旧版默认占比（响应偏高），未自定义过的用户迁移到新默认 */
const LEGACY_EDITOR_RATIO = 0.45;
/** 侧栏宽度范围（与 styles/sidebar.css 的 min-width / max-width 保持一致） */
const MIN_SIDEBAR_WIDTH = 260;
const MAX_SIDEBAR_WIDTH = 640;
const DEFAULT_SIDEBAR_WIDTH = 310;

/**
 * 计算编辑器默认占比：使响应面板默认高度等于窗体高度的 2/5。
 * 编辑器 / 响应在「窗体 - 顶栏」的内容区分栏，故：
 *   (1 - ratio) * (winH - TOOLBAR_H) = winH * 0.4
 */
function defaultEditorRatio(): number {
  const winH = typeof window === "undefined" ? 0 : window.innerHeight;
  const contentH = winH - TOOLBAR_H;
  if (contentH <= 160) return FALLBACK_EDITOR_RATIO;
  const ratio = 1 - (winH * RESPONSE_WINDOW_RATIO) / contentH;
  return Math.min(0.8, Math.max(0.2, ratio));
}

/**
 * 界面状态：toast 提示、左右分栏宽度、编辑器/响应上下分栏比例、
 * 空白区域右键菜单。
 */
export function useUi() {
  // ---------- toast ----------
  const [toast, setToast] = useState<string | null>(null);
  const toastTimer = useRef<number | null>(null);
  const showToast = useCallback((msg: string) => {
    setToast(msg);
    if (toastTimer.current) window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 2200);
  }, []);

  // ---------- 左右分栏宽度 ----------
  const [sidebarWidth, setSidebarWidth] = useState(() => {
    const saved = Number(localStorage.getItem("sidebar-width"));
    return saved >= MIN_SIDEBAR_WIDTH && saved <= MAX_SIDEBAR_WIDTH ? saved : DEFAULT_SIDEBAR_WIDTH;
  });
  const sidebarWidthRef = useRef(sidebarWidth);

  const startResize = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const startW = sidebarWidthRef.current;
    const onMove = (ev: MouseEvent) => {
      const w = Math.min(MAX_SIDEBAR_WIDTH, Math.max(MIN_SIDEBAR_WIDTH, startW + ev.clientX - startX));
      sidebarWidthRef.current = w;
      setSidebarWidth(w);
    };
    const onUp = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      localStorage.setItem("sidebar-width", String(sidebarWidthRef.current));
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  }, []);

  const resetSidebarWidth = useCallback(() => {
    setSidebarWidth(DEFAULT_SIDEBAR_WIDTH);
    sidebarWidthRef.current = DEFAULT_SIDEBAR_WIDTH;
    localStorage.setItem("sidebar-width", String(DEFAULT_SIDEBAR_WIDTH));
  }, []);

  // ---------- 编辑器 / 响应上下分栏比例 ----------
  const [editorRatio, setEditorRatio] = useState(() => {
    const saved = Number(localStorage.getItem("editor-ratio"));
    // 0.45 为旧版默认值（响应面板偏高）：未自定义过的用户迁移到新默认
    return saved >= 0.2 && saved <= 0.8 && saved !== LEGACY_EDITOR_RATIO
      ? saved
      : defaultEditorRatio();
  });
  const editorRatioRef = useRef(editorRatio);

  /** 拖动中直接操作 DOM + rAF 合并帧，避免每个 mousemove 触发 React 重渲染而卡顿 */
  const startVResize = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    const startY = e.clientY;
    const startRatio = editorRatioRef.current; // 拖动开始时的比例（基线，避免累计放大导致闪跳）
    const contentEl = (e.currentTarget as HTMLElement).parentElement as HTMLElement;
    const contentH = contentEl.clientHeight;
    const editorEl = contentEl.querySelector<HTMLElement>(".editor");
    // 预留分隔条 + 响应面板最小高度
    const maxRatio = Math.max(0.2, (contentH - 165) / contentH);
    let lastY = startY;
    let raf = 0;
    const onMove = (ev: MouseEvent) => {
      lastY = ev.clientY;
      if (raf) return; // 已有一帧待执行，丢弃中间事件
      raf = requestAnimationFrame(() => {
        raf = 0;
        const ratio = Math.min(maxRatio, Math.max(0.2, startRatio + (lastY - startY) / contentH));
        editorRatioRef.current = ratio;
        if (editorEl) editorEl.style.height = `${ratio * 100}%`;
      });
    };
    const onUp = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      if (raf) cancelAnimationFrame(raf);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      setEditorRatio(editorRatioRef.current);
      localStorage.setItem("editor-ratio", String(editorRatioRef.current));
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
    document.body.style.cursor = "row-resize";
    document.body.style.userSelect = "none";
  }, []);

  const resetEditorRatio = useCallback(() => {
    const ratio = defaultEditorRatio();
    setEditorRatio(ratio);
    editorRatioRef.current = ratio;
    localStorage.setItem("editor-ratio", String(ratio));
  }, []);

  // ---------- 空白区域右键菜单 ----------
  const [emptyMenu, setEmptyMenu] = useState<{ x: number; y: number } | null>(null);

  return {
    toast,
    showToast,
    sidebarWidth,
    startResize,
    resetSidebarWidth,
    editorRatio,
    startVResize,
    resetEditorRatio,
    emptyMenu,
    setEmptyMenu,
  };
}
