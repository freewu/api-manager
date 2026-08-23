// 对象唯一标识：属性按 key 字母排序，拼接 "key:kind[:itemKind][:refHash]" 后 SHA-256 前 12 位。
// 与 Rust 端（src-tauri/src/objects.rs object_hash）算法保持一致。
import { ObjectProp } from "../types";

export async function objectHash(props: ObjectProp[]): Promise<string> {
  const parts = (props || [])
    .map((p) => {
      let s = `${p.key.trim()}:${p.kind}`;
      if (p.kind === "list") s += `:${p.itemKind}`;
      if (
        (p.kind === "object" || (p.kind === "list" && p.itemKind === "object")) &&
        p.refHash
      ) {
        s += `:${p.refHash}`;
      }
      return s;
    })
    .sort()
    .join(",");
  const buf = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(parts));
  return [...new Uint8Array(buf)]
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("")
    .slice(0, 12);
}
