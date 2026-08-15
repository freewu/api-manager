import { useMemo, useState } from "react";
import { ApiFile } from "../types";
import { CODE_LANGS, CodeLang, generateRequestCode } from "../utils/codegen";

interface Props {
  api: ApiFile;
  baseUrl: string;
  defaultLang: string;
}

export function CodeTab({ api, baseUrl, defaultLang }: Props) {
  const [lang, setLang] = useState<CodeLang>(
    (CODE_LANGS.some((l) => l.value === defaultLang) ? defaultLang : "curl") as CodeLang
  );
  const [copied, setCopied] = useState(false);

  const code = useMemo(() => generateRequestCode(lang, api, baseUrl), [lang, api, baseUrl]);

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
      <textarea className="code-area codegen-area" value={code} readOnly spellCheck={false} />
      <div style={{ color: "var(--text-faint)", fontSize: 11, marginTop: 6 }}>
        代码自动跟随当前请求的 URL、Headers、Params、Path 与 Body；可在「设置 → 功能」中更改默认语言。
      </div>
    </div>
  );
}
