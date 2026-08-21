import { ApiFile } from "../types";

/** 转义正则特殊字符（用于按字面量构造 {变量名} 匹配） */
const escapeRe = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

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

/** 某种语言可选的库调用方式（value 与 generateRequestCode 的 lib 参数对应） */
export interface CodeLibOption {
  value: string;
  label: string;
  /** 选中该库时的提示文案（i18n key，可选） */
  hint?: string;
}

/** 支持库切换的语言 → 库列表（第一个为默认） */
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

/** 语言默认库：取 CODE_LIBS 第一项；不支持库切换的语言返回 undefined */
export function defaultLib(lang: CodeLang): string | undefined {
  return CODE_LIBS[lang]?.[0]?.value;
}

/** WebSocket 代码生成的可选库（目前 C / C++ / PHP / Ruby / Java / Swift / Perl / Julia / Kotlin 语言支持库切换） */
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

/** 转义字符串内容为 "..." 内的转义文本（引号 / 反斜杠 / 换行等） */
function esc(s: string, q: string): string {
  return s
    .replace(/\\/g, "\\\\")
    .split(q)
    .join("\\" + q)
    .replace(/\n/g, "\\n")
    .replace(/\r/g, "\\r")
    .replace(/\t/g, "\\t");
}

interface Req {
  method: string;
  url: string;
  headers: { key: string; value: string }[];
  body: string;
  bodyKind: "none" | "json" | "text";
  formText: { key: string; value: string }[];
  files: { key: string; path: string }[];
}

function buildReq(api: ApiFile, baseUrl: string): Req {
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
      .map((q) => `${encodeURIComponent(q.key)}=${encodeURIComponent(q.value)}`)
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

function genCurl(r: Req): string {
  const q = (s: string) => `'${s.replace(/'/g, "'\\''")}'`;
  const parts = [`curl -X ${r.method} ${q(r.url)}`];
  for (const h of r.headers) parts.push(`-H ${q(`${h.key}: ${h.value}`)}`);
  if (r.files.length) {
    // multipart：文本字段用 -F key=value，文件字段用 -F key=@路径
    for (const t of r.formText) parts.push(`-F ${q(`${t.key}=${t.value}`)}`);
    for (const f of r.files) parts.push(`-F ${q(`${f.key}=@${f.path}`)}`);
  } else if (r.body) {
    parts.push(`--data-raw ${q(r.body)}`);
  }
  return parts.map((p, i) => (i < parts.length - 1 ? p + " \\" : p)).join("\n");
}

function genPython(r: Req): string {
  const out: string[] = [];
  out.push("import requests");
  if (r.bodyKind === "json") out.push("import json");
  out.push("");
  out.push(`url = "${esc(r.url, '"')}"`);
  if (r.headers.length) {
    out.push("");
    out.push("headers = {");
    for (const h of r.headers) out.push(`    "${esc(h.key, '"')}": "${esc(h.value, '"')}",`);
    out.push("}");
  }
  if (r.body) {
    out.push("");
    out.push(`payload = """${esc(r.body, '"')}"""`);
  }
  if (r.files.length) {
    out.push("");
    if (r.formText.length) {
      out.push("data = {");
      for (const t of r.formText) out.push(`    "${esc(t.key, '"')}": "${esc(t.value, '"')}",`);
      out.push("}");
    }
    out.push("files = {");
    for (const f of r.files) out.push(`    "${esc(f.key, '"')}": open("${esc(f.path, '"')}", "rb"),`);
    out.push("}");
  }
  out.push("");
  const m = r.method.toLowerCase();
  const args: string[] = [];
  if (r.headers.length) args.push("headers=headers");
  if (r.bodyKind === "json") args.push("json=json.loads(payload)");
  else if (r.bodyKind === "text") args.push("data=payload");
  if (r.files.length) args.push("files=files");
  if (r.files.length && r.formText.length) {
    // 用 data 字典而不是 urlencoded payload
    args.splice(args.indexOf("data=payload"), 1, "data=data");
  }
  out.push(`response = requests.${m}(url${args.length ? ", " + args.join(", ") : ""})`);
  out.push("");
  out.push("print(response.status_code)");
  out.push("print(response.text)");
  return out.join("\n");
}

function genGo(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("// 该表单包含文件上传（multipart/form-data），Go 请使用 mime/multipart 构造请求");
  }
  out.push("package main");
  out.push("");
  out.push("import (");
  out.push("\t\"bytes\"");
  out.push("\t\"fmt\"");
  out.push("\t\"io\"");
  out.push("\t\"net/http\"");
  out.push(")");
  out.push("");
  out.push("func main() {");
  out.push(`\turl := "${esc(r.url, '"')}"`);
  if (r.body) {
    out.push("");
    out.push(`\treqBody := []byte("${esc(r.body, '"')}")`);
  }
  out.push("");
  if (r.body) {
    out.push(`\treq, err := http.NewRequest("${r.method}", url, bytes.NewBuffer(reqBody))`);
  } else {
    out.push(`\treq, err := http.NewRequest("${r.method}", url, nil)`);
  }
  out.push("\tif err != nil {");
  out.push("\t\tpanic(err)");
  out.push("\t}");
  for (const h of r.headers) {
    out.push(`\treq.Header.Set("${esc(h.key, '"')}", "${esc(h.value, '"')}")`);
  }
  out.push("");
  out.push("\tresp, err := http.DefaultClient.Do(req)");
  out.push("\tif err != nil {");
  out.push("\t\tpanic(err)");
  out.push("\t}");
  out.push("\tdefer resp.Body.Close()");
  out.push("");
  out.push("\tbody, err := io.ReadAll(resp.Body)");
  out.push("\tif err != nil {");
  out.push("\t\tpanic(err)");
  out.push("\t}");
  out.push("\tfmt.Println(string(body))");
  out.push("}");
  return out.join("\n");
}

/** Rust 原始字符串 r#"..."#，按内容自动选择 # 数量 */
function rustRaw(s: string): string {
  let hashes = 1;
  while (s.includes('"#' + "#".repeat(hashes - 1))) hashes++;
  const d = '"#' + "#".repeat(hashes - 1);
  return `r${d}${s}${d}`;
}

function genRust(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("// 该表单包含文件上传（multipart/form-data），Rust 请使用 reqwest::multipart 构造请求");
  }
  out.push("use reqwest;");
  out.push("");
  out.push("#[tokio::main]");
  out.push("async fn main() -> Result<(), Box<dyn std::error::Error>> {");
  out.push("    let client = reqwest::Client::new();");
  out.push("");
  out.push("    let resp = client");
  const m = r.method.toLowerCase();
  out.push(`        .${m}("${esc(r.url, '"')}")`);
  for (const h of r.headers) {
    out.push(`        .header("${esc(h.key, '"')}", "${esc(h.value, '"')}")`);
  }
  if (r.body) {
    out.push(`        .body(${rustRaw(r.body)})`);
  }
  out.push("        .send()");
  out.push("        .await?;");
  out.push("");
  out.push("    println!(\"{}\", resp.text().await?);");
  out.push("    Ok(())");
  out.push("}");
  return out.join("\n");
}

function genJava(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("// 该表单包含文件上传（multipart/form-data），Java 请使用 MultipartBody.Builder 构造请求");
  }
  out.push("import java.net.URI;");
  out.push("import java.net.http.HttpClient;");
  out.push("import java.net.http.HttpRequest;");
  out.push("import java.net.http.HttpResponse;");
  out.push("");
  out.push("public class Main {");
  out.push("    public static void main(String[] args) throws Exception {");
  out.push("        HttpClient client = HttpClient.newHttpClient();");
  out.push("");
  out.push("        HttpRequest request = HttpRequest.newBuilder()");
  out.push(`            .uri(URI.create("${esc(r.url, '"')}"))`);
  for (const h of r.headers) {
    out.push(`            .header("${esc(h.key, '"')}", "${esc(h.value, '"')}")`);
  }
  if (r.body) {
    out.push(`            .method("${r.method}", HttpRequest.BodyPublishers.ofString("${esc(r.body, '"')}"))`);
  } else {
    out.push(`            .method("${r.method}", HttpRequest.BodyPublishers.noBody())`);
  }
  out.push("            .build();");
  out.push("");
  out.push("        HttpResponse<String> response = client.send(request, HttpResponse.BodyHandlers.ofString());");
  out.push("        System.out.println(response.body());");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

function genJavaScript(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("// 该表单包含文件上传（multipart/form-data），浏览器环境请使用 FormData");
  }
  out.push(`const url = "${esc(r.url, '"')}";`);
  out.push("");
  if (r.headers.length) {
    out.push("const headers = {");
    for (const h of r.headers) out.push(`  "${esc(h.key, '"')}": "${esc(h.value, '"')}",`);
    out.push("};");
    out.push("");
  }
  if (r.bodyKind === "json") {
    out.push(`const payload = ${r.body};`);
    out.push("");
  } else if (r.bodyKind === "text") {
    out.push(`const payload = "${esc(r.body, '"')}";`);
    out.push("");
  }
  out.push("fetch(url, {");
  out.push(`  method: "${r.method}",`);
  if (r.headers.length) out.push("  headers,");
  if (r.body) out.push("  body: JSON.stringify(payload),");
  out.push("})");
  out.push("  .then((res) => res.text())");
  out.push("  .then((text) => console.log(text))");
  out.push("  .catch((err) => console.error(err));");
  return out.join("\n");
}

/** C（libcurl） */
function genC(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("// 该表单包含文件上传（multipart/form-data），请使用 curl_formadd() 构造 multipart 请求");
  }
  out.push("#include <stdio.h>");
  out.push("#include <string.h>");
  out.push("#include <curl/curl.h>");
  out.push("");
  out.push("static size_t write_cb(void *ptr, size_t size, size_t nmemb, void *userdata) {");
  out.push("    (void)userdata;");
  out.push("    return fwrite(ptr, size, nmemb, stdout);");
  out.push("}");
  out.push("");
  out.push("int main(void) {");
  out.push("    CURL *curl = curl_easy_init();");
  out.push("    if (!curl) return 1;");
  out.push("    struct curl_slist *headers = NULL;");
  for (const h of r.headers) {
    out.push(`    headers = curl_slist_append(headers, "${esc(`${h.key}: ${h.value}`, '"')}");`);
  }
  out.push(`    curl_easy_setopt(curl, CURLOPT_URL, "${esc(r.url, '"')}");`);
  out.push(`    curl_easy_setopt(curl, CURLOPT_CUSTOMREQUEST, "${r.method}");`);
  if (r.headers.length) out.push("    curl_easy_setopt(curl, CURLOPT_HTTPHEADER, headers);");
  if (r.body) {
    out.push(`    curl_easy_setopt(curl, CURLOPT_POSTFIELDS, "${esc(r.body, '"')}");`);
    out.push(`    curl_easy_setopt(curl, CURLOPT_POSTFIELDSIZE, (long)strlen("${esc(r.body, '"')}"));`);
  }
  out.push("    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, write_cb);");
  out.push("    CURLcode res = curl_easy_perform(curl);");
  out.push("    curl_slist_free_all(headers);");
  out.push("    curl_easy_cleanup(curl);");
  out.push("    return res != CURLE_OK;");
  out.push("}");
  return out.join("\n");
}

/** C++（libcurl） */
function genCpp(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("// 该表单包含文件上传（multipart/form-data），请使用 curl_formadd() 构造 multipart 请求");
  }
  out.push("#include <iostream>");
  out.push("#include <string>");
  out.push("#include <curl/curl.h>");
  out.push("");
  out.push("static size_t write_cb(void *ptr, size_t size, size_t nmemb, void *userdata) {");
  out.push("    (void)userdata;");
  out.push("    std::cout.write(static_cast<const char *>(ptr), size * nmemb);");
  out.push("    return size * nmemb;");
  out.push("}");
  out.push("");
  out.push("int main() {");
  out.push("    CURL *curl = curl_easy_init();");
  out.push("    if (!curl) return 1;");
  out.push("    struct curl_slist *headers = NULL;");
  for (const h of r.headers) {
    out.push(`    headers = curl_slist_append(headers, "${esc(`${h.key}: ${h.value}`, '"')}");`);
  }
  out.push(`    curl_easy_setopt(curl, CURLOPT_URL, "${esc(r.url, '"')}");`);
  out.push(`    curl_easy_setopt(curl, CURLOPT_CUSTOMREQUEST, "${r.method}");`);
  if (r.headers.length) out.push("    curl_easy_setopt(curl, CURLOPT_HTTPHEADER, headers);");
  if (r.body) {
    out.push(`    std::string body = "${esc(r.body, '"')}";`);
    out.push("    curl_easy_setopt(curl, CURLOPT_POSTFIELDS, body.c_str());");
    out.push("    curl_easy_setopt(curl, CURLOPT_POSTFIELDSIZE, (long)body.size());");
  }
  out.push("    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, write_cb);");
  out.push("    CURLcode res = curl_easy_perform(curl);");
  out.push("    curl_slist_free_all(headers);");
  out.push("    curl_easy_cleanup(curl);");
  out.push("    return res != CURLE_OK;");
  out.push("}");
  return out.join("\n");
}

const CS_METHODS: Record<string, string> = {
  GET: "Get",
  POST: "Post",
  PUT: "Put",
  DELETE: "Delete",
  PATCH: "Patch",
  HEAD: "Head",
  OPTIONS: "Options",
};

/** C#（HttpClient） */
function genCsharp(r: Req): string {
  const out: string[] = [];
  const contentType = r.bodyKind === "json" ? "application/json" : "text/plain";
  out.push("using System;");
  out.push("using System.Net.Http;");
  out.push("using System.Net.Http.Headers;");
  out.push("using System.Threading.Tasks;");
  out.push("");
  out.push("class Program");
  out.push("{");
  out.push("    static async Task Main()");
  out.push("    {");
  out.push("        using var client = new HttpClient();");
  out.push(`        var request = new HttpRequestMessage(HttpMethod.${CS_METHODS[r.method] ?? "Get"}, "${esc(r.url, '"')}");`);
  for (const h of r.headers) {
    out.push(`        request.Headers.TryAddWithoutValidation("${esc(h.key, '"')}", "${esc(h.value, '"')}");`);
  }
  if (r.body) {
    out.push("");
    out.push(`        var body = "${esc(r.body, '"')}";`);
    out.push("        request.Content = new StringContent(body);");
    out.push(`        request.Content.Headers.ContentType = new MediaTypeHeaderValue("${contentType}");`);
  } else if (r.files.length) {
    out.push("");
    out.push("        // 该表单包含文件上传（multipart/form-data），请使用 MultipartFormDataContent 构造请求");
  }
  out.push("");
  out.push("        var response = await client.SendAsync(request);");
  out.push("        Console.WriteLine((int)response.StatusCode);");
  out.push("        Console.WriteLine(await response.Content.ReadAsStringAsync());");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

/** Kotlin（OkHttp） */
function genKotlin(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("// 该表单包含文件上传（multipart/form-data），请使用 MultipartBody.Builder 构造请求");
  }
  out.push("import okhttp3.*");
  out.push("import okhttp3.MediaType.Companion.toMediaType");
  out.push("import okhttp3.RequestBody.Companion.toRequestBody");
  out.push("");
  out.push("fun main() {");
  out.push("    val client = OkHttpClient()");
  if (r.body) {
    const contentType = r.bodyKind === "json" ? "application/json; charset=utf-8" : "text/plain; charset=utf-8";
    out.push(`    val mediaType = "${contentType}".toMediaType()`);
    out.push(`    val body = "${esc(r.body, '"')}".toRequestBody(mediaType)`);
  }
  out.push("");
  out.push("    val request = Request.Builder()");
  out.push(`        .url("${esc(r.url, '"')}")`);
  for (const h of r.headers) {
    out.push(`        .header("${esc(h.key, '"')}", "${esc(h.value, '"')}")`);
  }
  if (r.body) {
    out.push(`        .method("${r.method}", body)`);
  } else if (r.method === "GET") {
    out.push("        .get()");
  } else {
    out.push(`        .method("${r.method}", ByteArray(0).toRequestBody(null))`);
  }
  out.push("        .build()");
  out.push("");
  out.push("    client.newCall(request).execute().use { resp ->");
  out.push("        println(resp.code)");
  out.push("        println(resp.body?.string())");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

/** TypeScript（fetch） */
function genTypeScript(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("// 该表单包含文件上传（multipart/form-data），浏览器环境请使用 FormData");
  }
  out.push(`const url: string = "${esc(r.url, '"')}";`);
  out.push("");
  if (r.headers.length) {
    out.push("const headers: Record<string, string> = {");
    for (const h of r.headers) out.push(`  "${esc(h.key, '"')}": "${esc(h.value, '"')}",`);
    out.push("};");
    out.push("");
  }
  if (r.bodyKind === "json") {
    out.push(`const payload: unknown = ${r.body};`);
    out.push("");
  } else if (r.bodyKind === "text") {
    out.push(`const payload: string = "${esc(r.body, '"')}";`);
    out.push("");
  }
  out.push("fetch(url, {");
  out.push(`  method: "${r.method}",`);
  if (r.headers.length) out.push("  headers,");
  if (r.body) out.push("  body: JSON.stringify(payload),");
  out.push("})");
  out.push("  .then((res) => res.text())");
  out.push("  .then((text) => console.log(text))");
  out.push("  .catch((err) => console.error(err));");
  return out.join("\n");
}

/** R（httr） */
function genR(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("# 该表单包含文件上传（multipart/form-data），请使用 httr::upload_file() 构造 multipart 请求");
  }
  out.push("library(httr)");
  out.push("");
  out.push(`url <- "${esc(r.url, '"')}"`);
  if (r.headers.length) {
    out.push("");
    out.push("headers <- c(");
    out.push(r.headers.map((h) => `  "${esc(h.key, '"')}" = "${esc(h.value, '"')}"`).join(",\n"));
    out.push(")");
  }
  if (r.body) {
    out.push("");
    out.push(`body <- "${esc(r.body, '"')}"`);
  }
  out.push("");
  const args: string[] = [`"${r.method}"`, "url"];
  if (r.headers.length) args.push("add_headers(headers)");
  if (r.body) args.push(`body = body${r.bodyKind === "json" ? ', encode = "json"' : ""}`);
  out.push(`resp <- VERB(${args.join(", ")})`);
  out.push("");
  out.push('cat(status_code(resp), "\\n")');
  out.push('cat(content(resp, "text", encoding = "UTF-8"), "\\n")');
  return out.join("\n");
}

const DELPHI_METHODS: Record<string, string> = {
  GET: "Get",
  POST: "Post",
  PUT: "Put",
  DELETE: "Delete",
  PATCH: "Patch",
  HEAD: "Head",
  OPTIONS: "Options",
};

