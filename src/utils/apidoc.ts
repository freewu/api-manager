import { isNetProtocol, type ApiFile, type DocParam, type DocSource, type KeyValue, type TreeNode } from "../types";
import { KIND_LABELS } from "./packet";
import { t } from "../i18n";

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
  // TCP / UDP 接口没有 HTTP 方法，协议名即方法；路径位置展示 host:port
  const isNet = isNetProtocol(api.protocol);
  const method =
    api.protocol === "websocket"
      ? "ws"
      : isNet
        ? api.protocol
        : (api.method || "GET").toLowerCase();
  const target = isNet
    ? `${api.net?.host || "127.0.0.1"}:${api.net?.port ?? 0}`
    : api.path;
  lines.push(` * @api {${method}} ${target} ${fmtDesc(api.name)}`);
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
  // TCP / UDP：用报文结构（封包 / 解包字段）代替 HTTP 参数与响应示例
  if (isNet) {
    for (const [title, fields] of [
      [t("net.docPackTitle"), api.pack || []],
      [t("net.docUnpackTitle"), api.unpack || []],
    ] as const) {
      if (fields.length === 0) continue;
      lines.push(" *");
      lines.push(` * 【${title}】`);
      for (const f of fields) {
        const bytes =
          f.kind === "varlen"
            ? t("net.docVarlenLen", { from: f.lenFrom != null ? f.lenFrom + 1 : "—" })
            : `${f.bytes}B`;
        const value = f.kind === "varlen" ? "" : ` = ${f.value}`;
        const desc = f.description ? ` ${fmtDesc(f.description)}` : "";
        lines.push(` *   ${f.key} (${t(KIND_LABELS[f.kind])} ${bytes})${value}${desc}`);
      }
    }
    lines.push(" */");
    return lines.join("\n");
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

/** 分组下接口的引用行：TCP/UDP 与 WebSocket 展示协议 + 地址，其余展示方法 + 路径 */
function nodeRef(n: TreeNode): string {
  const proto = (n.protocol || "http").toUpperCase();
  if (isNetProtocol(n.protocol) || n.protocol === "websocket" || n.protocol === "socketio") {
    return `${proto} ${n.endpoint || ""}`.trim();
  }
  return `${(n.method || "GET").toUpperCase()} ${n.endpoint || ""}`.trim();
}

/** 生成分组（目录）的 apiDoc 注释：@apiDefine 定义分组 + 列出其下接口 */
export function buildGroupApiDocComment(
  name: string,
  description: string,
  children: TreeNode[] = [],
): string {
  const title = name.trim() || "默认分组";
  // @apiDefine 的名称不能带空白字符
  const defineName = title.replace(/\s+/g, "");
  const lines: string[] = ["/**"];
  lines.push(` * @apiDefine ${defineName}${title === defineName ? "" : ` ${fmtDesc(title)}`}`);
  const desc = description.trim();
  if (desc) {
    lines.push(" *");
    for (const l of desc.split("\n")) {
      lines.push(` * ${l.trim()}`);
    }
  }
  const apis = children.filter((c) => c.kind !== "folder");
  if (apis.length > 0) {
    lines.push(" *");
    lines.push(` * 包含接口（${apis.length}）：`);
    for (const c of apis) {
      const ref = nodeRef(c);
      lines.push(` *   - ${c.name}${ref ? `：${ref}` : ""}`);
    }
  }
  lines.push(" */");
  return lines.join("\n");
}
