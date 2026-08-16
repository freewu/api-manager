import { Fragment, useEffect, useMemo, useState } from "react";
import { ApiFile, BODY_MODES, DOC_TYPES, DocParam, DocSource, KeyValue, METHODS, emptyDocParam } from "../types";
import { KeyValueEditor } from "./KeyValueEditor";
import { CodeTab } from "./CodeTab";
import { ExamplesTab } from "./ExamplesTab";

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
  /** 代码生成默认语言（bash / python / c / cpp / java / csharp / ...） */
  codegenLang?: string;
  /** 页签切换回调（App 据此隐藏/显示响应面板） */
  onTabChange?: (tab: string) => void;
}

export function Editor({ api, baseUrl, onChange, onSend, onSaveVersion, enableVersion, sending, style, onCommit, enableCodegen = true, codegenLang = "bash", onTabChange, currentVersion = 0 }: Props) {
  const [tab, setTab] = useState<Tab>("params");
  const effectiveUrl = api.url || (baseUrl + api.path);

  const switchTab = (t: Tab) => {
    setTab(t);
    onTabChange?.(t);
  };

  // 切换接口时回到 Query 页签
  useEffect(() => {
    setTab("params");
    onTabChange?.("params");
  }, [api.uuid]);

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
            title="{变量名} 为路径参数（在 Path 页签赋值）；{{变量名}} 为全局环境变量（请求时自动替换）"
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
          {sending ? "发送中…" : "发送"}
        </button>
        {enableVersion && (
          <button
            className="save-btn"
            onClick={onSaveVersion}
            title={
              currentVersion > 0
                ? `当前版本：${currentVersion}（点击将当前内容保存为新版本）`
                : "当前版本：暂无（点击保存生成第一个版本）"
            }
          >
            💾 保存
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
          Body{api.body.mode !== "none" && api.body.raw && <span className="count">•</span>}
        </div>
        <div className={`tab ${tab === "mock" ? "active" : ""}`} onClick={() => switchTab("mock")}>
          Mock{api.mock.enabled && <span className="count">●</span>}
        </div>
        <div className={`tab ${tab === "desc" ? "active" : ""}`} onClick={() => switchTab("desc")}>
          接口描述
        </div>
        <div className={`tab ${tab === "doc" ? "active" : ""}`} onClick={() => switchTab("doc")}>
          接口文档
        </div>
        {enableCodegen && (
          <div className={`tab ${tab === "code" ? "active" : ""}`} onClick={() => switchTab("code")}>
            生成代码
          </div>
        )}
        <div className={`tab ${tab === "examples" ? "active" : ""}`} onClick={() => switchTab("examples")}>
          示例
        </div>
      </div>

      <div className="editor-body">
        {tab === "params" && (
          <div>
            <KeyValueEditor
              rows={api.query}
              onChange={(rows) => set({ query: rows })}
              keyPlaceholder="参数名"
              valuePlaceholder="值"
            />
            <div className="section-title">
              查询参数 <span className="help">（发送请求时拼接到 URL 问号后面）</span>
            </div>
          </div>
        )}

        {tab === "path" && (
          <div>
            <div className="section-title">
              Path 变量 <span className="help">（与上方 URL 中的 {`{name}`} 一一对应，自动同步；{`{{name}}`} 为全局环境变量，不在此列；只做赋值与说明，不可增删）</span>
            </div>
            {api.params.length === 0 ? (
              <div style={{ color: "var(--text-faint)", fontSize: 12, padding: "4px 2px" }}>
                暂无路径参数，可在顶部 URL 中使用 {`{变量名}`}，例如 /users/{`{id}`}（{`{{变量名}}`} 表示全局环境变量，不会被当作路径参数）
              </div>
            ) : (
              <KeyValueEditor
                rows={api.params}
                onChange={(rows) => set({ params: rows })}
                keyPlaceholder="变量名"
                valuePlaceholder="示例值"
                showDescription
                showCheck={false}
                hideAdd
                hideRemove
                readonlyKey
              />
            )}
            <div style={{ color: "var(--text-faint)", fontSize: 11, marginTop: 6 }}>
              单大括号 {`{变量名}`} 为路径参数（顶部 URL 定义，此处自动生成并保持一一对应，只做赋值与说明）；双大括号 {`{{变量名}}`} 为全局环境变量（来自环境设置，请求时自动替换）。多个示例值可用逗号分隔（如 1,2,3），发送请求时取第一个。
            </div>
          </div>
        )}

        {tab === "headers" && (
          <KeyValueEditor
            rows={api.headers}
            onChange={(rows) => set({ headers: rows })}
            keyPlaceholder="Header 名"
            valuePlaceholder="值"
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
                  {m === "none" ? "无" : m === "raw" ? "原始文本" : m === "json" ? "JSON" : "表单"}
                </div>
              ))}
            </div>
            {api.body.mode === "none" && (
              <div style={{ color: "var(--text-faint)", fontSize: 12 }}>该请求没有请求体</div>
            )}
            {api.body.mode === "form" && (
              <>
                <KeyValueEditor
                  rows={api.body.form}
                  onChange={(rows) => set({ body: { ...api.body, form: rows } })}
                  keyPlaceholder="字段名"
                  valuePlaceholder={undefined}
                  showFileType
                />
                <div style={{ color: "var(--text-faint)", fontSize: 11, marginTop: 6 }}>
                  字段类型选择「文件」后可点击选择本地文件，发送时自动使用 multipart/form-data 上传。
                </div>
              </>
            )}
            {(api.body.mode === "raw" || api.body.mode === "json") && (
              <textarea
                className="code-area"
                value={api.body.raw}
                placeholder={api.body.mode === "json" ? '{\n  "key": "value"\n}' : "请求体原始内容"}
                onChange={(e) => set({ body: { ...api.body, raw: e.target.value } })}
                spellCheck={false}
              />
            )}
          </div>
        )}

        {tab === "mock" && (
          <div>
            <div className="meta-row">
              <label className="meta-item" style={{ cursor: "pointer" }}>
                <input
                  type="checkbox"
                  checked={api.mock.enabled}
                  onChange={(e) => set({ mock: { ...api.mock, enabled: e.target.checked } })}
                  style={{ width: "auto" }}
                />
                启用 Mock（保存后需刷新 Mock 服务）
              </label>
            </div>
            <div className="meta-row">
              <label className="meta-item">
                状态码
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
                延迟(ms)
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
            <div className="section-title">响应 Headers</div>
            <KeyValueEditor
              rows={api.mock.headers}
              onChange={(rows) => set({ mock: { ...api.mock, headers: rows } })}
              keyPlaceholder="Header 名"
              valuePlaceholder="值"
            />
            <div className="section-title">
              响应体 <span className="help">支持模板变量 {`{{path.id}}`}、{`{{query.page}}`}、{`{{method}}`}</span>
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
              placeholder="描述该接口的用途、参数、返回值等"
              onChange={(e) => set({ description: e.target.value })}
              onBlur={onCommit}
              spellCheck={false}
            />
            <div className="desc-hint">
              提示：接口名称 / 路径可在左侧右键「重命名」或顶部 URL 输入框中修改；说明文字失焦后自动保存。
            </div>
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
    if (headerNodes.length) out.push({ source: "header", title: "请求 Header", nodes: headerNodes, manual: false });
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
          接口文档 <span className="help">按请求 Header / Query / Path / Body / 响应分块；响应区分请求成功与请求失败</span>
        </div>
        <div style={{ color: "var(--text-faint)", fontSize: 12, padding: "0 2px 10px" }}>
          暂无请求参数。可在 Query / Path / Body / Mock 页签中添加参数后，在这里补全类型与说明。
        </div>
        <div className="doc-block doc-block-resp">
          <div className="doc-block-title">
            <span className="doc-source doc-source-resp">响应</span>
          </div>
          <div className="doc-sub">
            <div className="doc-sub-title">
              请求失败
              <button className="btn btn-sm" onClick={() => addDocRow("resp_fail", [])}>
                ＋ 添加字段
              </button>
            </div>
            <table className="kv-table doc-params-table">
              <thead>
                <tr>
                  <th>字段名</th>
                  <th style={{ width: 130 }}>值</th>
                  <th style={{ width: 108 }}>类型</th>
                  <th style={{ width: 120 }}>对象名</th>
                  <th>说明</th>
                  <th style={{ width: 62 }}>操作</th>
                </tr>
              </thead>
              <tbody>
                {failDocs.length === 0 ? (
                  <tr>
                    <td colSpan={6} className="doc-empty">
                      暂无字段，点击「＋ 添加字段」开始编写请求失败响应字段
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
                placeholder="字段名"
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
              title={row.typeAuto ? "自动推导类型，可手动选择覆盖" : "字段类型"}
              onChange={(e) => updateType(source, row.keys, e.target.value)}
            >
              <option value="">自动</option>
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
                  title="对象名称"
                  spellCheck={false}
                  onChange={(e) => updateName(source, row.keys, e.target.value)}
                />
              )}
            </td>
          )}
          <td>
            <input
              value={row.description}
              placeholder="字段说明"
              spellCheck={false}
              onChange={(e) => updateDesc(source, row.keys, e.target.value)}
            />
          </td>
          <td className="doc-ops">
            {manual && isObject && (
              <button className="doc-op" title="添加下级字段" onClick={() => addDocRow(source, row.keys)}>
                ＋
              </button>
            )}
            {manual && (
              <button className="doc-op doc-op-del" title="删除字段" onClick={() => removeDocRow(source, row.keys)}>
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
            ＋ 添加字段
          </button>
        )}
      </div>
      <table className="kv-table doc-params-table">
        <thead>
          <tr>
            <th>字段名</th>
            <th style={{ width: 130 }}>值</th>
            <th style={{ width: 108 }}>类型</th>
            {showObjectName && <th style={{ width: 120 }}>对象名</th>}
            <th>说明</th>
            {manual && <th style={{ width: 62 }}>操作</th>}
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
        接口文档 <span className="help">按请求 Header / Query / Path / Body / 响应分块；响应区分请求成功与请求失败</span>
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
            <span className="doc-source doc-source-resp">响应</span>
          </div>
          {respSuccessNodes.length > 0 && (
            <div className="doc-sub">
              <div className="doc-sub-title">请求成功</div>
              <table className="kv-table doc-params-table">
                <thead>
                  <tr>
                    <th>字段名</th>
                    <th style={{ width: 130 }}>值</th>
                    <th style={{ width: 108 }}>类型</th>
                    <th style={{ width: 120 }}>对象名</th>
                    <th>说明</th>
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
              请求失败
              <button className="btn btn-sm" onClick={() => addDocRow("resp_fail", [])}>
                ＋ 添加字段
              </button>
            </div>
            <table className="kv-table doc-params-table">
              <thead>
                <tr>
                  <th>字段名</th>
                  <th style={{ width: 130 }}>值</th>
                  <th style={{ width: 108 }}>类型</th>
                  <th style={{ width: 120 }}>对象名</th>
                  <th>说明</th>
                  <th style={{ width: 62 }}>操作</th>
                </tr>
              </thead>
              <tbody>
                {failDocs.length === 0 ? (
                  <tr>
                    <td colSpan={6} className="doc-empty">
                      暂无字段，点击「＋ 添加字段」开始编写请求失败响应字段
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
