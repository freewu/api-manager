// 将 logo.png（312x256）居中放到 1024x1024 透明画布，输出 logo-square.png
// 供 tauri icon 生成全套应用图标与 ico
import fs from "node:fs";
import zlib from "node:zlib";

const src = fs.readFileSync("logo.png");
const sw = src.readUInt32BE(16);
const sh = src.readUInt32BE(20);

// ---- 解码 PNG（支持 filter 0-4）----
function decodePng(buf) {
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
  const px = Buffer.alloc(sw * sh * 4);
  const stride = sw * 4 + 1;
  let prev = Buffer.alloc(sw * 4);
  const paeth = (a, b, c) => {
    const p = a + b - c;
    const pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c);
    return pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
  };
  for (let y = 0; y < sh; y++) {
    const f = raw[y * stride];
    const row = Buffer.from(raw.slice(y * stride + 1, (y + 1) * stride));
    for (let x = 0; x < sw; x++) {
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
      px.set(row.slice(i, i + 4), y * sw * 4 + i);
    }
  }
  return px;
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

// ---- 生成 1024x1024，logo 居中 ----
const W = 1024;
const px = decodePng(src);
const canvas = Buffer.alloc(W * W * 4);
const ox = Math.floor((W - sw) / 2);
const oy = Math.floor((W - sh) / 2);
for (let y = 0; y < sh; y++) {
  for (let x = 0; x < sw; x++) {
    const si = (y * sw + x) * 4;
    const di = ((oy + y) * W + (ox + x)) * 4;
    canvas[di] = px[si];
    canvas[di + 1] = px[si + 1];
    canvas[di + 2] = px[si + 2];
    canvas[di + 3] = px[si + 3];
  }
}
fs.writeFileSync("logo-square.png", encodePng(W, W, canvas));
console.log("logo-square.png written: 1024x1024, centered at", ox, oy);
