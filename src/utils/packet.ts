/**
 * TCP / UDP 报文封包 / 解包工具。
 *
 * 字段模型（与 Rust 侧 `PacketField` 对应）：
 * - 类型：固定值 fixed / 变量 var / 不定长变量 varlen
 * - 位数：fixed / var 占几个字节；varlen 忽略该值，长度取自「长度字段」
 * - 值：`0x` 开头按 hex 解析，否则按 UTF-8 文本编码；不足位数补 0x00，超出位数报错
 * - 不定长变量的长度字段（lenFrom，默认前一个字段）在封包时按实际字节数自动填充
 */
import { PacketField, PacketFieldKind } from "../types";

const encoder = new TextEncoder();

/** 字段类型显示名（i18n key） */
export const KIND_LABELS: Record<PacketFieldKind, string> = {
  fixed: "net.kindFixed",
  var: "net.kindVar",
  varlen: "net.kindVarlen",
};

/** 可国际化的错误：key + 插值参数（组件内用 t(key, params) 渲染） */
export interface PacketError {
  key: string;
  params?: Record<string, string | number>;
}

export function hexToBytes(hex: string): Uint8Array {
  const clean = hex.replace(/0x/gi, " ").replace(/[\s,]+/g, "");
  if (!clean) return new Uint8Array();
  const padded = clean.length % 2 === 1 ? "0" + clean : clean;
  const out = new Uint8Array(padded.length / 2);
  for (let i = 0; i < out.length; i++) {
    out[i] = parseInt(padded.slice(i * 2, i * 2 + 2), 16);
  }
  return out;
}

/** 字节 → hex 字符串（大写，空格分隔，与 Rust 侧一致） */
export function bytesToHex(bytes: Uint8Array | number[]): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).toUpperCase().padStart(2, "0"))
    .join(" ");
}

/** 字节 → 可打印文本（不可打印字符以 . 代替） */
export function bytesToText(bytes: Uint8Array | number[]): string {
  return Array.from(bytes)
    .map((c) => {
      if (c === 10 || c === 13 || c === 9) return String.fromCharCode(c);
      return c >= 0x20 && c < 0x7f ? String.fromCharCode(c) : ".";
    })
    .join("");
}

/** 把值编码为字节：0x 开头按 hex，否则按 UTF-8 文本 */
export function encodeValue(value: string): Uint8Array {
  const v = value.trim();
  if (/^0x[0-9a-fA-F]*$/.test(v)) return hexToBytes(v);
  return encoder.encode(value);
}

/** 大端无符号整数 → 字节；超出宽度返回 null */
function uintToBytes(value: number, width: number): Uint8Array | null {
  if (!Number.isFinite(value) || value < 0) return null;
  const out = new Uint8Array(width);
  let rest = Math.floor(value);
  for (let i = width - 1; i >= 0; i--) {
    out[i] = rest % 256;
    rest = Math.floor(rest / 256);
  }
  return rest > 0 ? null : out;
}

/** 字节 → 大端无符号整数 */
function bytesToUint(bytes: Uint8Array): number {
  let n = 0;
  for (const b of bytes) n = n * 256 + b;
  return n;
}

function padTo(bytes: Uint8Array, width: number): Uint8Array {
  if (bytes.length === width) return bytes;
  const out = new Uint8Array(width);
  out.set(bytes);
  return out;
}

/** 不定长变量的长度字段下标（未显式指定时取前一个字段） */
export function lengthFieldIndex(fields: PacketField[], index: number): number | null {
  const f = fields[index];
  if (f.kind !== "varlen") return null;
  if (typeof f.lenFrom === "number" && f.lenFrom >= 0 && f.lenFrom < fields.length && f.lenFrom !== index) {
    return f.lenFrom;
  }
  return index > 0 ? index - 1 : null;
}

export interface PacketFieldResult {
  index: number;
  key: string;
  kind: PacketFieldKind;
  /** 该字段实际占用的字节数 */
  size: number;
  hex: string;
  text: string;
  error?: PacketError;
}

export interface PacketBuildResult {
  bytes: Uint8Array;
  hex: string;
  text: string;
  /** 校验错误（值超出位数、长度超出长度字段宽度等） */
  errors: PacketError[];
  fields: PacketFieldResult[];
}

/**
 * 封包：按字段定义生成报文字节。
 * 长度字段（被不定长变量引用的字段）的值会按实际字节数自动填充。
 */
