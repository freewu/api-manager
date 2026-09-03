import type { ApiFile, DocParam, DocSource, KeyValue } from "../types";

/**
 * 说明字段统一迁移：header / query / path / body(form) 的参数说明原本存在
 * docParams（source + key 关联），请求页签与「接口文档」页签各自维护，容易不一致。
 * 现在统一存到 KeyValue.description（随行保存 / 删除，两个页签共同编辑同一个字段）。
 *
 * 读取接口时自动把 docParams 中的说明搬进对应行并清除 docParams 里的说明
 * （类型等其余信息保留；只剩说明的空条目整条移除）。幂等：已迁移过的文件
 * docParams 说明已为空，不会重复处理。
 */
export function migrateDocParamsToRows(api: ApiFile): ApiFile {
  const docParams = api.docParams;
  if (!docParams || docParams.length === 0) return api;

  const isFormBody = api.body && api.body.mode === "form";
  const rowsFor = (source: DocSource): KeyValue[] | null => {
    switch (source) {
      case "header":
        return api.headers || [];
      case "query":
        return api.query || [];
      case "path":
        return api.params || [];
      case "body":
        return isFormBody ? api.body.form || [] : null;
      default:
        return null;
    }
  };

  let changed = false;
  const next: DocParam[] = [];
  // 行数组只在实际搬入说明时按需复制（保持原对象不可变习惯）
  const copiedRows = new Map<DocSource, { src: KeyValue[]; dst: KeyValue[] }>();

  for (const d of docParams) {
    const rows = rowsFor(d.source);
    // 只有「扁平行条目」才属于行内说明（带 children 的 body 是 JSON 字段树 / 对象绑定，保留）
    const rowLike = rows !== null && (!d.children || d.children.length === 0);
    if (!rowLike) {
      next.push(d);
      continue;
    }
    const row = rows.find((r) => r.key.trim() === d.key);
    if (!row) {
      // 键在行里已不存在（行被删 / 改名）：保留原条目避免丢信息
      next.push(d);
      continue;
    }
    if (d.description && !row.description) {
      let bucket = copiedRows.get(d.source);
      if (!bucket) {
        bucket = { src: rows, dst: rows.map((r) => ({ ...r })) };
        copiedRows.set(d.source, bucket);
      }
      const idx = bucket.src.indexOf(row);
      if (idx >= 0) bucket.dst[idx].description = d.description;
      changed = true;
    }
    if (d.description) changed = true; // 旧说明被清除（并入行内或已被行内说明取代）
    const rest: DocParam = { ...d, description: "" };
    if (rest.type || rest.itemType || rest.objectName || (rest.children && rest.children.length > 0)) {
      next.push(rest);
    } else {
      changed = true; // 空条目整条移除
    }
  }
  if (!changed) return api;

  const patch: Partial<ApiFile> = { docParams: next };
  if (copiedRows.has("header")) patch.headers = copiedRows.get("header")!.dst;
  if (copiedRows.has("query")) patch.query = copiedRows.get("query")!.dst;
  if (copiedRows.has("path")) patch.params = copiedRows.get("path")!.dst;
  if (copiedRows.has("body")) patch.body = { ...api.body, form: copiedRows.get("body")!.dst };
  return { ...api, ...patch };
}
