import { Fragment, useEffect, useMemo, useState } from "react";
import { ApiFile, BODY_MODES, DOC_TYPES, DocParam, DocSource, KeyValue, METHODS, emptyDocParam } from "../types";
import { KeyValueEditor } from "./KeyValueEditor";
import { CodeTab } from "./CodeTab";
import { ExamplesTab } from "./ExamplesTab";
import { useT } from "../i18n";
import { pickFile } from "../commands";

/** 简易 XML 格式化：按标签层级缩进（支持注释 / CDATA / 声明 / 自闭合标签） */
function prettyXml(src: string): string {
  const re =
    /<!--[\s\S]*?-->|<![CDATA\[[\s\S]*?\]\]>|<\?[\s\S]*?\?>|<!DOCTYPE[\s\S]*?(?:\[[\s\S]*?\]\s*)?>|<\/?[^>]*>/g;
  const tokens: string[] = [];
  let last = 0;
  for (const m of src.matchAll(re)) {
    if (m.index !== undefined && m.index > last) tokens.push(src.slice(last, m.index));
    tokens.push(m[0]);
    last = (m.index ?? 0) + m[0].length;
  }
  if (last < src.length) tokens.push(src.slice(last));

  const out: string[] = [];
  const stack: string[] = [];
  let depth = 0;
  const indent = () => "  ".repeat(depth);

  for (const tok of tokens) {
    if (!tok.startsWith("<")) {
      const s = tok.trim();
      if (s) out.push(indent() + s);
      continue;
    }
    if (
      tok.startsWith("<!--") ||
      tok.startsWith("<![CDATA[") ||
      tok.startsWith("<?") ||
      tok.startsWith("<!DOCTYPE")
    ) {
      out.push(indent() + tok.trim());
      continue;
    }
    if (tok.startsWith("</")) {
      const name = tok.slice(2, -1).trim().split(/\s/)[0];
      if (stack.pop() !== name) throw new Error("mismatched tag");
      depth = Math.max(0, depth - 1);
      out.push(indent() + tok);
      continue;
    }
    const selfClose = /\/\s*>$/.test(tok);
    if (selfClose) {
      out.push(indent() + tok);
    } else {
      out.push(indent() + tok);
      stack.push(tok.slice(1).trim().split(/[\s/>]/)[0]);
      depth++;
    }
  }
  if (stack.length) throw new Error("unclosed tag");
  return out.join("\n");
}

type Tab = "params" | "path" | "headers" | "body" | "mock" | "desc" | "doc" | "code" | "examples";

interface Props {
  api: ApiFile;
  baseUrl: string;
  onChange: (api: ApiFile) => void;
  onSend: () => void;
  onSaveVersion: () => void;
  enableVersion: boolean;
  sending: boolean;
  /** 当前接口已保存的最新版本号（保存按钮 tip 展示） */
  currentVersion?: number;
  style?: React.CSSProperties;
  /** 失焦后自动保存（接口说明 textarea blur 时触发） */
  onCommit?: () => void;
  /** 是否启用代码生成（显示「代码」页签） */
  enableCodegen?: boolean;
  /** 是否启用 Mock（设置关闭时隐藏 Mock 页签） */
  enableMock?: boolean;
  /** 代码生成默认语言（bash / python / c / cpp / java / csharp / ...） */
  codegenLang?: string;
  /** 页签切换回调（App 据此隐藏/显示响应面板） */
  onTabChange?: (tab: string) => void;
}

