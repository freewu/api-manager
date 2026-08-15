import { ApiFile } from "../types";

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
  | "erlang";

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
  for (const p of api.params.filter((x) => x.enabled && x.key)) {
    const v = (p.value || "").split(",")[0].trim();
    url = url.split(`{${p.key}}`).join(v ? encodeURIComponent(v) : `{${p.key}}`);
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

export function generateRequestCode(lang: CodeLang, api: ApiFile, baseUrl: string): string {
  const r = buildReq(api, baseUrl);
  switch (lang) {
    case "bash":
    case "curl":
      return genCurl(r);
    case "python":
      return genPython(r);
    case "c":
      return genC(r);
    case "cpp":
      return genCpp(r);
    case "java":
      return genJava(r);
    case "csharp":
      return genCsharp(r);
    case "javascript":
      return genJavaScript(r);
    case "r":
      return genR(r);
    case "rust":
      return genRust(r);
    case "delphi":
      return genDelphi(r);
    case "php":
      return genPhp(r);
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
      return genTypeScript(r);
    case "erlang":
      return genErlang(r);
  }
}
