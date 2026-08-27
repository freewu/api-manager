/** 代码生成公共基础设施：语言类型 / 库列表 / 请求参数构造 / 转义工具 */
import { ApiFile } from "../../types";

export const escapeRe = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

// 编码 query 值但保留 {{xxx}} 全局环境变量占位符（不编码成 %7B%7Bxxx%7D%7D）
// 占位符用 !KVn! 包裹：!~ 等字符不被 encodeURIComponent 编码，编码后还原
function encValue(v: string): string {
  const parts: string[] = [];
  const masked = v.replace(/\{\{[^{}]+\}\}/g, (m) => {
    parts.push(m);
    return `!KV${parts.length - 1}!`;
  });
  return encodeURIComponent(masked).replace(/!KV(\d+)!/g, (_, i) => parts[+i]);
}

export type CodeLang =
  | "curl" // 旧值，兼容已保存的设置（等价 bash）
  | "bash"
  | "python"
  | "c"
  | "cpp"
  | "java"
  | "csharp"
  | "javascript"
  | "r"
  | "rust"
  | "delphi"
  | "php"
  | "go"
  | "ruby"
  | "swift"
  | "perl"
  | "objectivec"
  | "julia"
  | "kotlin"
  | "typescript"
  | "erlang"
  | "lua"
  | "powershell";

export interface CodeLibOption {
  value: string;
  label: string;
  /** 选中该库时的提示文案（i18n key，可选） */
  hint?: string;
}

export const CODE_LIBS: Partial<Record<CodeLang, CodeLibOption[]>> = {
  bash: [
    { value: "curl", label: "cURL" },
    { value: "wget", label: "Wget" },
    { value: "httpie", label: "HTTPie" },
  ],
  javascript: [
    { value: "fetch", label: "Fetch" },
    { value: "axios", label: "Axios" },
    { value: "unirest", label: "Unirest" },
    { value: "request", label: "Request" },
    { value: "native", label: "Native" },
  ],
  typescript: [
    { value: "fetch", label: "Fetch" },
    { value: "axios", label: "Axios" },
    { value: "unirest", label: "Unirest" },
    { value: "request", label: "Request" },
    { value: "native", label: "Native" },
  ],
  java: [
    { value: "okhttp", label: "OkHttp" },
    { value: "unirest", label: "Unirest" },
    { value: "webclient", label: "WebClient" },
    { value: "httpclient", label: "HttpClient" },
    { value: "retrofit2", label: "Retrofit2" },
    { value: "httpclient5", label: "HttpClient5" },
  ],
  php: [
    { value: "curl", label: "cURL" },
    { value: "pecl", label: "PECL" },
    { value: "snoopy", label: "Snoopy" },
    { value: "guzzle", label: "Guzzle" },
  ],
  python: [
    { value: "httpclient", label: "http.client" },
    { value: "requests", label: "Requests" },
  ],
  r: [
    { value: "httr", label: "httr" },
    { value: "rcurl", label: "RCurl" },
  ],
  lua: [
    { value: "luasocket", label: "lua-httpclient", hint: "codegen.luaHttpHint" },
    { value: "luacurl", label: "lua-curl" },
    { value: "resty", label: "lua-resty-httpclient", hint: "codegen.luaRestyHint" },
  ],
};

export function defaultLib(lang: CodeLang): string | undefined {
  return CODE_LIBS[lang]?.[0]?.value;
}

export const WS_CODE_LIBS: Partial<Record<CodeLang, CodeLibOption[]>> = {
  c: [
    { value: "libwebsockets", label: "libwebsockets" },
    { value: "libuvws", label: "libuv-ws" },
    { value: "wslay", label: "wslay" },
  ],
  cpp: [
    { value: "beast", label: "Boost.Beast" },
    { value: "libwebsockets", label: "libwebsockets" },
    { value: "uwebsockets", label: "uWebSockets" },
    { value: "qt", label: "Qt QWebSocket" },
  ],
  php: [
    { value: "swoole", label: "Swoole / OpenSwoole" },
    { value: "ratchet", label: "Ratchet" },
  ],
  ruby: [
    { value: "faye", label: "faye-websocket" },
    { value: "websocket-ruby", label: "websocket-ruby" },
    { value: "sinatra", label: "Sinatra + faye-websocket" },
    { value: "actioncable", label: "Rails ActionCable" },
  ],
  java: [
    { value: "jsr356", label: "JSR-356" },
    { value: "spring", label: "Spring WebSocket" },
    { value: "netty", label: "Netty" },
    { value: "okhttp", label: "OkHttp" },
  ],
  swift: [
    { value: "urlsession", label: "URLSession WebSocket" },
    { value: "starscream", label: "Starscream" },
    { value: "network", label: "Network.framework" },
  ],
  perl: [
    { value: "mojo", label: "Mojo::UserAgent" },
    { value: "anyevent", label: "AnyEvent::WebSocket::Client" },
  ],
  julia: [
    { value: "websocketjl", label: "WebSocket.jl" },
  ],
  kotlin: [
    { value: "okhttp", label: "OkHttp WebSocket" },
    { value: "java-websocket", label: "Java-WebSocket" },
  ],
  erlang: [
    { value: "gun", label: "gun" },
  ],
  powershell: [
    { value: "clientwebsocket", label: "ClientWebSocket" },
  ],
  lua: [
    { value: "resty", label: "lua-resty-websocket" },
    { value: "standalone", label: "lua-websocket" },
  ],
  python: [
    { value: "websockets", label: "websockets" },
    { value: "sync", label: "websocket-client" },
  ],
  rust: [
    { value: "tokio", label: "tokio-tungstenite" },
    { value: "sync", label: "tungstenite" },
  ],
  // Delphi 没有原生 WebSocket：Indy 10 从 Delphi XE8 起内置 TIdWebSocket
  delphi: [
    { value: "synapse", label: "Delphi-WebSocket（Synapse）" },
    { value: "indy", label: "Indy 10（TIdWebSocket）" },
    { value: "websocket4delphi", label: "Websocket4Delphi（WinHTTP）" },
  ],
};

