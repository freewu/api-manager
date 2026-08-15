import { useEffect, useMemo, useState } from "react";
import { ApiFile, BODY_MODES, DocParam, KeyValue, METHODS } from "../types";
import { KeyValueEditor } from "./KeyValueEditor";
import { CodeTab } from "./CodeTab";

type Tab = "params" | "path" | "headers" | "body" | "mock" | "desc" | "doc" | "code";

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

  // 切换接口时回到 Params 页签
  useEffect(() => {
    setTab("params");
    onTabChange?.("params");
  }, [api.uuid]);

  // URL / 路径中的 {xx} 占位符实时同步到 Path 页签（新增或删除）
  const pathSource = api.url || api.path;
  useEffect(() => {
    const names = new Set(
      [...pathSource.matchAll(/\{([^{}]+)\}/g)]
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
          Params{enabledCount(api.query) > 0 && <span className="count">{enabledCount(api.query)}</span>}
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
          描述
        </div>
        <div className={`tab ${tab === "doc" ? "active" : ""}`} onClick={() => switchTab("doc")}>
          接口文档
        </div>
        {enableCodegen && (
          <div className={`tab ${tab === "code" ? "active" : ""}`} onClick={() => switchTab("code")}>
            生成代码
          </div>
        )}
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
              Path 变量 <span className="help">（与上方 URL 中的 {`{name}`} 一一对应，自动同步；只做赋值与说明，不可增删）</span>
            </div>
            {api.params.length === 0 ? (
              <div style={{ color: "var(--text-faint)", fontSize: 12, padding: "4px 2px" }}>
                暂无路径参数，可在顶部 URL 中使用 {`{变量名}`}，例如 /users/{`{id}`}
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
              路径变量在顶部 URL 中用 {`{变量名}`} 定义，此处自动生成并保持一一对应；多个示例值可用逗号分隔（如 1,2,3），发送请求时取第一个；「说明」列用于描述该变量的含义。
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
            <div className="section-title">接口说明</div>
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
      </div>
    </div>
  );
}

/** 接口文档：汇总 query / path / body 请求参数，补充类型与说明 */
function DocParamsEditor({ api, set }: { api: ApiFile; set: (p: Partial<ApiFile>) => void }) {
  type Row = { source: DocParam["source"]; key: string; value: string };

  const rows: Row[] = useMemo(() => {
    const out: Row[] = [];
    api.query
      .filter((r) => r.key.trim())
      .forEach((r) => out.push({ source: "query", key: r.key, value: r.value }));
    api.params
      .filter((r) => r.key.trim())
      .forEach((r) => out.push({ source: "path", key: r.key, value: r.value }));
    if (api.body.mode === "form") {
      api.body.form
        .filter((r) => r.key.trim())
        .forEach((r) => out.push({ source: "body", key: r.key, value: r.value }));
    } else if (api.body.mode === "json") {
      try {
        const parsed = JSON.parse(api.body.raw);
        if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
          Object.entries(parsed).forEach(([k, v]) =>
            out.push({
              source: "body",
              key: k,
              value: typeof v === "string" ? v : JSON.stringify(v),
            })
          );
        }
      } catch {
        /* JSON 无法解析时忽略 */
      }
    }
    return out;
  }, [api]);

  const getDoc = (source: Row["source"], key: string): DocParam | undefined =>
    api.docParams.find((d) => d.source === source && d.key === key);

  const updateDoc = (source: Row["source"], key: string, patch: Partial<DocParam>) => {
    const idx = api.docParams.findIndex((d) => d.source === source && d.key === key);
    const next = [...api.docParams];
    if (idx >= 0) {
      next[idx] = { ...next[idx], ...patch };
    } else {
      next.push({ source, key, type: "", description: "", ...patch });
    }
    set({ docParams: next });
  };

  if (rows.length === 0) {
    return (
      <div style={{ color: "var(--text-faint)", fontSize: 12, padding: "6px 2px" }}>
        暂无请求参数。可在 Params / Path / Body 页签中添加参数后，在这里补全类型与说明。
      </div>
    );
  }

  return (
    <div>
      <div className="section-title">
        接口文档 <span className="help">自动汇总 Params / Path / Body 参数，补充类型与说明后随接口保存</span>
      </div>
      <table className="kv-table doc-params-table">
        <thead>
          <tr>
            <th style={{ width: 64 }}>位置</th>
            <th>参数名</th>
            <th>值</th>
            <th style={{ width: 150 }}>类型</th>
            <th>说明</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => {
            const doc = getDoc(r.source, r.key);
            return (
              <tr key={r.source + ":" + r.key}>
                <td>
                  <span className={`doc-source doc-source-${r.source}`}>
                    {r.source === "query" ? "Query" : r.source === "path" ? "Path" : "Body"}
                  </span>
                </td>
                <td>
                  <span className="doc-key">{r.key}</span>
                </td>
                <td>
                  <span className="doc-value">{r.value || "—"}</span>
                </td>
                <td>
                  <input
                    value={doc?.type || ""}
                    placeholder="如 string / number"
                    onChange={(e) => updateDoc(r.source, r.key, { type: e.target.value })}
                    spellCheck={false}
                  />
                </td>
                <td>
                  <input
                    value={doc?.description || ""}
                    placeholder="参数说明"
                    onChange={(e) => updateDoc(r.source, r.key, { description: e.target.value })}
                    spellCheck={false}
                  />
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
