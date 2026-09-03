import { Fragment, lazy, Suspense, useEffect, useMemo, useRef, useState } from "react";
import { ApiFile, BODY_MODES, BodyData, DOC_TYPES, DocParam, DocSource, KeyValue, METHODS, ObjectDef, ObjectGroup, ObjectStore, PrescriptResult, ResponseItem, emptyDocParam, emptyResponse, respSource } from "../types";
import { KeyValueEditor } from "./KeyValueEditor";
import { ExamplesTab } from "./ExamplesTab";
import { renderMarkdown } from "../commands";
import { useT } from "../i18n";
import { pickFile, listExamples, listCustomMocks, runPrescript, getGlobalVars, setGlobalVars } from "../commands";
import JsCodeEditor from "./JsCodeEditor";
import ObjectRefPicker from "./ObjectRefPicker";
import MockPicker from "./MockPicker";
import { Modal } from "./Modal";
import { renderMockBody } from "../utils/mockData";

// 代码生成页签：highlight.js 体积较大，按需懒加载（首次打开「代码」页签时才下载）
const CodeTab = lazy(() => import("./CodeTab").then((m) => ({ default: m.CodeTab })));

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

type Tab = "params" | "path" | "headers" | "body" | "prescript" | "response" | "mock" | "desc" | "doc" | "code" | "examples";

/** 文档页签 Path 变量可选类型（Path 仅支持基本标量类型） */
const PATH_DOC_TYPES = ["String", "Integer", "Float"];

/** 前置脚本常用代码片段（点击插入到编辑器末尾） */
const PRESCRIPT_SNIPPETS: { key: string; code: string }[] = [
  {
    key: "editor.snipLog",
    code: "// 打印请求参数\nconsole.log(ctx.query, ctx.path, ctx.headers, ctx.body);",
  },
  {
    key: "editor.snipMd5",
    code: "// MD5 签名\nconst sign = CryptoJS.MD5(ctx.query.t + ctx.global.get('secret')).toString();\nctx.global.set('sign', sign);",
  },
  {
    key: "editor.snipSm3",
    code: "// SM3 国密哈希（gb/t 32905）\nconst sign = sm3(ctx.query.t + ctx.global.get('secret'));\nctx.global.set('sign', sign);",
  },
  {
    key: "editor.snipSm3Hmac",
    code: "// SM3-HMAC 国密签名（key 传 hex 字符串）\nconst hmac = SM3.hmac(ctx.query.t, '6b6579');\nctx.global.set('sign', hmac);",
  },
  {
    key: "editor.snipHmac",
    code: "// HMAC-SHA256 签名\nconst hmac = CryptoJS.HmacSHA256(JSON.stringify(ctx.body), ctx.global.get('secret')).toString();\nctx.global.set('hmac', hmac);",
  },
  {
    key: "editor.snipAes",
    code: "// AES 加密（结果写回全局变量，可用 {{enc}} 绑定参数）\nconst enc = CryptoJS.AES.encrypt(JSON.stringify(ctx.body), ctx.global.get('secret')).toString();\nctx.global.set('enc', enc);",
  },
  {
    key: "editor.snipSort",
    code: "// 参数排序后拼接（签名常见场景）\nconst params = { ...ctx.query };\nconst sorted = Object.keys(params).sort().map(k => k + '=' + params[k]).join('&');\nctx.global.set('sorted', sorted);",
  },
  {
    key: "editor.snipTs",
    code: "// 时间戳 / 随机数\nconst ts = Date.now();\nconst nonce = Math.floor(Math.random() * 1e9);\nctx.global.set('ts', String(ts));\nctx.global.set('nonce', String(nonce));",
  },
  {
    key: "editor.snipGlobal",
    code: "// 读写全局变量（即环境变量）\nconst v = ctx.global.get('token');\nctx.global.set('token', 'new-value');",
  },
  {
    key: "editor.snipJson",
    code: "// 修改 body 后写回全局变量\nctx.body.extra = 'added';\nctx.global.set('body', JSON.stringify(ctx.body));",
  },
];

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
  /** 示例保存版本号：变化时重新拉取示例数量刷新角标 */
  exampleVersion?: number;
  /** 页签切换回调（App 据此隐藏/显示响应面板） */
  onTabChange?: (tab: string) => void;
  /** 前置脚本全局变量（即环境变量）被修改后的回调（App 据此刷新环境面板） */
  onEnvChanged?: () => void;
  /** 已定义对象列表（文档页签 Object 类型可引用） */
  objectsList?: ObjectDef[];
  /** 完整对象仓库（含分组），文档页签 Object 类型弹窗选择对象用（与对象管理一致） */
  objectsStore?: ObjectStore;
}

