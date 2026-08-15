// 解析 PE 资源，提取 exe 内嵌的 RT_GROUP_ICON / RT_ICON，解码最大 PNG 与 logo 对比
import fs from "node:fs";
import zlib from "node:zlib";

const exe = fs.readFileSync("src-tauri/target/release/api-manager.exe");

// DOS header
const peOff = exe.readUInt32LE(0x3c);
if (exe.toString("ascii", peOff, peOff + 4) !== "PE\0\0") throw new Error("not PE");
const numSections = exe.readUInt16LE(peOff + 6);
const optOff = peOff + 24;
const magic = exe.readUInt16LE(optOff);
const ddOff = magic === 0x20b ? optOff + 112 : optOff + 96;
const resDirRva = exe.readUInt32LE(ddOff + 2 * 8);
const resDirSize = exe.readUInt32LE(ddOff + 2 * 8 + 4);
const secOff = optOff + (magic === 0x20b ? 240 : 224);
const resDirOff = rvaToOff(resDirRva); // 资源目录文件偏移

function rvaToOff(rva) {
  for (let i = 0; i < numSections; i++) {
    const s = secOff + i * 40;
    const vs = exe.readUInt32LE(s + 8), va = exe.readUInt32LE(s + 12);
    const rs = exe.readUInt32LE(s + 16), ro = exe.readUInt32LE(s + 20);
    if (rva >= va && rva < va + Math.max(vs, rs)) return ro + (rva - va);
  }
  return -1;
}

function readDir(off, level, wantedType, tag) {
  const count = exe.readUInt16LE(off + 12) + exe.readUInt16LE(off + 14);
  const out = [];
  for (let i = 0; i < count; i++) {
    const e = off + 16 + i * 8;
    const name = exe.readUInt32LE(e);
    const val = exe.readUInt32LE(e + 4);
    const id = name & 0x80000000 ? null : name & 0xffff;
    if (level === 1 && id !== wantedType) continue;
    if (level === 3) {
      // 第三层条目指向数据项（Data Entry），其内为数据 RVA 与大小
      const de = resDirOff + (val & 0x7fffffff);
      const dataRva = exe.readUInt32LE(de);
      const size = exe.readUInt32LE(de + 4);
      out.push({ id: tag, dataOff: rvaToOff(dataRva), size });
    } else {
      const child = val & 0x80000000 ? resDirOff + (val & 0x7fffffff) : resDirOff + val;
      out.push(...readDir(child, level + 1, wantedType, level === 2 ? id : tag));
    }
  }
  return out;
}

function decodePng(b) {
  const w = b.readUInt32BE(16), h = b.readUInt32BE(20);
  let idat = Buffer.alloc(0), off = 8;
  while (off < b.length) {
    const len = b.readUInt32BE(off);
    const t = b.slice(off + 4, off + 8).toString();
    if (t === "IDAT") idat = Buffer.concat([idat, b.slice(off + 8, off + 8 + len)]);
    off += 12 + len;
    if (t === "IEND") break;
  }
  const raw = zlib.inflateSync(idat);
  const px = Buffer.alloc(w * h * 4);
  const stride = w * 4 + 1;
  let prev = Buffer.alloc(w * 4);
  const paeth = (a, b, c) => { const p = a + b - c, pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c); return pa <= pb && pa <= pc ? a : pb <= pc ? b : c; };
  for (let y = 0; y < h; y++) {
    const f = raw[y * stride];
    const row = Buffer.from(raw.slice(y * stride + 1, (y + 1) * stride));
    for (let x = 0; x < w; x++) {
      const i = x * 4;
      const a = x > 0 ? row[i - 4] : 0, b = y > 0 ? prev[i] : 0, c = x > 0 && y > 0 ? prev[i - 4] : 0;
      for (let k = 0; k < 4; k++) {
        let v = row[i + k];
        if (f === 1) v = (v + (x > 0 ? row[i - 4 + k] : 0)) & 255;
        else if (f === 2) v = (v + prev[i + k]) & 255;
        else if (f === 3) v = (v + ((a + b) >> 1)) & 255;
        else if (f === 4) v = (v + paeth(a, b, c)) & 255;
        row[i + k] = v;
      }
      prev[i] = row[i];
      px.set(row.slice(i, i + 4), y * w * 4 + i);
    }
  }
  return { w, h, px };
}

// 组图标（RT_GROUP_ICON=14）
const groups = readDir(resDirOff, 1, 14);
console.log("group icons:", groups.length);
const group = groups[0];
const g = exe.slice(group.dataOff);
const n = g.readUInt16LE(4);
console.log("group entries:", n);
let biggest = null, bigId = null, bigOff = 0, bigSize = 0;
for (let i = 0; i < n; i++) {
  const e = 6 + i * 14;
  const w = g[e] === 0 ? 256 : g[e], h = g[e + 1] === 0 ? 256 : g[e + 1];
  const size = g.readUInt32LE(e + 8), id = g.readUInt16LE(e + 12);
  if (!biggest || w * h > biggest) { biggest = w * h; bigId = id; bigSize = size; }
  console.log(` entry w=${w} h=${h} size=${size} id=${id}`);
}
// RT_ICON=3
const icons = readDir(resDirOff, 1, 3);
const icon = icons.find((x) => x.id === bigId);
if (!icon) { console.log("icons found:", icons.map((x) => x.id + ":" + x.size).join(", ")); throw new Error("big icon id " + bigId + " not found"); }
const png = exe.slice(icon.dataOff, icon.dataOff + icon.size);
const p = decodePng(png);
console.log("largest icon:", p.w + "x" + p.h);
let opaque = 0;
for (let i = 3; i < p.px.length; i += 4) if (p.px[i] > 60) opaque++;
const ratio = ((opaque / (p.w * p.h)) * 100).toFixed(2);
const c = (x, y) => { const i = (y * p.w + x) * 4; return [...p.px.slice(i, i + 4)]; };
console.log("opaque ratio:", ratio + "%");
console.log("corners:", c(2, 2), c(p.w - 3, 2), c(2, p.h - 3), c(p.w - 3, p.h - 3));
console.log("center:", c(p.w >> 1, p.h >> 1));
// 原 logo.png 徽标比例 3.79% 不透明；居中于 1024 画布缩放后应 ≈ 0.3%
console.log("判断:", ratio < 2 ? "✅ 透明背景 + 细徽标（logo.png 特征）" : "❌ 疑似旧图标（深色方块应 >20%）");
