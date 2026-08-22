import { useCallback, useRef, useState } from "react";

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
    return saved >= 200 && saved <= 640 ? saved : 310;
  });
  const sidebarWidthRef = useRef(sidebarWidth);

  const startResize = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const startW = sidebarWidthRef.current;
    const onMove = (ev: MouseEvent) => {
      const w = Math.min(640, Math.max(200, startW + ev.clientX - startX));
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
    setSidebarWidth(310);
    sidebarWidthRef.current = 310;
    localStorage.setItem("sidebar-width", "310");
  }, []);

  // ---------- 编辑器 / 响应上下分栏比例 ----------
  const [editorRatio, setEditorRatio] = useState(() => {
    const saved = Number(localStorage.getItem("editor-ratio"));
    return saved >= 0.2 && saved <= 0.8 ? saved : 0.45;
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
    setEditorRatio(0.45);
    editorRatioRef.current = 0.45;
    localStorage.setItem("editor-ratio", "0.45");
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