export function Editor({ api, baseUrl, onChange, onSend, onSaveVersion, enableVersion, sending, style, onCommit, enableCodegen = true, enableMock = true, codegenLang = "bash", onTabChange, onEnvChanged, currentVersion = 0, exampleVersion = 0, objectsList, objectsStore }: Props) {
  const t = useT();
  /** 是否 WebSocket 接口 */
  const isWs = api.protocol === "websocket";
  /** 是否 Socket.IO 接口（展示与 WebSocket 一致，仅不提供 ws/wss 切换） */
  const isSocketIo = api.protocol === "socketio";
  /** 实时类接口（WebSocket / Socket.IO）：无 Path / Mock 页签，Body 为消息格式 */
  const isRealtime = isWs || isSocketIo;
  /** 是否 GraphQL 接口（固定 POST + JSON body，不支持 Mock / Path 参数） */
  const isGraphql = api.protocol === "graphql";
  // WebSocket 消息格式：文本 / json / xml / binary（复用 body.mode，text 映射为 raw）
  const WS_MODES = ["raw", "json", "xml", "binary"] as const;
  const wsMode: BodyData["mode"] = (WS_MODES as readonly string[]).includes(api.body.mode)
    ? api.body.mode
    : "raw";
  const [tab, setTab] = useState<Tab>("params");
  /** JSON 格式化失败提示（body / mock 页签共用） */
  const [formatError, setFormatError] = useState<string | null>(null);
  /** 示例记录数（「示例」页签角标） */
  const [exampleCount, setExampleCount] = useState(0);
  // ---- Mock 页签：@ 弹窗插入 / 响应体测试 ----
  /** 输入 @ 时的光标位置（弹窗选中后在此插入占位符），null = 未打开 */
  const [mockAt, setMockAt] = useState<number | null>(null);
  /** Mock 响应体测试结果文本，null = 未测试 */
  const [mockTestResult, setMockTestResult] = useState<string | null>(null);
  const [mockTestOk, setMockTestOk] = useState(true);
  const [mockTesting, setMockTesting] = useState(false);
  const mockBodyRef = useRef<HTMLTextAreaElement | null>(null);
  // ---- 前置脚本页签：测试运行 / console 日志 / 全局变量 ----
  /** 全局变量（工作区级，脚本内 ctx.global.get / set 读写） */
  const [globals, setGlobals] = useState<Record<string, string>>({});
  const [preTesting, setPreTesting] = useState(false);
  const [preResult, setPreResult] = useState<PrescriptResult | null>(null);
  /** 测试运行弹窗是否打开 */
  const [testOpen, setTestOpen] = useState(false);
  // 打开接口时拉取工作区全局变量（切换接口同工作区，重新拉取一次无副作用）
  useEffect(() => {
    let alive = true;
    getGlobalVars()
      .then((g) => alive && setGlobals(g))
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, [api.uuid]);
  /** URL 为空时点击发送的红框提示 */
  const [urlError, setUrlError] = useState(false);
  /** URL 完全等于 bluefrog 时触发的彩蛋 */
  const [egg, setEgg] = useState(false);
  const eggTimerRef = useRef<number | null>(null);
  const triggerEgg = () => {
    setEgg(true);
    if (eggTimerRef.current) window.clearTimeout(eggTimerRef.current);
    eggTimerRef.current = window.setTimeout(() => setEgg(false), 3200);
  };
  const effectiveUrl = api.url || (api.path ? baseUrl + api.path : "");

  // 示例数量：接口切换或保存示例成功（exampleVersion 变化）时拉取；ExamplesTab 每次加载后也会回报最新数量
  useEffect(() => {
    let alive = true;
    listExamples(api.uuid)
      .then((l) => alive && setExampleCount(l.length))
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, [api.uuid, exampleVersion]);

  const switchTab = (t: Tab) => {
    setTab(t);
    onTabChange?.(t);
  };

  /** Mock 页签「测试」：按当前响应体渲染 mock.js 占位符 + 自定义占位符，检查编写情况 */
  const runMockTest = async () => {
    setMockTesting(true);
    try {
      const customs = await listCustomMocks().catch(() => []);
      const r = renderMockBody(api.mock.body, customs.filter((c) => c.enabled));
      setMockTestOk(r.ok);
      setMockTestResult(r.ok ? r.text : t("editor.mockTestError") + "：" + r.text);
    } finally {
      setMockTesting(false);
    }
  };

  // 前置脚本测试：把当前请求参数（query/path/headers/body）+ 全局变量传给后端执行，
  // 展示 console.log 日志与返回值，脚本内 global.set 的变量自动写回工作区
  const runPreScriptTest = async () => {
    setPreTesting(true);
    try {
      const enabled = (rows: KeyValue[]) =>
        rows.filter((r) => r.enabled && r.key.trim());
      let body = api.body.raw;
      if (api.body.mode === "form") {
        const obj: Record<string, string> = {};
        api.body.form.forEach((f) => {
          if (f.enabled && f.key.trim()) obj[f.key.trim()] = f.value;
        });
        body = JSON.stringify(obj);
      }
      const r = await runPrescript({
        code: api.prescript ?? "",
        query: enabled(api.query),
        path: enabled(api.params),
        headers: enabled(api.headers),
        body,
        globals,
      });
      setPreResult(r);
      setGlobals(r.globals);
      void setGlobalVars(r.globals);
      onEnvChanged?.();
    } catch (e) {
      setPreResult({ logs: ["[error] " + String(e)], result: "", globals });
    } finally {
      setPreTesting(false);
    }
  };

  /** 全局变量：Record → KeyValue 行（编辑用） */


  /** 代码片段面板是否展开 */
  const [snippetsOpen, setSnippetsOpen] = useState(false);

  /** 在编辑器末尾插入代码片段 */
  const insertSnippet = (code: string) => {
    const cur = api.prescript ?? "";
    const next = cur.trim() ? cur + "\n\n" + code : code;
    set({ prescript: next });
  };

  /** 根据当前接口参数快速生成示例脚本（追加到末尾） */
  const genExample = () => {
    const q = api.query.filter((x) => x.enabled && x.key.trim()).map((x) => x.key.trim());
    const p = api.params.filter((x) => x.enabled && x.key.trim()).map((x) => x.key.trim());
    const h = api.headers.filter((x) => x.enabled && x.key.trim()).map((x) => x.key.trim());
    let bodyProps: string[] = [];
    try {
      const obj = JSON.parse(api.body.raw);
      if (obj && typeof obj === "object" && !Array.isArray(obj)) {
        bodyProps = Object.keys(obj).slice(0, 8);
      }
    } catch {
      // 非 JSON body：保持空列表
    }
    const lines: string[] = [];
    lines.push("// ===== 由当前接口参数生成的示例脚本 =====");
    lines.push("// 读取请求参数：");
    if (q.length) lines.push(`console.log('query:', ctx.query); // ${q.join(", ")}`);
    if (p.length) lines.push(`console.log('path:', ctx.path); // ${p.join(", ")}`);
    if (h.length) lines.push(`console.log('headers:', ctx.headers); // ${h.join(", ")}`);
    if (bodyProps.length) lines.push(`console.log('body:', ctx.body); // ${bodyProps.join(", ")}`);
    else lines.push("console.log('body:', ctx.body);");
    lines.push("");
    lines.push("// 读取全局变量（即当前环境变量）");
    lines.push("const secret = ctx.global.get('secret');");
    lines.push("");
    lines.push("// 计算签名并写回全局变量，之后可用 {{sign}} 绑定到 query / headers");
    lines.push("const sign = CryptoJS.MD5(secret + (ctx.query.t || '')).toString();");
    lines.push("ctx.global.set('sign', sign);");
    insertSnippet(lines.join("\n"));
  };

  // 切换接口时回到默认页签：GraphQL 默认 Body（GraphQL 请求体），
  // WebSocket / Socket.IO 默认消息（Body 页签即消息编辑），
  // HTTP 的 POST / PUT / PATCH 也默认 Body（这类方法通常带请求体），其余回 Query
  useEffect(() => {
    const def: Tab =
      isGraphql || isRealtime || api.method === "POST" || api.method === "PUT" || api.method === "PATCH"
        ? "body"
        : "params";
    setTab(def);
    setFormatError(null);
    onTabChange?.(def);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [api.uuid]);

  // 设置中全局关闭 Mock 时，若当前停留在 Mock 页签则切回 Query；
  // WebSocket 无 Path / Mock 页签，若停留在这两个页签则切回 Query
  useEffect(() => {
    if ((!enableMock || isRealtime || isGraphql) && tab === "mock") {
      setTab("params");
      onTabChange?.("params");
    }
    if ((isRealtime || isGraphql) && tab === "path") {
      setTab("params");
      onTabChange?.("params");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enableMock, isRealtime, isGraphql, tab]);

  // URL / 路径中的 {xx} 占位符实时同步到 Path 页签（新增或删除）；
  // {{xx}} 是全局环境变量（双大括号），不会被当作路径参数
  // WebSocket 不使用路径参数，跳过同步
  const pathSource = api.url || api.path;
  useEffect(() => {
    if (isRealtime || isGraphql) return;
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
  }, [pathSource, isRealtime]);

  const set = (patch: Partial<ApiFile>) => onChange({ ...api, ...patch });

  /** ws/wss 协议切换 */
  const switchScheme = (scheme: "ws" | "wss") => {
    const cur = effectiveUrl;
    const replaced = cur.replace(/^(wss?:)\/\//, `${scheme}://`);
    const next = cur.startsWith("http") ? cur.replace(/^(https?:)\/\//, `${scheme}://`) : replaced;
    onChange({ ...api, url: next, path: api.path });
  };

  /** 响应页签：更新某条返回（名称 / 状态码 / 内容类型 / 示例体） */
  const updateResponse = (id: string, patch: Partial<ResponseItem>) =>
    set({ responses: (api.responses || []).map((r) => (r.id === id ? { ...r, ...patch } : r)) });
  /** 响应页签：新增一条返回（默认命名为「返回失败」，可改名） */
  const addResponse = () =>
    set({ responses: [...(api.responses || []), emptyResponse(t("editor.responseErrorName"))] });
  /** 响应页签：删除一条返回 */
  const removeResponse = (id: string) =>
    set({ responses: (api.responses || []).filter((r) => r.id !== id) });

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
      {egg && (
        <div className="egg-overlay">
          {Array.from({ length: 26 }, (_, i) => (
            <span
              key={i}
              className="egg-item"
              style={{
                left: `${(i * 37 + 5) % 100}%`,
                animationDelay: `${(i % 9) * 0.4}s`,
                animationDuration: `${2.2 + (i % 4) * 0.5}s`,
                fontSize: `${14 + (i % 6) * 7}px`,
              }}
            >
              {["🎉", "✨", "⭐", "🎊", "💫", "🎈", "🌟", "🎇"][i % 8]}
            </span>
          ))}
          <div className="egg-text">🎉 Bluefrog 🎉</div>
        </div>
      )}
      <div className="editor-head">
        {isWs ? (
          <div className="scheme-switch" title={t("editor.wsType")}>
            <button
              className={`scheme-btn${effectiveUrl.startsWith("wss://") ? " active" : ""}`}
              onClick={() => switchScheme("wss")}
            >
              wss
            </button>
            <button
              className={`scheme-btn${!effectiveUrl.startsWith("wss://") ? " active" : ""}`}
              onClick={() => switchScheme("ws")}
            >
              ws
            </button>
          </div>
        ) : isSocketIo ? (
          // Socket.IO：不显示 method，也不提供 ws/wss 切换
          <span />
        ) : isGraphql ? (
          <select className="method-select" value="POST" disabled title={t("editor.graphqlMethodTip")}>
            <option value="POST">POST</option>
          </select>
        ) : (
          <select
            className="method-select"
            value={api.method}
            onChange={(e) => {
              const v = e.target.value;
              set({ method: v });
              // POST / PUT / PATCH 有请求体，切换时默认选 Body 页签
              if (v === "POST" || v === "PUT" || v === "PATCH") {
                setTab("body");
                onTabChange?.("body");
              }
            }}
          >
            {METHODS.map((m) => (
              <option key={m} value={m}>
                {m}
              </option>
            ))}
          </select>
        )}
        <div className={`url-input-wrap${urlError ? " url-error" : ""}`}>
          <span className="url-scheme">{isWs ? "WS" : isSocketIo ? "SIO" : "URL"}</span>
          <input
            className="url-input"
            value={effectiveUrl}
            placeholder={isWs ? t("editor.wsUrlPlaceholder") : isSocketIo ? t("editor.socketIoUrlPlaceholder") : "https://api.example.com/v1/users"}
            title={t("editor.urlTitle")}
            onChange={(e) => {
              const v = e.target.value;
              // 输入内容后红框提示消失
              if (urlError) setUrlError(false);
              // URL 完全等于 bluefrog 时触发彩蛋
              if (v.trim() === "bluefrog") triggerEgg();
              if (!v) {
                // 支持清空为空的 URL
                onChange({ ...api, url: "", path: "" });
              } else if (!isRealtime && v.startsWith(baseUrl) && baseUrl) {
                onChange({ ...api, url: "", path: v.slice(baseUrl.length) || "/" });
              } else {
                onChange({ ...api, url: v, path: api.path });
              }
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !sending) {
                if (!effectiveUrl.trim()) setUrlError(true);
                onSend();
              }
            }}
            spellCheck={false}
          />
        </div>
        <button
          className="send-btn"
          onClick={() => {
            // URL 为空：红框提示并仍然发送请求
            if (!effectiveUrl.trim()) setUrlError(true);
            onSend();
          }}
          disabled={sending}
        >
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
          {isRealtime && (
            <span className="tab-hint" title={t("editor.handshakeOnly")}>
              {t("editor.handshakeBadge")}
            </span>
          )}
        </div>
        {!isRealtime && !isGraphql && (
          <div className={`tab ${tab === "path" ? "active" : ""}`} onClick={() => switchTab("path")}>
            Path{enabledCount(api.params) > 0 && <span className="count">{enabledCount(api.params)}</span>}
          </div>
        )}
        <div className={`tab ${tab === "headers" ? "active" : ""}`} onClick={() => switchTab("headers")}>
          Headers{enabledCount(api.headers) > 0 && <span className="count">{enabledCount(api.headers)}</span>}
          {isRealtime && (
            <span className="tab-hint" title={t("editor.handshakeOnly")}>
              {t("editor.handshakeBadge")}
            </span>
          )}
        </div>
        <div
          className={`tab ${tab === "body" ? "active" : ""}`}
          onClick={() => switchTab("body")}
        >
          {isRealtime ? t("editor.message") : "Body"}
          {!isRealtime &&
            api.body.mode !== "none" &&
            ((api.body.mode === "binary" && api.body.binaryPath) ||
              (api.body.mode !== "binary" && api.body.raw)) && (
              <span className="count">•</span>
            )}
          {isRealtime && api.body.raw.trim() && <span className="count">•</span>}
        </div>
        {!isRealtime && !isGraphql && (
          <div className={`tab ${tab === "prescript" ? "active" : ""}`} onClick={() => switchTab("prescript")}>
            {t("editor.prescriptTab")}
            {(api.prescript ?? "").trim() && <span className="count">•</span>}
          </div>
        )}
        <div
          className={`tab ${tab === "response" ? "active" : ""}`}
          onClick={() => switchTab("response")}
        >
          {t("editor.responseTab")}
          {(api.responses?.length ?? 0) > 0 && <span className="count">{api.responses.length}</span>}
        </div>
        {enableMock && !isRealtime && !isGraphql && (
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
          {exampleCount > 0 && <span className="count">{exampleCount}</span>}
        </div>
      </div>

      <div className="editor-body">
        {tab === "params" && (
          <div>
            {isRealtime && <div className="tab-hint-bar">{t("editor.handshakeOnly")}</div>}
            <KeyValueEditor
              rows={api.query}
              onChange={(rows) => set({ query: rows })}
              keyPlaceholder={t("editor.paramName")}
              showDescription
              allowBatch
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
          <div>
            {isRealtime && <div className="tab-hint-bar">{t("editor.handshakeOnly")}</div>}
            <KeyValueEditor
              rows={api.headers}
              onChange={(rows) => set({ headers: rows })}
              keyPlaceholder={t("editor.headerName")}
              showDescription
              allowBatch
            />
          </div>
        )}

        {tab === "body" && isRealtime && (
          <div>
            <div className="section-title">
              {t("editor.message")} <span className="help">{t("editor.messagePlaceholder")}</span>
            </div>
            <div className="body-modes">
              {WS_MODES.map((m) => (
                <div
                  key={m}
                  className={`body-mode ${wsMode === m ? "active" : ""}`}
                  onClick={() => set({ body: { ...api.body, mode: m } })}
                >
                  {m === "raw" ? t("editor.wsText") : m === "binary" ? t("editor.binary") : m}
                </div>
              ))}
            </div>
            {wsMode === "binary" ? (
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
                      type="button"
                      className="btn small"
                      onClick={() => set({ body: { ...api.body, binaryPath: "" } })}
                    >
                      ✕
                    </button>
                  </div>
                ) : null}
              </div>
            ) : (
              <div className="body-raw-wrap">
                <div className="body-raw-toolbar">
                  <div className="example-list-header">
                    <span className="help">{t("editor.messagePlaceholder")}</span>
                  </div>
                  {wsMode === "json" && (
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
                  )}
                  {wsMode === "xml" && (
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
                  )}
                  {formatError && <span className="body-format-error">{formatError}</span>}
                </div>
                <textarea
                  className="code-area"
                  value={api.body.raw}
                  placeholder={
                    wsMode === "json"
                      ? '{\n  "key": "value"\n}'
                      : wsMode === "xml"
                        ? "<root>\n  <item>value</item>\n</root>"
                        : t("editor.messagePlaceholder")
                  }
                  spellCheck={false}
                  onChange={(e) => set({ body: { ...api.body, raw: e.target.value } })}
                />
              </div>
            )}
            <div style={{ color: "var(--text-faint)", fontSize: 12, marginTop: 8 }}>
              {t("editor.wsNoMock")}
            </div>
          </div>
        )}

        {tab === "body" && !isRealtime && (
          <div>
            <div className="body-modes">
              {isGraphql ? (
                <div className="body-mode active" title={t("editor.graphqlBodyTip")}>
                  JSON
                </div>
              ) : (
                BODY_MODES.map((m) => (
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
                ))
              )}
            </div>
            {!isGraphql && api.body.mode === "none" && (
              <div style={{ color: "var(--text-faint)", fontSize: 12 }}>{t("editor.noBody")}</div>
            )}
            {!isGraphql && api.body.mode === "form" && (
              <>
                <KeyValueEditor
                  rows={api.body.form}
                  onChange={(rows) => set({ body: { ...api.body, form: rows } })}
                  keyPlaceholder={t("editor.fieldName")}
                  valuePlaceholder={undefined}
                  showDescription
                  showFileType
                  allowBatch
                />
                <div style={{ color: "var(--text-faint)", fontSize: 11, marginTop: 6 }}>
                  {t("editor.fileTypeHint")}
                </div>
              </>
            )}
            {(isGraphql || api.body.mode === "raw" || api.body.mode === "json" || api.body.mode === "xml") && (
              <div className="body-raw-wrap">
                <div className="body-raw-toolbar">
                  {(isGraphql || api.body.mode === "json") ? (
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
                    isGraphql
                      ? '{\n  "query": "query { user(id: 1) { id name } }"\n}'
                      : api.body.mode === "json"
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

        {tab === "response" && (
          <div className="response-list">
            <div className="section-title">
              {t("editor.responseTab")}{" "}
              <span className="help">{t("editor.responseTabHint")}</span>
            </div>
            {(api.responses || []).map((r) => (
              <div className="response-item" key={r.id}>
                <div className="response-item-head">
                  <input
                    className="response-name-input"
                    value={r.name}
                    placeholder={t("editor.responseName")}
                    spellCheck={false}
                    onChange={(e) => updateResponse(r.id, { name: e.target.value })}
                  />
                  <input
                    className="response-status-input"
                    type="number"
                    min={100}
                    max={599}
                    value={r.status || ""}
                    placeholder={t("editor.responseStatus")}
                    onChange={(e) =>
                      updateResponse(r.id, { status: parseInt(e.target.value || "0", 10) || 0 })
                    }
                  />
                  <select
                    className="response-content-type"
                    value={r.contentType}
                    onChange={(e) => updateResponse(r.id, { contentType: e.target.value })}
                  >
                    <option>application/json</option>
                    <option>text/plain</option>
                    <option>application/xml</option>
                    <option>text/html</option>
                  </select>
                  <button
                    className="btn small"
                    title={t("editor.removeResponse")}
                    onClick={() => removeResponse(r.id)}
                  >
                    ✕
                  </button>
                </div>
                <textarea
                  className="code-area response-body-input"
                  value={r.body}
                  placeholder={t("editor.responseBodyPlaceholder")}
                  spellCheck={false}
                  onChange={(e) => updateResponse(r.id, { body: e.target.value })}
                />
              </div>
            ))}
            <div className="response-actions">
              <button className="btn btn-sm" onClick={addResponse}>
                ＋ {t("editor.addResponse")}
              </button>
            </div>
          </div>
        )}

        {!isRealtime && !isGraphql && tab === "prescript" && (
          <div className="prescript-pane">
            <div className="section-title">
              {t("editor.prescriptHint")}{" "}
              <span className="help">ctx.query / ctx.path / ctx.headers / ctx.body</span>
            </div>
            <div className="prescript-editor">
              <JsCodeEditor
                value={api.prescript ?? ""}
                onChange={(v) => set({ prescript: v })}
                placeholder={"// 发送请求前执行，示例：\n// console.log(ctx.query, ctx.path, ctx.body);\n// const sign = CryptoJS.MD5(ctx.global.get('secret') + ctx.query.t).toString();\n// ctx.global.set('sign', sign);"}
              />
            </div>
            <div className="prescript-toolbar">
              <button className="btn-sm mock-body-test" onClick={genExample}>
                ✨ {t("editor.prescriptGen")}
              </button>
              <button
                className="btn-sm mock-body-test"
                onClick={() => setSnippetsOpen(true)}
              >
                📋 {t("editor.prescriptSnippets")}
              </button>
              <button
                className="btn-sm mock-body-test"
                disabled={preTesting}
                onClick={() => {
                  setTestOpen(true);
                  void runPreScriptTest();
                }}
              >
                {preTesting ? "…" : "▶"} {t("editor.prescriptTest")}
              </button>
              <span className="mock-body-hint">{t("editor.prescriptHelp")}</span>
            </div>
          </div>
        )}

        {/* 代码片段弹窗：点击插入到编辑器末尾 */}
        {snippetsOpen && (
          <Modal title={t("editor.prescriptSnippets")} onClose={() => setSnippetsOpen(false)} className="prescript-snippets-modal">
            <div className="prescript-snippets">
              {PRESCRIPT_SNIPPETS.map((s) => (
                <button
                  key={s.key}
                  className="prescript-snippet"
                  onClick={() => {
                    insertSnippet(s.code);
                    setSnippetsOpen(false);
                  }}
                  title={s.code}
                >
                  {t(s.key)}
                </button>
              ))}
            </div>
          </Modal>
        )}

        {/* 测试运行弹窗：展示 console 日志与脚本返回值 */}
        {testOpen && (
          <Modal title={t("editor.prescriptTest")} onClose={() => setTestOpen(false)} className="prescript-result-modal">
            {preTesting ? (
              <div className="prescript-running">{t("editor.prescriptRunning")}</div>
            ) : preResult ? (
              <div className="prescript-result">
                <div className="prescript-result-title">{t("editor.prescriptLogs")}</div>
                <pre className="prescript-logs">
                  {preResult.logs.length ? preResult.logs.join("\n") : t("editor.prescriptNoLogs")}
                </pre>
                {preResult.result !== "" && (
                  <>
                    <div className="prescript-result-title">{t("editor.prescriptResult")}</div>
                    <pre className="prescript-logs">{preResult.result}</pre>
                  </>
                )}
              </div>
            ) : null}
          </Modal>
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
              ref={mockBodyRef}
              className="code-area"
              value={api.mock.body}
              placeholder={'{\n  "code": 0,\n  "data": null\n}'}
              onChange={(e) => {
                const next = e.target.value;
                const prev = api.mock.body;
                set({ mock: { ...api.mock, body: next } });
                // 输入 @：弹出占位符选择。纯字符串比较（不依赖 selectionStart，
                // 与输入法/光标位置无关）：新值恰好在某处插入单个 @ 字符时触发
                if (next.length === prev.length + 1) {
                  let i = 0;
                  while (i < prev.length && prev[i] === next[i]) i++;
                  if (next[i] === "@") setMockAt(i);
                }
              }}
              spellCheck={false}
            />
            <div className="mock-body-toolbar">
              <span className="mock-body-hint">
                {t("editor.mockBodyHint")}{" "}
                <code>@cname</code> <code>@integer(1,100)</code> <code>"list|1-5": […]</code>
              </span>
              <button
                className="btn-sm mock-body-test"
                disabled={mockTesting}
                onClick={() => void runMockTest()}
              >
                {mockTesting ? "…" : "🧪"} {t("editor.mockTest")}
              </button>
            </div>
            {mockAt !== null && (
              <MockPicker
                onPick={(v) => {
                  const pos = mockAt;
                  const cur = api.mock.body;
                  // pos 是用户刚输入的 @（占位符触发符），选择后将其替换掉，避免残留 @@xxx
                  const after = cur[pos] === "@" ? pos + 1 : pos;
                  set({ mock: { ...api.mock, body: cur.slice(0, pos) + v + cur.slice(after) } });
                  setMockAt(null);
                  requestAnimationFrame(() => {
                    const el = mockBodyRef.current;
                    if (el) {
                      el.focus();
                      const caret = pos + v.length;
                      el.setSelectionRange(caret, caret);
                    }
                  });
                }}
                onClose={() => setMockAt(null)}
              />
            )}
            {mockTestResult !== null && (
              <Modal
                title={`${t("editor.mockTest")}${mockTestOk ? "" : " — " + t("editor.mockTestError")}`}
                onClose={() => setMockTestResult(null)}
                className="modal-mock-test"
                maskClassName="objects-import-mask"
              >
                <pre className={`mock-test-pre${mockTestOk ? "" : " mock-test-fail"}`}>{mockTestResult}</pre>
              </Modal>
            )}
          </div>
        )}

        {tab === "desc" && <DescEditor value={api.description} onChange={(v) => set({ description: v })} onCommit={onCommit} />}

        {tab === "doc" && <DocParamsEditor api={api} set={set} objectsList={objectsList} objectsStore={objectsStore} />}

        {tab === "code" && enableCodegen && (
          <Suspense fallback={<div className="tab-loading">{t("examples.loading")}</div>}>
            <CodeTab api={api} baseUrl={baseUrl} defaultLang={codegenLang} />
          </Suspense>
        )}
        {tab === "examples" && (
          <ExamplesTab uuid={api.uuid} api={api} onChange={onChange} onCountChange={setExampleCount} />
        )}
      </div>
    </div>
  );
}

/** 接口文档：按 请求Header / Query / Path / Body / 响应 分块（没有值的块不渲染）；
 *  响应分为「请求成功」（从 Mock 响应体 JSON 推导）与「请求失败」（手动添加）两种情况；
 *  说明字段与请求页签共用同一 KeyValue.description（旧 docParams 说明在读取时已迁移）；
 *  Header 分块无类型列；Path 类型仅 String / Integer / Float；
 *  字段类型可选 String / Integer / Float / Boolean / List / Object，Object 可绑定对象名，下级字段用树状表单表示；
 *  Body 可整体绑定对象管理中的对象，按对象属性展开请求体字段（仅文档展示） */
function DocParamsEditor({ api, set, objectsList, objectsStore }: { api: ApiFile; set: (p: Partial<ApiFile>) => void; objectsList?: ObjectDef[]; objectsStore?: ObjectStore }) {
  const T = useT();
  /** 正在选择对象名的文档行（source + keys 路径），null = 未打开 */
  const [objPick, setObjPick] = useState<{ source: DocSource; keys: string[] } | null>(null);
  /** 选择弹窗用的对象仓库：优先完整 objectsStore（与对象管理分组一致），否则从对象列表构造 */
  const pickerStore: ObjectStore | null = useMemo(() => {
    if (objectsStore) return objectsStore;
    if (!objectsList || objectsList.length === 0) return null;
    const groups: ObjectGroup[] = [];
    for (const o of objectsList) {
      if (!o.group || groups.some((g) => g.id === o.group)) continue;
      groups.push({ id: o.group, name: o.group.split("/").pop() || o.group, deprecated: false });
    }
    return { groups, objects: objectsList };
  }, [objectsStore, objectsList]);
  // 对象查找：按 hash / 名称（对象绑定与 Object 字段的引用展示用）
  const objByHash = useMemo(() => new Map((objectsList || []).map((o) => [o.hash, o])), [objectsList]);
  const objByName = useMemo(() => new Map((objectsList || []).map((o) => [o.name, o])), [objectsList]);
  /** 对象属性 kind / itemKind → 文档字段类型（与 DOC_TYPES 对应） */
  const kindToType = (k: string): string => {
    const map: Record<string, string> = {
      string: "String",
      number: "Integer",
      boolean: "Boolean",
      datetime: "Datetime",
      date: "Date",
      time: "Time",
      object: "Object",
      list: "List",
      any: "Any",
    };
    return map[k] || "Any";
  };
  /** 对象属性 → 树节点（Object / List(object) 属性展开引用对象的属性，seen 防止循环引用） */
  const objToNodes = (o: ObjectDef, seen: Set<string>): RNode[] =>
    (o.properties || []).map((p) => {
      const mock = p.mock || "";
      if (p.kind === "object") {
        const ref = p.refHash ? objByHash.get(p.refHash) : undefined;
        return {
          key: p.key,
          value: mock,
          guess: "Object",
          objName: ref?.name || "",
          children:
            ref && !seen.has(ref.hash) ? objToNodes(ref, new Set(seen).add(ref.hash)) : undefined,
        };
      }
      if (p.kind === "list") {
        let children: RNode[] | undefined;
        if (p.itemKind === "object" && p.refHash) {
          const ref = objByHash.get(p.refHash);
          if (ref && !seen.has(ref.hash)) {
            children = objToNodes(ref, new Set(seen).add(ref.hash));
          }
        }
        return {
          key: p.key,
          value: mock,
          guess: "List",
          guessItem: kindToType(p.itemKind || "string"),
          children,
        };
      }
      return { key: p.key, value: mock, guess: kindToType(p.kind) };
    });
  /** 根据当前 Body 类型给出建议的 Content-Type（header 说明留空时自动提示） */
  const contentTypeHint = (): string => {
    if (api.protocol === "websocket" || api.protocol === "socketio") return "";
    switch (api.body.mode) {
      case "json":
        return "application/json";
      case "xml":
        return "text/xml";
      case "form": {
        const hasFile = (api.body.form || []).some((f) => f.isFile);
        return hasFile ? "multipart/form-data" : "application/x-www-form-urlencoded";
      }
      case "raw":
        return "text/plain";
      case "binary":
        return "application/octet-stream";
      default:
        return "";
    }
  };
  // 行内说明：header / query / path / body(form) 的来源行数组；body 在 json/对象绑定模式下返回 null
  const rowsFor = (source: DocSource): KeyValue[] | null => {
    if (source === "header") return api.headers || [];
    if (source === "query") return api.query || [];
    if (source === "path") return api.params || [];
    if (source === "body" && api.body.mode === "form") return api.body.form || [];
    return null;
  };
  /** 行的说明被修改时写回对应字段 */
  const rowsPatch = (source: DocSource, rows: KeyValue[]): Partial<ApiFile> => {
    if (source === "header") return { headers: rows };
    if (source === "query") return { query: rows };
    if (source === "path") return { params: rows };
    return { body: { ...api.body, form: rows } };
  };
  // ---- 树节点（由请求配置 / Mock 响应 JSON 推导） ----
  type RNode = {
    key: string;
    value: string;
    guess: string; // 推导类型
    guessItem?: string; // List 推导元素类型
    /** 所属 KeyValue 行在源数组中的下标（header/query/path/body-form 的行内说明据此定位） */
    rowIdx?: number;
    /** Object 节点默认引用的对象名（来自对象定义的属性引用） */
    objName?: string;
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
      datetime: "Datetime",
      timestamp: "Datetime",
      date: "Date",
      time: "Time",
      bool: "Boolean",
      boolean: "Boolean",
      any: "Any",
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
      .map((r, i) => ({ r, i }))
      .filter(({ r }) => r.key.trim())
      .map(({ r, i }) => ({ key: r.key, value: r.value, guess: guessFromText(r.value), rowIdx: i }));

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

  // Body 文档绑定：docParams 中 source=body 且 key 为空的对象引用（仅文档展示用）
  const bodyBinding = useMemo(
    () => api.docParams.find((d) => d.source === "body" && d.key === ""),
    [api.docParams]
  );
  const boundBodyObj = bodyBinding && bodyBinding.objectName ? objByName.get(bodyBinding.objectName) : undefined;
  const bindBodyObject = (name: string) => {
    const next = api.docParams.filter((d) => !(d.source === "body" && d.key === ""));
    next.push({
      source: "body",
      key: "",
      type: "Object",
      description: "",
      itemType: "",
      objectName: name,
      children: [],
    });
    set({ docParams: next });
  };
  const unbindBodyObject = () =>
    set({ docParams: api.docParams.filter((d) => !(d.source === "body" && d.key === "")) });

  // ---- 分块推导（请求侧来自真实配置，响应侧来自 Mock 体 / 手动条目） ----
  type Block = { source: DocSource; title: string; nodes: RNode[] };

  const blocks = useMemo<Block[]>(() => {
    const out: Block[] = [];
    const headerNodes = kvNodes(api.headers);
    if (headerNodes.length) out.push({ source: "header", title: T("editor.requestHeader"), nodes: headerNodes });
    const queryNodes = kvNodes(api.query);
    if (queryNodes.length) out.push({ source: "query", title: "Query", nodes: queryNodes });
    const pathNodes = kvNodes(api.params);
    if (pathNodes.length) out.push({ source: "path", title: "Path", nodes: pathNodes });
    let bodyNodes: RNode[] = [];
    // Body 已绑定对象：按对象属性展开为请求体字段（仅文档展示）；否则按 form / json 推导
    const bindEntry = api.docParams.find((d) => d.source === "body" && d.key === "");
    const bindObj = bindEntry && bindEntry.objectName ? objByName.get(bindEntry.objectName) : undefined;
    if (bindObj) {
      bodyNodes = objToNodes(bindObj, new Set([bindObj.hash]));
    } else if (api.body.mode === "form") {
      bodyNodes = kvNodes(api.body.form);
    } else if (api.body.mode === "json") {
      try {
        bodyNodes = jsonToNodes(JSON.parse(api.body.raw));
      } catch {
        /* JSON 无法解析时不生成 */
      }
    }
    if (bodyNodes.length) out.push({ source: "body", title: "Body", nodes: bodyNodes });
    return out;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [api, objByName]);

  // 响应页签条目 → 文档块：每个条目从示例体推导字段（docParams 按 resp:<id> 覆盖）
  const respBlocks = useMemo(
    () =>
      (api.responses || []).map((r) => {
        let nodes: RNode[] = [];
        try {
          nodes = jsonToNodes(JSON.parse(r.body));
        } catch {
          /* 非 JSON 响应体则走手动条目 */
        }
        return { entry: r, nodes };
      }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [api.responses]
  );
  const respDocs = (id: string) => api.docParams.filter((d) => d.source === respSource(id));

  // ---- 行视图（统一 树形/手动 两种来源的渲染） ----
  type RowView = {
    keys: string[];
    key: string;
    keyEditable: boolean;
    rowIdx?: number;
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
    const kv = rowsFor(source);
    // 行内说明（header/query/path/body-form）：取 KeyValue.description，旧 docParams 说明仅作回退展示
    const rowDesc = node.rowIdx != null && kv && kv[node.rowIdx] ? kv[node.rowIdx].description : null;
    return {
      keys,
      key: node.key,
      keyEditable: false,
      rowIdx: node.rowIdx,
      value: node.value,
      type: storedType || node.guess || "",
      typeAuto: !storedType,
      objectName: doc?.objectName || node.objName || node.key,
      description:
        rowDesc !== null ? rowDesc || (doc ? doc.description || "" : "") : doc ? doc.description || "" : "",
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
  const updateKey = (source: DocSource, keys: string[], v: string) =>
    updateDocAt(source, keys, { key: v });
  /** 清除某键在 docParams 中遗留的旧说明（存在才清，避免凭空创建空条目） */
  const clearDocDesc = (source: DocSource, keys: string[]) => {
    if (!api.docParams.length) return;
    let arr = api.docParams;
    let cur: DocParam | undefined;
    for (const k of keys) {
      cur = arr.find((d) => d.source === source && d.key === k);
      if (!cur) return;
      arr = cur.children || [];
    }
    if (cur && cur.description) updateDocAt(source, keys, { description: "" });
  };
  const updateDesc = (source: DocSource, keys: string[], v: string, rowIdx?: number) => {
    // 行内说明（header / query / path / body-form）：与请求页签编辑的是同一个 KeyValue.description 字段
    const kv = rowsFor(source);
    if (rowIdx != null && kv && kv[rowIdx]) {
      const next = kv.map((x, i) => (i === rowIdx ? { ...x, description: v } : x));
      set(rowsPatch(source, next));
      clearDocDesc(source, keys);
      return;
    }
    updateDocAt(source, keys, { description: v });
  };

  const renderRow = (row: RowView, depth: number, source: DocSource, opts: BlockOpts) => {
    const isObject = row.type === "Object";
    // header 的 Content-Type 行：说明留空时按当前 Body 类型自动提示
    const isContentType = source === "header" && row.key.trim().toLowerCase() === "content-type";
    const typeOptions =
      !row.typeAuto && row.type && !opts.typeOptions.includes(row.type)
        ? [row.type, ...opts.typeOptions]
        : opts.typeOptions;
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
          {opts.showType && (
            <td>
              <select
                className={`doc-type-select${row.typeAuto ? " doc-type-auto" : ""}`}
                value={row.type}
                title={row.typeAuto ? T("editor.typeAutoHint") : T("kv.fileType")}
                onChange={(e) => updateType(source, row.keys, e.target.value)}
              >
                <option value="">{T("editor.auto")}</option>
                {typeOptions.map((t) => (
                  <option key={t} value={t}>
                    {t}
                  </option>
                ))}
              </select>
            </td>
          )}
          {opts.showObjectName && (
            <td>
              {isObject &&
                (pickerStore && objectsList && objectsList.length > 0 ? (
                  <div className="doc-object-pick-wrap">
                    <button
                      className="doc-name-input doc-object-pick"
                      title={T("editor.objectName")}
                      onClick={() => setObjPick({ source, keys: row.keys })}
                    >
                      <span className="doc-object-pick-name">{row.objectName || "—"}</span>
                      <span className="doc-object-pick-caret">▾</span>
                    </button>
                    {row.objectName && (
                      <button
                        className="doc-object-pick-clear"
                        title={T("editor.clearObjectName")}
                        onClick={() => updateName(source, row.keys, "")}
                      >
                        ✕
                      </button>
                    )}
                  </div>
                ) : (
                  <input
                    className="doc-name-input"
                    value={row.objectName}
                    placeholder={row.key}
                    title={T("editor.objectName")}
                    spellCheck={false}
                    onChange={(e) => updateName(source, row.keys, e.target.value)}
                  />
                ))}
            </td>
          )}
          <td>
            <input
              value={row.description}
              placeholder={
                isContentType && contentTypeHint() ? contentTypeHint() : T("editor.fieldDesc")
              }
              title={isContentType ? T("editor.contentTypeHint") : undefined}
              spellCheck={false}
              onChange={(e) => updateDesc(source, row.keys, e.target.value, row.rowIdx)}
            />
          </td>
        </tr>
        {row.children.map((c) => renderRow(c, depth + 1, source, opts))}
      </Fragment>
    );
  };

  /** 文档分块列配置：showType=false（header 不需要类型）、typeOptions 限定可选类型、showObjectName */
  type BlockOpts = { showType: boolean; typeOptions: string[]; showObjectName: boolean };

  const renderBlock = (
    title: string,
    badgeClass: string,
    rows: RowView[],
    source: DocSource,
    opts: BlockOpts,
    headerExtra?: React.ReactNode
  ) => (
    <div className="doc-block">
      <div className="doc-block-title">
        <span className={`doc-source ${badgeClass}`}>{title}</span>
        <span className="doc-hr" />
        {headerExtra && <div className="doc-block-extra">{headerExtra}</div>}
      </div>
      <table className="kv-table doc-params-table">
        <thead>
          <tr>
            <th>{T("editor.fieldName")}</th>
            {opts.showType && <th style={{ width: 108 }}>{T("kv.type")}</th>}
            {opts.showObjectName && <th style={{ width: 120 }}>{T("editor.objectName")}</th>}
            <th>{T("kv.desc")}</th>
          </tr>
        </thead>
        <tbody>{rows.map((r) => renderRow(r, 0, source, opts))}</tbody>
      </table>
    </div>
  );

  const badgeFor = (source: DocSource) => {
    if (source.startsWith("resp:")) {
      const id = source.slice(5);
      const entry = (api.responses || []).find((r) => r.id === id);
      const ok = !entry || entry.status < 400;
      return ok ? "doc-source-resp-ok" : "doc-source-resp-fail";
    }
    return source === "header"
      ? "doc-source-header"
      : source === "query"
        ? "doc-source-query"
        : source === "path"
          ? "doc-source-path"
          : source === "body"
            ? "doc-source-body"
            : "doc-source-resp-fail";
  };

  return (
    <div>
      <div className="section-title">
        {T("tab.doc")} <span className="help">{T("editor.docBlockHint")}</span>
      </div>
      {blocks.map((b) => {
        const isBody = b.source === "body";
        return renderBlock(
          b.title,
          badgeFor(b.source),
          b.nodes.map((n) => derivedView(n, b.source, [])),
          b.source,
          {
            showType: b.source !== "header",
            typeOptions: b.source === "path" ? PATH_DOC_TYPES : DOC_TYPES,
            showObjectName: isBody,
          },
          isBody ? (
            <>
              {boundBodyObj && (
                <>
                  <span className="doc-body-bound" title={T("editor.bodyBindHint")}>
                    {T("editor.bindBodyObject")}: {boundBodyObj.displayName || boundBodyObj.name}
                  </span>
                  <button
                    className="btn-sm"
                    title={T("editor.clearObjectName")}
                    onClick={unbindBodyObject}
                  >
                    ✕
                  </button>
                </>
              )}
              {pickerStore && objectsList && objectsList.length > 0 ? (
                <button
                  className="btn-sm"
                  title={T("editor.bodyBindHint")}
                  onClick={() => setObjPick({ source: "body", keys: [] })}
                >
                  {boundBodyObj ? T("editor.changeBodyObject") : T("editor.bindBodyObject")}
                </button>
              ) : null}
            </>
          ) : undefined
        );
      })}
      {respBlocks.length > 0 && (
        <div className="doc-block doc-block-resp">
          <div className="doc-block-title">
            <span className="doc-source doc-source-resp">{T("editor.response")}</span>
            <span className="doc-hr" />
          </div>
          {respBlocks.map(({ entry, nodes }) => {
            const source = respSource(entry.id);
            const docs = respDocs(entry.id);
            return (
              <div className="doc-sub" key={entry.id}>
                <div className="doc-sub-title">
                  {entry.name || T("editor.response")}
                  {entry.status > 0 && <span className="resp-status-badge">HTTP {entry.status}</span>}
                </div>
                <table className="kv-table doc-params-table">
                  <thead>
                    <tr>
                      <th>{T("editor.fieldName")}</th>
                      <th style={{ width: 108 }}>{T("kv.type")}</th>
                      <th style={{ width: 120 }}>{T("editor.objectName")}</th>
                      <th>{T("kv.desc")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {nodes.length > 0 ? (
                      nodes.map((n) =>
                        renderRow(derivedView(n, source, []), 0, source, {
                          showType: true,
                          typeOptions: DOC_TYPES,
                          showObjectName: true,
                        })
                      )
                    ) : docs.length > 0 ? (
                      docs.map((d) =>
                        renderRow(manualView(d, source, []), 0, source, {
                          showType: true,
                          typeOptions: DOC_TYPES,
                          showObjectName: true,
                        })
                      )
                    ) : (
                      <tr>
                        <td colSpan={4} className="doc-empty">
                          {T("editor.noRespFields")}
                        </td>
                      </tr>
                    )}
                  </tbody>
                </table>
              </div>
            );
          })}
        </div>
      )}
      {objPick && pickerStore && (
        <ObjectRefPicker
          store={pickerStore}
          excludeUuid=""
          currentHash={
            (() => {
              if (objPick.keys.length === 0) {
                // Body 分块绑定：按 docParams 中的空 key 根条目反查已绑定对象
                const d = api.docParams.find((x) => x.source === objPick.source && x.key === "");
                return (objectsList || []).find((o) => o.name === d?.objectName)?.hash || "";
              }
              const target = getDocAt(objPick.source, objPick.keys);
              return (objectsList || []).find((o) => o.name === target?.objectName)?.hash || "";
            })()
          }
          onPick={(hash) => {
            const o = (objectsList || []).find((x) => x.hash === hash);
            if (o) {
              if (objPick.keys.length === 0 && objPick.source === "body") bindBodyObject(o.name);
              else updateName(objPick.source, objPick.keys, o.name);
            }
            setObjPick(null);
          }}
          onClose={() => setObjPick(null)}
        />
      )}
    </div>
  );
}

/** 接口描述：Markdown 编辑 / 预览切换（预览由后端 md_to_html 渲染） */
function DescEditor({
  value,
  onChange,
  onCommit,
}: {
  value: string;
  onChange: (v: string) => void;
  onCommit?: () => void;
}) {
  const t = useT();
  const [mode, setMode] = useState<"edit" | "preview">("edit");
  const [html, setHtml] = useState("");
  const [busy, setBusy] = useState(false);

  const toPreview = async () => {
    setBusy(true);
    try {
      setHtml(await renderMarkdown(value || ""));
      setMode("preview");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="desc-root">
      <div className="desc-toolbar">
        <button
          className={`btn-sm desc-mode-btn${mode === "edit" ? " active" : ""}`}
          onClick={() => setMode("edit")}
        >
          ✏️ {t("editor.descEdit")}
        </button>
        <button
          className={`btn-sm desc-mode-btn${mode === "preview" ? " active" : ""}`}
          disabled={busy}
          onClick={() => void toPreview()}
        >
          👁 {t("editor.descPreview")}
        </button>
        {mode === "preview" && (
          <span className="desc-mode-tip">{t("editor.descPreviewTip")}</span>
        )}
      </div>
      {mode === "edit" ? (
        <textarea
          className="desc-area"
          value={value}
          placeholder={t("editor.descPlaceholder")}
          onChange={(e) => onChange(e.target.value)}
          onBlur={onCommit}
          spellCheck={false}
        />
      ) : (
        <div className="desc-preview md-preview" dangerouslySetInnerHTML={{ __html: html }} />
      )}
      <div className="desc-hint">{t("editor.descHint")}</div>
    </div>
  );
}