export function Editor({ api, baseUrl, onChange, onSend, onSaveVersion, enableVersion, sending, style, onCommit, enableCodegen = true, enableMock = true, codegenLang = "bash", onTabChange, currentVersion = 0 }: Props) {
  const t = useT();
  const [tab, setTab] = useState<Tab>("params");
  /** JSON 格式化失败提示（body / mock 页签共用） */
  const [formatError, setFormatError] = useState<string | null>(null);
  const effectiveUrl = api.url || (baseUrl + api.path);

  const switchTab = (t: Tab) => {
    setTab(t);
    onTabChange?.(t);
  };

  // 切换接口时回到 Query 页签
  useEffect(() => {
    setTab("params");
    setFormatError(null);
    onTabChange?.("params");
  }, [api.uuid]);

  // 设置中全局关闭 Mock 时，若当前停留在 Mock 页签则切回 Query
  useEffect(() => {
    if (!enableMock && tab === "mock") {
      setTab("params");
      onTabChange?.("params");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enableMock]);

  // URL / 路径中的 {xx} 占位符实时同步到 Path 页签（新增或删除）；
  // {{xx}} 是全局环境变量（双大括号），不会被当作路径参数
  const pathSource = api.url || api.path;
  useEffect(() => {
    const names = new Set(
      [...pathSource.matchAll(/(?<!\{)\{([^{}]+)\}(?!\})/g)]
        .map((m) => m[1].trim())
        .filter(Boolean)
    );
    const cur = api.params;
    const missing = [...names].filter((n) => !cur.some((r) => r.key.trim() === n));
    const stale = cur.filter((r) => r.key.trim() && !names.has(r.key.trim()));
    if (missing.length === 0 && stale.length === 0) return;
    const next = cur
      .filter((r) => !r.key.trim() || names.has(r.key.trim()))
      .concat(
        missing.map((n) => ({ key: n, value: "", enabled: true, description: "" }))
      );
    onChange({ ...api, params: next });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pathSource]);

  const set = (patch: Partial<ApiFile>) => onChange({ ...api, ...patch });

  const enabledCount = (rows: KeyValue[]) => rows.filter((r) => r.enabled && r.key).length;

  /** 将原始文本按 JSON 格式化（2 空格缩进）；非 JSON 时提示错误 */
  const formatJson = (raw: string, onFormatted: (text: string) => void) => {
    setFormatError(null);
    const text = raw.trim();
    if (!text) return;
    try {
      onFormatted(JSON.stringify(JSON.parse(text), null, 2));
    } catch {
      setFormatError(t("editor.formatJsonFailed"));
    }
  };

  /** 将原始文本按 XML 格式化（缩进排版）；非合法 XML 时提示错误 */
  const formatXml = (raw: string, onFormatted: (text: string) => void) => {
    setFormatError(null);
    const text = raw.trim();
    if (!text) return;
    try {
      onFormatted(prettyXml(text));
    } catch {
      setFormatError(t("editor.formatXmlFailed"));
    }
  };

  /** 二进制模式：弹出系统文件选择框，记录文件路径 */
  const pickBinaryFile = async () => {
    try {
      const p = await pickFile();
      if (p) set({ body: { ...api.body, binaryPath: p } });
    } catch {
      /* 用户取消或出错时忽略 */
    }
  };

  return (
    <div className="editor" style={style}>
      <div className="editor-head">
        <select
          className="method-select"
          value={api.method}
          onChange={(e) => set({ method: e.target.value })}
        >
          {METHODS.map((m) => (
            <option key={m} value={m}>
              {m}
            </option>
          ))}
        </select>
        <div className="url-input-wrap">
          <span className="url-scheme">URL</span>
          <input
            className="url-input"
            value={effectiveUrl}
            placeholder="https://api.example.com/v1/users"
            title={t("editor.urlTitle")}
            onChange={(e) => {
              const v = e.target.value;
              if (v.startsWith(baseUrl) && baseUrl) {
                onChange({ ...api, url: "", path: v.slice(baseUrl.length) || "/" });
              } else {
                onChange({ ...api, url: v, path: api.path });
              }
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !sending) onSend();
            }}
            spellCheck={false}
          />
        </div>
        <button className="send-btn" onClick={onSend} disabled={sending}>
          {sending ? t("tab.sending") : t("tab.send")}
        </button>
        {enableVersion && (
          <button
            className="save-btn"
            onClick={onSaveVersion}
            title={
              currentVersion > 0
                ? t("tab.currentVersion", { v: currentVersion })
                : t("tab.noVersion")
            }
          >
            💾 {t("tab.save")}
          </button>
        )}
      </div>

      <div className="tabs">
        <div className={`tab ${tab === "params" ? "active" : ""}`} onClick={() => switchTab("params")}>
          Query{enabledCount(api.query) > 0 && <span className="count">{enabledCount(api.query)}</span>}
        </div>
        <div className={`tab ${tab === "path" ? "active" : ""}`} onClick={() => switchTab("path")}>
          Path{enabledCount(api.params) > 0 && <span className="count">{enabledCount(api.params)}</span>}
        </div>
        <div className={`tab ${tab === "headers" ? "active" : ""}`} onClick={() => switchTab("headers")}>
          Headers{enabledCount(api.headers) > 0 && <span className="count">{enabledCount(api.headers)}</span>}
        </div>
        <div
          className={`tab ${tab === "body" ? "active" : ""}`}
          onClick={() => switchTab("body")}
        >
          Body
          {api.body.mode !== "none" &&
            ((api.body.mode === "binary" && api.body.binaryPath) ||
              (api.body.mode !== "binary" && api.body.raw)) && (
              <span className="count">•</span>
            )}
        </div>
        {enableMock && (
          <div className={`tab ${tab === "mock" ? "active" : ""}`} onClick={() => switchTab("mock")}>
            Mock{api.mock.enabled && <span className="count">●</span>}
          </div>
        )}
        <div className={`tab ${tab === "desc" ? "active" : ""}`} onClick={() => switchTab("desc")}>
          {t("editor.descTab")}
        </div>
        <div className={`tab ${tab === "doc" ? "active" : ""}`} onClick={() => switchTab("doc")}>
          {t("tab.doc")}
        </div>
        {enableCodegen && (
          <div className={`tab ${tab === "code" ? "active" : ""}`} onClick={() => switchTab("code")}>
            {t("editor.codeTab")}
          </div>
        )}
        <div className={`tab ${tab === "examples" ? "active" : ""}`} onClick={() => switchTab("examples")}>
          {t("tab.examples")}
        </div>
      </div>

      <div className="editor-body">
        {tab === "params" && (
          <div>
            <KeyValueEditor
              rows={api.query}
              onChange={(rows) => set({ query: rows })}
              keyPlaceholder={t("editor.paramName")}
            />
            <div className="section-title">
              {t("editor.queryParams")} <span className="help">{t("editor.queryParamsHint")}</span>
            </div>
          </div>
        )}

        {tab === "path" && (
          <div>
            <div className="section-title">
              {t("editor.pathVars")} <span className="help">{t("editor.pathVarsHint")}</span>
            </div>
            {api.params.length === 0 ? (
              <div style={{ color: "var(--text-faint)", fontSize: 12, padding: "4px 2px" }}>
                {t("editor.pathEmpty")}
              </div>
            ) : (
              <KeyValueEditor
                rows={api.params}
                onChange={(rows) => set({ params: rows })}
                keyPlaceholder={t("editor.varName")}
                valuePlaceholder={t("editor.sampleValue")}
                showDescription
                showCheck={false}
                hideAdd
                hideRemove
                readonlyKey
              />
            )}
            <div style={{ color: "var(--text-faint)", fontSize: 11, marginTop: 6 }}>
              {t("editor.pathHint")}
            </div>
          </div>
        )}

        {tab === "headers" && (
          <KeyValueEditor
            rows={api.headers}
            onChange={(rows) => set({ headers: rows })}
            keyPlaceholder={t("editor.headerName")}
          />
        )}

        {tab === "body" && (
          <div>
            <div className="body-modes">
              {BODY_MODES.map((m) => (
                <div
                  key={m}
                  className={`body-mode ${api.body.mode === m ? "active" : ""}`}
                  onClick={() => set({ body: { ...api.body, mode: m } })}
                >
                  {m === "none"
                    ? t("editor.none")
                    : m === "raw"
                      ? t("editor.raw")
                      : m === "json"
                        ? "JSON"
                        : m === "xml"
                          ? "XML"
                          : m === "binary"
                            ? t("editor.binary")
                            : t("editor.form")}
                </div>
              ))}
            </div>
            {api.body.mode === "none" && (
              <div style={{ color: "var(--text-faint)", fontSize: 12 }}>{t("editor.noBody")}</div>
            )}
            {api.body.mode === "form" && (
              <>
                <KeyValueEditor
                  rows={api.body.form}
                  onChange={(rows) => set({ body: { ...api.body, form: rows } })}
                  keyPlaceholder={t("editor.fieldName")}
                  valuePlaceholder={undefined}
                  showFileType
                />
                <div style={{ color: "var(--text-faint)", fontSize: 11, marginTop: 6 }}>
                  {t("editor.fileTypeHint")}
                </div>
              </>
            )}
            {(api.body.mode === "raw" || api.body.mode === "json" || api.body.mode === "xml") && (
              <div className="body-raw-wrap">
                <div className="body-raw-toolbar">
                  {api.body.mode === "json" ? (
                    <button
                      className="btn small"
                      onClick={() =>
                        formatJson(api.body.raw, (text) =>
                          set({ body: { ...api.body, raw: text } })
                        )
                      }
                      title={t("editor.formatJsonTip")}
                    >
                      {t("editor.formatJson")}
                    </button>
                  ) : api.body.mode === "xml" ? (
                    <button
                      className="btn small"
                      onClick={() =>
                        formatXml(api.body.raw, (text) =>
                          set({ body: { ...api.body, raw: text } })
                        )
                      }
                      title={t("editor.formatXmlTip")}
                    >
                      {t("editor.formatXml")}
                    </button>
                  ) : null}
                  {formatError && <span className="body-format-error">{formatError}</span>}
                </div>
                <textarea
                  className="code-area"
                  value={api.body.raw}
                  placeholder={
                    api.body.mode === "json"
                      ? '{\n  "key": "value"\n}'
                      : api.body.mode === "xml"
                        ? '<root>\n  <item>value</item>\n</root>'
                        : t("editor.bodyRaw")
                  }
                  onChange={(e) => set({ body: { ...api.body, raw: e.target.value } })}
                  spellCheck={false}
                />
              </div>
            )}
            {api.body.mode === "binary" && (
              <div className="binary-picker">
                <button className="btn" onClick={pickBinaryFile}>
                  📁 {t("editor.pickFile")}
                </button>
                {api.body.binaryPath ? (
                  <div className="binary-file">
                    <span className="binary-file-path" title={api.body.binaryPath}>
                      📄 {api.body.binaryPath}
                    </span>
                    <button
                      className="btn small"
                      onClick={() => set({ body: { ...api.body, binaryPath: "" } })}
                    >
                      {t("editor.clearFile")}
                    </button>
                  </div>
                ) : (
                  <div style={{ color: "var(--text-faint)", fontSize: 12 }}>
                    {t("editor.binaryHint")}
                  </div>
                )}
              </div>
            )}
          </div>
        )}

        {enableMock && tab === "mock" && (
          <div>
            <div className="meta-row">
              <label className="meta-item" style={{ cursor: "pointer" }}>
                <input
                  type="checkbox"
                  checked={api.mock.enabled}
                  onChange={(e) => set({ mock: { ...api.mock, enabled: e.target.checked } })}
                  style={{ width: "auto" }}
                />
                {t("editor.enableMockHint")}
              </label>
            </div>
            <div className="meta-row">
              <label className="meta-item">
                {t("editor.statusCode")}
                <input
                  type="number"
                  min={100}
                  max={599}
                  value={api.mock.status}
                  onChange={(e) =>
                    set({ mock: { ...api.mock, status: Number(e.target.value) || 200 } })
                  }
                />
              </label>
              <label className="meta-item">
                {t("editor.delayMs")}
                <input
                  type="number"
                  min={0}
                  value={api.mock.delay}
                  onChange={(e) =>
                    set({ mock: { ...api.mock, delay: Number(e.target.value) || 0 } })
                  }
                />
              </label>
            </div>
            <div className="section-title">{t("editor.respHeaders")}</div>
            <KeyValueEditor
              rows={api.mock.headers}
              onChange={(rows) => set({ mock: { ...api.mock, headers: rows } })}
              keyPlaceholder={t("editor.headerName")}
            />
            <div className="section-title">
              {t("editor.respBody")} <span className="help">{t("editor.respBodyHint")}</span>
            </div>
            <textarea
              className="code-area"
              value={api.mock.body}
              placeholder={'{\n  "code": 0,\n  "data": null\n}'}
              onChange={(e) => set({ mock: { ...api.mock, body: e.target.value } })}
              spellCheck={false}
            />
          </div>
        )}

        {tab === "desc" && (
          <div className="desc-root">
            <textarea
              className="desc-area"
              value={api.description}
              placeholder={t("editor.descPlaceholder")}
              onChange={(e) => set({ description: e.target.value })}
              onBlur={onCommit}
              spellCheck={false}
            />
            <div className="desc-hint">{t("editor.descHint")}</div>
          </div>
        )}

        {tab === "doc" && <DocParamsEditor api={api} set={set} />}

        {tab === "code" && enableCodegen && <CodeTab api={api} baseUrl={baseUrl} defaultLang={codegenLang} />}
        {tab === "examples" && <ExamplesTab uuid={api.uuid} api={api} onChange={onChange} />}
      </div>
    </div>
  );
}

