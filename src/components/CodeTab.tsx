import { useMemo, useState } from "react";
import hljs from "highlight.js/lib/core";
import bash from "highlight.js/lib/languages/bash";
import c from "highlight.js/lib/languages/c";
import cpp from "highlight.js/lib/languages/cpp";
import csharp from "highlight.js/lib/languages/csharp";
import delphi from "highlight.js/lib/languages/delphi";
import erlang from "highlight.js/lib/languages/erlang";
import go from "highlight.js/lib/languages/go";
import java from "highlight.js/lib/languages/java";
import javascript from "highlight.js/lib/languages/javascript";
import julia from "highlight.js/lib/languages/julia";
import kotlin from "highlight.js/lib/languages/kotlin";
import objectivec from "highlight.js/lib/languages/objectivec";
import perl from "highlight.js/lib/languages/perl";
import php from "highlight.js/lib/languages/php";
import python from "highlight.js/lib/languages/python";
import r from "highlight.js/lib/languages/r";
import ruby from "highlight.js/lib/languages/ruby";
import rust from "highlight.js/lib/languages/rust";
import swift from "highlight.js/lib/languages/swift";
import typescript from "highlight.js/lib/languages/typescript";
import "highlight.js/styles/github-dark.css";
import { ApiFile } from "../types";
import { CODE_LANGS, CodeLang, generateRequestCode } from "../utils/codegen";

hljs.registerLanguage("bash", bash);
hljs.registerLanguage("c", c);
hljs.registerLanguage("cpp", cpp);
hljs.registerLanguage("csharp", csharp);
hljs.registerLanguage("delphi", delphi);
hljs.registerLanguage("erlang", erlang);
hljs.registerLanguage("go", go);
hljs.registerLanguage("java", java);
hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("julia", julia);
hljs.registerLanguage("kotlin", kotlin);
hljs.registerLanguage("objectivec", objectivec);
hljs.registerLanguage("perl", perl);
hljs.registerLanguage("php", php);
hljs.registerLanguage("python", python);
hljs.registerLanguage("r", r);
hljs.registerLanguage("ruby", ruby);
hljs.registerLanguage("rust", rust);
hljs.registerLanguage("swift", swift);
hljs.registerLanguage("typescript", typescript);

/** CodeLang（含旧值 curl）→ highlight.js 语言 id */
const HLJS_LANG: Record<CodeLang, string> = {
  curl: "bash",
  bash: "bash",
  python: "python",
  c: "c",
  cpp: "cpp",
  java: "java",
  csharp: "csharp",
  javascript: "javascript",
  r: "r",
  rust: "rust",
  delphi: "delphi",
  php: "php",
  go: "go",
  ruby: "ruby",
  swift: "swift",
  perl: "perl",
  objectivec: "objectivec",
  julia: "julia",
  kotlin: "kotlin",
  typescript: "typescript",
  erlang: "erlang",
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
    <div className="codegen-root">
      <div className="section-title codegen-head">
        <span>代码生成</span>
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
      <div className="codegen-hint">
        代码自动跟随当前请求的 URL、Headers、Params、Path 与 Body；可在「设置 → 功能」中更改默认语言。
      </div>
    </div>
  );
}
