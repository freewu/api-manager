import { useEffect, useRef, useState } from "react";
import type Vditor from "vditor";
import { t, useLang } from "../../../i18n";
import { CONTENT_THEME, LUTE_PATH, loadIcons, loadTips, vditorLang } from "./vditorAssets";

type VditorInstance = InstanceType<typeof Vditor>;

/** 工具栏：仅保留本地即可完成的功能，避免触发 Vditor 的 CDN 请求（上传 / 导出 / 内容主题等已去除） */
const TOOLBAR = [
  "headings",
  "bold",
  "italic",
  "strike",
  "link",
  "|",
  "list",
  "ordered-list",
  "check",
  "table",
  "|",
  "quote",
  "line",
  "code",
  "inline-code",
  "|",
  "undo",
  "redo",
  "|",
  "outline",
  "preview",
  "edit-mode",
  "fullscreen",
];

/** 跟随 <html data-theme>（由 useBootstrap 按显示模式写入）判断当前是否深色 */
function useIsDark(): boolean {
  const read = () => document.documentElement.getAttribute("data-theme") === "dark";
  const [dark, setDark] = useState(read);
  useEffect(() => {
    const el = document.documentElement;
    const sync = () => setDark(read());
    sync();
    const observer = new MutationObserver(sync);
    observer.observe(el, { attributes: true, attributeFilter: ["data-theme"] });
    return () => observer.disconnect();
  }, []);
  return dark;
}

/**
 * 接口描述：基于 Vditor 的 Markdown 编辑器（即时渲染模式，编辑时即所见即所得）。
 * 内容变更通过 onChange 向上抛出，失焦时通过 onCommit 触发保存。
 */
export function DescEditor({
  value,
  onChange,
  onCommit,
}: {
  value: string;
  onChange: (v: string) => void;
  onCommit?: () => void;
}) {
  const lang = useLang();
  const dark = useIsDark();
  const holderRef = useRef<HTMLDivElement | null>(null);
  const vditorRef = useRef<VditorInstance | null>(null);
  /** 编辑器是否已初始化完成：初始化过程中的 input 回调不外抛，避免把空内容写回接口 */
  const readyRef = useRef(false);
  /** 编辑器当前内容：与外部 value 相同说明是编辑器自身改动，无需 setValue */
  const valueRef = useRef(value || "");
  const valuePropRef = useRef(value || "");
  const onChangeRef = useRef(onChange);
  const onCommitRef = useRef(onCommit);
  valuePropRef.current = value || "";
  onChangeRef.current = onChange;
  onCommitRef.current = onCommit;

  // 创建 / 重建编辑器（语言、主题变化时重建，以刷新提示文案与配色）
  useEffect(() => {
    let disposed = false;
    let instance: VditorInstance | undefined;
    readyRef.current = false;

    void (async () => {
      // 提示文案、图标精灵、样式与主体均为 Vditor 官方资源，首次打开页签时按需加载（离线可用）
      const [tips] = await Promise.all([
        loadTips(lang),
        loadIcons(),
        import("vditor/dist/index.css"),
      ]);
      const { default: VditorCtor } = await import("vditor");
      if (disposed || !holderRef.current) return;
      instance = new VditorCtor(holderRef.current, {
        cdn: "", // 不使用 CDN：Lute / 文案 / 图标 / 内容主题全部走打包资源
        _lutePath: LUTE_PATH,
        i18n: tips,
        lang: vditorLang(lang),
        icon: "ant",
        mode: "ir",
        value: valuePropRef.current,
        height: "100%",
        placeholder: t("editor.descPlaceholder"),
        theme: dark ? "dark" : "classic",
        preview: {
          theme: dark ? CONTENT_THEME.dark : CONTENT_THEME.light,
          hljs: { enable: false, lineNumber: false },
        },
        cache: { enable: false },
        toolbar: TOOLBAR,
        input: (v) => {
          if (!readyRef.current) return;
          valueRef.current = v;
          onChangeRef.current(v);
        },
        blur: () => onCommitRef.current?.(),
        after: () => {
          valueRef.current = valuePropRef.current;
          readyRef.current = true;
        },
      });
      if (disposed) {
        instance.destroy();
        return;
      }
      vditorRef.current = instance;
    })().catch(() => {
      /* 资源加载失败时保持空白容器，不影响其它页签 */
    });

    return () => {
      disposed = true;
      vditorRef.current = null;
      instance?.destroy();
    };
  }, [lang, dark]);

  // 外部内容变化（切换接口、撤销等）时同步进编辑器
  useEffect(() => {
    const editor = vditorRef.current;
    const next = value || "";
    if (!editor || !readyRef.current || next === valueRef.current) return;
    valueRef.current = next;
    editor.setValue(next);
  }, [value]);

  return (
    <div className="desc-root">
      <div className="desc-vditor" ref={holderRef} />
      <div className="desc-hint">{t("editor.descHint")}</div>
    </div>
  );
}
