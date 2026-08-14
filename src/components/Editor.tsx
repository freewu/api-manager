import { useEffect, useState } from "react";
import { ApiFile, BODY_MODES, KeyValue, METHODS } from "../types";
import { KeyValueEditor } from "./KeyValueEditor";

type Tab = "params" | "headers" | "body" | "mock" | "desc";

interface Props {
  api: ApiFile;
  baseUrl: string;
  onChange: (api: ApiFile) => void;
  onSend: () => void;
  onSaveVersion: () => void;
  enableVersion: boolean;
  sending: boolean;
  style?: React.CSSProperties;
  /** 失焦后自动保存（接口说明 textarea blur 时触发） */
  onCommit?: () => void;
}

export function Editor({ api, baseUrl, onChange, onSend, onSaveVersion, enableVersion, sending, style, onCommit }: Props) {
  const [tab, setTab] = useState<Tab>("params");
  const effectiveUrl = api.url || (baseUrl + api.path);

  useEffect(() => {
    setTab("params");
  }, [api.path]);

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
            title="将当前接口内容保存为新版本（.version/<uuid>/<名称>.<版本号>.json）"
          >
            💾 保存
          </button>
        )}
      </div>

      <div className="tabs">
        <div className={`tab ${tab === "params" ? "active" : ""}`} onClick={() => setTab("params")}>
          Params{enabledCount(api.query) > 0 && <span className="count">{enabledCount(api.query)}</span>}
        </div>
        <div className={`tab ${tab === "headers" ? "active" : ""}`} onClick={() => setTab("headers")}>
          Headers{enabledCount(api.headers) > 0 && <span className="count">{enabledCount(api.headers)}</span>}
        </div>
        <div
          className={`tab ${tab === "body" ? "active" : ""}`}
          onClick={() => setTab("body")}
        >
          Body{api.body.mode !== "none" && api.body.raw && <span className="count">•</span>}
        </div>
        <div className={`tab ${tab === "mock" ? "active" : ""}`} onClick={() => setTab("mock")}>
          Mock{api.mock.enabled && <span className="count">●</span>}
        </div>
        <div className={`tab ${tab === "desc" ? "active" : ""}`} onClick={() => setTab("desc")}>
          描述
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
              Path 变量 <span className="help">（来自下方“路径参数”，请求时替换 {`{name}`}）</span>
            </div>
            {api.params.length === 0 ? (
              <div style={{ color: "var(--text-faint)", fontSize: 12, padding: "4px 2px" }}>
                暂无路径参数，可在上方 URL 中使用 {`{变量名}`}，例如 /users/{`{id}`}
              </div>
            ) : (
              <KeyValueEditor
                rows={api.params}
                onChange={(rows) => set({ params: rows })}
                keyPlaceholder="变量名"
                valuePlaceholder="值"
              />
            )}
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
              <KeyValueEditor
                rows={api.body.form}
                onChange={(rows) => set({ body: { ...api.body, form: rows } })}
                keyPlaceholder="字段名"
                valuePlaceholder="值"
              />
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
          <div>
            <div className="section-title">接口说明</div>
            <textarea
              className="desc-area"
              value={api.description}
              placeholder="描述该接口的用途、参数、返回值等"
              onChange={(e) => set({ description: e.target.value })}
              onBlur={onCommit}
              spellCheck={false}
            />
            <div style={{ color: "var(--text-faint)", fontSize: 11, marginTop: 8 }}>
              提示：接口名称 / 路径可在左侧右键「重命名」或顶部 URL 输入框中修改；说明文字失焦后自动保存。
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
