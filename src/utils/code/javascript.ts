/** JavaScript（Fetch / Axios / Unirest / Request / Native；浏览器 WebSocket）代码生成 */

import { esc, Req, WsReq } from "./shared";
export function genJavaScript(r: Req): string {
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

export function genJsAxios(r: Req): string {
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

export function genJsUnirest(r: Req): string {
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

export function genJsRequest(r: Req): string {
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

export function genJsNative(r: Req): string {
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

export function genWsJavaScript(r: WsReq): string {
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

export function genJsDispatch(lib: string | undefined, r: Req): string {
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
