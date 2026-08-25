import { useMemo, useRef } from "react";
import hljs from "highlight.js/lib/core";
import javascript from "highlight.js/lib/languages/javascript";

hljs.registerLanguage("javascript", javascript);

interface Props {
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
}

/**
 * JS 代码编辑器：textarea 输入 + 下层 pre 语法高亮（hljs），
 * 输入/滚动同步，占满父容器剩余空间。
 */
export default function JsCodeEditor({ value, onChange, placeholder }: Props) {
  const taRef = useRef<HTMLTextAreaElement>(null);
  const preRef = useRef<HTMLPreElement>(null);

  const html = useMemo(() => {
    const src = value.replace(/\n$/, "\n\u200b"); // 末尾换行补零宽空格占位
    try {
      return hljs.highlight(src, { language: "javascript" }).value || "";
    } catch {
      return src.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
    }
  }, [value]);

  const syncScroll = () => {
    if (taRef.current && preRef.current) {
      preRef.current.scrollTop = taRef.current.scrollTop;
      preRef.current.scrollLeft = taRef.current.scrollLeft;
    }
  };

  return (
    <div className="js-code-editor">
      <pre ref={preRef} className="js-code-editor-pre hljs" aria-hidden="true">
        <code dangerouslySetInnerHTML={{ __html: html }} />
      </pre>
      <textarea
        ref={taRef}
        className="js-code-editor-ta"
        value={value}
        placeholder={placeholder}
        spellCheck={false}
        autoCapitalize="off"
        autoComplete="off"
        autoCorrect="off"
        onChange={(e) => onChange(e.target.value)}
        onScroll={syncScroll}
      />
    </div>
  );
}