/** 接口文档：按 请求Header / Query / Path / Body / 响应 分块（没有值的块不渲染）；
 *  响应分为「请求成功」（从 Mock 响应体 JSON 推导）与「请求失败」（手动添加）两种情况；
 *  字段类型可选 String / Integer / Float / Boolean / List / Object，
 *  List 可再选元素类型，Object 可设置对象名称，下级字段用树状表单表示 */
function DocParamsEditor({ api, set }: { api: ApiFile; set: (p: Partial<ApiFile>) => void }) {
  const T = useT();
  // ---- 树节点（由请求配置 / Mock 响应 JSON 推导） ----
  type RNode = {
    key: string;
    value: string;
    guess: string; // 推导类型
    guessItem?: string; // List 推导元素类型
    children?: RNode[];
  };

  // 旧文档里的自由文本类型 → 规范化到下拉选项
  const normalizeType = (t: string): string => {
    const map: Record<string, string> = {
      string: "String",
      str: "String",
      text: "String",
      number: "Integer",
      int: "Integer",
      integer: "Integer",
      long: "Integer",
      float: "Float",
      double: "Float",
      decimal: "Float",
      bool: "Boolean",
      boolean: "Boolean",
      list: "List",
      array: "List",
      object: "Object",
      map: "Object",
    };
    const k = t.trim().toLowerCase();
    return map[k] || t.trim();
  };

  const guessType = (v: unknown): string => {
    if (v === null || v === undefined) return "String";
    if (typeof v === "boolean") return "Boolean";
    if (typeof v === "number") return Number.isInteger(v) ? "Integer" : "Float";
    if (Array.isArray(v)) return "List";
    if (typeof v === "object") return "Object";
    return "String";
  };

  const guessFromText = (s: string): string => {
    const t = s.trim();
    if (!t) return "String";
    if (/^-?\d+$/.test(t)) return "Integer";
    if (/^-?\d*\.\d+$/.test(t)) return "Float";
    if (t === "true" || t === "false") return "Boolean";
    return "String";
  };

  const fmt = (v: unknown): string => {
    if (typeof v === "string") return v;
    if (v === null || v === undefined) return "null";
    const s = JSON.stringify(v);
    return s && s.length > 40 ? s.slice(0, 40) + "…" : s || "";
  };

  // JSON 值 → 树节点（对象展开为下级，数组为 List 节点并推导元素类型）
  const jsonToNodes = (val: unknown): RNode[] => {
    if (val === null || typeof val !== "object") return [];
    if (Array.isArray(val)) {
      const first = val[0];
      return [
        {
          key: "items",
          value: fmt(val),
          guess: "List",
          guessItem: first === undefined || first === null ? "String" : guessType(first),
          children:
            first !== undefined && first !== null && typeof first === "object" && !Array.isArray(first)
              ? jsonToNodes(first)
              : undefined,
        },
      ];
    }
    return Object.entries(val).map(([k, v]) => {
      if (Array.isArray(v)) {
        const first = v[0];
        return {
          key: k,
          value: fmt(v),
          guess: "List",
          guessItem: first === undefined || first === null ? "String" : guessType(first),
          children:
            first !== undefined && first !== null && typeof first === "object" && !Array.isArray(first)
              ? jsonToNodes(first)
              : undefined,
        };
      }
      if (v !== null && typeof v === "object") {
        return { key: k, value: fmt(v), guess: "Object", children: jsonToNodes(v) };
      }
      return { key: k, value: fmt(v), guess: guessType(v) };
    });
  };

  const kvNodes = (rows: KeyValue[]): RNode[] =>
    rows
      .filter((r) => r.key.trim())
      .map((r) => ({ key: r.key, value: r.value, guess: guessFromText(r.value) }));

  // ---- docParams 定位（按 source + key 路径） ----
  const getDocAt = (source: DocSource, keys: string[]): DocParam | undefined => {
    let arr = api.docParams;
    let cur: DocParam | undefined;
    for (const k of keys) {
      cur = arr.find((d) => d.source === source && d.key === k);
      if (!cur) return undefined;
      arr = cur.children || [];
    }
    return cur;
  };

  const updateDocAt = (source: DocSource, keys: string[], patch: Partial<DocParam>) => {
    const next = [...api.docParams];
    let arr = next;
    let cur: DocParam | undefined;
    for (let i = 0; i < keys.length; i++) {
      const k = keys[i];
      const idx = arr.findIndex((d) => d.source === source && d.key === k);
      if (idx >= 0) {
        cur = arr[idx];
      } else {
        cur = emptyDocParam(source);
        cur.key = k;
        arr.push(cur);
      }
      if (i < keys.length - 1) {
        if (!cur.children) cur.children = [];
        arr = cur.children;
      }
    }
    if (cur) Object.assign(cur, patch);
    set({ docParams: next });
  };

  const addDocRow = (source: DocSource, parentKeys: string[]) => {
    const next = [...api.docParams];
    let arr = next;
    for (const pk of parentKeys) {
      const idx = arr.findIndex((d) => d.source === source && d.key === pk);
      if (idx >= 0) {
        if (!arr[idx].children) arr[idx].children = [];
        arr = arr[idx].children;
      } else {
        const n = emptyDocParam(source);
        n.key = pk;
        arr.push(n);
        arr = n.children;
      }
    }
    arr.push(emptyDocParam(source));
    set({ docParams: next });
  };

  const removeDocRow = (source: DocSource, keys: string[]) => {
    const parentKeys = keys.slice(0, -1);
    const key = keys[keys.length - 1];
    const next = [...api.docParams];
    let arr = next;
    for (const pk of parentKeys) {
      const idx = arr.findIndex((d) => d.source === source && d.key === pk);
      if (idx < 0) return;
      arr = arr[idx].children || [];
    }
    const idx = arr.findIndex((d) => d.source === source && d.key === key);
    if (idx >= 0) arr.splice(idx, 1);
    set({ docParams: next });
  };

  // ---- 分块推导（请求侧来自真实配置，响应侧来自 Mock 体 / 手动条目） ----
  type Block = { source: DocSource; title: string; nodes: RNode[]; manual: boolean };

  const blocks = useMemo<Block[]>(() => {
    const out: Block[] = [];
    const headerNodes = kvNodes(api.headers);
    if (headerNodes.length) out.push({ source: "header", title: T("editor.requestHeader"), nodes: headerNodes, manual: false });
    const queryNodes = kvNodes(api.query);
    if (queryNodes.length) out.push({ source: "query", title: "Query", nodes: queryNodes, manual: false });
    const pathNodes = kvNodes(api.params);
    if (pathNodes.length) out.push({ source: "path", title: "Path", nodes: pathNodes, manual: false });
    let bodyNodes: RNode[] = [];
    if (api.body.mode === "form") {
      bodyNodes = kvNodes(api.body.form);
    } else if (api.body.mode === "json") {
      try {
        bodyNodes = jsonToNodes(JSON.parse(api.body.raw));
      } catch {
        /* JSON 无法解析时不生成 */
      }
    }
    if (bodyNodes.length) out.push({ source: "body", title: "Body", nodes: bodyNodes, manual: false });
    return out;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [api]);

  let mockJson: unknown = null;
  try {
    mockJson = JSON.parse(api.mock.body);
  } catch {
    /* 非 JSON 响应体则无法推导 */
  }
  const respSuccessNodes: RNode[] = useMemo(() => {
    if (mockJson === null) return [];
    const nodes = jsonToNodes(mockJson);
    // JSON 根节点是普通对象时展开成字段列表；数组根会生成一个 items 节点
    return nodes;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [api.mock.body]);
  const failDocs: DocParam[] = api.docParams.filter((d) => d.source === "resp_fail");

  if (blocks.length === 0 && respSuccessNodes.length === 0 && failDocs.length === 0) {
    // 完全空时也保留「请求失败」添加入口，便于编写响应文档
    return (
      <div>
        <div className="section-title">
          {T("tab.doc")} <span className="help">{T("editor.docBlockHint")}</span>
        </div>
        <div style={{ color: "var(--text-faint)", fontSize: 12, padding: "0 2px 10px" }}>
          {T("editor.noParams")}
        </div>
        <div className="doc-block doc-block-resp">
          <div className="doc-block-title">
            <span className="doc-source doc-source-resp">{T("editor.response")}</span>
          </div>
          <div className="doc-sub">
            <div className="doc-sub-title">
              {T("editor.requestFailed")}
              <button className="btn btn-sm" onClick={() => addDocRow("resp_fail", [])}>
                {T("editor.addField")}
              </button>
            </div>
            <table className="kv-table doc-params-table">
              <thead>
                <tr>
                  <th>{T("editor.fieldName")}</th>
                  <th style={{ width: 130 }}>{T("common.value")}</th>
                  <th style={{ width: 108 }}>{T("kv.type")}</th>
                  <th style={{ width: 120 }}>{T("editor.objectName")}</th>
                  <th>{T("kv.desc")}</th>
                  <th style={{ width: 62 }}>{T("common.operation")}</th>
                </tr>
              </thead>
              <tbody>
                {failDocs.length === 0 ? (
                  <tr>
                    <td colSpan={6} className="doc-empty">
                      {T("editor.noFailFields")}
                    </td>
                  </tr>
                ) : (
                  failDocs.map((d) => renderRow(manualView(d, "resp_fail", []), 0, "resp_fail", true, true))
                )}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    );
  }

  // ---- 行视图（统一 树形/手动 两种来源的渲染） ----
  type RowView = {
    keys: string[];
    key: string;
    keyEditable: boolean;
    value: string;
    type: string;
    typeAuto: boolean;
    objectName: string;
    description: string;
    children: RowView[];
    removable: boolean;
  };

  const derivedView = (node: RNode, source: DocSource, parentKeys: string[]): RowView => {
    const keys = [...parentKeys, node.key];
    const doc = getDocAt(source, keys);
    const storedType = doc ? normalizeType(doc.type) : "";
    return {
      keys,
      key: node.key,
      keyEditable: false,
      value: node.value,
      type: storedType || node.guess || "",
      typeAuto: !storedType,
      objectName: doc?.objectName || node.key,
      description: doc?.description || "",
      children: (node.children || []).map((c) => derivedView(c, source, keys)),
      removable: false,
    };
  };

  const manualView = (d: DocParam, source: DocSource, parentKeys: string[]): RowView => {
    const keys = [...parentKeys, d.key];
    const t = normalizeType(d.type) || (d.children && d.children.length ? "Object" : "");
    return {
      keys,
      key: d.key,
      keyEditable: true,
      value: "",
      type: t,
      typeAuto: false,
      objectName: d.objectName || d.key,
      description: d.description || "",
      children: (d.children || []).map((c) => manualView(c, source, keys)),
      removable: true,
    };
  };

  const updateType = (source: DocSource, keys: string[], v: string) =>
    updateDocAt(source, keys, { type: v });
  const updateName = (source: DocSource, keys: string[], v: string) =>
    updateDocAt(source, keys, { objectName: v });
  const updateDesc = (source: DocSource, keys: string[], v: string) =>
    updateDocAt(source, keys, { description: v });
  const updateKey = (source: DocSource, keys: string[], v: string) =>
    updateDocAt(source, keys, { key: v });

  const renderRow = (row: RowView, depth: number, source: DocSource, manual: boolean, showObjectName: boolean) => {
    const isObject = row.type === "Object";
    return (
      <Fragment key={row.keys.join("/")}>
        <tr>
          <td style={{ paddingLeft: 10 + depth * 22 }}>
            {row.keyEditable ? (
              <input
                className="doc-key-input"
                value={row.key}
                placeholder={T("editor.fieldName")}
                spellCheck={false}
                onChange={(e) => updateKey(source, row.keys, e.target.value)}
              />
            ) : (
              <span className="doc-key">{row.key}</span>
            )}
          </td>
          <td>
            <span className={`doc-value${row.keyEditable ? " doc-value-manual" : ""}`}>
              {row.value || "—"}
            </span>
          </td>
          <td>
            <select
              className={`doc-type-select${row.typeAuto ? " doc-type-auto" : ""}`}
              value={row.type}
              title={row.typeAuto ? T("editor.typeAutoHint") : T("kv.fileType")}
              onChange={(e) => updateType(source, row.keys, e.target.value)}
            >
              <option value="">{T("editor.auto")}</option>
              {DOC_TYPES.map((t) => (
                <option key={t} value={t}>
                  {t}
                </option>
              ))}
            </select>
          </td>
          {showObjectName && (
            <td>
              {isObject && (
                <input
                  className="doc-name-input"
                  value={row.objectName}
                  placeholder={row.key}
                  title={T("editor.objectName")}
                  spellCheck={false}
                  onChange={(e) => updateName(source, row.keys, e.target.value)}
                />
              )}
            </td>
          )}
          <td>
            <input
              value={row.description}
              placeholder={T("editor.fieldDesc")}
              spellCheck={false}
              onChange={(e) => updateDesc(source, row.keys, e.target.value)}
            />
          </td>
          <td className="doc-ops">
            {manual && isObject && (
              <button className="doc-op" title={T("editor.addSubField")} onClick={() => addDocRow(source, row.keys)}>
                ＋
              </button>
            )}
            {manual && (
              <button className="doc-op doc-op-del" title={T("editor.delField")} onClick={() => removeDocRow(source, row.keys)}>
                ✕
              </button>
            )}
          </td>
        </tr>
        {row.children.map((c) => renderRow(c, depth + 1, source, manual, showObjectName))}
      </Fragment>
    );
  };

  const renderBlock = (title: string, badgeClass: string, rows: RowView[], source: DocSource, manual: boolean, showObjectName: boolean) => (
    <div className="doc-block">
      <div className="doc-block-title">
        <span className={`doc-source ${badgeClass}`}>{title}</span>
        {manual && (
          <button className="btn btn-sm" onClick={() => addDocRow(source, [])}>
            {T("editor.addField")}
          </button>
        )}
      </div>
      <table className="kv-table doc-params-table">
        <thead>
          <tr>
            <th>{T("editor.fieldName")}</th>
            <th style={{ width: 130 }}>{T("common.value")}</th>
            <th style={{ width: 108 }}>{T("kv.type")}</th>
            {showObjectName && <th style={{ width: 120 }}>{T("editor.objectName")}</th>}
            <th>{T("kv.desc")}</th>
            {manual && <th style={{ width: 62 }}>{T("common.operation")}</th>}
          </tr>
        </thead>
        <tbody>{rows.map((r) => renderRow(r, 0, source, manual, showObjectName))}</tbody>
      </table>
    </div>
  );

  const badgeFor = (source: DocSource) =>
    source === "header"
      ? "doc-source-header"
      : source === "query"
        ? "doc-source-query"
        : source === "path"
          ? "doc-source-path"
          : source === "body"
            ? "doc-source-body"
            : source === "resp_success"
              ? "doc-source-resp-ok"
              : "doc-source-resp-fail";

  return (
    <div>
      <div className="section-title">
        {T("tab.doc")} <span className="help">{T("editor.docBlockHint")}</span>
      </div>
      {blocks.map((b) =>
        renderBlock(
          b.title,
          badgeFor(b.source),
          b.nodes.map((n) => derivedView(n, b.source, [])),
          b.source,
          false,
          b.source === "body"
        )
      )}
      {(respSuccessNodes.length > 0 || failDocs.length > 0) && (
        <div className="doc-block doc-block-resp">
          <div className="doc-block-title">
            <span className="doc-source doc-source-resp">{T("editor.response")}</span>
          </div>
          {respSuccessNodes.length > 0 && (
            <div className="doc-sub">
              <div className="doc-sub-title">{T("editor.requestSuccess")}</div>
              <table className="kv-table doc-params-table">
                <thead>
                  <tr>
                    <th>{T("editor.fieldName")}</th>
                    <th style={{ width: 130 }}>{T("common.value")}</th>
                    <th style={{ width: 108 }}>{T("kv.type")}</th>
                    <th style={{ width: 120 }}>{T("editor.objectName")}</th>
                    <th>{T("kv.desc")}</th>
                  </tr>
                </thead>
                <tbody>
                  {respSuccessNodes.map((n) =>
                    renderRow(derivedView(n, "resp_success", []), 0, "resp_success", false, true)
                  )}
                </tbody>
              </table>
            </div>
          )}
          <div className="doc-sub">
            <div className="doc-sub-title">
              {T("editor.requestFailed")}
              <button className="btn btn-sm" onClick={() => addDocRow("resp_fail", [])}>
                {T("editor.addField")}
              </button>
            </div>
            <table className="kv-table doc-params-table">
              <thead>
                <tr>
                  <th>{T("editor.fieldName")}</th>
                  <th style={{ width: 130 }}>{T("common.value")}</th>
                  <th style={{ width: 108 }}>{T("kv.type")}</th>
                  <th style={{ width: 120 }}>{T("editor.objectName")}</th>
                  <th>{T("kv.desc")}</th>
                  <th style={{ width: 62 }}>{T("common.operation")}</th>
                </tr>
              </thead>
              <tbody>
                {failDocs.length === 0 ? (
                  <tr>
                    <td colSpan={6} className="doc-empty">
                      {T("editor.noFailFields")}
                    </td>
                  </tr>
                ) : (
                  failDocs.map((d) => renderRow(manualView(d, "resp_fail", []), 0, "resp_fail", true, true))
                )}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
