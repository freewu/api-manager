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
import lua from "highlight.js/lib/languages/lua";
import objectivec from "highlight.js/lib/languages/objectivec";
import perl from "highlight.js/lib/languages/perl";
import php from "highlight.js/lib/languages/php";
import powershell from "highlight.js/lib/languages/powershell";
import python from "highlight.js/lib/languages/python";
import r from "highlight.js/lib/languages/r";
import ruby from "highlight.js/lib/languages/ruby";
import rust from "highlight.js/lib/languages/rust";
import swift from "highlight.js/lib/languages/swift";
import typescript from "highlight.js/lib/languages/typescript";
import "highlight.js/styles/github-dark.css";
import { ApiFile } from "../types";
import { CODE_LANGS, CODE_LIBS, WS_CODE_LIBS, CodeLang, defaultLib, generateRequestCode, generateWebSocketCode } from "../utils/codegen";
import { useT } from "../i18n";

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
hljs.registerLanguage("lua", lua);
hljs.registerLanguage("objectivec", objectivec);
hljs.registerLanguage("perl", perl);
hljs.registerLanguage("php", php);
hljs.registerLanguage("powershell", powershell);
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
  lua: "lua",
  powershell: "powershell",
};

interface Props {
  api: ApiFile;
  baseUrl: string;
  defaultLang: string;
}

export function CodeTab({ api, baseUrl, defaultLang }: Props) {
  const t = useT();
  const isWs = api.protocol === "websocket";
  const [lang, setLang] = useState<CodeLang>(
    (CODE_LANGS.some((l) => l.value === defaultLang) ? defaultLang : "bash") as CodeLang
  );
  const [lib, setLib] = useState<string | undefined>(() =>
    isWs ? WS_CODE_LIBS[lang]?.[0]?.value : defaultLib(lang)
  );
  const [copied, setCopied] = useState(false);

  const libs = isWs ? WS_CODE_LIBS[lang] : CODE_LIBS[lang];
  const activeLib = libs?.find((l) => l.value === lib);

  const code = useMemo(
    () => (isWs ? generateWebSocketCode(lang, api, baseUrl, lib) : generateRequestCode(lang, api, baseUrl, lib)),
    [isWs, lang, lib, api, baseUrl]
  );
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
        <span>{t("codegen.title")}</span>
        <select
          className="codegen-lang"
          value={lang}
          onChange={(e) => {
            const next = e.target.value as CodeLang;
            setLang(next);
            setLib(isWs ? WS_CODE_LIBS[next]?.[0]?.value : defaultLib(next));
          }}
          title={t("codegen.switchLang")}
        >
          {CODE_LANGS.map((l) => (
            <option key={l.value} value={l.value}>
              {l.label}
            </option>
          ))}
        </select>
        <button className="btn small" onClick={copy}>
          {copied ? t("resp.copied") : "📋 " + t("common.copy")}
        </button>
      </div>
      {libs && (
        <div className="codegen-libs" role="tablist" aria-label={t("codegen.library")}>
          {libs.map((l) => (
            <button
              key={l.value}
              role="tab"
              aria-selected={lib === l.value}
              className={`codegen-lib ${lib === l.value ? "active" : ""}`}
              onClick={() => setLib(l.value)}
            >
              {l.label}
            </button>
          ))}
        </div>
      )}
      {activeLib?.hint && <div className="codegen-lib-hint">💡 {t(activeLib.hint)}</div>}
      <pre className="codegen-pre">
        <code
          className="hljs codegen-code"
          dangerouslySetInnerHTML={{ __html: html }}
        />
      </pre>
      <div className="codegen-hint">{t("codegen.hint")}</div>
    </div>
  );
}
