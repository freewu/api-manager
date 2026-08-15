// 将 logo.png 处理为 1024x1024 方形图标源图（供 tauri icon 生成全套应用图标与 ico）
// 注意：logo.png 已是正方形时，直接交给 tauri icon 使用原图（不做任何透明度处理）；
// 本脚本仅用于非正方形 logo：
//   1. 尺寸裁剪 —— 裁掉透明边距，保持宽高比缩放到铺满画布（约 90%，四周留 5% 安全边）
//   2. 透明度 —— 双线性采样对 RGBA 四通道一视同仁（alpha 独立平均），不预乘、不混合，
//      不调整原图透明度
import fs from "node:fs";
import zlib from "node:zlib";

const src = fs.readFileSync("logo.png");
const sw = src.readUInt32BE(16);
const sh = src.readUInt32BE(20);

// ---- 解码 PNG（支持 filter 0-4）----
function decodePng(buf, w, h) {
  let idat = Buffer.alloc(0);
  let off = 8;
  while (off < buf.length) {
    const len = buf.readUInt32BE(off);
    const type = buf.slice(off + 4, off + 8).toString();
    if (type === "IDAT") idat = Buffer.concat([idat, buf.slice(off + 8, off + 8 + len)]);
    off += 12 + len;
    if (type === "IEND") break;
  }
  const raw = zlib.inflateSync(idat);
  const px = Buffer.alloc(w * h * 4);
  const stride = w * 4 + 1;
  let prev = Buffer.alloc(w * 4);
  const paeth = (a, b, c) => {
    const p = a + b - c;
    const pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c);
    return pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
  };
  for (let y = 0; y < h; y++) {
    const f = raw[y * stride];
    const row = Buffer.from(raw.slice(y * stride + 1, (y + 1) * stride));
    for (let x = 0; x < w; x++) {
      const i = x * 4;
      const a = x > 0 ? row[i - 4] : 0;
      const b = y > 0 ? prev[i] : 0;
      const c = x > 0 && y > 0 ? prev[i - 4] : 0;
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
  return px;
}

// ---- 步骤 1：尺寸裁剪 —— 裁掉透明边距 ----
function cropAlpha(px, w, h, alphaThreshold = 8) {
  let minX = w, minY = h, maxX = -1, maxY = -1;
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      if (px[(y * w + x) * 4 + 3] > alphaThreshold) {
        if (x < minX) minX = x;
        if (x > maxX) maxX = x;
        if (y < minY) minY = y;
        if (y > maxY) maxY = y;
      }
    }
  }
  if (maxX < 0) throw new Error("图片内容为空（无可见像素）");
  const cw = maxX - minX + 1, ch = maxY - minY + 1;
  const out = Buffer.alloc(cw * ch * 4);
  for (let y = 0; y < ch; y++) {
    px.copy(out, y * cw * 4, (minY + y) * w * 4 + minX * 4, (minY + y) * w * 4 + minX * 4 + cw * 4);
  }
  return { px: out, w: cw, h: ch };
}

// ---- 步骤 2：普通 RGBA 双线性缩放（不调整透明度）----
// 四通道（含 alpha）各自独立双线性插值，alpha 不被预乘混合，原图透明度保持不变
function resizeAlpha(px, sw, sh, dw, dh) {
  const out = Buffer.alloc(dw * dh * 4);
  const sx = sw / dw, sy = sh / dh;
  for (let y = 0; y < dh; y++) {
    const fy = (y + 0.5) * sy - 0.5;
    for (let x = 0; x < dw; x++) {
      const fx = (x + 0.5) * sx - 0.5;
      const x0 = Math.max(0, Math.floor(fx)), y0 = Math.max(0, Math.floor(fy));
      const x1 = Math.min(sw - 1, x0 + 1), y1 = Math.min(sh - 1, y0 + 1);
      const tx = fx - x0, ty = fy - y0;
      for (let k = 0; k < 4; k++) {
        const src = (i) => px[(i * 4) + k];
        const v =
          src(y0 * sw + x0) * (1 - tx) * (1 - ty) +
          src(y0 * sw + x1) * tx * (1 - ty) +
          src(y1 * sw + x0) * (1 - tx) * ty +
          src(y1 * sw + x1) * tx * ty;
        out[(y * dw + x) * 4 + k] = Math.round(v);
      }
    }
  }
  return out;
}

// ---- PNG 编码（8bit RGBA，filter 0）----
const crcTable = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();
function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = crcTable[(c ^ buf[i]) & 255] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const t = Buffer.from(type, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([t, data])));
  return Buffer.concat([len, t, data, crc]);
}
function encodePng(w, h, px) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // color type RGBA
  const stride = w * 4 + 1;
  const raw = Buffer.alloc(stride * h);
  for (let y = 0; y < h; y++) {
    raw[y * stride] = 0; // filter none
    px.copy(raw, y * stride + 1, y * w * 4, (y + 1) * w * 4);
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", zlib.deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

// ---- 生成 1024x1024，logo 铺满画布约 90% ----
const W = 1024;
const FILL = 0.9; // 内容占画布比例（四周各留 5% 安全边）
const px0 = decodePng(src, sw, sh);
const cropped = cropAlpha(px0, sw, sh);
const scale = (W * FILL) / Math.max(cropped.w, cropped.h);
const nw = Math.max(1, Math.round(cropped.w * scale));
const nh = Math.max(1, Math.round(cropped.h * scale));
const resized = resizeAlpha(cropped.px, cropped.w, cropped.h, nw, nh);

const canvas = Buffer.alloc(W * W * 4);
const ox = Math.floor((W - nw) / 2);
const oy = Math.floor((W - nh) / 2);
for (let y = 0; y < nh; y++) {
  for (let x = 0; x < nw; x++) {
    const si = (y * nw + x) * 4;
    const di = ((oy + y) * W + (ox + x)) * 4;
    canvas[di] = resized[si];
    canvas[di + 1] = resized[si + 1];
    canvas[di + 2] = resized[si + 2];
    canvas[di + 3] = resized[si + 3];
  }
}
fs.writeFileSync("logo-square.png", encodePng(W, W, canvas));
console.log(
  `logo-square.png written: 1024x1024 (裁剪 ${sw}x${sh} -> ${cropped.w}x${cropped.h}，缩放 -> ${nw}x${nh}，位置 ${ox},${oy})`
);
