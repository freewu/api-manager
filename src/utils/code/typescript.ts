/** TypeScript（Fetch / Axios / Unirest / Request / Native；浏览器 WebSocket）代码生成 */

import { esc, Req, WsReq } from "./shared";
export function genTypeScript(r: Req): string {
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

export function genTsAxios(r: Req): string {
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

export function genTsUnirest(r: Req): string {
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

export function genTsRequest(r: Req): string {
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

export function genTsNative(r: Req): string {
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

export function genWsTypeScript(r: WsReq): string {
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

export function genTsDispatch(lib: string | undefined, r: Req): string {
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
