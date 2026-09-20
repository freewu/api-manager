import { useEffect, useState } from "react";

/**
 * 带语言 logo 图片的自定义下拉（div 实现，替代原生 select）。
 * 用于「代码生成」页签的语言切换与「设置」中的默认语言选择。
 * 图片来源：src/assets/code/（Vite 托管，<4KB 自动 base64 内联）。
 */

/** 语言 logo 图片映射：value → src/assets/code/ 下的文件名（大小写以实际为准） */
const LANG_ICON_FILES: Record<string, string> = {
  bash: "bash.png",
  python: "Python.png",
  c: "c.png",
  cpp: "C++.png",
  java: "java.png",
  csharp: "csharp.png",
  dart: "dart.png",
  javascript: "javascript.png",
  r: "R.png",
  rust: "rust.png",
  delphi: "delphi.png",
  php: "php.png",
  go: "go.png",
  ruby: "Ruby.png",
  swift: "swift.png",
  perl: "perl.png",
  objectivec: "objective-c.png",
  julia: "Julia.png",
  kotlin: "kotlin.png",
  typescript: "typescript.png",
  erlang: "Erlang.png",
  lua: "Lua.png",
  powershell: "powershell.png",
};
const langIconImgs = import.meta.glob<string>("../assets/code/*.png", {
  eager: true,
  import: "default",
});
const LANG_ICON: Record<string, string> = {};
for (const [path, url] of Object.entries(langIconImgs)) {
  const file = path.slice(path.lastIndexOf("/") + 1);
  for (const [value, f] of Object.entries(LANG_ICON_FILES)) {
    if (f === file) LANG_ICON[value] = url;
  }
}

function LangIcon({ value }: { value: string }) {
  const src = LANG_ICON[value];
  if (!src) return <span className="codegen-lang-icon codegen-lang-icon-empty" />;
  return <img className="codegen-lang-icon" src={src} alt="" />;
}

export interface LangOption<T extends string> {
  value: T;
  label: string;
}

export function LangSelect<T extends string>({
  value,
  options,
  onChange,
  className = "",
  title,
}: {
  value: T;
  options: LangOption<T>[];
  onChange: (v: T) => void;
  className?: string;
  title?: string;
}) {
  const [open, setOpen] = useState(false);

  // 菜单打开时按 ESC 关闭
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        setOpen(false);
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [open]);
  const cur = options.find((o) => o.value === value) ?? options[0];
  return (
    <div className="codegen-lang-select-wrap">
      <div
        className={`codegen-lang-select${className ? " " + className : ""}${open ? " open" : ""}`}
        onClick={() => setOpen((s) => !s)}
        title={title}
        role="listbox"
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            setOpen((s) => !s);
          } else if (e.key === "Escape") {
            setOpen(false);
          }
        }}
      >
        <LangIcon value={cur.value} />
        <span className="codegen-lang-label">{cur.label}</span>
        <span className="codegen-lang-caret">▾</span>
      </div>
      {open && (
        <>
          <div className="menu-mask" onClick={() => setOpen(false)} />
          <div className="codegen-lang-pop">
            {options.map((o) => (
              <div
                key={o.value}
                role="option"
                aria-selected={o.value === value}
                className={`codegen-lang-opt${o.value === value ? " active" : ""}`}
                onClick={() => {
                  onChange(o.value);
                  setOpen(false);
                }}
              >
                <LangIcon value={o.value} />
                {o.label}
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