/** Delphi（Indy TIdHTTP） */
function genDelphi(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("// 该表单包含文件上传（multipart/form-data），请使用 TIdMultiPartFormDataStream 构造请求");
  }
  out.push("uses");
  out.push("  System.SysUtils, IdHTTP, IdSSLOpenSSL;");
  out.push("");
  out.push("procedure DoRequest;");
  out.push("var");
  out.push("  HTTP: TIdHTTP;");
  out.push("  SSL: TIdSSLIOHandlerSocketOpenSSL;");
  out.push("  Resp: string;");
  if (r.body) out.push("  Stream: TStringStream;");
  out.push("begin");
  out.push("  HTTP := TIdHTTP.Create(nil);");
  out.push("  SSL := TIdSSLIOHandlerSocketOpenSSL.Create(nil);");
  out.push("  HTTP.IOHandler := SSL;");
  out.push("  try");
  out.push(`    HTTP.Request.Method := '${r.method}';`);
  for (const h of r.headers) {
    out.push(`    HTTP.Request.CustomHeaders.AddValue('${esc(h.key, "'")}', '${esc(h.value, "'")}');`);
  }
  const m = DELPHI_METHODS[r.method] ?? "Get";
  if (r.body) {
    out.push(`    Stream := TStringStream.Create('${esc(r.body, "'")}', TEncoding.UTF8);`);
    out.push("    try");
    out.push(`      Resp := HTTP.${m}('${esc(r.url, "'")}', Stream);`);
    out.push("    finally");
    out.push("      Stream.Free;");
    out.push("    end;");
  } else {
    out.push(`    Resp := HTTP.${m}('${esc(r.url, "'")}');`);
  }
  out.push("    WriteLn(Resp);");
  out.push("  finally");
  out.push("    SSL.Free;");
  out.push("    HTTP.Free;");
  out.push("  end;");
  out.push("end;");
  return out.join("\n");
}

/** PHP（cURL，文件上传自动使用 CURLFile 构造 multipart） */
function genPhp(r: Req): string {
  const out: string[] = [];
  out.push("<?php");
  out.push("");
  out.push(`$url = '${esc(r.url, "'")}';`);
  if (r.headers.length) {
    out.push("");
    out.push("$headers = [");
    for (const h of r.headers) out.push(`    '${esc(`${h.key}: ${h.value}`, "'")}',`);
    out.push("];");
  }
  if (r.files.length) {
    out.push("");
    out.push("// 文件上传（multipart/form-data）：文本字段与 CURLFile 文件混用");
    out.push("$postData = [");
    for (const t of r.formText) out.push(`    '${esc(t.key, "'")}' => '${esc(t.value, "'")}',`);
    for (const f of r.files) out.push(`    '${esc(f.key, "'")}' => new CURLFile('${esc(f.path, "'")}'),`);
    out.push("];");
  } else if (r.body) {
    out.push("");
    out.push(`$body = '${esc(r.body, "'")}';`);
  }
  out.push("");
  out.push("$ch = curl_init($url);");
  out.push("curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);");
  out.push(`curl_setopt($ch, CURLOPT_CUSTOMREQUEST, '${r.method}');`);
  if (r.headers.length) out.push("curl_setopt($ch, CURLOPT_HTTPHEADER, $headers);");
  if (r.files.length) {
    out.push("curl_setopt($ch, CURLOPT_POSTFIELDS, $postData);");
  } else if (r.body) {
    out.push("curl_setopt($ch, CURLOPT_POSTFIELDS, $body);");
  }
  out.push("$response = curl_exec($ch);");
  out.push("$status = curl_getinfo($ch, CURLINFO_HTTP_CODE);");
  out.push("curl_close($ch);");
  out.push("");
  out.push('echo $status . "\\n";');
  out.push("echo $response;");
  return out.join("\n");
}

const RUBY_CLASSES: Record<string, string> = {
  GET: "Net::HTTP::Get",
  POST: "Net::HTTP::Post",
  PUT: "Net::HTTP::Put",
  DELETE: "Net::HTTP::Delete",
  PATCH: "Net::HTTP::Patch",
  HEAD: "Net::HTTP::Head",
  OPTIONS: "Net::HTTP::Options",
};

/** Ruby（net/http） */
function genRuby(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("# 该表单包含文件上传（multipart/form-data），请使用 Net::HTTP 配合 multipart 构造请求");
  }
  out.push("require 'net/http'");
  out.push("require 'uri'");
  if (r.bodyKind === "json") out.push("require 'json'");
  out.push("");
  out.push(`uri = URI.parse("${esc(r.url, '"')}")`);
  out.push("http = Net::HTTP.new(uri.host, uri.port)");
  out.push("http.use_ssl = uri.scheme == 'https'");
  out.push("");
  out.push(`request = ${RUBY_CLASSES[r.method] ?? "Net::HTTP::Get"}.new(uri.request_uri)`);
  for (const h of r.headers) {
    out.push(`request['${esc(h.key, "'")}'] = '${esc(h.value, "'")}'`);
  }
  if (r.body) {
    out.push(`request.body = '${esc(r.body, "'")}'`);
    out.push(`request.content_type = '${r.bodyKind === "json" ? "application/json" : "text/plain"}'`);
  }
  out.push("");
  out.push("response = http.request(request)");
  out.push("puts response.code");
  out.push("puts response.body");
  return out.join("\n");
}

/** Swift（URLSession） */
function genSwift(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("// 该表单包含文件上传（multipart/form-data），请使用 URLSession uploadTask 配合 multipart body 构造请求");
  }
  out.push("import Foundation");
  out.push("");
  out.push(`let url = URL(string: "${esc(r.url, '"')}")!`);
  out.push("var request = URLRequest(url: url)");
  out.push(`request.httpMethod = "${r.method}"`);
  for (const h of r.headers) {
    out.push(`request.setValue("${esc(h.value, '"')}", forHTTPHeaderField: "${esc(h.key, '"')}")`);
  }
  if (r.body) {
    out.push(`request.httpBody = "${esc(r.body, '"')}".data(using: .utf8)`);
  }
  out.push("");
  out.push("let semaphore = DispatchSemaphore(value: 0)");
  out.push("let task = URLSession.shared.dataTask(with: request) { data, response, error in");
  out.push("    if let error = error {");
  out.push("        print(\"Error: \\(error)\")");
  out.push("    } else if let data = data {");
  out.push("        print(String(data: data, encoding: .utf8) ?? \"\")");
  out.push("    }");
  out.push("    semaphore.signal()");
  out.push("}");
  out.push("task.resume()");
  out.push("semaphore.wait()");
  return out.join("\n");
}

/** Perl（LWP::UserAgent） */
function genPerl(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("# 该表单包含文件上传（multipart/form-data），请使用 HTTP::Request::Common 构造请求");
  }
  out.push("#!/usr/bin/perl");
  out.push("use strict;");
  out.push("use warnings;");
  out.push("use LWP::UserAgent;");
  out.push("use HTTP::Request;");
  out.push("");
  out.push(`my $url = '${esc(r.url, "'")}';`);
  out.push("my $ua = LWP::UserAgent->new;");
  out.push(`my $req = HTTP::Request->new('${r.method}', $url);`);
  for (const h of r.headers) {
    out.push(`$req->header('${esc(h.key, "'")}' => '${esc(h.value, "'")}');`);
  }
  if (r.body) {
    out.push(`$req->content('${esc(r.body, "'")}');`);
    out.push(`$req->content_type('${r.bodyKind === "json" ? "application/json" : "text/plain"}');`);
  }
  out.push("");
  out.push("my $resp = $ua->request($req);");
  out.push('print $resp->code, "\\n";');
  out.push('print $resp->decoded_content, "\\n";');
  return out.join("\n");
}

/** Objective-C（NSURLSession） */
function genObjectiveC(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("// 该表单包含文件上传（multipart/form-data），请使用 NSURLSessionUploadTask 配合 multipart body 构造请求");
  }
  out.push("#import <Foundation/Foundation.h>");
  out.push("");
  out.push("int main(int argc, const char * argv[]) {");
  out.push("    @autoreleasepool {");
  out.push(`        NSURL *url = [NSURL URLWithString:@"${esc(r.url, '"')}"];`);
  out.push("        NSMutableURLRequest *request = [NSMutableURLRequest requestWithURL:url];");
  out.push(`        request.HTTPMethod = @"${r.method}";`);
  for (const h of r.headers) {
    out.push(`        [request setValue:@"${esc(h.value, '"')}" forHTTPHeaderField:@"${esc(h.key, '"')}"];`);
  }
  if (r.body) {
    out.push(`        request.HTTPBody = [@"${esc(r.body, '"')}" dataUsingEncoding:NSUTF8StringEncoding];`);
  }
  out.push("");
  out.push("        dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);");
  out.push("        NSURLSession *session = [NSURLSession sharedSession];");
  out.push("        NSURLSessionDataTask *task = [session dataTaskWithRequest:request");
  out.push("            completionHandler:^(NSData *data, NSURLResponse *response, NSError *error) {");
  out.push("                if (error) {");
  out.push("                    NSLog(@\"Error: %@\", error);");
  out.push("                } else if (data) {");
  out.push("                    NSLog(@\"%@\", [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding]);");
  out.push("                }");
  out.push("                dispatch_semaphore_signal(semaphore);");
  out.push("            }];");
  out.push("        [task resume];");
  out.push("        dispatch_semaphore_wait(semaphore, DISPATCH_TIME_FOREVER);");
  out.push("    }");
  out.push("    return 0;");
  out.push("}");
  return out.join("\n");
}

/** Julia（HTTP.jl） */
function genJulia(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("# 该表单包含文件上传（multipart/form-data），请使用 HTTP.Multipart 构造请求");
  }
  out.push("using HTTP");
  out.push("");
  out.push(`url = "${esc(r.url, '"')}"`);
  if (r.headers.length) {
    out.push("");
    out.push("headers = [");
    for (const h of r.headers) out.push(`    "${esc(h.key, '"')}" => "${esc(h.value, '"')}",`);
    out.push("]");
  }
  if (r.body) {
    out.push("");
    out.push(`body = "${esc(r.body, '"')}"`);
  }
  out.push("");
  const args: string[] = [`"${r.method}"`, "url"];
  if (r.headers.length) args.push("headers");
  if (r.body) args.push("body");
  out.push(`resp = HTTP.request(${args.join(", ")})`);
  out.push("println(resp.status)");
  out.push("println(String(resp.body))");
  return out.join("\n");
}

/** Erlang（httpc / inets） */
function genErlang(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("%% 该表单包含文件上传（multipart/form-data），请使用 httpc multipart 或 ibrowse 构造请求");
  }
  out.push("-module(request).");
  out.push("-export([main/0]).");
  out.push("");
  out.push("main() ->");
  out.push("    inets:start(),");
  out.push("    ssl:start(),");
  out.push(`    URL = "${esc(r.url, '"')}",`);
  out.push("    Headers = [");
  for (const h of r.headers) {
    out.push(`        {"${esc(h.key, '"')}", "${esc(h.value, '"')}"},`);
  }
  out.push("    ],");
  if (r.body) {
    out.push(`    Body = "${esc(r.body, '"')}",`);
    out.push(`    ContentType = "${r.bodyKind === "json" ? "application/json" : "text/plain"}",`);
  }
  out.push(`    Method = ${r.method.toLowerCase()},`);
  if (r.body) {
    out.push("    Request = {URL, Headers, ContentType, Body},");
  } else {
    out.push("    Request = {URL, Headers},");
  }
  out.push("    case httpc:request(Method, Request, [], []) of");
  out.push("        {ok, {{_, Status, _}, _, RespBody}} ->");
  out.push('            io:format("~p~n", [Status]),');
  out.push('            io:format("~s~n", [RespBody]);');
  out.push("        {error, Reason} ->");
  out.push('            io:format("Error: ~p~n", [Reason])');
  out.push("    end.");
  return out.join("\n");
}

/* ==================== Bash：Wget / HTTPie ==================== */

function genBashWget(r: Req): string {
  const q = (s: string) => `'${s.replace(/'/g, "'\\''")}'`;
  const parts = [`wget --method=${r.method} ${q(r.url)}`];
  for (const h of r.headers) parts.push(`--header=${q(`${h.key}: ${h.value}`)}`);
  if (r.body) parts.push(`--body-data=${q(r.body)}`);
  parts.push("-O -");
  const lines = parts.map((p, i) => (i < parts.length - 1 ? p + " \\" : p));
  if (r.files.length) {
    lines.unshift("# 注意：wget 不支持 multipart/form-data 文件上传");
  }
  return lines.join("\n");
}

function genBashHttpie(r: Req): string {
  const q = (s: string) => `'${s.replace(/'/g, "'\\''")}'`;
  const parts = [`http ${r.method} ${q(r.url)}`];
  for (const h of r.headers) parts.push(q(`${h.key}: ${h.value}`));
  if (r.files.length) {
    for (const t of r.formText) parts.push(`${t.key}=${q(t.value)}`);
    for (const f of r.files) parts.push(`${f.key}@${q(f.path)}`);
  } else if (r.body) {
    parts.push(`--raw=${q(r.body)}`);
  }
  return parts.map((p, i) => (i < parts.length - 1 ? p + " \\" : p)).join("\n");
}

/* ==================== Python：http.client ==================== */

function genPythonHttpClient(r: Req): string {
  const out: string[] = [];
  out.push("import http.client");
  out.push("import json");
  out.push("from urllib.parse import urlparse");
  out.push("");
  out.push(`url = "${esc(r.url, '"')}"`);
  if (r.headers.length) {
    out.push("");
    out.push("headers = {");
    for (const h of r.headers) out.push(`    "${esc(h.key, '"')}": "${esc(h.value, '"')}",`);
    out.push("}");
  }
  if (r.body) {
    out.push("");
    out.push(`payload = """${esc(r.body, '"')}"""`);
  }
  out.push("");
  out.push("u = urlparse(url)");
  out.push('conn = (http.client.HTTPSConnection if u.scheme == "https" else http.client.HTTPConnection)(');
  out.push('    u.hostname, u.port or (443 if u.scheme == "https" else 80))');
  out.push('path = u.path + (("?" + u.query) if u.query else "")');
  const args: string[] = [`"${r.method}"`, "path"];
  if (r.headers.length) args.push("headers=headers");
  if (r.body) args.push("payload");
  out.push(`conn.request(${args.join(", ")})`);
  out.push("res = conn.getresponse()");
  out.push("print(res.status, res.reason)");
  out.push('print(res.read().decode("utf-8"))');
  out.push("conn.close()");
  return out.join("\n");
}

/* ==================== JavaScript：Axios / Unirest / Request / Native ==================== */

function genJsAxios(r: Req): string {
  const out: string[] = [];
  out.push('const axios = require("axios");');
  out.push("");
  out.push(`const url = "${esc(r.url, '"')}";`);
  if (r.headers.length) {
    out.push("");
    out.push("const headers = {");
    for (const h of r.headers) out.push(`  "${esc(h.key, '"')}": "${esc(h.value, '"')}",`);
    out.push("};");
  }
  if (r.bodyKind === "json") {
    out.push("");
    out.push(`const payload = ${r.body};`);
  } else if (r.bodyKind === "text") {
    out.push("");
    out.push(`const payload = "${esc(r.body, '"')}";`);
  }
  out.push("");
  out.push("axios({");
  out.push(`  method: "${r.method}",`);
  out.push("  url,");
  if (r.headers.length) out.push("  headers,");
  if (r.body) out.push("  data: payload,");
  out.push("})");
  out.push("  .then((res) => console.log(res.data))");
  out.push("  .catch((err) => console.error(err));");
  return out.join("\n");
}