export const CODE_LANGS: { value: CodeLang; label: string }[] = [
  { value: "bash", label: "Bash" },
  { value: "python", label: "Python" },
  { value: "c", label: "C" },
  { value: "cpp", label: "C++" },
  { value: "java", label: "Java" },
  { value: "csharp", label: "C#" },
  { value: "javascript", label: "JavaScript" },
  { value: "r", label: "R" },
  { value: "rust", label: "Rust" },
  { value: "delphi", label: "Delphi" },
  { value: "php", label: "PHP" },
  { value: "go", label: "Go" },
  { value: "ruby", label: "Ruby" },
  { value: "swift", label: "Swift" },
  { value: "perl", label: "Perl" },
  { value: "objectivec", label: "Objective-C" },
  { value: "julia", label: "Julia" },
  { value: "kotlin", label: "Kotlin" },
  { value: "typescript", label: "TypeScript" },
  { value: "erlang", label: "Erlang" },
  { value: "lua", label: "Lua" },
  { value: "powershell", label: "PowerShell" },
];

export function esc(s: string, q: string): string {
  return s
    .replace(/\\/g, "\\\\")
    .split(q)
    .join("\\" + q)
    .replace(/\n/g, "\\n")
    .replace(/\r/g, "\\r")
    .replace(/\t/g, "\\t");
}

export interface Req {
  method: string;
  url: string;
  headers: { key: string; value: string }[];
  body: string;
  bodyKind: "none" | "json" | "text";
  formText: { key: string; value: string }[];
  files: { key: string; path: string }[];
}

export function buildReq(api: ApiFile, baseUrl: string): Req {
  let url = api.url || baseUrl + (api.path || "/");
  // 仅替换单大括号 {变量名} 路径参数，不触碰 {{变量名}} 全局环境变量
  for (const p of api.params.filter((x) => x.enabled && x.key)) {
    const v = (p.value || "").split(",")[0].trim();
    const rx = new RegExp(`(?<!\\{)\\{${escapeRe(p.key)}\\}(?!\\})`, "g");
    url = url.replace(rx, v ? encodeURIComponent(v) : `{${p.key}}`);
  }
  const query = api.query.filter((q) => q.enabled && q.key.trim());
  if (query.length) {
    const qs = query
      .map((q) => `${encValue(q.key)}=${encValue(q.value)}`)
      .join("&");
    url += (url.includes("?") ? "&" : "?") + qs;
  }
  const headers = api.headers
    .filter((h) => h.enabled && h.key.trim())
    .map((h) => ({ key: h.key, value: h.value }));
  let body = "";
  let bodyKind: Req["bodyKind"] = "none";
  let formText: Req["formText"] = [];
  let files: Req["files"] = [];
  if (api.body.mode === "json") {
    body = api.body.raw.trim();
    if (body) bodyKind = "json";
  } else if (api.body.mode === "raw") {
    body = api.body.raw;
    if (body.trim()) bodyKind = "text";
  } else if (api.body.mode === "form") {
    const f = api.body.form.filter((r) => r.enabled && r.key.trim());
    const fileRows = f.filter((r) => r.isFile && r.value.trim());
    const textRows = f.filter((r) => !r.isFile);
    files = fileRows.map((r) => ({ key: r.key, path: r.value }));
    formText = textRows.map((r) => ({ key: r.key, value: r.value }));
    if (formText.length) {
      body = formText.map((r) => `${r.key}=${r.value}`).join("&");
      bodyKind = "text";
    }
  }
  return { method: api.method, url, headers, body, bodyKind, formText, files };
}

export interface WsReq {
  url: string;
  headers: { key: string; value: string }[];
  message: string;
}

export function buildWsReq(api: ApiFile, baseUrl: string): WsReq {
  let url = api.url || baseUrl + (api.path || "/");
  // 兼容把 ws 地址误填成 http 的情况
  if (/^https?:\/\//i.test(url)) {
    url = url.replace(/^https/i, "ws");
  }
  // 仅替换单大括号 {变量名} 路径参数，不触碰 {{变量名}} 全局环境变量
  for (const p of api.params.filter((x) => x.enabled && x.key)) {
    const v = (p.value || "").split(",")[0].trim();
    const rx = new RegExp(`(?<!\{)\{${escapeRe(p.key)}\}(?!\})`, "g");
    url = url.replace(rx, v ? encodeURIComponent(v) : `{${p.key}}`);
  }
  const qs = api.query
    .filter((q) => q.enabled && q.key.trim())
    .map((q) => `${encValue(q.key)}=${encValue(q.value)}`)
    .join("&");
  if (qs) url += (url.includes("?") ? "&" : "?") + qs;
  const headers = api.headers
    .filter((h) => h.enabled && h.key.trim())
    .map((h) => ({ key: h.key, value: h.value }));
  return { url, headers, message: api.body.raw ?? "" };
}

export function parseWsUrl(url: string): { scheme: string; host: string; port: number; path: string } {
  const m = /^(wss?):\/\/([^/?#:]+)(?::(\d+))?([^?#]*)(\?[^#]*)?$/i.exec(url);
  const scheme = (m?.[1] || "ws").toLowerCase();
  const host = m?.[2] || "127.0.0.1";
  const port = m?.[3] ? Number(m[3]) : scheme === "wss" ? 443 : 80;
  const path = (m?.[4] || "/") + (m?.[5] || "");
  return { scheme, host, port, path };
}
