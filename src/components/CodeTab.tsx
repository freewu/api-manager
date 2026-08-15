import { useMemo, useState } from "react";
import hljs from "highlight.js/lib/core";
import bash from "highlight.js/lib/languages/bash";
import go from "highlight.js/lib/languages/go";
import rust from "highlight.js/lib/languages/rust";
import java from "highlight.js/lib/languages/java";
import python from "highlight.js/lib/languages/python";
import javascript from "highlight.js/lib/languages/javascript";
import "highlight.js/styles/github-dark.css";
import { ApiFile } from "../types";
import { CODE_LANGS, CodeLang, generateRequestCode } from "../utils/codegen";

hljs.registerLanguage("bash", bash);
hljs.registerLanguage("go", go);
hljs.registerLanguage("rust", rust);
hljs.registerLanguage("java", java);
hljs.registerLanguage("python", python);
hljs.registerLanguage("javascript", javascript);

/** CodeLang（含旧值 curl）→ highlight.js 语言 id */
const HLJS_LANG: Record<CodeLang, string> = {
  curl: "bash",
  bash: "bash",
  go: "go",
  rust: "rust",
  java: "java",
  python: "python",
  javascript: "javascript",
};

interface Props {
  api: ApiFile;
  baseUrl: string;
  defaultLang: string;
}

export function CodeTab({ api, baseUrl, defaultLang }: Props) {
  const [lang, setLang] = useState<CodeLang>(
    (CODE_LANGS.some((l) => l.value === defaultLang) ? defaultLang : "bash") as CodeLang
  );
  const [copied, setCopied] = useState(false);

  const code = useMemo(() => generateRequestCode(lang, api, baseUrl), [lang, api, baseUrl]);
  const html = useMemo(() => {
    try {
      return hljs.highlight(code, { language: HLJS_LANG[lang] }).value;
    } catch {
      return code.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
    }
  }, [code, lang]);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      /* 剪贴板不可用时忽略 */
    }
  };

  return (
    <div>
      <div className="section-title codegen-head">
        <span>请求代码生成</span>
        <select
          className="codegen-lang"
          value={lang}
          onChange={(e) => setLang(e.target.value as CodeLang)}
          title="切换开发语言"
        >
          {CODE_LANGS.map((l) => (
            <option key={l.value} value={l.value}>
              {l.label}
            </option>
          ))}
        </select>
        <button className="btn small" onClick={copy}>
          {copied ? "✓ 已复制" : "📋 复制"}
        </button>
      </div>
      <pre className="codegen-pre">
        <code
          className="hljs codegen-code"
          dangerouslySetInnerHTML={{ __html: html }}
        />
      </pre>
      <div style={{ color: "var(--text-faint)", fontSize: 11, marginTop: 6 }}>
        代码自动跟随当前请求的 URL、Headers、Params、Path 与 Body；可在「设置 → 功能」中更改默认语言。
      </div>
    </div>
  );
}
