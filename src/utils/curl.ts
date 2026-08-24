/**
 * 解析 curl 命令字符串 → http 接口字段。
 * 支持：-X/--request、-H/--header、-d/--data/--data-raw/--data-binary、
 *      --data-urlencode、-F/--form、-u/--user、-G/--get、URL 内 query 参数。
 * 引号：单引号、双引号（含 \" 转义）、反斜杠行尾续行。
 */
import { KeyValue } from "../types";

export interface CurlParseResult {
  method: string;
  url: string;
  /** URL 去掉 query 后的地址 */
  baseUrl: string;
  query: KeyValue[];
  headers: KeyValue[];
  bodyMode: "none" | "raw" | "json" | "form";
  bodyRaw: string;
}

/** 简单的 shell 分词：返回 token 数组（已去除引号包裹） */
function tokenize(cmd: string): string[] {
  const tokens: string[] = [];
  let cur = "";
  let quote: "'" | '"' | null = null;
  let i = 0;
  const n = cmd.length;
  while (i < n) {
    const c = cmd[i];
    if (quote === "'") {
      if (c === "'") quote = null;
      else cur += c;
      i++;
      continue;
    }
    if (quote === '"') {
      if (c === '"') quote = null;
      else if (c === "\\" && i + 1 < n) {
        const nx = cmd[i + 1];
        // 双引号内只转义 " \ $ ` 等；其余保持原样
        if (nx === '"' || nx === "\\" || nx === "$" || nx === "`") cur += nx;
        else cur += "\\" + nx;
        i += 2;
        continue;
      } else cur += c;
      i++;
      continue;
    }
    if (c === "'" || c === '"') {
      quote = c;
      i++;
      continue;
    }
    if (c === "\\" && i + 1 < n) {
      // 行尾反斜杠 = 续行
      if (cmd[i + 1] === "\n" || cmd[i + 1] === "\r") {
        i += 2;
        continue;
      }
      cur += cmd[i + 1];
      i += 2;
      continue;
    }
    if (c === " " || c === "\t" || c === "\n" || c === "\r") {
      if (cur) {
        tokens.push(cur);
        cur = "";
      }
      i++;
      continue;
    }
    cur += c;
    i++;
  }
  if (cur) tokens.push(cur);
  return tokens;
}

/** 解析 "Name: Value" 形式的请求头 */
function splitHeader(h: string): [string, string] {
  const idx = h.indexOf(":");
  if (idx < 0) return [h.trim(), ""];
  return [h.slice(0, idx).trim(), h.slice(idx + 1).trim()];
}

export function parseCurl(cmd: string): CurlParseResult {
  const tokens = tokenize(cmd.trim());
  if (tokens.length === 0) throw new Error("命令为空");

  let method = "GET";
  const headers: KeyValue[] = [];
  const dataParts: string[] = [];
  const formParts: [string, string][] = [];
  let url = "";
  let user: string | null = null;
  let isGet = false;
  let i = 0;

  // 跳过开头的 curl 可执行名
  if (tokens[0].toLowerCase() === "curl" || tokens[0].toLowerCase().endsWith("curl")) i = 1;

  const takeValue = (flag: string): string => {
    if (i + 1 >= tokens.length) throw new Error(`缺少 ${flag} 的参数`);
    return tokens[++i];
  };

  for (; i < tokens.length; i++) {
    const tok = tokens[i];
    if (tok === "-X" || tok === "--request") {
      method = takeValue(tok).toUpperCase();
    } else if (tok === "-H" || tok === "--header") {
      const [k, v] = splitHeader(takeValue(tok));
      headers.push({ key: k, value: v, enabled: true, description: "" });
    } else if (
      tok === "-d" || tok === "--data" || tok === "--data-raw" ||
      tok === "--data-binary" || tok === "--data-urlencode"
    ) {
      dataParts.push(takeValue(tok));
    } else if (tok === "-F" || tok === "--form") {
      const fv = takeValue(tok);
      const eq = fv.indexOf("=");
      if (eq >= 0) formParts.push([fv.slice(0, eq).trim(), fv.slice(eq + 1)]);
    } else if (tok === "-u" || tok === "--user") {
      user = takeValue(tok);
    } else if (tok === "-G" || tok === "--get") {
      isGet = true;
    } else if (
      tok === "--compressed" || tok === "-s" || tok === "--silent" ||
      tok === "-v" || tok === "--verbose" || tok === "-k" || tok === "--insecure" ||
      tok === "-L" || tok === "--location" || tok === "-i" || tok === "--include"
    ) {
      // 无参数开关，忽略
    } else if (tok === "-o" || tok === "--output") {
      takeValue(tok); // 跳过输出文件名
    } else if (tok.startsWith("-") && tok.length > 1 && !/^-\d/.test(tok)) {
      // 其他未知开关：跳过（及可能跟随的单独参数值）
      if (i + 1 < tokens.length && !tokens[i + 1].startsWith("-")) i++;
    } else if (!url) {
      url = tok;
    }
    // 其余 token 忽略
  }

  if (!url) throw new Error("未找到请求 URL");

  // -G 模式下 -d 内容并入 query；否则作为 body
  let rawData = dataParts.join("&");

  // 分离 URL 中的 query
  const qIdx = url.indexOf("?");
  let baseUrl = url;
  const query: KeyValue[] = [];
  const addQuery = (qs: string) => {
    for (const part of qs.split("&")) {
      if (!part) continue;
      const eq = part.indexOf("=");
      let k: string;
      let v: string;
      try {
        k = eq >= 0 ? decodeURIComponent(part.slice(0, eq)) : decodeURIComponent(part);
        v = eq >= 0 ? decodeURIComponent(part.slice(eq + 1)) : "";
      } catch {
        k = eq >= 0 ? part.slice(0, eq) : part;
        v = eq >= 0 ? part.slice(eq + 1) : "";
      }
      if (k) query.push({ key: k, value: v, enabled: true, description: "" });
    }
  };
  if (qIdx >= 0) {
    baseUrl = url.slice(0, qIdx);
    addQuery(url.slice(qIdx + 1));
  }
  if (isGet && rawData) {
    addQuery(rawData);
    rawData = "";
  }

  if (user) {
    const b64 = btoa(unescape(encodeURIComponent(user)));
    headers.push({ key: "Authorization", value: `Basic ${b64}`, enabled: true, description: "" });
  }

  // 推断 body
  let bodyMode: CurlParseResult["bodyMode"] = "none";
  let bodyRaw = "";
  const jsonLike = (s: string) => {
    const t0 = s.trim();
    return t0.startsWith("{") || t0.startsWith("[");
  };
  if (formParts.length > 0) {
    bodyMode = "form";
    bodyRaw = formParts.map(([k, v]) => `${k}=${v}`).join("&");
  } else if (rawData) {
    bodyRaw = rawData;
    if (jsonLike(rawData)) bodyMode = "json";
    else if (rawData.includes("=")) bodyMode = "form";
    else bodyMode = "raw";
  }

  return { method, url: baseUrl, baseUrl, query, headers, bodyMode, bodyRaw };
}
