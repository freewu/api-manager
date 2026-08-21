/** Bash / cURL / Wget / HTTPie 代码生成 */

import { esc, Req, WsReq } from "./shared";
export function genCurl(r: Req): string {
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

export function genBashWget(r: Req): string {
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

export function genBashHttpie(r: Req): string {
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

export function genWsBash(r: WsReq): string {
  const out: string[] = [];
  out.push("# 需要安装 websocat: https://github.com/vi/websocat");
  for (const h of r.headers) out.push(`# 请求头：${h.key}: ${h.value}`);
  out.push(`printf '%s' ${JSON.stringify(r.message)} | websocat '${esc(r.url, "'")}'`);
  return out.join("\n");
}
