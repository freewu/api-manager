/** R（httr / RCurl）代码生成 */

import { esc, Req } from "./shared";
export function genR(r: Req): string {
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

export function genRRCurl(r: Req): string {
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
