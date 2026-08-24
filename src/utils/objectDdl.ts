import { ObjectDef } from "../types";

/** 属性类型 → SQL 列类型 */
export function sqlType(p: { kind: string; itemKind: string; refHash: string }): string {
  if (p.refHash) return "BIGINT";
  switch (p.kind) {
    case "number":
      return "BIGINT";
    case "boolean":
      return "TINYINT(1)";
    case "datetime":
      return "DATETIME";
    case "date":
      return "DATE";
    case "time":
      return "TIME";
    case "object":
      return "JSON";
    case "list":
      // list 的元素类型：基本类型展开为同类型多值字段仍用 JSON 存储；object 引用同样 JSON
      return "JSON";
    case "any":
      return "TEXT";
    default:
      return "VARCHAR(255)";
  }
}

const esc = (s: string) => s.replace(/'/g, "''");

/** 根据对象属性生成 MySQL 建表语句 */
export function generateCreateTable(obj: ObjectDef): string {
  const table = obj.object_name || obj.name;
  const cols: string[] = [];
  for (const p of obj.properties) {
    let col = `  \`${p.key}\` ${sqlType(p)}`;
    if (p.description) col += ` COMMENT '${esc(p.description)}'`;
    cols.push(col);
  }
  let sql = `CREATE TABLE \`${table}\` (\n${cols.join(",\n")}\n)`;
  if (obj.description) {
    sql += ` COMMENT='${esc(obj.description)}'`;
  }
  sql += ";";
  return sql;
}
