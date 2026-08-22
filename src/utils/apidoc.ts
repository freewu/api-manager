import type { ApiFile, DocParam, DocSource, KeyValue } from "../types";

/** 按 source+key 查找文档补充说明 */
function findDoc(docs: DocParam[], source: DocSource, key: string): DocParam | undefined {
  return docs.find((d) => d.source === source && d.key === key);
}

/** JSON 值 → apiDoc 类型 */
function typeOf(v: unknown): string {
  if (v === null) return "String";
  if (Array.isArray(v)) return "List";
  switch (typeof v) {
    case "number":
      return Number.isInteger(v) ? "Integer" : "Float";
    case "boolean":
      return "Boolean";
    case "object":
      return "Object";
    default:
      return "String";
  }
}

function fmtDesc(s: string): string {
  return s.replace(/\s*\n\s*/g, " ").trim();
}

function lineAt(lines: string[], tag: string, type: string | undefined, key: string, desc: string) {
  const t = type || "String";
  const d = desc ? " " + fmtDesc(desc) : "";
  lines.push(` * @${tag} {${t}} ${key}${d}`);
}

function fieldLines(
  api: ApiFile,
  source: DocSource,
  kv: KeyValue[],
  tag: "apiHeader" | "apiQuery" | "apiParam" | "apiBody",
  out: string[],
) {
  for (const k of kv) {
    if (!k.enabled || !k.key.trim()) continue;
    const key = k.key.trim();
    const doc = findDoc(api.docParams, source, key);
    lineAt(out, tag, doc?.type, k.key.trim(), doc?.description || k.description);
  }
}

/** 递归展开 JSON 值 → apiDoc body 字段 */
function jsonFields(
  api: ApiFile,
  key: string,
  value: unknown,
  depth: number,
  out: string[],
) {
  const doc = findDoc(api.docParams, "body", key);
  if (Array.isArray(value)) {
    const t = value.length > 0 ? typeOf(value[0]) : "Object";
    lineAt(out, "apiBody", `List<${t}>`, key, doc?.description || "");
    if (value.length > 0 && typeof value[0] === "object" && value[0] !== null) {
      for (const [sub, v] of Object.entries(value[0] as Record<string, unknown>)) {
        jsonFields(api, `${key}[].${sub}`, v, depth + 1, out);
      }
    }
    return;
  }
  if (value !== null && typeof value === "object") {
    lineAt(out, "apiBody", "Object", key, doc?.description || "");
    for (const [sub, v] of Object.entries(value as Record<string, unknown>)) {
      jsonFields(api, `${key}.${sub}`, v, depth + 1, out);
    }
    return;
  }
  lineAt(out, "apiBody", doc?.type || typeOf(value), key, doc?.description || "");
}

/** 生成 apiDoc 注释块（含开始结束标记） */
export function buildApiDocComment(api: ApiFile, groupPath: string): string {
  const lines: string[] = ["/**"];
  const method =
    api.protocol === "websocket" ? "ws" : (api.method || "GET").toLowerCase();
  lines.push(` * @api {${method}} ${api.path} ${fmtDesc(api.name)}`);
  const groupName = groupPath
    .split(/[\\/]/)
    .filter(Boolean)
    .pop();
  if (groupName) {
    lines.push(` * @apiGroup ${fmtDesc(groupName)}`);
  }
  if (api.name.trim()) {
    lines.push(` * @apiName ${fmtDesc(api.name)}`);
  }
  if (api.description.trim()) {
    lines.push(` * @apiDescription ${fmtDesc(api.description)}`);
  }
  // 路径参数
  fieldLines(api, "path", api.params, "apiParam", lines);
  // 请求头
  fieldLines(api, "header", api.headers, "apiHeader", lines);
  // 查询参数
  fieldLines(api, "query", api.query, "apiQuery", lines);
  // body
  if (api.body.mode === "form") {
    fieldLines(api, "body", api.body.form, "apiBody", lines);
  } else if (api.body.mode === "json" && api.body.raw.trim()) {
    try {
      const parsed: unknown = JSON.parse(api.body.raw);
      if (parsed !== null && typeof parsed === "object" && !Array.isArray(parsed)) {
        for (const [k, v] of Object.entries(parsed as Record<string, unknown>)) {
          jsonFields(api, k, v, 0, lines);
        }
      } else {
        lineAt(lines, "apiBody", typeOf(parsed), "body", "");
      }
    } catch {
      lineAt(lines, "apiBody", "String", "body", "");
    }
  } else if (api.body.raw.trim()) {
    lineAt(lines, "apiBody", "String", "body", "");
  }
  // 响应示例
  for (const r of api.responses) {
    if (!r.body.trim()) continue;
    const tag = r.status >= 400 || /失败|错误/i.test(r.name) ? "apiErrorExample" : "apiSuccessExample";
    lines.push(` * @${tag} {${r.contentType || "json"}} ${fmtDesc(r.name || "响应")}`);
    const bodyLines = r.body.split("\n");
    for (const bl of bodyLines) {
      lines.push(` * ${bl}`);
    }
  }
  lines.push(" */");
  return lines.join("\n");
}