function genJsUnirest(r: Req): string {
  const out: string[] = [];
  out.push('const unirest = require("unirest");');
  out.push("");
  out.push(`unirest("${r.method}", "${esc(r.url, '"')}")`);
  if (r.headers.length) {
    out.push("  .headers({");
    for (const h of r.headers) out.push(`    "${esc(h.key, '"')}": "${esc(h.value, '"')}",`);
    out.push("  })");
  }
  if (r.files.length) {
    for (const f of r.files) out.push(`  .attach("${esc(f.key, '"')}", "${esc(f.path, '"')}")`);
    for (const t of r.formText) out.push(`  .field("${esc(t.key, '"')}", "${esc(t.value, '"')}")`);
  } else if (r.body) {
    out.push(`  .send(${r.bodyKind === "json" ? r.body : `"${esc(r.body, '"')}"`})`);
  }
  out.push("  .then((res) => console.log(res.body));");
  return out.join("\n");
}

function genJsRequest(r: Req): string {
  const out: string[] = [];
  out.push('const request = require("request");');
  if (r.files.length) out.push('const fs = require("fs");');
  out.push("");
  out.push(`const url = "${esc(r.url, '"')}";`);
  if (r.headers.length) {
    out.push("");
    out.push("const headers = {");
    for (const h of r.headers) out.push(`  "${esc(h.key, '"')}": "${esc(h.value, '"')}",`);
    out.push("};");
  }
  if (r.bodyKind === "json") {
    out.push("");
    out.push(`const payload = ${r.body};`);
  }
  out.push("");
  out.push("request({");
  out.push(`  method: "${r.method}",`);
  out.push("  url,");
  if (r.headers.length) out.push("  headers,");
  if (r.files.length) {
    out.push("  // 文件上传（multipart）");
    out.push("  formData: {");
    for (const t of r.formText) out.push(`    "${esc(t.key, '"')}": "${esc(t.value, '"')}",`);
    for (const f of r.files) out.push(`    "${esc(f.key, '"')}": fs.createReadStream("${esc(f.path, '"')}"),`);
    out.push("  },");
  } else if (r.bodyKind === "json") {
    out.push("  json: payload,");
  } else if (r.body) {
    out.push(`  body: "${esc(r.body, '"')}",`);
  }
  out.push("}, (error, response, body) => {");
  out.push("  if (error) return console.error(error);");
  out.push("  console.log(body);");
  out.push("});");
  return out.join("\n");
}

function genJsNative(r: Req): string {
  const out: string[] = [];
  out.push(`const url = "${esc(r.url, '"')}";`);
  if (r.headers.length) {
    out.push("");
    out.push("const headers = {");
    for (const h of r.headers) out.push(`  "${esc(h.key, '"')}": "${esc(h.value, '"')}",`);
    out.push("};");
  }
  if (r.bodyKind === "json") {
    out.push("");
    out.push(`const payload = ${r.body};`);
  } else if (r.bodyKind === "text") {
    out.push("");
    out.push(`const payload = "${esc(r.body, '"')}";`);
  }
  out.push("");
  out.push("const xhr = new XMLHttpRequest();");
  out.push(`xhr.open("${r.method}", url);`);
  for (const h of r.headers) out.push(`xhr.setRequestHeader("${esc(h.key, '"')}", "${esc(h.value, '"')}");`);
  out.push("xhr.onload = () => console.log(xhr.responseText);");
  out.push("xhr.onerror = (e) => console.error(e);");
  out.push(`xhr.send(${r.body ? (r.bodyKind === "json" ? "JSON.stringify(payload)" : "payload") : "null"});`);
  return out.join("\n");
}

/* ==================== TypeScript：Axios / Unirest / Request / Native ==================== */

function genTsAxios(r: Req): string {
  const out: string[] = [];
  out.push('import axios from "axios";');
  out.push("");
  out.push(`const url: string = "${esc(r.url, '"')}";`);
  if (r.headers.length) {
    out.push("");
    out.push("const headers: Record<string, string> = {");
    for (const h of r.headers) out.push(`  "${esc(h.key, '"')}": "${esc(h.value, '"')}",`);
    out.push("};");
  }
  if (r.bodyKind === "json") {
    out.push("");
    out.push(`const payload: unknown = ${r.body};`);
  } else if (r.bodyKind === "text") {
    out.push("");
    out.push(`const payload: string = "${esc(r.body, '"')}";`);
  }
  out.push("");
  out.push("axios({");
  out.push(`  method: "${r.method}",`);
  out.push("  url,");
  if (r.headers.length) out.push("  headers,");
  if (r.body) out.push("  data: payload,");
  out.push("})");
  out.push("  .then((res) => console.log(res.data))");
  out.push("  .catch((err) => console.error(err));");
  return out.join("\n");
}

function genTsUnirest(r: Req): string {
  const out: string[] = [];
  out.push('import unirest from "unirest";');
  out.push("");
  out.push(`unirest("${r.method}", "${esc(r.url, '"')}")`);
  if (r.headers.length) {
    out.push("  .headers({");
    for (const h of r.headers) out.push(`    "${esc(h.key, '"')}": "${esc(h.value, '"')}",`);
    out.push("  })");
  }
  if (r.files.length) {
    for (const f of r.files) out.push(`  .attach("${esc(f.key, '"')}", "${esc(f.path, '"')}")`);
    for (const t of r.formText) out.push(`  .field("${esc(t.key, '"')}", "${esc(t.value, '"')}")`);
  } else if (r.body) {
    out.push(`  .send(${r.bodyKind === "json" ? r.body : `"${esc(r.body, '"')}"`})`);
  }
  out.push("  .then((res) => console.log(res.body));");
  return out.join("\n");
}

function genTsRequest(r: Req): string {
  const out: string[] = [];
  out.push('import request from "request";');
  if (r.files.length) out.push('import fs from "fs";');
  out.push("");
  out.push(`const url: string = "${esc(r.url, '"')}";`);
  if (r.headers.length) {
    out.push("");
    out.push("const headers: Record<string, string> = {");
    for (const h of r.headers) out.push(`  "${esc(h.key, '"')}": "${esc(h.value, '"')}",`);
    out.push("};");
  }
  if (r.bodyKind === "json") {
    out.push("");
    out.push(`const payload: unknown = ${r.body};`);
  }
  out.push("");
  out.push("request({");
  out.push(`  method: "${r.method}",`);
  out.push("  url,");
  if (r.headers.length) out.push("  headers,");
  if (r.files.length) {
    out.push("  // 文件上传（multipart）");
    out.push("  formData: {");
    for (const t of r.formText) out.push(`    "${esc(t.key, '"')}": "${esc(t.value, '"')}",`);
    for (const f of r.files) out.push(`    "${esc(f.key, '"')}": fs.createReadStream("${esc(f.path, '"')}"),`);
    out.push("  },");
  } else if (r.bodyKind === "json") {
    out.push("  json: payload,");
  } else if (r.body) {
    out.push(`  body: "${esc(r.body, '"')}",`);
  }
  out.push("}, (error, response, body) => {");
  out.push("  if (error) return console.error(error);");
  out.push("  console.log(body);");
  out.push("});");
  return out.join("\n");
}

function genTsNative(r: Req): string {
  const out: string[] = [];
  out.push(`const url: string = "${esc(r.url, '"')}";`);
  if (r.headers.length) {
    out.push("");
    out.push("const headers: Record<string, string> = {");
    for (const h of r.headers) out.push(`  "${esc(h.key, '"')}": "${esc(h.value, '"')}",`);
    out.push("};");
  }
  if (r.bodyKind === "json") {
    out.push("");
    out.push(`const payload: unknown = ${r.body};`);
  } else if (r.bodyKind === "text") {
    out.push("");
    out.push(`const payload: string = "${esc(r.body, '"')}";`);
  }
  out.push("");
  out.push("const xhr: XMLHttpRequest = new XMLHttpRequest();");
  out.push(`xhr.open("${r.method}", url);`);
  for (const h of r.headers) out.push(`xhr.setRequestHeader("${esc(h.key, '"')}", "${esc(h.value, '"')}");`);
  out.push("xhr.onload = () => console.log(xhr.responseText);");
  out.push("xhr.onerror = (e) => console.error(e);");
  out.push(`xhr.send(${r.body ? (r.bodyKind === "json" ? "JSON.stringify(payload)" : "payload") : "null"});`);
  return out.join("\n");
}

/* ==================== Java：OkHttp / Unirest / WebClient / Retrofit2 / HttpClient5 ==================== */

function genJavaOkHttp(r: Req): string {
  const out: string[] = [];
  out.push("import okhttp3.MediaType;");
  out.push("import okhttp3.OkHttpClient;");
  out.push("import okhttp3.Request;");
  out.push("import okhttp3.RequestBody;");
  out.push("import okhttp3.Response;");
  if (r.files.length) {
    out.push("import okhttp3.MultipartBody;");
    out.push("import java.io.File;");
  }
  out.push("");
  out.push("public class Main {");
  out.push("    public static void main(String[] args) throws Exception {");
  out.push("        OkHttpClient client = new OkHttpClient();");
  out.push("");
  if (r.files.length) {
    out.push("        // 文件上传：使用 MultipartBody.Builder 构造请求体");
    out.push("        RequestBody body = new MultipartBody.Builder()");
    out.push("                .setType(MultipartBody.FORM)");
    for (const t of r.formText) out.push(`                .addFormDataPart("${esc(t.key, '"')}", "${esc(t.value, '"')}")`);
    for (const f of r.files) {
      const fname = (f.path.split(/[\\/]/).pop() || "file").replace(/"/g, "");
      out.push(`                .addFormDataPart("${esc(f.key, '"')}", "${esc(fname, '"')}", RequestBody.create(new File("${esc(f.path, '"')}"), MediaType.parse("application/octet-stream")))`);
    }
    out.push("                .build();");
  } else if (r.body) {
    const mt = r.bodyKind === "json" ? "application/json; charset=utf-8" : "text/plain; charset=utf-8";
    out.push(`        RequestBody body = RequestBody.create("${esc(r.body, '"')}", MediaType.parse("${mt}"));`);
  } else {
    out.push("        RequestBody body = null;");
  }
  out.push("");
  out.push("        Request request = new Request.Builder()");
  out.push(`                .url("${esc(r.url, '"')}")`);
  for (const h of r.headers) out.push(`                .addHeader("${esc(h.key, '"')}", "${esc(h.value, '"')}")`);
  out.push(`                .method("${r.method}", body)`);
  out.push("                .build();");
  out.push("");
  out.push("        try (Response response = client.newCall(request).execute()) {");
  out.push("            System.out.println(response.body().string());");
  out.push("        }");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

function genJavaUnirest(r: Req): string {
  const out: string[] = [];
  out.push("import kong.unirest.HttpResponse;");
  out.push("import kong.unirest.Unirest;");
  if (r.files.length) out.push("import java.io.File;");
  out.push("");
  out.push("public class Main {");
  out.push("    public static void main(String[] args) {");
  out.push(`        HttpResponse<String> response = Unirest.${r.method.toLowerCase()}(${JSON.stringify(r.url)})`);
  for (const h of r.headers) out.push(`                .header("${esc(h.key, '"')}", "${esc(h.value, '"')}")`);
  if (r.files.length) {
    for (const t of r.formText) out.push(`                .field("${esc(t.key, '"')}", "${esc(t.value, '"')}")`);
    for (const f of r.files) out.push(`                .field("${esc(f.key, '"')}", new File("${esc(f.path, '"')}"))`);
  } else if (r.body) {
    out.push(`                .body("${esc(r.body, '"')}")`);
  }
  out.push("                .asString();");
  out.push("        System.out.println(response.getBody());");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

function genJavaWebClient(r: Req): string {
  const out: string[] = [];
  out.push("import org.springframework.web.reactive.function.client.WebClient;");
  out.push("");
  out.push("public class Main {");
  out.push("    public static void main(String[] args) {");
  out.push("        WebClient client = WebClient.builder().build();");
  out.push("");
  out.push(`        String response = client.${r.method.toLowerCase()}()`);
  out.push(`                .uri("${esc(r.url, '"')}")`);
  for (const h of r.headers) out.push(`                .header("${esc(h.key, '"')}", "${esc(h.value, '"')}")`);
  if (r.body) out.push(`                .bodyValue("${esc(r.body, '"')}")`);
  out.push("                .retrieve()");
  out.push("                .bodyToMono(String.class)");
  out.push("                .block();");
  out.push("        System.out.println(response);");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

function genJavaRetrofit2(r: Req): string {
  let basePart = r.url;
  let pathPart = r.url;
  try {
    const u = new URL(r.url);
    pathPart = u.pathname + u.search;
    basePart = u.origin + "/";
  } catch {
    /* URL 解析失败时保留原始值 */
  }
  const out: string[] = [];
  out.push("import retrofit2.Call;");
  out.push("import retrofit2.Response;");
  out.push("import retrofit2.Retrofit;");
  out.push("import retrofit2.converter.scalars.ScalarsConverterFactory;");
  out.push("import retrofit2.http.Body;");
  out.push("import retrofit2.http.DELETE;");
  out.push("import retrofit2.http.GET;");
  out.push("import retrofit2.http.Header;");
  out.push("import retrofit2.http.POST;");
  out.push("import retrofit2.http.PUT;");
  out.push("");
  out.push("public interface ApiService {");
  out.push(`    @${r.method}("${pathPart}")`);
  const params: string[] = [];
  r.headers.forEach((h, i) => params.push(`@Header("${esc(h.key, '"')}") String h${i}`));
  if (r.body) params.push("@Body String body");
  out.push(`    Call<String> request(${params.join(", ")});`);
  out.push("}");
  out.push("");
  out.push("// 使用示例");
  out.push("public class Main {");
  out.push("    public static void main(String[] args) throws Exception {");
  out.push("        Retrofit retrofit = new Retrofit.Builder()");
  out.push(`                .baseUrl("${basePart}")`);
  out.push("                .addConverterFactory(ScalarsConverterFactory.create())");
  out.push("                .build();");
  out.push("        ApiService service = retrofit.create(ApiService.class);");
  const callArgs: string[] = [];
  for (const h of r.headers) callArgs.push(JSON.stringify(h.value));
  if (r.body) callArgs.push(r.bodyKind === "json" ? r.body : JSON.stringify(r.body));
  out.push(`        Call<String> call = service.request(${callArgs.join(", ")});`);
  out.push("        Response<String> response = call.execute();");
  out.push("        System.out.println(response.body());");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

function genJavaHttpClient5(r: Req): string {
  const M5: Record<string, string> = {
    GET: "HttpGet",
    POST: "HttpPost",
    PUT: "HttpPut",
    DELETE: "HttpDelete",
    PATCH: "HttpPatch",
    HEAD: "HttpHead",
    OPTIONS: "HttpOptions",
  };
  const cls = M5[r.method] || "HttpPost";
  const out: string[] = [];
  out.push(`import org.apache.hc.client5.http.classic.methods.${cls};`);
  out.push("import org.apache.hc.client5.http.impl.classic.CloseableHttpClient;");
  out.push("import org.apache.hc.client5.http.impl.classic.HttpClients;");
  out.push("import org.apache.hc.core5.http.io.entity.EntityUtils;");
  if (r.body) out.push("import org.apache.hc.core5.http.io.entity.StringEntity;");
  out.push("");
  out.push("public class Main {");
  out.push("    public static void main(String[] args) throws Exception {");
  out.push("        try (CloseableHttpClient client = HttpClients.createDefault()) {");
  out.push(`            ${cls} request = new ${cls}(${JSON.stringify(r.url)});`);
  for (const h of r.headers) out.push(`            request.setHeader("${esc(h.key, '"')}", "${esc(h.value, '"')}");`);
  if (r.body) out.push(`            request.setEntity(new StringEntity(${JSON.stringify(r.body)}));`);
  out.push("            client.execute(request, response -> {");
  out.push("                System.out.println(EntityUtils.toString(response.getEntity()));");
  out.push("                return null;");
  out.push("            });");
  out.push("        }");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

/* ==================== PHP：PECL / Snoopy / Guzzle ==================== */

function genPhpPecl(r: Req): string {
  const out: string[] = [];
  out.push("<?php");
  out.push("");
  out.push("$client = new http\\Client;");
  out.push(`$request = new http\\Client\\Request("${r.method}", '${esc(r.url, "'")}');`);
  if (r.headers.length || r.bodyKind === "json") {
    out.push("$request->setOptions([");
    out.push("    'headers' => [");
    for (const h of r.headers) out.push(`        '${esc(h.key, "'")}' => '${esc(h.value, "'")}',`);
    if (r.bodyKind === "json" && !r.headers.some((h) => h.key.toLowerCase() === "content-type")) {
      out.push("        'Content-Type' => 'application/json',");
    }
    out.push("    ],");
    out.push("]);");
  }
  if (r.body) out.push(`$request->setBody('${esc(r.body, "'")}');`);
  out.push("$client->enqueue($request)->send();");
  out.push("$response = $request->getResponse();");
  out.push("");
  out.push('echo $response->getStatusCode() . "\\n";');
  out.push("echo $response->getBody();");
  return out.join("\n");
}

function genPhpSnoopy(r: Req): string {
  const out: string[] = [];
  out.push("<?php");
  out.push("");
  out.push("require_once 'Snoopy.class.php';");
  out.push("");
  out.push("$snoopy = new Snoopy;");
  if (r.headers.length || r.bodyKind === "json") {
    out.push("$snoopy->rawheaders = [");
    for (const h of r.headers) out.push(`    '${esc(h.key, "'")}' => '${esc(h.value, "'")}',`);
    if (r.bodyKind === "json" && !r.headers.some((h) => h.key.toLowerCase() === "content-type")) {
      out.push("    'Content-Type' => 'application/json',");
    }
    out.push("];");
  }
  if (r.files.length) {
    out.push("// 注意：Snoopy 不支持 multipart 文件上传，请改用 cURL / Guzzle");
  } else if (r.body) {
    out.push(`$snoopy->submit('${esc(r.url, "'")}', ['payload' => '${esc(r.body, "'")}']);`);
  } else {
    out.push(`$snoopy->fetch('${esc(r.url, "'")}');`);
  }
  out.push("");
  out.push('echo $snoopy->status . "\\n";');
  out.push("echo $snoopy->results;");
  return out.join("\n");
}

function genPhpGuzzle(r: Req): string {
  const out: string[] = [];
  out.push("<?php");
  out.push("");
  out.push("require 'vendor/autoload.php';");
  out.push("");
  out.push("use GuzzleHttp\\Client;");
  out.push("");
  out.push("$client = new Client();");
  out.push("$options = [");
  if (r.headers.length || (r.bodyKind === "json" && !r.headers.some((h) => h.key.toLowerCase() === "content-type"))) {
    out.push("    'headers' => [");
    for (const h of r.headers) out.push(`        '${esc(h.key, "'")}' => '${esc(h.value, "'")}',`);
    if (r.bodyKind === "json" && !r.headers.some((h) => h.key.toLowerCase() === "content-type")) {
      out.push("        'Content-Type' => 'application/json',");
    }
    out.push("    ],");
  }
  if (r.files.length) {
    out.push("    // 文件上传（multipart）");
    out.push("    'multipart' => [");
    for (const t of r.formText) out.push(`        ['name' => '${esc(t.key, "'")}', 'contents' => '${esc(t.value, "'")}'],`);
    for (const f of r.files) out.push(`        ['name' => '${esc(f.key, "'")}', 'contents' => fopen('${esc(f.path, "'")}', 'r')],`);
    out.push("    ],");
  } else if (r.body) {
    out.push(`    'body' => '${esc(r.body, "'")}',`);
  }
  out.push("];");
  out.push(`$response = $client->request('${r.method}', '${esc(r.url, "'")}', $options);`);
  out.push("");
  out.push('echo $response->getStatusCode() . "\\n";');
  out.push("echo $response->getBody();");
  return out.join("\n");
}

/* ==================== R：RCurl ==================== */

function genRRCurl(r: Req): string {
  const out: string[] = [];
  out.push("library(RCurl)");
  out.push("");
  out.push(`url <- "${esc(r.url, '"')}"`);
  const hdrs: string[] = r.headers.map((h) => `  "${esc(h.key, '"')}" = "${esc(h.value, '"')}"`);
  if (r.bodyKind === "json" && !r.headers.some((h) => h.key.toLowerCase() === "content-type")) {
    hdrs.push('  "Content-Type" = "application/json"');
  }
  if (hdrs.length) {
    out.push("");
    out.push("headers <- c(");
    out.push(hdrs.join(",\n"));
    out.push(")");
  }
  if (r.body) {
    out.push("");
    out.push(`body <- "${esc(r.body, '"')}"`);
  }
  out.push("");
  const args: string[] = ["url"];
  if (hdrs.length) args.push("httpheader = headers");
  if (r.body) args.push("postfields = body");
  args.push(`customrequest = "${r.method}"`);
  args.push("ssl.verifypeer = FALSE");
  out.push(`resp <- getURL(${args.join(", ")})`);
  out.push("");
  out.push("cat(resp)");
  return out.join("\n");
}

/* ==================== Lua：lua-httpclient / lua-curl / lua-resty-httpclient ==================== */

function genLua(r: Req): string {
  const out: string[] = [];
  out.push('local http = require("socket.http")');
  out.push('local ltn12 = require("ltn12")');
  out.push("");
  out.push(`local url = "${esc(r.url, '"')}"`);
  if (r.body) {
    out.push("");
    out.push(`local payload = "${esc(r.body, '"')}"`);
  }
  out.push("");
  out.push("local response_body = {}");
  out.push("local res, code, headers = http.request{");
  out.push("    url = url,");
  out.push(`    method = "${r.method}",`);
  if (r.headers.length || r.bodyKind === "json") {
    out.push("    headers = {");
    for (const h of r.headers) out.push(`        ["${esc(h.key, '"')}"] = "${esc(h.value, '"')}",`);
    if (r.bodyKind === "json" && !r.headers.some((h) => h.key.toLowerCase() === "content-type")) {
      out.push('        ["Content-Type"] = "application/json",');
    }
    out.push("    },");
  }
  if (r.body) out.push("    source = ltn12.source.string(payload),");
  out.push("    sink = ltn12.sink.table(response_body),");
  out.push("}");
  out.push("");
  out.push("print(code)");
  out.push("print(table.concat(response_body))");
  return out.join("\n");
}

function genLuaCurl(r: Req): string {
  const out: string[] = [];
  out.push('local curl = require("lcurl")');
  out.push("");
  out.push("local c = curl.easy()");
  out.push(`c:setopt(curl.OPT_URL, "${esc(r.url, '"')}")`);
  out.push(`c:setopt(curl.OPT_CUSTOMREQUEST, "${r.method}")`);
  if (r.headers.length || r.bodyKind === "json") {
    out.push("c:setopt(curl.OPT_HTTPHEADER, {");
    for (const h of r.headers) out.push(`    "${esc(h.key, '"')}: ${esc(h.value, '"')}",`);
    if (r.bodyKind === "json" && !r.headers.some((h) => h.key.toLowerCase() === "content-type")) {
      out.push('    "Content-Type: application/json",');
    }
    out.push("})");
  }
  if (r.files.length) {
    out.push("// 注意：lua-curl 文件上传请使用 OPT_HTTPPOST / OPT_MIMEPOST 构造 multipart");
    for (const f of r.files) out.push(`c:setopt(curl.OPT_HTTPPOST, { "${esc(f.key, '"')}" = { file = "${esc(f.path, '"')}" } })`);
  } else if (r.body) {
    out.push(`c:setopt(curl.OPT_POSTFIELDS, "${esc(r.body, '"')}")`);
  }
  out.push("c:setopt(curl.OPT_WRITEFUNCTION, function(buffer)");
  out.push("    io.write(buffer)");
  out.push("    return #buffer");
  out.push("end)");
  out.push("");
  out.push("local ok, err = c:perform()");
  out.push("if not ok then");
  out.push('    io.stderr:write(err .. "\\n")');
  out.push("end");
  out.push("c:close()");
  return out.join("\n");
}

function genLuaResty(r: Req): string {
  const out: string[] = [];
  out.push("-- 需要 OpenResty / Nginx Lua 环境（ngx_lua + lua-resty-http）");
  out.push('local http = require("resty.http")');
  out.push("local httpc = http.new()");
  out.push("");
  out.push(`local res, err = httpc:request_uri("${esc(r.url, '"')}", {`);
  out.push(`    method = "${r.method}",`);
  if (r.headers.length || r.bodyKind === "json") {
    out.push("    headers = {");
    for (const h of r.headers) out.push(`        ["${esc(h.key, '"')}"] = "${esc(h.value, '"')}",`);
    if (r.bodyKind === "json" && !r.headers.some((h) => h.key.toLowerCase() === "content-type")) {
      out.push('        ["Content-Type"] = "application/json",');
    }
    out.push("    },");
  }
  if (r.body) out.push(`    body = "${esc(r.body, '"')}",`);
  out.push("})");
  out.push("");
  out.push("if not res then");
  out.push("    ngx.log(ngx.ERR, err)");
  out.push("    return");
  out.push("end");
  out.push("ngx.say(res.status)");
  out.push("ngx.say(res.body)");
  out.push("httpc:close()");
  return out.join("\n");
}

/* ==================== PowerShell ==================== */

function genPowershell(r: Req): string {
  const out: string[] = [];
  if (r.headers.length) {
    out.push("$headers = @{");
    for (const h of r.headers) out.push(`    "${esc(h.key, '"')}" = "${esc(h.value, '"')}"`);
    out.push("}");
  }
  if (r.body) {
    out.push("");
    out.push(`$body = '${esc(r.body, "'")}'`);
  }
  if (r.files.length) {
    out.push("");
    out.push("# 文件上传（PowerShell 7+）：使用 -Form 参数");
    out.push("$form = @{");
    for (const t of r.formText) out.push(`    "${esc(t.key, '"')}" = "${esc(t.value, '"')}"`);
    for (const f of r.files) out.push(`    "${esc(f.key, '"')}" = Get-Item "${esc(f.path, '"')}"`);
    out.push("}");
  }
  out.push("");
  const args: string[] = [`-Uri "${esc(r.url, '"')}"`, `-Method ${r.method}`];
  if (r.headers.length) args.push("-Headers $headers");
  if (r.files.length) args.push("-Form $form");
  else if (r.body) args.push("-Body $body");
  out.push(`$response = Invoke-RestMethod ${args.join(" ")}`);
  out.push("");
  out.push("$response | ConvertTo-Json -Depth 10");
  return out.join("\n");
}

/* ==================== WebSocket 客户端代码生成（无库切换） ==================== */

export interface WsReq {
  url: string;
  headers: { key: string; value: string }[];
  message: string;
}

/** 构造 WebSocket 请求参数：拼接 query、过滤启用的 header、取消息体 */
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
    .map((q) => `${encodeURIComponent(q.key)}=${encodeURIComponent(q.value)}`)
    .join("&");
  if (qs) url += (url.includes("?") ? "&" : "?") + qs;
  const headers = api.headers
    .filter((h) => h.enabled && h.key.trim())
    .map((h) => ({ key: h.key, value: h.value }));
  return { url, headers, message: api.body.raw ?? "" };
}

function genWsBash(r: WsReq): string {
  const out: string[] = [];
  out.push("# 需要安装 websocat: https://github.com/vi/websocat");
  for (const h of r.headers) out.push(`# 请求头：${h.key}: ${h.value}`);
  out.push(`printf '%s' ${JSON.stringify(r.message)} | websocat '${esc(r.url, "'")}'`);
  return out.join("\n");
}

function genWsPython(r: WsReq): string {
  const out: string[] = [];
  out.push("import asyncio");
  out.push("import json");
  out.push("import websockets");
  out.push("");
  out.push("");
  out.push("async def main():");
  if (r.headers.length) {
    out.push(`    headers = ${JSON.stringify(Object.fromEntries(r.headers.map((h) => [h.key, h.value])))}`);
    out.push(`    async with websockets.connect(${JSON.stringify(r.url)}, additional_headers=headers) as ws:`);
  } else {
    out.push(`    async with websockets.connect(${JSON.stringify(r.url)}) as ws:`);
  }
  if (r.message) {
    out.push(`    message = ${JSON.stringify(r.message)}`);
    out.push("    print('>>> 发送:', message)");
    out.push("    await ws.send(message)");
    out.push("");
    out.push("    # 循环接收服务器回传的信息（path / query / header / message）");
    out.push("    try:");
    out.push("        while True:");
    out.push("            reply = await asyncio.wait_for(ws.recv(), timeout=5)");
    out.push("            print('<<< 接收:', reply)");
    out.push("    except asyncio.TimeoutError:");
    out.push("        pass");
  } else {
    out.push("    # 建立连接后即可收发消息");
    out.push("    async for reply in ws:");
    out.push("        print('<<< 接收:', reply)");
  }
  out.push("");
  out.push("");
  out.push("asyncio.run(main())");
  return out.join("\n");
}

function genWsJavaScript(r: WsReq): string {
  const out: string[] = [];
  out.push(`const ws = new WebSocket(${JSON.stringify(r.url)});`);
  out.push("");
  out.push("ws.onopen = () => {");
  if (r.headers.length) {
    out.push("  // 浏览器原生 WebSocket 无法自定义请求头，如需自定义请求头请使用 ws / websockets 等服务端库，");
    out.push("  // 或在使用前通过 Cookie/鉴权参数（query）传递。");
    for (const h of r.headers) out.push(`  // ${h.key}: ${h.value}`);
  }
  if (r.message) {
    out.push(`  ws.send(${JSON.stringify(r.message)});`);
  } else {
    out.push("  console.log('连接成功，等待消息...');");
  }
  out.push("});");
  out.push("");
  out.push("ws.onmessage = (event) => {");
  out.push("  console.log('<<< 接收:', event.data);");
  out.push("});");
  out.push("");
  out.push("ws.onerror = (err) => { console.error('错误:', err); };");
  out.push("ws.onclose = () => { console.log('连接已关闭'); };");
  return out.join("\n");
}

function genWsTypeScript(r: WsReq): string {
  const out: string[] = [];
  out.push(`const ws: WebSocket = new WebSocket(${JSON.stringify(r.url)});`);
  out.push("");
  out.push("ws.onopen = (): void => {");
  if (r.headers.length) {
    out.push("  // 浏览器原生 WebSocket 无法自定义请求头，如需自定义请求头请使用 ws / websockets 等服务端库。");
    for (const h of r.headers) out.push(`  // ${h.key}: ${h.value}`);
  }
  if (r.message) {
    out.push(`  ws.send(${JSON.stringify(r.message)});`);
  } else {
    out.push("  console.log('连接成功，等待消息...');");
  }
  out.push("});");
  out.push("");
  out.push("ws.onmessage = (event: MessageEvent): void => {");
  out.push("  console.log('<<< 接收:', event.data);");
  out.push("});");
  out.push("");
  out.push("ws.onerror = (err: Event): void => { console.error('错误:', err); };");
  out.push("ws.onclose = (): void => { console.log('连接已关闭'); };");
  return out.join("\n");
}

function genWsGo(r: WsReq): string {
  const hdrArg = r.headers.length ? ", httpHeader()" : ", nil";
  const out: string[] = [];
  out.push("package main");
  out.push("");
  out.push("import (");
  out.push("    \"fmt\"");
  out.push("    \"log\"");
  if (r.headers.length) out.push("    \"net/http\"");
  out.push("    \"github.com/gorilla/websocket\"");
  out.push(")");
  out.push("");
  if (r.headers.length) {
    out.push("func httpHeader() http.Header {");
    out.push("    header := http.Header{}");
    for (const h of r.headers) out.push(`    header.Set(${JSON.stringify(h.key)}, ${JSON.stringify(h.value)})`);
    out.push("    return header");
    out.push("}");
    out.push("");
  }
  out.push("func main() {");
  out.push(`    conn, _, err := websocket.DefaultDialer.Dial(${JSON.stringify(r.url)}${hdrArg})`);
  out.push("    if err != nil {");
  out.push("        log.Fatal(\"连接失败:\", err)");
  out.push("    }");
  out.push("    defer conn.Close()");
  out.push("");
  if (r.message) {
    out.push(`    message := ${JSON.stringify(r.message)}`);
    out.push("    fmt.Println(\">>> 发送:\", message)");
    out.push("    if err := conn.WriteMessage(websocket.TextMessage, []byte(message)); err != nil {");
    out.push("        log.Fatal(\"发送失败:\", err)");
    out.push("    }");
    out.push("");
    out.push("    if _, p, err := conn.ReadMessage(); err != nil {");
    out.push("        log.Fatal(\"接收失败:\", err)");
    out.push("    } else {");
    out.push("        fmt.Println(\"<<< 接收:\", string(p))");
    out.push("    }");
  } else {
    out.push("    if _, p, err := conn.ReadMessage(); err != nil {");
    out.push("        log.Fatal(\"接收失败:\", err)");
    out.push("    } else {");
    out.push("        fmt.Println(\"<<< 接收:\", string(p))");
    out.push("    }");
  }
  out.push("}");
  return out.join("\n");
}

function genWsJavaDispatch(r: WsReq, lib?: string): string {
  switch (lib) {
    case "spring":
      return genWsJavaSpring(r);
    case "netty":
      return genWsJavaNetty(r);
    case "okhttp":
      return genWsJavaOkhttp(r);
    default:
      return genWsJavaJsr356(r);
  }
}

function genWsJavaJsr356(r: WsReq): string {
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（JSR-356：Java 标准 WebSocket API，JavaEE/JakartaEE 标准）");
  out.push(" * 规范官网: https://jakarta.ee/specifications/websocket/");
  out.push(" * 教程: https://javaee.github.io/tutorial/websocket.html");
  out.push(" * 容器内置: Tomcat（tomcat-websocket）、Jetty（jetty-websocket）");
  out.push(" * 依赖（以 Tomcat 为例，Maven）:");
  out.push(" *   <dependency>");
  out.push(" *     <groupId>org.apache.tomcat</groupId>");
  out.push(" *     <artifactId>tomcat-websocket</artifactId>");
  out.push(" *     <version>9.0.102</version>");
  out.push(" *   </dependency>");
  out.push(" * 注意: JakartaEE 9+ 将包名 javax.websocket 改为 jakarta.websocket");
  out.push(" */");
  out.push("import javax.websocket.*;");
  out.push("import java.net.URI;");
  out.push("import java.util.List;");
  out.push("import java.util.Map;");
  out.push("");
  out.push("@ClientEndpoint");
  out.push("public class Main {");
  out.push("    public static void main(String[] args) throws Exception {");
  out.push("        WebSocketContainer container = ContainerProvider.getWebSocketContainer();");
  if (r.headers.length) {
    out.push("        ClientEndpointConfig config = ClientEndpointConfig.Builder.create()");
    out.push("            .configurator(new ClientEndpointConfig.Configurator() {");
    out.push("                @Override");
    out.push("                public void beforeRequest(Map<String, List<String>> headers) {");
    for (const h of r.headers) out.push(`                    headers.put(${JSON.stringify(h.key)}, List.of(${JSON.stringify(h.value)}));`);
    out.push("                }");
    out.push("            }).build();");
    out.push(`        Session session = container.connectToServer(Main.class, config, URI.create(${JSON.stringify(r.url)}));`);
  } else {
    out.push(`        Session session = container.connectToServer(Main.class, URI.create(${JSON.stringify(r.url)}));`);
  }
  out.push("        // 保持主线程存活，等待回调");
  out.push("        Thread.sleep(5000);");
  out.push("        session.close();");
  out.push("    }");
  out.push("");
  out.push("    @OnOpen");
  out.push("    public static void onOpen(Session session) {");
  out.push("        System.out.println(\">>> 连接成功\");");
  out.push(`        String msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")};`);
  out.push("        System.out.println(\">>> 发送: \" + msg);");
  out.push("        try { session.getBasicRemote().sendText(msg); } catch (Exception e) { }");
  out.push("    }");
  out.push("");
  out.push("    @OnMessage");
  out.push("    public static void onMessage(String message, Session session) {");
  out.push("        System.out.println(\"<<< 接收: \" + message);");
  out.push("        try { session.close(); } catch (Exception e) { }");
  out.push("    }");
  out.push("");
  out.push("    @OnError");
  out.push("    public static void onError(Session session, Throwable t) {");
  out.push("        System.out.println(\"连接失败: \" + t.getMessage());");
  out.push("    }");
  out.push("");
  out.push("    @OnClose");
  out.push("    public static void onClose(Session session, CloseReason reason) {");
  out.push("        System.out.println(\"连接已关闭: \" + reason.getReasonPhrase());");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

function genWsJavaSpring(r: WsReq): string {
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（Spring WebSocket：SpringBoot 项目首选，底层可用 JSR356 / Netty / Jetty）");
  out.push(" * 官网: https://spring.io/projects/spring-websocket");
  out.push(" * 文档: https://docs.spring.io/spring-framework/reference/web/websocket.html");
  out.push(" * 依赖: spring-boot-starter-websocket（或 spring-websocket）");
  out.push(" * 说明: 底层客户端可切换 StandardWebSocketClient（JSR-356）/ JettyWebSocketClient /");
  out.push(" *       ReactorNettyWebSocketClient（WebFlux）；如需 STOMP 子协议，");
  out.push(" *       改用 spring-messaging 的 WebSocketStompClient + StompSession");
  out.push(" */");
  out.push("import org.springframework.web.socket.*;");
  out.push("import org.springframework.web.socket.client.WebSocketClient;");
  out.push("import org.springframework.web.socket.client.standard.StandardWebSocketClient;");
  out.push("import java.util.concurrent.CompletableFuture;");
  out.push("");
  out.push("public class Main {");
  out.push("    public static void main(String[] args) throws Exception {");
  out.push("        WebSocketClient client = new StandardWebSocketClient();");
  if (r.headers.length) {
    out.push("        HttpHeaders headers = new HttpHeaders();");
    for (const h of r.headers) out.push(`        headers.set(${JSON.stringify(h.key)}, ${JSON.stringify(h.value)});`);
    out.push(`        CompletableFuture<WebSocketSession> future = client.execute(handler(), ${JSON.stringify(r.url)}, headers);`);
  } else {
    out.push(`        CompletableFuture<WebSocketSession> future = client.execute(handler(), ${JSON.stringify(r.url)});`);
  }
  out.push("        WebSocketSession session = future.get();");
  out.push("        Thread.sleep(5000);");
  out.push("        session.close();");
  out.push("    }");
  out.push("");
  out.push("    private static WebSocketHandler handler() {");
  out.push("        return new WebSocketHandler() {");
  out.push("            @Override");
  out.push("            public void afterConnectionEstablished(WebSocketSession session) throws Exception {");
  out.push("                System.out.println(\">>> 连接成功\");");
  out.push(`                session.sendMessage(new TextMessage(${JSON.stringify(r.message || "hello, this is a websocket echo message")}));`);
  out.push("                System.out.println(\">>> 发送完成\");");
  out.push("            }");
  out.push("");
  out.push("            @Override");
  out.push("            public void handleMessage(WebSocketSession session, WebSocketMessage<?> message) throws Exception {");
  out.push("                if (message instanceof TextMessage) {");
  out.push("                    System.out.println(\"<<< 接收: \" + ((TextMessage) message).getPayload());");
  out.push("                }");
  out.push("            }");
  out.push("");
  out.push("            @Override");
  out.push("            public void handleTransportError(WebSocketSession session, Throwable exception) throws Exception {");
  out.push("                System.out.println(\"连接失败: \" + exception.getMessage());");
  out.push("            }");
  out.push("");
  out.push("            @Override");
  out.push("            public void afterConnectionClosed(WebSocketSession session, CloseStatus closeStatus) throws Exception {");
  out.push("                System.out.println(\"连接已关闭: \" + closeStatus);");
  out.push("            }");
  out.push("");
  out.push("            @Override");
  out.push("            public boolean supportsPartialMessages() { return false; }");
  out.push("        };");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

function genWsJavaNetty(r: WsReq): string {
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（Netty 原生：高性能底层，高并发网关、IM 服务）");
  out.push(" * 官网: https://netty.io/");
  out.push(" * 依赖（Maven）:");
  out.push(" *   <dependency>");
  out.push(" *     <groupId>io.netty</groupId>");
  out.push(" *     <artifactId>netty-codec-http</artifactId>");
  out.push(" *     <version>4.1.115.Final</version>");
  out.push(" *   </dependency>");
  out.push(" *   （wss:// 时另需 netty-handler 与 netty-tcnative/OpenSSL）");
  out.push(" */");
  out.push("import io.netty.bootstrap.Bootstrap;");
  out.push("import io.netty.channel.*;");
  out.push("import io.netty.channel.nio.NioEventLoopGroup;");
  out.push("import io.netty.channel.socket.SocketChannel;");
  out.push("import io.netty.channel.socket.nio.NioSocketChannel;");
  out.push("import io.netty.handler.codec.http.DefaultHttpHeaders;");
  out.push("import io.netty.handler.codec.http.HttpClientCodec;");
  out.push("import io.netty.handler.codec.http.HttpObjectAggregator;");
  out.push("import io.netty.handler.codec.http.websocketx.*;");
  out.push("import io.netty.handler.ssl.SslContextBuilder;");
  out.push("import java.net.URI;");
  out.push("");
  out.push("public class Main {");
  out.push("    public static void main(String[] args) throws Exception {");
  out.push(`        URI uri = new URI(${JSON.stringify(r.url)});`);
  out.push("        String scheme = uri.getScheme();");
  out.push("        boolean ssl = \"wss\".equals(scheme);");
  out.push("        String host = uri.getHost();");
  out.push("        int port = uri.getPort();");
  out.push("        if (port == -1) port = ssl ? 443 : 80;");
  out.push("");
  if (r.headers.length) {
    out.push("        DefaultHttpHeaders headers = new DefaultHttpHeaders();");
    for (const h of r.headers) out.push(`        headers.add(${JSON.stringify(h.key)}, ${JSON.stringify(h.value)});`);
  } else {
    out.push("        DefaultHttpHeaders headers = new DefaultHttpHeaders();");
  }
  out.push(`        WebSocketClientHandshaker handshaker = WebSocketClientHandshakerFactory`);
  out.push(`                .newHandshaker(uri, WebSocketVersion.V13, null, true, headers);`);
  out.push("");
  out.push("        EventLoopGroup group = new NioEventLoopGroup();");
  out.push("        try {");
  out.push("            Bootstrap bootstrap = new Bootstrap();");
  out.push("            bootstrap.group(group)");
  out.push("                .channel(NioSocketChannel.class)");
  out.push("                .handler(new ChannelInitializer<SocketChannel>() {");
  out.push("                    @Override");
  out.push("                    protected void initChannel(SocketChannel ch) {");
  out.push("                        ChannelPipeline p = ch.pipeline();");
  out.push("                        if (ssl) {");
  out.push("                            p.addLast(SslContextBuilder.forClient().build()");
  out.push("                                    .newHandler(ch.alloc(), host, port));");
  out.push("                        }");
  out.push("                        p.addLast(new HttpClientCodec());");
  out.push("                        p.addLast(new HttpObjectAggregator(8192));");
  out.push("                        p.addLast(new WebSocketClientProtocolHandler(handshaker, true));");
  out.push("                        p.addLast(new SimpleChannelInboundHandler<WebSocketFrame>() {");
  out.push("                            @Override");
  out.push("                            protected void channelRead0(ChannelHandlerContext ctx, WebSocketFrame frame) {");
  out.push("                                if (frame instanceof TextWebSocketFrame) {");
  out.push("                                    System.out.println(\"<<< 接收: \" + ((TextWebSocketFrame) frame).text());");
  out.push("                                    ctx.close();");
  out.push("                                }");
  out.push("                            }");
  out.push("");
  out.push("                            @Override");
  out.push("                            public void exceptionCaught(ChannelHandlerContext ctx, Throwable cause) {");
  out.push("                                System.out.println(\"连接失败: \" + cause.getMessage());");
  out.push("                                ctx.close();");
  out.push("                            }");
  out.push("                        });");
  out.push("                    }");
  out.push("                });");
  out.push("");
  out.push("            Channel ch = bootstrap.connect(host, port).sync().channel();");
  out.push("            handshaker.handshakeFuture().sync();   // 等待握手完成");
  out.push("            System.out.println(\">>> 连接成功\");");
  out.push(`            String msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")};`);
  out.push("            System.out.println(\">>> 发送: \" + msg);");
  out.push("            ch.writeAndFlush(new TextWebSocketFrame(msg));");
  out.push("            ch.closeFuture().sync();");
  out.push("        } finally {");
  out.push("            group.shutdownGracefully();");
  out.push("        }");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

function genWsJavaOkhttp(r: WsReq): string {
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（OkHttp：轻量易用，Android 与 JVM 通用）");
  out.push(" * 官网: https://square.github.io/okhttp/");
  out.push(" * GitHub: https://github.com/square/okhttp");
  out.push(" * 依赖（Maven）:");
  out.push(" *   <dependency>");
  out.push(" *     <groupId>com.squareup.okhttp3</groupId>");
  out.push(" *     <artifactId>okhttp</artifactId>");
  out.push(" *     <version>4.12.0</version>");
  out.push(" *   </dependency>");
  out.push(" */");
  out.push("import okhttp3.*;");
  out.push("import okio.ByteString;");
  out.push("");
  out.push("public class Main {");
  out.push("    public static void main(String[] args) throws Exception {");
  out.push("        OkHttpClient client = new OkHttpClient();");
  out.push("        Request request = new Request.Builder()");
  out.push(`            .url(${JSON.stringify(r.url)})`);
  for (const h of r.headers) out.push(`            .addHeader(${JSON.stringify(h.key)}, ${JSON.stringify(h.value)})`);
  out.push("            .build();");
  out.push("");
  out.push("        WebSocket ws = client.newWebSocket(request, new WebSocketListener() {");
  out.push("            @Override");
  out.push("            public void onOpen(WebSocket webSocket, Response response) {");
  out.push("                System.out.println(\">>> 连接成功\");");
  out.push(`                webSocket.send(${JSON.stringify(r.message || "hello, this is a websocket echo message")});`);
  out.push("                System.out.println(\">>> 发送完成\");");
  out.push("            }");
  out.push("");
  out.push("            @Override");
  out.push("            public void onMessage(WebSocket webSocket, String text) {");
  out.push("                System.out.println(\"<<< 接收: \" + text);");
  out.push("                webSocket.close(1000, \"bye\");");
  out.push("            }");
  out.push("");
  out.push("            @Override");
  out.push("            public void onMessage(WebSocket webSocket, ByteString bytes) {");
  out.push("                System.out.println(\"<<< 接收(binary): \" + bytes.hex());");
  out.push("            }");
  out.push("");
  out.push("            @Override");
  out.push("            public void onFailure(WebSocket webSocket, Throwable t, Response response) {");
  out.push("                System.out.println(\"连接失败: \" + t.getMessage());");
  out.push("            }");
  out.push("");
  out.push("            @Override");
  out.push("            public void onClosed(WebSocket webSocket, int code, String reason) {");
  out.push("                System.out.println(\"连接已关闭: \" + reason);");
  out.push("            }");
  out.push("        });");
  out.push("");
  out.push("        // 保持主线程存活");
  out.push("        Thread.sleep(5000);");
  out.push("        client.dispatcher().executorService().shutdown();");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

function genWsCsharp(r: WsReq): string {
  const out: string[] = [];
  out.push("using System;");
  out.push("using System.Net.WebSockets;");
  out.push("using System.Text;");
  out.push("using System.Threading;");
  out.push("using System.Threading.Tasks;");
  out.push("");
  out.push("class Program");
  out.push("{");
  out.push("    static async Task Main()");
  out.push("    {");
  out.push("        using var ws = new ClientWebSocket();");
  if (r.headers.length) {
    for (const h of r.headers) out.push(`        ws.Options.SetRequestHeader(${JSON.stringify(h.key)}, ${JSON.stringify(h.value)});`);
  }
  out.push(`        await ws.ConnectAsync(new Uri(${JSON.stringify(r.url)}), CancellationToken.None);`);
  if (r.message) {
    out.push("");
    out.push(`        var message = ${JSON.stringify(r.message)};`);
    out.push("        var bytes = Encoding.UTF8.GetBytes(message);");
    out.push("        await ws.SendAsync(new ArraySegment<byte>(bytes), WebSocketMessageType.Text, true, CancellationToken.None);");
    out.push("");
    out.push("        var buffer = new byte[4096];");
    out.push("        var result = await ws.ReceiveAsync(new ArraySegment<byte>(buffer), CancellationToken.None);");
    out.push("        Console.WriteLine(\"<<< 接收:\" + Encoding.UTF8.GetString(buffer, 0, result.Count));");
  } else {
    out.push("");
    out.push("        var buffer = new byte[4096];");
    out.push("        var result = await ws.ReceiveAsync(new ArraySegment<byte>(buffer), CancellationToken.None);");
    out.push("        Console.WriteLine(\"<<< 接收:\" + Encoding.UTF8.GetString(buffer, 0, result.Count));");
  }
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

function genWsRust(r: WsReq): string {
  const out: string[] = [];
  out.push("use futures_util::{SinkExt, StreamExt};");
  out.push("use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};");
  out.push("");
  out.push("#[tokio::main]");
  out.push("async fn main() -> Result<(), Box<dyn std::error::Error>> {");
  if (r.headers.length) {
    out.push("    let mut request = " + JSON.stringify(r.url) + ".to_string();");
    for (const h of r.headers) out.push(`    // 请求头：${h.key}: ${h.value}（可通过 tungstenite 的 handshake 设置）`);
    out.push("    let (mut ws, _) = connect_async(request).await?;");
  } else {
    out.push(`    let (mut ws, _) = connect_async(${JSON.stringify(r.url)}).await?;`);
  }
  out.push("");
  if (r.message) {
    out.push(`    let message = ${JSON.stringify(r.message)}.to_string();`);
    out.push("    println!(\">>> 发送: {}\", message);");
    out.push("    ws.send(Message::Text(message.into())).await?;");
    out.push("");
    out.push("    if let Some(Ok(msg)) = ws.next().await {");
    out.push("        println!(\"<<< 接收: {}\", msg);");
    out.push("    }");
  } else {
    out.push("    if let Some(Ok(msg)) = ws.next().await {");
    out.push("        println!(\"<<< 接收: {}\", msg);");
    out.push("    }");
  }
  out.push("    Ok(())");
  out.push("}");
  return out.join("\n");
}

/** 解析 ws/wss URL 的 scheme / host / port / path（供 C 代码嵌入） */
function parseWsUrl(url: string): { scheme: string; host: string; port: number; path: string } {
  const m = /^(wss?):\/\/([^/?#:]+)(?::(\d+))?([^?#]*)(\?[^#]*)?$/i.exec(url);
  const scheme = (m?.[1] || "ws").toLowerCase();
  const host = m?.[2] || "127.0.0.1";
  const port = m?.[3] ? Number(m[3]) : scheme === "wss" ? 443 : 80;
  const path = (m?.[4] || "/") + (m?.[5] || "");
  return { scheme, host, port, path };
}

function genWsC(r: WsReq, lib?: string): string {
  switch (lib) {
    case "libuvws":
      return genWsCLibuvws(r);
    case "wslay":
      return genWsCWslay(r);
    default:
      return genWsCLibwebsockets(r);
  }
}

function genWsCLibwebsockets(r: WsReq): string {
  const u = parseWsUrl(r.url);
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（libwebsockets）");
  out.push(" * 官网: https://libwebsockets.org");
  out.push(" * GitHub: https://github.com/warmcat/libwebsockets");
  out.push(" * 安装: sudo apt install libwebsockets-dev    (Ubuntu/Debian)");
  out.push(" *       brew install libwebsockets             (macOS)");
  out.push(" * 编译: gcc -o ws_client ws_client.c -lwebsockets");
  out.push(" *       （连接 wss:// 时需另链接 OpenSSL: -lssl -lcrypto，并启用下方 wss 两行）");
  out.push(" *");
  for (const h of r.headers) out.push(` * 请求头: ${h.key}: ${h.value}（通过 LWS_CALLBACK_CLIENT_APPEND_HANDSHAKE_HEADER 回调追加）`);
  out.push(" */");
  out.push("#include <stdio.h>");
  out.push("#include <string.h>");
  out.push("#include <libwebsockets.h>");
  out.push("");
  out.push(`#define MSG ${JSON.stringify(r.message || "hello, this is a websocket echo message")}`);
  out.push("");
  out.push("static int g_done = 0;");
  out.push("");
  out.push("static int ws_callback(struct lws *wsi, enum lws_callback_reasons reason,");
  out.push("                       void *user, void *in, size_t len) {");
  out.push("    (void)user;");
  out.push("    switch (reason) {");
  out.push("    case LWS_CALLBACK_CLIENT_ESTABLISHED: {");
  out.push("        unsigned char buf[LWS_PRE + 512];");
  out.push("        size_t n = strlen(MSG);");
  out.push("        if (n > sizeof(buf) - LWS_PRE - 1) n = sizeof(buf) - LWS_PRE - 1;");
  out.push("        memcpy(buf + LWS_PRE, MSG, n);");
  out.push("        printf(\">>> 发送: %s\\n\", MSG);");
  out.push("        lws_write(wsi, buf + LWS_PRE, n, LWS_WRITE_TEXT);");
  out.push("        break;");
  out.push("    }");
  out.push("    case LWS_CALLBACK_CLIENT_RECEIVE:");
  out.push("        printf(\"<<< 接收: %.*s\\n\", (int)len, (const char *)in);");
  out.push("        g_done = 1;");
  out.push("        break;");
  out.push("    case LWS_CALLBACK_CLIENT_CONNECTION_ERROR:");
  out.push("        fprintf(stderr, \"连接失败\\n\");");
  out.push("        g_done = 1;");
  out.push("        break;");
  out.push("    case LWS_CALLBACK_CLIENT_CLOSED:");
  out.push("        g_done = 1;");
  out.push("        break;");
  out.push("    default:");
  out.push("        break;");
  out.push("    }");
  out.push("    return 0;");
  out.push("}");
  out.push("");
  out.push("static const struct lws_protocols protocols[] = {");
  out.push("    { \"api-manager\", ws_callback, 0, 4096, 0, NULL, 0 },");
  out.push("    LWS_PROTOCOL_LIST_TERM");
  out.push("};");
  out.push("");
  out.push("int main(void) {");
  out.push("    struct lws_context_creation_info info;");
  out.push("    memset(&info, 0, sizeof(info));");
  out.push("    info.port = CONTEXT_PORT_NO_LISTEN;");
  out.push("    info.protocols = protocols;");
  out.push("    struct lws_context *ctx = lws_create_context(&info);");
  out.push("    if (!ctx) {");
  out.push("        fprintf(stderr, \"创建上下文失败\\n\");");
  out.push("        return 1;");
  out.push("    }");
  out.push("");
  out.push("    struct lws_client_connect_info cci;");
  out.push("    memset(&cci, 0, sizeof(cci));");
  out.push("    cci.context = ctx;");
  out.push(`    cci.address = ${JSON.stringify(u.host)};`);
  out.push(`    cci.port = ${u.port};`);
  out.push(`    cci.path = ${JSON.stringify(u.path)};`);
  out.push("    cci.host = cci.address;   /* Host 请求头 */");
  out.push("    cci.origin = \"api-manager\";");
  out.push("    cci.protocol = protocols[0].name;");
  if (u.scheme === "wss") {
    out.push("    cci.ssl_connection = LCCSCF_USE_SSL | LCCSCF_ALLOW_SELFSIGNED; /* wss */");
  } else {
    out.push("    /* wss: cci.ssl_connection = LCCSCF_USE_SSL | LCCSCF_ALLOW_SELFSIGNED; */");
  }
  out.push("    if (!lws_client_connect_via_info(&cci)) {");
  out.push("        fprintf(stderr, \"发起连接失败\\n\");");
  out.push("        lws_context_destroy(ctx);");
  out.push("        return 1;");
  out.push("    }");
  out.push("    while (!g_done) {");
  out.push("        lws_service(ctx, 50);");
  out.push("    }");
  out.push("    lws_context_destroy(ctx);");
  out.push("    return 0;");
  out.push("}");
  return out.join("\n");
}

function genWsCLibuvws(r: WsReq): string {
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（libuv-ws：基于 libuv 的轻量 WebSocket 库）");
  out.push(" * GitHub/官网: https://github.com/meojson/libuv-ws");
  out.push(" * 依赖:");
  out.push(" *   1. libuv:    sudo apt install libuv1-dev   (Ubuntu/Debian)");
  out.push(" *   2. libuv-ws: git clone https://github.com/meojson/libuv-ws");
  out.push(" * 编译: gcc -o ws_client ws_client.c uv_ws.c -I<libuv-ws目录> -luv -lpthread");
  out.push(" *       （将本文件与 libuv-ws 仓库根目录的 uv_ws.c 一起编译）");
  out.push(" *");
  for (const h of r.headers) out.push(` * 请求头: ${h.key}: ${h.value}（libuv-ws 不支持自定义请求头，请改用 query 参数/Cookie）`);
  out.push(" */");
  out.push("#include <stdio.h>");
  out.push("#include <string.h>");
  out.push("#include \"uv_ws.h\"");
  out.push("");
  out.push("static void on_open(uv_ws_client_t *client) {");
  out.push(`    const char *msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")};`);
  out.push("    printf(\">>> 发送: %s\\n\", msg);");
  out.push("    uv_ws_send(client, msg, strlen(msg));");
  out.push("}");
  out.push("");
  out.push("static void on_message(uv_ws_client_t *client, ssize_t size, uv_buf_t *buf) {");
  out.push("    printf(\"<<< 接收: %.*s\\n\", (int)size, buf->base);");
  out.push("    uv_ws_close(client); /* 收到回显后断开 */");
  out.push("}");
  out.push("");
  out.push("static void on_error(uv_ws_client_t *client) {");
  out.push("    fprintf(stderr, \"连接失败\\n\");");
  out.push("    uv_ws_close(client);");
  out.push("}");
  out.push("");
  out.push("static void on_close(uv_ws_client_t *client) {");
  out.push("    printf(\"连接已关闭\\n\");");
  out.push("}");
  out.push("");
  out.push("int main(void) {");
  out.push("    uv_loop_t *loop = uv_default_loop();");
  out.push("    uv_ws_client_t client;");
  out.push(`    uv_ws_init(loop, &client, ${JSON.stringify(r.url)});`);
  out.push("    client.on_open = on_open;");
  out.push("    client.on_message = on_message;");
  out.push("    client.on_error = on_error;");
  out.push("    client.on_close = on_close;");
  out.push("    uv_ws_connect(&client);");
  out.push("    return uv_run(loop, UV_RUN_DEFAULT);");
  out.push("}");
  return out.join("\n");
}

function genWsCWslay(r: WsReq): string {
  const u = parseWsUrl(r.url);
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（wslay：只做帧编解码，TCP 连接与 HTTP Upgrade 握手自行实现）");
  out.push(" * 官网/GitHub: https://github.com/tatsuhiro-t/wslay");
  out.push(" * 安装: sudo apt install libwslay-dev   (Ubuntu/Debian)");
  out.push(" * 编译: gcc -o ws_client ws_client.c -lwslay");
  out.push(" *");
  for (const h of r.headers) out.push(` * 请求头: ${h.key}: ${h.value}（已加入下方握手请求）`);
  out.push(" */");
  out.push("#include <stdio.h>");
  out.push("#include <stdint.h>");
  out.push("#include <stdlib.h>");
  out.push("#include <string.h>");
  out.push("#include <time.h>");
  out.push("#include <unistd.h>");
  out.push("#include <arpa/inet.h>");
  out.push("#include <sys/select.h>");
  out.push("#include <sys/socket.h>");
  out.push("#include <wslay/wslay.h>");
  out.push("");
  out.push("static int g_done = 0;");
  out.push("");
  out.push("/* 简易 base64 编码：用于生成握手所需的 Sec-WebSocket-Key */");
  out.push("static void base64_encode(const unsigned char *in, size_t len, char *out) {");
  out.push("    static const char tbl[] = \"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/\";");
  out.push("    size_t i = 0, o = 0;");
  out.push("    while (i + 2 < len) {");
  out.push("        unsigned v = (in[i] << 16) | (in[i + 1] << 8) | in[i + 2];");
  out.push("        out[o++] = tbl[(v >> 18) & 63];");
  out.push("        out[o++] = tbl[(v >> 12) & 63];");
  out.push("        out[o++] = tbl[(v >> 6) & 63];");
  out.push("        out[o++] = tbl[v & 63];");
  out.push("        i += 3;");
  out.push("    }");
  out.push("    if (i < len) {");
  out.push("        unsigned v = (unsigned)in[i] << 16;");
  out.push("        if (i + 1 < len) v |= (unsigned)in[i + 1] << 8;");
  out.push("        out[o++] = tbl[(v >> 18) & 63];");
  out.push("        out[o++] = tbl[(v >> 12) & 63];");
  out.push("        out[o++] = (i + 1 < len) ? tbl[(v >> 6) & 63] : '=';");
  out.push("        out[o++] = '=';");
  out.push("    }");
  out.push("    out[o] = '\\0';");
  out.push("}");
  out.push("");
  out.push("/* wslay 回调：底层 socket 收发 + 收到消息时回调 */");
  out.push("static int send_cb(wslay_event_context_ptr ctx, const uint8_t *data, size_t len,");
  out.push("                    int flags, void *user_data) {");
  out.push("    (void)ctx; (void)flags;");
  out.push("    int fd = *(int *)user_data;");
  out.push("    return write(fd, data, len) == (ssize_t)len ? 0 : WSLAY_ERR_CALLBACK_FAILURE;");
  out.push("}");
  out.push("");
  out.push("static int recv_cb(wslay_event_context_ptr ctx, uint8_t *data, size_t len,");
  out.push("                    int flags, void *user_data) {");
  out.push("    (void)ctx; (void)flags;");
  out.push("    int fd = *(int *)user_data;");
  out.push("    ssize_t n = read(fd, data, len);");
  out.push("    return n < 0 ? WSLAY_ERR_CALLBACK_FAILURE : (int)n;");
  out.push("}");
  out.push("");
  out.push("static void msg_recv_cb(wslay_event_context_ptr ctx,");
  out.push("                        const struct wslay_event_on_msg_recv_arg *arg,");
  out.push("                        void *user_data) {");
  out.push("    (void)ctx; (void)user_data;");
  out.push("    if (arg->opcode == WSLAY_TEXT_FRAME || arg->opcode == WSLAY_BINARY_FRAME) {");
  out.push("        printf(\"<<< 接收: %.*s\\n\", (int)arg->msg_length, (const char *)arg->msg);");
  out.push("        g_done = 1;");
  out.push("    }");
  out.push("}");
  out.push("");
  out.push("static int genmask_cb(wslay_event_context_ptr ctx, uint8_t *buf, size_t len,");
  out.push("                      void *user_data) {");
  out.push("    (void)ctx; (void)user_data;");
  out.push("    size_t i;");
  out.push("    for (i = 0; i < len; ++i) buf[i] = (uint8_t)(rand() & 0xff);");
  out.push("    return 0;");
  out.push("}");
  out.push("");
  out.push("int main(void) {");
  out.push(`    const char *host = ${JSON.stringify(u.host)};`);
  out.push(`    int port = ${u.port};`);
  out.push(`    const char *path = ${JSON.stringify(u.path)};`);
  out.push("");
  out.push("    /* 1. 建立 TCP 连接 */");
  out.push("    int fd = socket(AF_INET, SOCK_STREAM, 0);");
  out.push("    if (fd < 0) { perror(\"socket\"); return 1; }");
  out.push("    struct sockaddr_in addr;");
  out.push("    memset(&addr, 0, sizeof(addr));");
  out.push("    addr.sin_family = AF_INET;");
  out.push("    addr.sin_port = htons((uint16_t)port);");
  out.push("    if (inet_pton(AF_INET, host, &addr.sin_addr) != 1) { perror(\"inet_pton\"); return 1; }");
  out.push("    if (connect(fd, (struct sockaddr *)&addr, sizeof(addr)) != 0) { perror(\"connect\"); return 1; }");
  out.push("");
  out.push("    /* 2. HTTP Upgrade 握手（Sec-WebSocket-Key 为 16 字节随机数的 base64；");
  out.push("         如需校验服务端 Sec-WebSocket-Accept，可另实现 SHA1 + base64） */");
  out.push("    char key_buf[24], req[1024];");
  out.push("    unsigned char raw[16];");
  out.push("    int i;");
  out.push("    srand((unsigned)time(NULL));");
  out.push("    for (i = 0; i < 16; ++i) raw[i] = (unsigned char)(rand() & 0xff);");
  out.push("    base64_encode(raw, sizeof(raw), key_buf);");
  out.push("    snprintf(req, sizeof(req),");
  out.push("             \"GET %s HTTP/1.1\\r\\n\"");
  out.push("             \"Host: %s:%d\\r\\n\"");
  out.push("             \"Upgrade: websocket\\r\\n\"");
  out.push("             \"Connection: Upgrade\\r\\n\"");
  for (const h of r.headers) {
    out.push(`             ${JSON.stringify(`${h.key}: ${h.value}\r\n`)}`);
  }
  out.push("             \"Sec-WebSocket-Key: %s\\r\\n\"");
  out.push("             \"Sec-WebSocket-Version: 13\\r\\n\\r\\n\",");
  out.push("             path, host, port, key_buf);");
  out.push("    if (write(fd, req, strlen(req)) != (ssize_t)strlen(req)) { perror(\"write\"); return 1; }");
  out.push("    char resp[1024];");
  out.push("    ssize_t n = read(fd, resp, sizeof(resp) - 1);");
  out.push("    if (n <= 0) { perror(\"read\"); return 1; }");
  out.push("    resp[n] = '\\0';");
  out.push("    if (!strstr(resp, \" 101 \")) { fprintf(stderr, \"握手失败: %s\\n\", resp); return 1; }");
  out.push("    printf(\">>> 握手成功\\n\");");
  out.push("");
  out.push("    /* 3. 初始化 wslay 并发送一条文本消息 */");
  out.push("    struct wslay_event_callbacks callbacks = {");
  out.push("        .recv_callback = recv_cb,");
  out.push("        .send_callback = send_cb,");
  out.push("        .genmask_callback = genmask_cb,");
  out.push("        .on_msg_recv_callback = msg_recv_cb,");
  out.push("    };");
  out.push("    wslay_event_context_ptr ctx;");
  out.push("    if (wslay_event_context_client_init(&ctx, &callbacks, &fd) != 0) {");
  out.push("        fprintf(stderr, \"wslay 初始化失败\\n\");");
  out.push("        return 1;");
  out.push("    }");
  out.push(`    const char *msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")};`);
  out.push("    wslay_event_queue_msg(ctx,");
  out.push("                          wslay_event_create_text_message((const uint8_t *)msg, strlen(msg)));");
  out.push("    printf(\">>> 发送: %s\\n\", msg);");
  out.push("    wslay_event_send(ctx);");
  out.push("");
  out.push("    /* 4. 接收回显（select 等待 5 秒，收到后退出） */");
  out.push("    while (!g_done) {");
  out.push("        fd_set rfds;");
  out.push("        FD_ZERO(&rfds);");
  out.push("        FD_SET(fd, &rfds);");
  out.push("        struct timeval tv = { 5, 0 };");
  out.push("        if (select(fd + 1, &rfds, NULL, NULL, &tv) <= 0) break;");
  out.push("        if (wslay_event_recv(ctx) != 0) break;");
  out.push("    }");
  out.push("    wslay_event_context_free(ctx);");
  out.push("    close(fd);");
  out.push("    return 0;");
  out.push("}");
  return out.join("\n");
}

function genWsCpp(r: WsReq, lib?: string): string {
  switch (lib) {
    case "libwebsockets":
      return genWsCppLibwebsockets(r);
    case "uwebsockets":
      return genWsCppUwebsockets(r);
    case "qt":
      return genWsCppQt(r);
    default:
      return genWsCppBeast(r);
  }
}

function genWsCppBeast(r: WsReq): string {
  const u = parseWsUrl(r.url);
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（Boost.Beast，C++ 原生，工业级，推荐）");
  out.push(" * 官网: https://www.boost.org/doc/libs/release/libs/beast/");
  out.push(" * GitHub: https://github.com/boostorg/beast");
  out.push(" * 安装: sudo apt install libboost-all-dev   (Ubuntu/Debian)");
  out.push(" * 编译: g++ -std=c++17 -o ws_client ws_client.cpp -lboost_system -lpthread");
  out.push(" *       （或 CMake: find_package(Boost) / find_package(Threads)）");
  out.push(" *       （wss:// 需改用 ssl::stream<tcp::socket> 并链接 OpenSSL）");
  out.push(" *");
  for (const h of r.headers) out.push(` * 请求头: ${h.key}: ${h.value}（通过 set_option(decorator) 设置）`);
  out.push(" */");
  out.push("#include <boost/beast/core.hpp>");
  out.push("#include <boost/beast/websocket.hpp>");
  out.push("#include <boost/asio/connect.hpp>");
  out.push("#include <boost/asio/ip/tcp.hpp>");
  out.push("#include <cstdlib>");
  out.push("#include <iostream>");
  out.push("#include <string>");
  out.push("");
  out.push("namespace beast = boost::beast;");
  out.push("namespace http = beast::http;");
  out.push("namespace websocket = beast::websocket;");
  out.push("namespace net = boost::asio;");
  out.push("using tcp = net::ip::tcp;");
  out.push("");
  out.push("int main() {");
  out.push("    try {");
  out.push(`        const std::string host = ${JSON.stringify(u.host)};`);
  out.push(`        const std::string port = ${JSON.stringify(String(u.port))};`);
  out.push(`        const std::string path = ${JSON.stringify(u.path)};`);
  out.push(`        const std::string msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")};`);
  out.push("");
  out.push("        net::io_context ioc;");
  out.push("        tcp::resolver resolver{ioc};");
  out.push("        auto const results = resolver.resolve(host, port);");
  out.push("        websocket::stream<tcp::socket> ws{ioc};");
  if (r.headers.length) {
    out.push("        ws.set_option(websocket::stream_base::decorator([](websocket::request_type &req) {");
    for (const h of r.headers) out.push(`            req.set(${JSON.stringify(h.key)}, ${JSON.stringify(h.value)});`);
    out.push("        }));");
  }
  out.push("        auto ep = net::connect(ws.next_layer(), results);");
  out.push("        (void)ep;");
  out.push("        ws.handshake(host, path);");
  out.push("");
  out.push("        ws.write(net::buffer(msg));");
  out.push("        std::cout << \">>> 发送: \" << msg << std::endl;");
  out.push("");
  out.push("        beast::flat_buffer buffer;");
  out.push("        ws.read(buffer);");
  out.push("        std::cout << \"<<< 接收: \" << beast::make_printable(buffer.data()) << std::endl;");
  out.push("");
  out.push("        ws.close(websocket::close_code::normal);");
  out.push("    } catch (std::exception const &e) {");
  out.push("        std::cerr << \"错误: \" << e.what() << std::endl;");
  out.push("        return 1;");
  out.push("    }");
  out.push("    return 0;");
  out.push("}");
  return out.join("\n");
}

function genWsCppLibwebsockets(r: WsReq): string {
  const u = parseWsUrl(r.url);
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（libwebsockets：C 库，C++ 可直接调用）");
  out.push(" * 官网: https://libwebsockets.org");
  out.push(" * GitHub: https://github.com/warmcat/libwebsockets");
  out.push(" * 安装: sudo apt install libwebsockets-dev    (Ubuntu/Debian)");
  out.push(" *       brew install libwebsockets             (macOS)");
  out.push(" * 编译: g++ -std=c++17 -o ws_client ws_client.cpp -lwebsockets");
  out.push(" *       （连接 wss:// 时需另链接 OpenSSL: -lssl -lcrypto，并启用下方 wss 两行）");
  out.push(" *");
  for (const h of r.headers) out.push(` * 请求头: ${h.key}: ${h.value}（通过 LWS_CALLBACK_CLIENT_APPEND_HANDSHAKE_HEADER 回调追加）`);
  out.push(" */");
  out.push("#include <cstdio>");
  out.push("#include <cstring>");
  out.push("#include <string>");
  out.push("#include <libwebsockets.h>");
  out.push("");
  out.push(`static const std::string MSG = ${JSON.stringify(r.message || "hello, this is a websocket echo message")};`);
  out.push("");
  out.push("static int g_done = 0;");
  out.push("");
  out.push("static int ws_callback(struct lws *wsi, enum lws_callback_reasons reason,");
  out.push("                       void *user, void *in, size_t len) {");
  out.push("    (void)user;");
  out.push("    switch (reason) {");
  out.push("    case LWS_CALLBACK_CLIENT_ESTABLISHED: {");
  out.push("        std::string buf(LWS_PRE + MSG.size(), 0);");
  out.push("        MSG.copy(&buf[LWS_PRE], MSG.size());");
  out.push("        std::printf(\">>> 发送: %s\\n\", MSG.c_str());");
  out.push("        lws_write(wsi, reinterpret_cast<unsigned char *>(&buf[LWS_PRE]), MSG.size(), LWS_WRITE_TEXT);");
  out.push("        break;");
  out.push("    }");
  out.push("    case LWS_CALLBACK_CLIENT_RECEIVE:");
  out.push("        std::printf(\"<<< 接收: %.*s\\n\", static_cast<int>(len), static_cast<const char *>(in));");
  out.push("        g_done = 1;");
  out.push("        break;");
  out.push("    case LWS_CALLBACK_CLIENT_CONNECTION_ERROR:");
  out.push("        std::fprintf(stderr, \"连接失败\\n\");");
  out.push("        g_done = 1;");
  out.push("        break;");
  out.push("    case LWS_CALLBACK_CLIENT_CLOSED:");
  out.push("        g_done = 1;");
  out.push("        break;");
  out.push("    default:");
  out.push("        break;");
  out.push("    }");
  out.push("    return 0;");
  out.push("}");
  out.push("");
  out.push("static const struct lws_protocols protocols[] = {");
  out.push("    { \"api-manager\", ws_callback, 0, 4096, 0, nullptr, 0 },");
  out.push("    LWS_PROTOCOL_LIST_TERM");
  out.push("};");
  out.push("");
  out.push("int main() {");
  out.push("    struct lws_context_creation_info info;");
  out.push("    std::memset(&info, 0, sizeof(info));");
  out.push("    info.port = CONTEXT_PORT_NO_LISTEN;");
  out.push("    info.protocols = protocols;");
  out.push("    struct lws_context *ctx = lws_create_context(&info);");
  out.push("    if (!ctx) {");
  out.push("        std::fprintf(stderr, \"创建上下文失败\\n\");");
  out.push("        return 1;");
  out.push("    }");
  out.push("");
  out.push("    struct lws_client_connect_info cci;");
  out.push("    std::memset(&cci, 0, sizeof(cci));");
  out.push("    cci.context = ctx;");
  out.push(`    cci.address = ${JSON.stringify(u.host)};`);
  out.push(`    cci.port = ${u.port};`);
  out.push(`    cci.path = ${JSON.stringify(u.path)};`);
  out.push("    cci.host = cci.address;   /* Host 请求头 */");
  out.push("    cci.origin = \"api-manager\";");
  out.push("    cci.protocol = protocols[0].name;");
  if (u.scheme === "wss") {
    out.push("    cci.ssl_connection = LCCSCF_USE_SSL | LCCSCF_ALLOW_SELFSIGNED; /* wss */");
  } else {
    out.push("    /* wss: cci.ssl_connection = LCCSCF_USE_SSL | LCCSCF_ALLOW_SELFSIGNED; */");
  }
  out.push("    if (!lws_client_connect_via_info(&cci)) {");
  out.push("        std::fprintf(stderr, \"发起连接失败\\n\");");
  out.push("        lws_context_destroy(ctx);");
  out.push("        return 1;");
  out.push("    }");
  out.push("    while (!g_done) {");
  out.push("        lws_service(ctx, 50);");
  out.push("    }");
  out.push("    lws_context_destroy(ctx);");
  out.push("    return 0;");
  out.push("}");
  return out.join("\n");
}

function genWsCppUwebsockets(r: WsReq): string {
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（uWebSockets：高性能、事件驱动、人气很高）");
  out.push(" * GitHub/官网: https://github.com/uNetworking/uWebSockets");
  out.push(" * 依赖: uSockets（https://github.com/uNetworking/uSockets）");
  out.push(" * 安装:");
  out.push(" *   git clone https://github.com/uNetworking/uWebSockets");
  out.push(" *   git clone https://github.com/uNetworking/uSockets");
  out.push(" * 编译: g++ -std=c++17 -IuWebSockets/src -IuSockets/src \\");
  out.push(" *       ws_client.cpp uSockets/src/uSockets.c -lpthread -o ws_client");
  out.push(" *");
  for (const h of r.headers) out.push(` * 请求头: ${h.key}: ${h.value}（uWS 客户端暂不支持自定义请求头，请改用 query 参数）`);
  out.push(" */");
  out.push("#include <App.h>");
  out.push("#include <iostream>");
  out.push("#include <string>");
  out.push("");
  out.push(`static const std::string MSG = ${JSON.stringify(r.message || "hello, this is a websocket echo message")};`);
  out.push("");
  out.push("int main() {");
  out.push(`    uWS::App().connect(${JSON.stringify(r.url)}, {`);
  out.push("        .open = [](auto *ws) {");
  out.push("            std::cout << \">>> 发送: \" << MSG << std::endl;");
  out.push("            ws->send(MSG, uWS::OpCode::TEXT);");
  out.push("        },");
  out.push("        .message = [](auto *ws, std::string_view message, uWS::OpCode opCode) {");
  out.push("            (void)opCode;");
  out.push("            std::cout << \"<<< 接收: \" << message << std::endl;");
  out.push("            ws->close();");
  out.push("        },");
  out.push("        .close = [](auto *ws, int code, std::string_view message) {");
  out.push("            (void)ws; (void)code; (void)message;");
  out.push("            std::cout << \"连接已关闭\" << std::endl;");
  out.push("        },");
  out.push("        .error = [](auto *err) {");
  out.push("            std::cerr << \"连接失败: \" << err->what() << std::endl;");
  out.push("        },");
  out.push("    }).run();");
  out.push("    return 0;");
  out.push("}");
  return out.join("\n");
}

function genWsCppQt(r: WsReq): string {
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（Qt QWebSocket：Qt 框架，快速开发）");
  out.push(" * 官网: https://doc.qt.io/qt-6/qwebsocket.html");
  out.push(" * 安装: sudo apt install qt6-websockets-dev    (Ubuntu/Debian, Qt6)");
  out.push(" * 编译: g++ -std=c++17 -fPIC ws_client.cpp -o ws_client \\");
  out.push(" *       $(pkg-config --cflags --libs Qt6WebSockets Qt6Core)");
  out.push(" *       （或 CMake: find_package(Qt6 COMPONENTS WebSockets Core)）");
  out.push(" *");
  for (const h of r.headers) out.push(` * 请求头: ${h.key}: ${h.value}（通过 setRequestHeaders 设置）`);
  out.push(" */");
  out.push("#include <QCoreApplication>");
  out.push("#include <QWebSocket>");
  out.push("#include <QDebug>");
  out.push("#include <QUrl>");
  out.push("");
  out.push(`static const QString MSG = QStringLiteral(${JSON.stringify(r.message || "hello, this is a websocket echo message")});`);
  out.push("");
  out.push("int main(int argc, char *argv[]) {");
  out.push("    QCoreApplication app(argc, argv);");
  out.push("    QWebSocket socket;");
  if (r.headers.length) {
    out.push("    socket.setRequestHeaders({");
    for (const h of r.headers) out.push(`        { QStringLiteral(${JSON.stringify(h.key)}), QStringLiteral(${JSON.stringify(h.value)}) },`);
    out.push("    });");
  }
  out.push("");
  out.push("    QObject::connect(&socket, &QWebSocket::connected, [&socket]() {");
  out.push("        qDebug() << \">>> 连接成功\";");
  out.push("        socket.sendTextMessage(MSG);");
  out.push("    });");
  out.push("");
  out.push("    QObject::connect(&socket, &QWebSocket::textMessageReceived,");
  out.push("                    [&socket](const QString &message) {");
  out.push("        qDebug() << \"<<< 接收:\" << message;");
  out.push("        socket.close();");
  out.push("    });");
  out.push("");
  out.push("    QObject::connect(&socket, &QWebSocket::errorOccurred,");
  out.push("                    [&app, &socket](QAbstractSocket::SocketError error) {");
  out.push("        (void)error;");
  out.push("        qDebug() << \"连接失败:\" << socket.errorString();");
  out.push("        app.quit();");
  out.push("    });");
  out.push("");
  out.push("    QObject::connect(&socket, &QWebSocket::disconnected, &app, &QCoreApplication::quit);");
  out.push(`    socket.open(QUrl(${JSON.stringify(r.url)}));`);
  out.push("    return app.exec();");
  out.push("}");
  return out.join("\n");
}

function genWsPhp(r: WsReq, lib?: string): string {
  switch (lib) {
    case "ratchet":
      return genWsPhpRatchet(r);
    default:
      return genWsPhpSwoole(r);
  }
}

function genWsPhpSwoole(r: WsReq): string {
  const u = parseWsUrl(r.url);
  const out: string[] = [];
  out.push("<?php");
  out.push("/**");
  out.push(" * WebSocket 客户端示例（Swoole / OpenSwoole，协程，生产环境首选）");
  out.push(" * Swoole 官网: https://www.swoole.com/");
  out.push(" * Swoole 文档: https://wiki.swoole.com/#/websocket_client");
  out.push(" * OpenSwoole 官网: https://openswoole.com/");
  out.push(" * 安装: pecl install swoole      （Swoole）");
  out.push(" *       pecl install openswoole  （OpenSwoole）");
  out.push(" * 运行: php ws_client.php");
  out.push(" */");
  out.push("Co\\run(function () {");
  out.push(`    $client = new Swoole\\WebSocket\\Client(${JSON.stringify(u.host)}, ${u.port}, ${JSON.stringify(u.path)});`);
  out.push("    // 使用 OpenSwoole 时改为：");
  out.push(`    // $client = new OpenSwoole\\WebSocket\\Client(${JSON.stringify(u.host)}, ${u.port}, ${JSON.stringify(u.path)});`);
  if (r.headers.length) {
    out.push("    $client->setHeaders([");
    for (const h of r.headers) out.push(`        ${JSON.stringify(h.key)} => ${JSON.stringify(h.value)},`);
    out.push("    ]);");
  }
  out.push("");
  out.push("    $client->on('open', function (Swoole\\WebSocket\\Client $client) {");
  out.push(`        $msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")};`);
  out.push("        echo '>>> 发送: ' . $msg . PHP_EOL;");
  out.push("        $client->push($msg);");
  out.push("    });");
  out.push("");
  out.push("    $client->on('message', function (Swoole\\WebSocket\\Client $client, Swoole\\WebSocket\\Frame $frame) {");
  out.push("        echo '<<< 接收: ' . $frame->data . PHP_EOL;");
  out.push("        $client->close();");
  out.push("    });");
  out.push("");
  out.push("    $client->on('error', function (Swoole\\WebSocket\\Client $client, $error) {");
  out.push("        echo '连接失败: ' . $error . PHP_EOL;");
  out.push("        $client->close();");
  out.push("    });");
  out.push("");
  out.push("    $client->on('close', function (Swoole\\WebSocket\\Client $client) {");
  out.push("        echo '连接已关闭' . PHP_EOL;");
  out.push("    });");
  out.push("");
  out.push("    if (!$client->connect()) {");
  out.push("        echo '连接失败' . PHP_EOL;");
  out.push("    }");
  out.push("});");
  return out.join("\n");
}

function genWsPhpRatchet(r: WsReq): string {
  const out: string[] = [];
  out.push("<?php");
  out.push("/**");
  out.push(" * WebSocket 客户端示例（Ratchet：PHP 纯用户态库，基于 ReactPHP，传统 PHP）");
  out.push(" * 官网: http://socketo.me/");
  out.push(" * GitHub: https://github.com/ratchetphp/Ratchet");
  out.push(" * 客户端库 Pawl: https://github.com/ratchetphp/Pawl");
  out.push(" * 安装: composer require ratchet/pawl");
  out.push(" * 运行: php ws_client.php");
  out.push(" */");
  out.push("require __DIR__ . '/vendor/autoload.php';");
  out.push("");
  out.push("use Ratchet\\Client\\Connector;");
  out.push("use Ratchet\\Client\\WebSocket;");
  out.push("use Ratchet\\RFC6455\\Messaging\\MessageInterface;");
  out.push("use React\\EventLoop\\Loop;");
  out.push("");
  out.push("$loop = Loop::get();");
  out.push("$connector = new Connector($loop);");
  out.push("");
  out.push(`$connector(${JSON.stringify(r.url)}, [], [`);
  for (const h of r.headers) out.push(`    ${JSON.stringify(h.key)} => ${JSON.stringify(h.value)},`);
  out.push("])->then(function (WebSocket $conn) {");
  out.push(`    $msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")};`);
  out.push("    echo '>>> 发送: ' . $msg . PHP_EOL;");
  out.push("    $conn->send($msg);");
  out.push("");
  out.push("    $conn->on('message', function (MessageInterface $message) use ($conn) {");
  out.push("        echo '<<< 接收: ' . $message . PHP_EOL;");
  out.push("        $conn->close();");
  out.push("    });");
  out.push("");
  out.push("    $conn->on('close', function () {");
  out.push("        echo '连接已关闭' . PHP_EOL;");
  out.push("    });");
  out.push("}, function (\\Exception $e) {");
  out.push("    echo '连接失败: ' . $e->getMessage() . PHP_EOL;");
  out.push("});");
  out.push("");
  out.push("$loop->run();");
  return out.join("\n");
}

function genWsRuby(r: WsReq, lib?: string): string {
  switch (lib) {
    case "websocket-ruby":
      return genWsRubyWebsocket(r);
    case "sinatra":
      return genWsRubySinatra(r);
    case "actioncable":
      return genWsRubyActionCable(r);
    default:
      return genWsRubyFaye(r);
  }
}

function genWsRubyFaye(r: WsReq): string {
  const out: string[] = [];
  out.push("# WebSocket 客户端示例（faye-websocket：最经典，配合 EventMachine，可做服务端 + 客户端）");
  out.push("# 官网: https://github.com/faye/faye-websocket-ruby");
  out.push("# 安装: gem install faye-websocket eventmachine");
  out.push("# 运行: ruby ws_client.rb");
  out.push("");
  out.push("require 'faye/websocket'");
  out.push("require 'eventmachine'");
  out.push("");
  out.push("EM.run do");
  if (r.headers.length) {
    out.push(`  ws = Faye::WebSocket::Client.new(${JSON.stringify(r.url)}, [], headers: {`);
    for (const h of r.headers) out.push(`    ${JSON.stringify(h.key)} => ${JSON.stringify(h.value)},`);
    out.push("  })");
  } else {
    out.push(`  ws = Faye::WebSocket::Client.new(${JSON.stringify(r.url)})`);
  }
  out.push("");
  out.push("  ws.on :open do |_event|");
  out.push(`    msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")}`);
  out.push("    puts \">>> 发送: #{msg}\"");
  out.push("    ws.send(msg)");
  out.push("  end");
  out.push("");
  out.push("  ws.on :message do |event|");
  out.push("    puts \"<<< 接收: #{event.data}\"");
  out.push("    ws.close");
  out.push("  end");
  out.push("");
  out.push("  ws.on :error do |event|");
  out.push("    puts \"连接失败: #{event.message}\"");
  out.push("  end");
  out.push("");
  out.push("  ws.on :close do |_event|");
  out.push("    puts \"连接已关闭\"");
  out.push("    EM.stop");
  out.push("  end");
  out.push("end");
  return out.join("\n");
}

function genWsRubyWebsocket(r: WsReq): string {
  const u = parseWsUrl(r.url);
  const out: string[] = [];
  out.push("# WebSocket 客户端示例（websocket-ruby：轻量、纯 Ruby，负责帧编解码与握手，socket 层自实现）");
  out.push("# 官网: https://github.com/imanel/websocket-ruby");
  out.push("# 安装: gem install websocket");
  out.push("# 运行: ruby ws_client.rb");
  out.push("");
  out.push("require 'socket'");
  out.push("require 'websocket'");
  out.push("");
  out.push(`host = ${JSON.stringify(u.host)}`);
  out.push(`port = ${u.port}`);
  out.push(`url = ${JSON.stringify(r.url)}`);
  out.push("");
  out.push("# 1. 建立 TCP 连接");
  out.push("socket = TCPSocket.new(host, port)");
  out.push("");
  out.push("# 2. HTTP Upgrade 握手");
  if (r.headers.length) {
    out.push("handshake = WebSocket::ClientHandshake.new(url: url, headers: {");
    for (const h of r.headers) out.push(`  ${JSON.stringify(h.key)} => ${JSON.stringify(h.value)},`);
    out.push("})");
  } else {
    out.push("handshake = WebSocket::ClientHandshake.new(url: url)");
  }
  out.push("socket.write(handshake.to_s)");
  out.push("until handshake.finished?");
  out.push("  handshake << socket.readpartial(4096)");
  out.push("end");
  out.push("unless handshake.valid?");
  out.push("  puts \"握手失败: #{handshake.error}\"");
  out.push("  exit 1");
  out.push("end");
  out.push("puts \">>> 握手成功\"");
  out.push("");
  out.push("# 3. 发送一条文本消息");
  out.push(`msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")}`);
  out.push("frame = WebSocket::Frame::Outgoing::Client.new(version: handshake.version, data: msg, type: :text).to_s");
  out.push("socket.write(frame)");
  out.push("puts \">>> 发送: #{msg}\"");
  out.push("");
  out.push("# 4. 接收回显");
  out.push("incoming = WebSocket::Frame::Incoming::Client.new(version: handshake.version)");
  out.push("loop do");
  out.push("  incoming << socket.readpartial(4096)");
  out.push("  while (f = incoming.next)");
  out.push("    if %i[text binary].include?(f.type)");
  out.push("      puts \"<<< 接收: #{f.to_s}\"");
  out.push("      socket.close");
  out.push("      exit 0");
  out.push("    end");
  out.push("  end");
  out.push("end");
  return out.join("\n");
}

function genWsRubySinatra(r: WsReq): string {
  const u = parseWsUrl(r.url);
  const endpoint = (u.path.split("?")[0] || "/");
  const out: string[] = [];
  out.push("# WebSocket 服务端集成示例（Sinatra + faye-websocket：web 项目集成，与普通 HTTP 路由共存）");
  out.push("# Sinatra 官网: https://sinatrarb.com/");
  out.push("# faye-websocket 官网: https://github.com/faye/faye-websocket-ruby");
  out.push("# 安装: gem install sinatra faye-websocket puma");
  out.push("# 运行: ruby ws_server.rb   （启动后监听 4567 端口）");
  out.push("# 客户端连接: ws://127.0.0.1:4567" + endpoint);
  out.push("");
  out.push("require 'sinatra'");
  out.push("require 'faye/websocket'");
  out.push("");
  out.push("set :server, :puma");
  out.push("Faye::WebSocket.load_adapter('puma')");
  out.push("");
  out.push(`get ${JSON.stringify(endpoint)} do`);
  out.push("  if Faye::WebSocket.websocket?(request.env)");
  out.push("    ws = Faye::WebSocket.new(request.env, [], ping: 15)");
  out.push("");
  out.push("    ws.on :open do |_event|");
  out.push("      puts \"客户端已连接\"");
  out.push("    end");
  out.push("");
  out.push("    ws.on :message do |event|");
  out.push("      puts \">>> 接收: #{event.data}\"");
  out.push("      ws.send(event.data)   # 回显");
  out.push("    end");
  out.push("");
  out.push("    ws.on :close do |_event|");
  out.push("      puts \"客户端已断开\"");
  out.push("      ws = nil");
  out.push("    end");
  out.push("");
  out.push("    ws.rack_response");
  out.push("  else");
  out.push(`    \"WebSocket 服务已启动，请使用 ws://127.0.0.1:4567${endpoint} 连接\\n\"`);
  out.push("  end");
  out.push("end");
  return out.join("\n");
}

function genWsRubyActionCable(r: WsReq): string {
  const out: string[] = [];
  out.push("# Rails ActionCable 示例（Rails 框架内置 WebSocket，生产常用）");
  out.push("# 官网: https://guides.rubyonrails.org/action_cable_overview.html");
  out.push("# 内置（Rails 5+ 自带 ActionCable），无需额外安装 gem");
  out.push("# 1) 生成 Channel:  bin/rails generate channel Echo");
  out.push("# 2) 挂载路由:      config/routes.rb 中需有 mount ActionCable.server => '/cable'");
  out.push("# 3) 前端连接:      ActionCable 自带 consumer（app/javascript/channels/consumer.js）");
  out.push("");
  out.push("# ==== 服务端: app/channels/echo_channel.rb ====");
  out.push("class EchoChannel < ApplicationCable::Channel");
  out.push("  def subscribed");
  out.push(`    stream_from ${JSON.stringify("echo_channel")}`);
  out.push("  end");
  out.push("");
  out.push("  def echo(data)");
  out.push("    ActionCable.server.broadcast \"echo_channel\", data");
  out.push("  end");
  out.push("end");
  out.push("");
  out.push("# ==== 客户端: app/javascript/channels/echo_channel.js ====");
  out.push("import consumer from \"./consumer\"");
  out.push("");
  out.push("consumer.subscriptions.create(\"EchoChannel\", {");
  out.push("  received(data) {");
  out.push("    console.log(\"<<< 接收:\", data)");
  out.push("  },");
  out.push("");
  out.push("  echo(message) {");
  out.push(`    this.perform(\"echo\", { message: ${JSON.stringify(r.message || "hello, this is a websocket echo message")} })`);
  out.push("  }");
  out.push("});");
  return out.join("\n");
}

function genWsSwiftDispatch(r: WsReq, lib?: string): string {
  switch (lib) {
    case "starscream":
      return genWsSwiftStarscream(r);
    case "network":
      return genWsSwiftNetwork(r);
    default:
      return genWsSwiftUrlSession(r);
  }
}

function genWsSwiftUrlSession(r: WsReq): string {
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（URLSession WebSocket：系统原生，Swift 5.5+，iOS 13+ / macOS 10.15+）");
  out.push(" * 官网: https://developer.apple.com/documentation/foundation/urlsessionwebsockettask");
  out.push(" * 无需第三方库，系统自带");
  out.push(" */");
  out.push("import Foundation");
  out.push("");
  out.push("// 1. 发起连接（支持自定义请求头）");
  out.push(`var request = URLRequest(url: URL(string: ${JSON.stringify(r.url)})!)`);
  for (const h of r.headers) out.push(`request.setValue(${JSON.stringify(h.value)}, forHTTPHeaderField: ${JSON.stringify(h.key)})`);
  out.push("let session = URLSession(configuration: .default)");
  out.push("let wsTask = session.webSocketTask(with: request)");
  out.push("wsTask.resume()");
  out.push("print(\">>> 连接中...\")");
  out.push("");
  out.push("// 2. 发送一条文本消息");
  out.push(`let msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")}`);
  out.push("wsTask.send(.string(msg)) { error in");
  out.push("    if let error = error {");
  out.push("        print(\"发送失败: \" + error.localizedDescription)");
  out.push("    } else {");
  out.push("        print(\">>> 发送: \" + msg)");
  out.push("    }");
  out.push("}");
  out.push("");
  out.push("// 3. 循环接收消息");
  out.push("func receive() {");
  out.push("    wsTask.receive { result in");
  out.push("        switch result {");
  out.push("        case .success(let message):");
  out.push("            switch message {");
  out.push("            case .string(let text):");
  out.push("                print(\"<<< 接收: \" + text)");
  out.push("            case .data(let data):");
  out.push("                print(\"<<< 接收(binary): \" + data.base64EncodedString())");
  out.push("            @unknown default:");
  out.push("                break");
  out.push("            }");
  out.push("            receive()   // 继续接收下一条");
  out.push("        case .failure(let error):");
  out.push("            print(\"接收失败: \" + error.localizedDescription)");
  out.push("            exit(0)");
  out.push("        }");
  out.push("    }");
  out.push("}");
  out.push("receive()");
  out.push("");
  out.push("// 4. 保持主线程运行");
  out.push("dispatchMain()");
  return out.join("\n");
}

function genWsSwiftStarscream(r: WsReq): string {
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（Starscream：最流行的第三方 WebSocket 库，老系统兼容）");
  out.push(" * 官网: https://github.com/daltoniam/Starscream");
  out.push(" * 安装（Swift Package Manager）:");
  out.push(" *   dependencies: [.package(url: \"https://github.com/daltoniam/Starscream.git\", from: \"4.0.6\")]");
  out.push(" * 支持 iOS 8+ / macOS 10.10+，老系统也可用");
  out.push(" */");
  out.push("import Starscream");
  out.push("");
  out.push(`let url = URL(string: ${JSON.stringify(r.url)})!`);
  out.push("var request = URLRequest(url: url)");
  for (const h of r.headers) out.push(`request.setValue(${JSON.stringify(h.value)}, forHTTPHeaderField: ${JSON.stringify(h.key)})`);
  out.push("");
  out.push("let socket = WebSocket(request: request)");
  out.push("socket.onEvent = { event in");
  out.push("    switch event {");
  out.push("    case .connected(let headers):");
  out.push("        print(\">>> 连接成功\")");
  out.push(`        socket.write(string: ${JSON.stringify(r.message || "hello, this is a websocket echo message")})`);
  out.push("        print(\">>> 发送完成\")");
  out.push("    case .text(let text):");
  out.push("        print(\"<<< 接收: \" + text)");
  out.push("        socket.disconnect()");
  out.push("    case .binary(let data):");
  out.push("        print(\"<<< 接收(binary): \" + data.base64EncodedString())");
  out.push("    case .error(let error):");
  out.push("        print(\"连接失败: \" + (error?.localizedDescription ?? \"unknown\"))");
  out.push("    case .disconnected(let reason, let code):");
  out.push("        print(\"连接已关闭: \" + reason + \" (\" + String(code) + \")\")");
  out.push("    default:");
  out.push("        break");
  out.push("    }");
  out.push("}");
  out.push("socket.connect()");
  out.push("");
  out.push("// 保持主线程运行");
  out.push("dispatchMain()");
  return out.join("\n");
}

function genWsSwiftNetwork(r: WsReq): string {
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（Network.framework NWWebSocket：Apple Network 框架，底层高性能）");
  out.push(" * 官网: https://developer.apple.com/documentation/network/nwprotocolwebsocket");
  out.push(" * 系统自带（iOS 13+ / macOS 10.15+），基于 nw_connection，性能高");
  out.push(" */");
  out.push("import Network");
  out.push("import Foundation");
  out.push("");
  out.push("// 1. 解析 URL 并配置参数");
  out.push(`let url = URL(string: ${JSON.stringify(r.url)})!`);
  out.push("let params = NWParameters(url: url)!");
  out.push("params.allowLocalEndpointReuse = true");
  out.push("");
  if (r.headers.length) {
    out.push("// 2. 附加自定义请求头（WebSocket metadata）");
    out.push("let handshake = NWProtocolWebSocket.Metadata()");
    out.push("handshake.setAdditionalHeaders([" + r.headers.map((h) => `(${JSON.stringify(h.key)}, ${JSON.stringify(h.value)})`).join(", ") + "])");
    out.push("params.defaultProtocolStack.applicationProtocols.insert(handshake, at: 0)");
    out.push("");
  }
  out.push("// 3. 建立连接");
  out.push("let connection = NWConnection(to: .url(url), using: params)");
  out.push("");
  out.push("// 4. 循环接收消息");
  out.push("func receive() {");
  out.push("    connection.receiveMessage { content, context, isComplete, error in");
  out.push("        if let content = content, let text = String(data: content, encoding: .utf8) {");
  out.push("            print(\"<<< 接收: \" + text)");
  out.push("            connection.cancel()");
  out.push("        } else if let error = error {");
  out.push("            print(\"接收失败: \" + error.localizedDescription)");
  out.push("            connection.cancel()");
  out.push("        }");
  out.push("    }");
  out.push("}");
  out.push("");
  out.push("connection.stateUpdateHandler = { state in");
  out.push("    switch state {");
  out.push("    case .ready:");
  out.push("        print(\">>> 连接成功\")");
  out.push("        // 发送一条文本消息");
  out.push(`        let msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")}`);
  out.push("        let content = Data(msg.utf8)");
  out.push("        let meta = NWProtocolWebSocket.Metadata(opcode: .text)");
  out.push("        let context = NWConnection.ContentContext(identifier: \"text\", metadata: [meta])");
  out.push("        connection.send(content: content, contentContext: context, isComplete: true) { error in");
  out.push("            if let error = error {");
  out.push("                print(\"发送失败: \" + error.localizedDescription)");
  out.push("            } else {");
  out.push("                print(\">>> 发送: \" + msg)");
  out.push("            }");
  out.push("        }");
  out.push("        receive()");
  out.push("    case .waiting(let error):");
  out.push("        print(\"等待连接: \" + error.localizedDescription)");
  out.push("    case .failed(let error):");
  out.push("        print(\"连接失败: \" + error.localizedDescription)");
  out.push("    default:");
  out.push("        break");
  out.push("    }");
  out.push("}");
  out.push("connection.start(queue: .main)");
  out.push("");
  out.push("// 5. 保持主线程运行");
  out.push("dispatchMain()");
  return out.join("\n");
}

function genWsPerlDispatch(r: WsReq, lib?: string): string {
  switch (lib) {
    case "anyevent":
      return genWsPerlAnyEvent(r);
    default:
      return genWsPerlMojo(r);
  }
}

function genWsPerlMojo(r: WsReq): string {
  const out: string[] = [];
  out.push("#!/usr/bin/env perl");
  out.push("# WebSocket 客户端示例（Mojo::UserAgent：Mojolicious 全家桶，工业级，推荐）");
  out.push("# 官网: https://mojolicious.org/");
  out.push("# 文档: https://docs.mojolicious.org/Mojo/UserAgent");
  out.push("# 特性: 支持 ws/wss、文本/二进制帧、ping/pong（自动处理）");
  out.push("# 安装: cpanm Mojolicious   （或 apt install libmojolicious-perl）");
  out.push("# 运行: perl ws_client.pl");
  out.push("use strict;");
  out.push("use warnings;");
  out.push("use v5.10;");
  out.push("use Mojo::UserAgent;");
  out.push("");
  out.push("my $ua = Mojo::UserAgent->new;");
  if (r.headers.length) {
    out.push("");
    out.push("# 自定义请求头（握手时发送）");
    out.push("$ua->on(start => sub {");
    out.push("    my ($ua, $tx) = @_;");
    for (const h of r.headers) out.push(`    $tx->req->headers->header(${JSON.stringify(h.key)} => ${JSON.stringify(h.value)});`);
    out.push("});");
  }
  out.push("");
  out.push(`$ua->websocket(${JSON.stringify(r.url)} => sub {`);
  out.push("    my ($ua, $tx) = @_;");
  out.push("");
  out.push("    unless ($tx->is_websocket) {");
  out.push("        say '连接失败: ' . ($tx->res->message || 'unknown');");
  out.push("        return;");
  out.push("    }");
  out.push("");
  out.push("    say '>>> 连接成功';");
  out.push(`    my $msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")};`);
  out.push("    say \">>> 发送: $msg\";");
  out.push("    $tx->send($msg);                  # 文本帧");
  out.push("    # $tx->send({binary => $msg});   # 二进制帧");
  out.push("");
  out.push("    $tx->on(message => sub {");
  out.push("        my ($tx, $msg) = @_;");
  out.push("        say \"<<< 接收: $msg\";");
  out.push("        $tx->close;");
  out.push("    });");
  out.push("");
  out.push("    $tx->on(finish => sub {");
  out.push("        my ($tx, $code, $reason) = @_;");
  out.push("        say \"连接已关闭: $code $reason\";");
  out.push("        Mojo::IOLoop->stop;");
  out.push("    });");
  out.push("");
  out.push("    $tx->on(error => sub {");
  out.push("        my ($tx, $err) = @_;");
  out.push("        say \"错误: $err\";");
  out.push("    });");
  out.push("});");
  out.push("");
  out.push("# 启动事件循环（保持运行）");
  out.push("Mojo::IOLoop->start unless Mojo::IOLoop->is_running;");
  return out.join("\n");
}

function genWsPerlAnyEvent(r: WsReq): string {
  const out: string[] = [];
  out.push("#!/usr/bin/env perl");
  out.push("# WebSocket 客户端示例（AnyEvent::WebSocket::Client：AnyEvent 事件驱动，非阻塞）");
  out.push("# 官网: https://metacpan.org/pod/AnyEvent::WebSocket::Client");
  out.push("# AnyEvent 官网: https://metacpan.org/pod/AnyEvent");
  out.push("# 安装: cpanm AnyEvent AnyEvent::WebSocket::Client");
  out.push("# 运行: perl ws_client.pl");
  out.push("use strict;");
  out.push("use warnings;");
  out.push("use v5.10;");
  out.push("use AnyEvent;");
  out.push("use AnyEvent::WebSocket::Client 0.22;");
  out.push("");
  out.push("my $client = AnyEvent::WebSocket::Client->new;");
  out.push("");
  if (r.headers.length) {
    out.push("# 自定义请求头（握手时发送）");
    out.push("my %headers = (");
    for (const h of r.headers) out.push(`    ${JSON.stringify(h.key)} => ${JSON.stringify(h.value)},`);
    out.push(");");
    out.push("");
    out.push(`$client->connect(${JSON.stringify(r.url)}, headers => \\%headers)->cb(sub {`);
  } else {
    out.push(`$client->connect(${JSON.stringify(r.url)})->cb(sub {`);
  }
  out.push("    my $cv = shift;");
  out.push("    my $conn = eval { $cv->recv };");
  out.push("    unless ($conn) {");
  out.push("        say \"连接失败: $@\";");
  out.push("        return;");
  out.push("    }");
  out.push("");
  out.push("    say '>>> 连接成功';");
  out.push(`    my $msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")};`);
  out.push("    say \">>> 发送: $msg\";");
  out.push("    $conn->send($msg);   # 文本消息");
  out.push("");
  out.push("    $conn->on(each_message => sub {");
  out.push("        my ($conn, $message) = @_;");
  out.push("        say \"<<< 接收: \" . $message->body;");
  out.push("    });");
  out.push("");
  out.push("    $conn->on(finish => sub {");
  out.push("        my ($conn, $code, $reason) = @_;");
  out.push("        say \"连接已关闭: $code $reason\";");
  out.push("        exit 0;");
  out.push("    });");
  out.push("");
  out.push("    $conn->on(error => sub {");
  out.push("        my ($conn, $err) = @_;");
  out.push("        say \"错误: $err\";");
  out.push("    });");
  out.push("});");
  out.push("");
  out.push("# 保持运行（事件循环）");
  out.push("AnyEvent->condvar->recv;");
  return out.join("\n");
}

function genWsJulia(r: WsReq): string {
  const out: string[] = [];
  out.push("# WebSocket 客户端示例（WebSocket.jl：Julia 生态的 WebSocket 库）");
  out.push("# 官网: https://github.com/JuliaWeb/WebSocket.jl");
  out.push("# 安装: Pkg.add(\"WebSockets\")   （包名 WebSockets，仓库名 WebSocket.jl）");
  out.push("# 运行: julia ws_client.jl");
  out.push("using WebSockets");
  out.push("");
  if (r.headers.length) {
    out.push("# 自定义请求头（握手时发送）");
    out.push("headers = Dict{String,String}(");
    for (const h of r.headers) out.push(`    ${JSON.stringify(h.key)} => ${JSON.stringify(h.value)},`);
    out.push(")");
  } else {
    out.push("headers = Dict{String,String}()");
  }
  out.push("");
  out.push("# 建立连接（握手完成后回调 do 块，connect 返回后台任务）");
  out.push(`task = WebSockets.connect(${JSON.stringify(r.url)}, headers) do ws`);
  out.push("    println(\">>> 连接成功\")");
  out.push("");
  out.push("    # 发送一条文本消息");
  out.push(`    msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")}`);
  out.push("    send(ws, msg)");
  out.push("    println(\">>> 发送: \", msg)");
  out.push("");
  out.push("    # 循环接收消息");
  out.push("    while isopen(ws)");
  out.push("        message = receive(ws)");
  out.push("        if message.opcode == WebSockets.TEXT");
  out.push("            println(\"<<< 接收: \", String(message.data))");
  out.push("        end");
  out.push("    end");
  out.push("    println(\"连接已关闭\")");
  out.push("end");
  out.push("");
  out.push("# 等待连接任务结束");
  out.push("wait(task)");
  return out.join("\n");
}

function genWsKotlinDispatch(r: WsReq, lib?: string): string {
  switch (lib) {
    case "java-websocket":
      return genWsKotlinJavaWebSocket(r);
    default:
      return genWsKotlinOkhttp(r);
  }
}

function genWsKotlinOkhttp(r: WsReq): string {
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（OkHttp：最常用，Android / JVM 后端通用，生产首选）");
  out.push(" * 官网: https://square.github.io/okhttp/");
  out.push(" * GitHub: https://github.com/square/okhttp");
  out.push(" * 依赖（Gradle）:");
  out.push(" *   implementation(\"com.squareup.okhttp3:okhttp:4.12.0\")");
  out.push(" */");
  out.push("import okhttp3.OkHttpClient");
  out.push("import okhttp3.Request");
  out.push("import okhttp3.Response");
  out.push("import okhttp3.WebSocket");
  out.push("import okhttp3.WebSocketListener");
  out.push("import okio.ByteString");
  out.push("");
  out.push("fun main() {");
  out.push("    val client = OkHttpClient()");
  out.push("    val request = Request.Builder()");
  out.push(`        .url(${JSON.stringify(r.url)})`);
  for (const h of r.headers) out.push(`        .addHeader(${JSON.stringify(h.key)}, ${JSON.stringify(h.value)})`);
  out.push("        .build()");
  out.push("");
  out.push("    val ws = client.newWebSocket(request, object : WebSocketListener() {");
  out.push("        override fun onOpen(webSocket: WebSocket, response: Response) {");
  out.push("            println(\">>> 连接成功\")");
  out.push(`            webSocket.send(${JSON.stringify(r.message || "hello, this is a websocket echo message")})`);
  out.push("            println(\">>> 发送完成\")");
  out.push("        }");
  out.push("");
  out.push("        override fun onMessage(webSocket: WebSocket, text: String) {");
  out.push("            println(\"<<< 接收: \" + text)");
  out.push("            webSocket.close(1000, \"bye\")");
  out.push("        }");
  out.push("");
  out.push("        override fun onMessage(webSocket: WebSocket, bytes: ByteString) {");
  out.push("            println(\"<<< 接收(binary): \" + bytes.hex())");
  out.push("        }");
  out.push("");
  out.push("        override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {");
  out.push("            println(\"连接失败: \" + t.message)");
  out.push("        }");
  out.push("");
  out.push("        override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {");
  out.push("            println(\"连接已关闭: \" + reason)");
  out.push("        }");
  out.push("    })");
  out.push("");
  out.push("    // 保持主线程存活（命令行场景）");
  out.push("    Thread.sleep(5000)");
  out.push("    client.dispatcher.executorService.shutdown()");
  out.push("}");
  return out.join("\n");
}

function genWsKotlinJavaWebSocket(r: WsReq): string {
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（Java-WebSocket：独立 websocket 库，不依赖 okhttp）");
  out.push(" * 官网: https://github.com/TooTallNate/Java-WebSocket");
  out.push(" * 依赖（Gradle）:");
  out.push(" *   implementation(\"org.java-websocket:Java-WebSocket:1.5.7\")");
  out.push(" */");
  out.push("import org.java_websocket.client.WebSocketClient");
  out.push("import org.java_websocket.handshake.ServerHandshake");
  out.push("import java.net.URI");
  out.push("");
  out.push("fun main() {");
  out.push("    val client = object : WebSocketClient(URI(\"" + r.url + "\")) {");
  out.push("        override fun onOpen(handshakedata: ServerHandshake) {");
  out.push("            println(\">>> 连接成功\")");
  out.push(`            send(${JSON.stringify(r.message || "hello, this is a websocket echo message")})`);
  out.push("            println(\">>> 发送完成\")");
  out.push("        }");
  out.push("");
  out.push("        override fun onMessage(message: String) {");
  out.push("            println(\"<<< 接收: \" + message)");
  out.push("            close()");
  out.push("        }");
  out.push("");
  out.push("        override fun onClose(code: Int, reason: String, remote: Boolean) {");
  out.push("            println(\"连接已关闭: \" + reason)");
  out.push("        }");
  out.push("");
  out.push("        override fun onError(ex: Exception) {");
  out.push("            println(\"连接失败: \" + ex.message)");
  out.push("        }");
  out.push("    }");
  out.push("");
  if (r.headers.length) {
    out.push("    // 自定义请求头（握手前设置）");
    out.push("    val headers = mapOf(" + r.headers.map((h) => `"${h.key}" to "${h.value}"`).join(", ") + ")");
    out.push("    headers.forEach { (k, v) -> client.addHeader(k, v) }");
    out.push("");
  }
  out.push("    client.connect()");
  out.push("    // 保持主线程存活");
  out.push("    Thread.sleep(5000)");
  out.push("}");
  return out.join("\n");
}

function genWsErlang(r: WsReq): string {
  const u = parseWsUrl(r.url);
  const out: string[] = [];
  out.push("%% WebSocket 客户端示例（gun：http/https/ws/wss，高性能，工业级，支持 HTTP2，erlang 生态首选）");
  out.push("%% 官网: https://ninenines.eu/docs/en/gun/2.1/");
  out.push("%% GitHub: https://github.com/ninenines/gun");
  out.push("%% 依赖（rebar3）: {deps, [{gun, \"2.1.0\"}]}");
  out.push("%% 编译运行: rebar3 compile && rebar3 shell");
  out.push("-module(ws_client).");
  out.push("-export([main/0]).");
  out.push("");
  out.push("main() ->");
  out.push(`    %% 1. 建立 TCP/TLS 连接（wss:// 时加 #{transport => tls}）`);
  out.push(`    {ok, Conn} = gun:open(${JSON.stringify(u.host)}, ${u.port}),`);
  out.push("");
  out.push("    %% 2. 发起 WebSocket 握手升级（可携带自定义请求头）");
  if (r.headers.length) {
    out.push(`    StreamRef = gun:ws_upgrade(Conn, ${JSON.stringify(u.path)}, #{`);
    for (const h of r.headers) out.push(`        ${JSON.stringify(h.key)} => ${JSON.stringify(h.value)},`);
    out.push("    }),");
  } else {
    out.push(`    StreamRef = gun:ws_upgrade(Conn, ${JSON.stringify(u.path)}),`);
  }
  out.push("");
  out.push("    receive");
  out.push("        {gun_upgrade, Conn, StreamRef, _} ->");
  out.push("            io:format(\">>> 连接成功~n\")");
  out.push("            %% 3. 发送一条文本消息");
  out.push(`            gun:ws_send(Conn, StreamRef, {text, ${JSON.stringify(r.message || "hello, this is a websocket echo message")}}),`);
  out.push("            io:format(\">>> 发送完成~n\")");
  out.push("            receive_loop(Conn, StreamRef);");
  out.push("        {gun_response, Conn, StreamRef, fin, Status, _Headers} ->");
  out.push("            io:format(\"连接失败: HTTP ~p~n\", [Status]);");
  out.push("        {gun_error, Conn, StreamRef, Reason} ->");
  out.push("            io:format(\"连接失败: ~p~n\", [Reason])");
  out.push("    after 5000 ->");
  out.push("        io:format(\"连接超时~n\")");
  out.push("    end.");
  out.push("");
  out.push("receive_loop(Conn, StreamRef) ->");
  out.push("    receive");
  out.push("        {gun_ws, Conn, StreamRef, {text, Data}} ->");
  out.push("            io:format(\"<<< 接收: ~s~n\", [Data]),");
  out.push("            gun:close(Conn);");
  out.push("        {gun_ws, Conn, StreamRef, {binary, Data}} ->");
  out.push("            io:format(\"<<< 接收(binary): ~s~n\", [Data]),");
  out.push("            receive_loop(Conn, StreamRef);");
  out.push("        {gun_ws, Conn, StreamRef, {close, Code, Reason}} ->");
  out.push("            io:format(\"连接已关闭: ~p ~s~n\", [Code, Reason]);");
  out.push("        {gun_down, Conn, _Reason, _} ->");
  out.push("            io:format(\"连接已关闭~n\")");
  out.push("    end.");
  return out.join("\n");
}

function genWsUnsupported(lang: string): string {
  return `// ${lang}：暂未内置 WebSocket 客户端代码生成`;
}

/** 生成 WebSocket 客户端代码（C/C++/PHP/Ruby 支持库切换，详见各分发函数） */
export function generateWebSocketCode(lang: CodeLang, api: ApiFile, baseUrl: string, lib?: string): string {
  const r = buildWsReq(api, baseUrl);
  switch (lang) {
    case "bash":
    case "curl":
      return genWsBash(r);
    case "python":
      return genWsPython(r);
    case "javascript":
      return genWsJavaScript(r);
    case "typescript":
      return genWsTypeScript(r);
    case "go":
      return genWsGo(r);
    case "java":
      return genWsJavaDispatch(r, lib);
    case "csharp":
      return genWsCsharp(r);
    case "rust":
      return genWsRust(r);
    case "c":
      return genWsC(r, lib);
    case "cpp":
      return genWsCpp(r, lib);
    case "php":
      return genWsPhp(r, lib);
    case "ruby":
      return genWsRuby(r, lib);
    case "swift":
      return genWsSwiftDispatch(r, lib);
    case "perl":
      return genWsPerlDispatch(r, lib);
    case "julia":
      return genWsJulia(r);
    case "kotlin":
      return genWsKotlinDispatch(r, lib);
    case "erlang":
      return genWsErlang(r);
    default:
      return genWsUnsupported(lang);
  }
}

export function generateRequestCode(lang: CodeLang, api: ApiFile, baseUrl: string, lib?: string): string {
  const r = buildReq(api, baseUrl);
  switch (lang) {
    case "bash":
    case "curl":
      if (lib === "wget") return genBashWget(r);
      if (lib === "httpie") return genBashHttpie(r);
      return genCurl(r);
    case "python":
      return lib === "httpclient" ? genPythonHttpClient(r) : genPython(r);
    case "c":
      return genC(r);
    case "cpp":
      return genCpp(r);
    case "java":
      return genJavaDispatch(lib, r);
    case "csharp":
      return genCsharp(r);
    case "javascript":
      return genJsDispatch(lib, r);
    case "r":
      return lib === "rcurl" ? genRRCurl(r) : genR(r);
    case "rust":
      return genRust(r);
    case "delphi":
      return genDelphi(r);
    case "php":
      return genPhpDispatch(lib, r);
    case "go":
      return genGo(r);
    case "ruby":
      return genRuby(r);
    case "swift":
      return genSwift(r);
    case "perl":
      return genPerl(r);
    case "objectivec":
      return genObjectiveC(r);
    case "julia":
      return genJulia(r);
    case "kotlin":
      return genKotlin(r);
    case "typescript":
      return genTsDispatch(lib, r);
    case "erlang":
      return genErlang(r);
    case "lua":
      return genLuaDispatch(lib, r);
    case "powershell":
      return genPowershell(r);
  }
}

/* ==================== 库分发 ==================== */

function genJsDispatch(lib: string | undefined, r: Req): string {
  switch (lib) {
    case "axios":
      return genJsAxios(r);
    case "unirest":
      return genJsUnirest(r);
    case "request":
      return genJsRequest(r);
    case "native":
      return genJsNative(r);
    default:
      return genJavaScript(r);
  }
}

function genTsDispatch(lib: string | undefined, r: Req): string {
  switch (lib) {
    case "axios":
      return genTsAxios(r);
    case "unirest":
      return genTsUnirest(r);
    case "request":
      return genTsRequest(r);
    case "native":
      return genTsNative(r);
    default:
      return genTypeScript(r);
  }
}

function genJavaDispatch(lib: string | undefined, r: Req): string {
  switch (lib) {
    case "okhttp":
      return genJavaOkHttp(r);
    case "unirest":
      return genJavaUnirest(r);
    case "webclient":
      return genJavaWebClient(r);
    case "retrofit2":
      return genJavaRetrofit2(r);
    case "httpclient5":
      return genJavaHttpClient5(r);
    default:
      return genJava(r);
  }
}

function genPhpDispatch(lib: string | undefined, r: Req): string {
  switch (lib) {
    case "pecl":
      return genPhpPecl(r);
    case "snoopy":
      return genPhpSnoopy(r);
    case "guzzle":
      return genPhpGuzzle(r);
    default:
      return genPhp(r);
  }
}

function genLuaDispatch(lib: string | undefined, r: Req): string {
  switch (lib) {
    case "luacurl":
      return genLuaCurl(r);
    case "resty":
      return genLuaResty(r);
    default:
      return genLua(r);
  }
}
