import { ApiFile } from "../types";

export type CodeLang = "curl" | "bash" | "go" | "rust" | "java" | "python" | "javascript";

export const CODE_LANGS: { value: CodeLang; label: string }[] = [
  { value: "bash", label: "bash" },
  { value: "go", label: "Go" },
  { value: "rust", label: "Rust" },
  { value: "java", label: "Java" },
  { value: "python", label: "Python" },
  { value: "javascript", label: "JavaScript" },
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
  if (api.body.mode === "json") {
    body = api.body.raw.trim();
    if (body) bodyKind = "json";
  } else if (api.body.mode === "raw") {
    body = api.body.raw;
    if (body.trim()) bodyKind = "text";
  } else if (api.body.mode === "form") {
    const f = api.body.form.filter((r) => r.enabled && r.key.trim());
    if (f.length) {
      body = f.map((r) => `${r.key}=${r.value}`).join("&");
      bodyKind = "text";
    }
  }
  return { method: api.method, url, headers, body, bodyKind };
}

function genCurl(r: Req): string {
  const q = (s: string) => `'${s.replace(/'/g, "'\\''")}'`;
  const parts = [`curl -X ${r.method} ${q(r.url)}`];
  for (const h of r.headers) parts.push(`-H ${q(`${h.key}: ${h.value}`)}`);
  if (r.body) parts.push(`--data-raw ${q(r.body)}`);
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
  out.push("");
  const m = r.method.toLowerCase();
  if (r.bodyKind === "json") {
    out.push(`response = requests.${m}(url, headers=headers, json=json.loads(payload))`);
  } else if (r.bodyKind === "text") {
    out.push(`response = requests.${m}(url, headers=headers, data=payload)`);
  } else {
    out.push(`response = requests.${m}(url, headers=headers)`);
  }
  out.push("");
  out.push("print(response.status_code)");
  out.push("print(response.text)");
  return out.join("\n");
}

function genGo(r: Req): string {
  const out: string[] = [];
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
  out.push("use reqwest;");
  out.push("");
  out.push("#[tokio::main]");
  out.push("async fn main() -> Result<(), Box<dyn std::error::Error>> {");
  out.push("    let client = reqwest::Client::new();");
  out.push("");
  out.push(`    let resp = client`);
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

export function generateRequestCode(lang: CodeLang, api: ApiFile, baseUrl: string): string {
  const r = buildReq(api, baseUrl);
  switch (lang) {
    case "curl":
    case "bash":
      return genCurl(r);
    case "go":
      return genGo(r);
    case "rust":
      return genRust(r);
    case "java":
      return genJava(r);
    case "python":
      return genPython(r);
    case "javascript":
      return genJavaScript(r);
  }
}
