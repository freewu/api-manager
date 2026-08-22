import { useState } from "react";

/**
 * 带格式 logo 的自定义下拉（div 实现，替代原生 select）。
 * 用于「导出弹窗」格式选择。
 * 图片来源：src/assets/import/（Vite 托管，<4KB 自动 base64 内联）。
 */

/** 格式 logo 图片映射：value → src/assets/import/ 下的文件名 */
const FORMAT_ICON_FILES: Record<string, string> = {
  postman: "postman.png",
  openapi: "swagger.png",
  apifox: "apifox.png",
  apipost: "apipost.png",
  docsify: "docsify.svg",
  markdown: "markdown.png",
  html: "html.png",
  raml: "raml.png",
  wadl: "wadl.png",
  har: "har.png",
  yapi: "yapi.png",
};

const formatIconImgs = import.meta.glob<string>("../assets/import/*.{png,svg}", {
  eager: true,
  import: "default",
});
const FORMAT_ICON: Record<string, string> = {};
for (const [path, url] of Object.entries(formatIconImgs)) {
  const file = path.slice(path.lastIndexOf("/") + 1);
  for (const [value, f] of Object.entries(FORMAT_ICON_FILES)) {
    if (f === file) FORMAT_ICON[value] = url;
  }
}

/** 格式图标（无对应图标时显示占位方块） */
export function FormatIcon({ value, className = "" }: { value: string; className?: string }) {
  const src = FORMAT_ICON[value];
  if (!src) return <span className={`format-icon format-icon-empty${className ? " " + className : ""}`} />;
  return <img className={`format-icon${className ? " " + className : ""}`} src={src} alt="" />;
}

export interface FormatOption<T extends string> {
  value: T;
  label: string;
}

/** 带格式 logo 的下拉（div 实现） */
export function FormatSelect<T extends string>({
  value,
  options,
  onChange,
  className = "",
  title,
}: {
  value: T;
  options: FormatOption<T>[];
  onChange: (v: T) => void;
  className?: string;
  title?: string;
}) {
  const [open, setOpen] = useState(false);
  const cur = options.find((o) => o.value === value) ?? options[0];
  return (
    <div className="format-select-wrap">
      <div
        className={`format-select${className ? " " + className : ""}${open ? " open" : ""}`}
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
        <FormatIcon value={cur.value} />
        <span className="format-label">{cur.label}</span>
        <span className="format-caret">▾</span>
      </div>
      {open && (
        <>
          <div className="menu-mask" onClick={() => setOpen(false)} />
          <div className="format-pop">
            {options.map((o) => (
              <div
                key={o.value}
                role="option"
                aria-selected={o.value === value}
                className={`format-opt${o.value === value ? " active" : ""}`}
                onClick={() => {
                  onChange(o.value);
                  setOpen(false);
                }}
              >
                <FormatIcon value={o.value} />
                {o.label}
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