export function buildPacket(fields: PacketField[]): PacketBuildResult {
  const errors: PacketError[] = [];
  const results: PacketFieldResult[] = [];
  /** 需要自动填充长度值的字段下标 → 长度值 */
  const autoLen = new Map<number, number>();

  // 第一遍：计算不定长变量的实际字节数，登记到对应的长度字段
  for (let i = 0; i < fields.length; i++) {
    const f = fields[i];
    if (f.kind !== "varlen") continue;
    const len = encodeValue(f.value).length;
    const ref = lengthFieldIndex(fields, i);
    if (ref == null) {
      errors.push({ key: "net.errVarlenNoLen", params: { field: f.key || i + 1 } });
      continue;
    }
    autoLen.set(ref, len);
  }

  // 第二遍：编码每个字段
  const chunks: Uint8Array[] = [];
  for (let i = 0; i < fields.length; i++) {
    const f = fields[i];
    const width = Math.max(0, Math.floor(f.bytes || 0));
    let bytes: Uint8Array;
    let error: PacketError | undefined;

    if (autoLen.has(i)) {
      // 长度字段：按实际长度自动填充（大端）
      const len = autoLen.get(i)!;
      const encoded = uintToBytes(len, width);
      if (!encoded) {
        error = { key: "net.errLenOverflow", params: { len, width, field: f.key || i + 1 } };
        errors.push(error);
        bytes = new Uint8Array(width);
      } else {
        bytes = encoded;
      }
    } else {
      const raw = encodeValue(f.value);
      if (f.kind === "varlen") {
        bytes = raw;
      } else if (width === 0) {
        error = { key: "net.errBytesZero", params: { field: f.key || i + 1 } };
        errors.push(error);
        bytes = raw;
      } else if (raw.length > width) {
        error = { key: "net.errValueTooLong", params: { field: f.key || i + 1, width, len: raw.length } };
        errors.push(error);
        bytes = raw.slice(0, width);
      } else {
        // 不足位数补 0x00
        bytes = padTo(raw, width);
      }
    }

    results.push({
      index: i,
      key: f.key,
      kind: f.kind,
      size: bytes.length,
      hex: bytesToHex(bytes),
      text: bytesToText(bytes),
      error,
    });
    chunks.push(bytes);
  }

  const total = chunks.reduce((n, c) => n + c.length, 0);
  const bytes = new Uint8Array(total);
  let offset = 0;
  for (const c of chunks) {
    bytes.set(c, offset);
    offset += c.length;
  }
  return { bytes, hex: bytesToHex(bytes), text: bytesToText(bytes), errors, fields: results };
}

/**
 * 解包：按字段定义解析响应字节。
 * 不定长变量的长度取自其长度字段解析出的整数值。
 */
export function parsePacket(fields: PacketField[], data: Uint8Array): PacketFieldResult[] {
  const out: PacketFieldResult[] = [];
  let offset = 0;
  for (let i = 0; i < fields.length; i++) {
    const f = fields[i];
    const width = Math.max(0, Math.floor(f.bytes || 0));
    if (f.kind === "varlen") {
      const ref = lengthFieldIndex(fields, i);
      let len: number | null = null;
      if (ref != null && ref < out.length) {
        // 长度字段一律按「解析出的字节」当作大端整数
        len = bytesToUint(hexToBytes(out[ref].hex.replace(/ /g, "")));
      }
      if (len == null) {
        out.push({
          index: i,
          key: f.key,
          kind: f.kind,
          size: 0,
          hex: "",
          text: "",
          error: { key: "net.errVarlenNoLenField" },
        });
        continue;
      }
      const slice = data.slice(offset, Math.min(offset + len, data.length));
      out.push({
        index: i,
        key: f.key,
        kind: f.kind,
        size: slice.length,
        hex: bytesToHex(slice),
        text: bytesToText(slice),
        error:
          slice.length < len
            ? { key: "net.errRespShort", params: { want: len, got: slice.length } }
            : undefined,
      });
      offset += slice.length;
      continue;
    }
    const slice = data.slice(offset, Math.min(offset + width, data.length));
    out.push({
      index: i,
      key: f.key,
      kind: f.kind,
      size: slice.length,
      hex: bytesToHex(slice),
      text: bytesToText(slice),
      error:
        slice.length < width
          ? { key: "net.errRespShort", params: { want: width, got: slice.length } }
          : undefined,
    });
    offset += slice.length;
  }
  return out;
}

/** 字段在解析结果中的显示值：原字段值以 0x 开头时按 hex 显示，否则显示文本 */
export function displayValue(field: PacketField | undefined, result: PacketFieldResult): string {
  const hint = (field?.value || "").trim();
  if (hint.startsWith("0x") || hint.startsWith("0X")) return "0x" + result.hex.replace(/ /g, "");
  const text = result.text.trim();
  return text || (result.hex ? "0x" + result.hex.replace(/ /g, "") : "");
}
