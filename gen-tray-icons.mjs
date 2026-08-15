// 生成系统托盘菜单图标（16x16 PNG），输出到 src-tauri/tray-icons/*.png
// 绘制方式：64x64 超大画布上用 SDF（有向距离场）画矢量图形，再 4x4 盒式降采样，
// 得到平滑抗锯齿的 16x16 图标；纯色 + 透明背景，深浅色菜单下都清晰
import fs from "node:fs";
import path from "node:path";
import zlib from "node:zlib";

const OUT = path.join("src-tauri", "tray-icons");
const S = 64; // 超采样画布尺寸
const COLOR = [74, 125, 240, 255]; // 应用主色调蓝色

// ---------- PNG 编码 ----------
function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const body = Buffer.concat([Buffer.from(type), data]);
  const crcBuf = Buffer.alloc(4);
  crcBuf.writeUInt32BE(crc32(body) >>> 0, 0);
  return Buffer.concat([len, body, crcBuf]);
}
function crc32(buf) {
  let c = ~0;
  for (let i = 0; i < buf.length; i++) {
    c ^= buf[i];
    for (let k = 0; k < 8; k++) c = c >>> 1 ^ (0xedb88320 & -(c & 1));
  }
  return ~c;
}
function encodePng(w, h, px) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr[8] = 8;
  ihdr[9] = 6; // RGBA
  const stride = w * 4 + 1;
  const raw = Buffer.alloc(stride * h);
  for (let y = 0; y < h; y++) {
    raw[y * stride] = 0;
    px.copy(raw, y * stride + 1, y * w * 4, (y + 1) * w * 4);
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", zlib.deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

// ---------- SDF 形状（坐标为 64 画布，负值=内部） ----------
const canvas = new Float64Array(S * S); // 有向距离场
function reset() {
  canvas.fill(1e9);
}
function apply(fn) {
  for (let y = 0; y < S; y++)
    for (let x = 0; x < S; x++) {
      const d = fn(x + 0.5, y + 0.5);
      canvas[y * S + x] = Math.min(canvas[y * S + x], d);
    }
}
function fillCircle(cx, cy, r) {
  apply((x, y) => Math.hypot(x - cx, y - cy) - r);
}
function strokeCircle(cx, cy, r, w) {
  apply((x, y) => Math.abs(Math.hypot(x - cx, y - cy) - r) - w / 2);
}
function fillRect(x0, y0, x1, y1) {
  apply((x, y) => {
    const qx = Math.abs(x - (x0 + x1) / 2) - (x1 - x0) / 2;
    const qy = Math.abs(y - (y0 + y1) / 2) - (y1 - y0) / 2;
    return Math.hypot(Math.max(qx, 0), Math.max(qy, 0)) + Math.min(Math.max(qx, qy), 0);
  });
}
function sdfRoundRect(x, y, x0, y0, x1, y1, r) {
  const hw = (x1 - x0) / 2 - r;
  const hh = (y1 - y0) / 2 - r;
  const cx = (x0 + x1) / 2;
  const cy = (y0 + y1) / 2;
  const qx = Math.abs(x - cx) - hw;
  const qy = Math.abs(y - cy) - hh;
  return Math.hypot(Math.max(qx, 0), Math.max(qy, 0)) + Math.min(Math.max(qx, qy), 0) - r;
}
function fillRoundRect(x0, y0, x1, y1, r) {
  apply((x, y) => sdfRoundRect(x, y, x0, y0, x1, y1, r));
}
function strokeRoundRect(x0, y0, x1, y1, r, w) {
  apply((x, y) => Math.abs(sdfRoundRect(x, y, x0, y0, x1, y1, r)) - w / 2);
}
function strokeLine(x0, y0, x1, y1, w) {
  const dx = x1 - x0;
  const dy = y1 - y0;
  const len2 = dx * dx + dy * dy;
  apply((x, y) => {
    const t = Math.max(0, Math.min(1, ((x - x0) * dx + (y - y0) * dy) / len2));
    return Math.hypot(x - (x0 + t * dx), y - (y0 + t * dy)) - w / 2;
  });
}
// iq 三角形 SDF（顶点按逆时针，内部为负）
function sdTriangle(x, y, p0, p1, p2) {
  const e0 = [p1[0] - p0[0], p1[1] - p0[1]];
  const e1 = [p2[0] - p1[0], p2[1] - p1[1]];
  const e2 = [p0[0] - p2[0], p0[1] - p2[1]];
  const v0 = [x - p0[0], y - p0[1]];
  const v1 = [x - p1[0], y - p1[1]];
  const v2 = [x - p2[0], y - p2[1]];
  const cl = (a, lo, hi) => Math.max(lo, Math.min(hi, a));
  const pq0 = [v0[0] - e0[0] * cl((v0[0] * e0[0] + v0[1] * e0[1]) / (e0[0] * e0[0] + e0[1] * e0[1]), 0, 1), v0[1] - e0[1] * cl((v0[0] * e0[0] + v0[1] * e0[1]) / (e0[0] * e0[0] + e0[1] * e0[1]), 0, 1)];
  const pq1 = [v1[0] - e1[0] * cl((v1[0] * e1[0] + v1[1] * e1[1]) / (e1[0] * e1[0] + e1[1] * e1[1]), 0, 1), v1[1] - e1[1] * cl((v1[0] * e1[0] + v1[1] * e1[1]) / (e1[0] * e1[0] + e1[1] * e1[1]), 0, 1)];
  const pq2 = [v2[0] - e2[0] * cl((v2[0] * e2[0] + v2[1] * e2[1]) / (e2[0] * e2[0] + e2[1] * e2[1]), 0, 1), v2[1] - e2[1] * cl((v2[0] * e2[0] + v2[1] * e2[1]) / (e2[0] * e2[0] + e2[1] * e2[1]), 0, 1)];
  const d0 = pq0[0] * pq0[0] + pq0[1] * pq0[1];
  const d1 = pq1[0] * pq1[0] + pq1[1] * pq1[1];
  const d2 = pq2[0] * pq2[0] + pq2[1] * pq2[1];
  const s = Math.sign(e0[0] * e2[1] - e0[1] * e2[0]);
  return -s * Math.sqrt(Math.min(d0, Math.min(d1, d2)));
}
function strokeTriangle(p0, p1, p2, w) {
  apply((x, y) => Math.abs(sdTriangle(x, y, p0, p1, p2)) - w / 2);
}
// 环形（电源键）：圆环但顶部开口
function powerRing(cx, cy, r, w, gapDeg) {
  const g = (gapDeg * Math.PI) / 180;
  apply((x, y) => {
    const ang = Math.atan2(y - cy, x - cx); // 正上方 = PI/2
    const inGap = Math.abs(ang - Math.PI / 2) < g / 2;
    return inGap ? 1e9 : Math.abs(Math.hypot(x - cx, y - cy) - r) - w / 2;
  });
}
function render(name) {
  // 4x4 盒式降采样：16x16
  const out = Buffer.alloc(16 * 16 * 4);
  for (let y = 0; y < 16; y++)
    for (let x = 0; x < 16; x++) {
      let r = 0, g = 0, b = 0, a = 0;
      for (let sy = 0; sy < 4; sy++)
        for (let sx = 0; sx < 4; sx++) {
          const i = (y * 4 + sy) * S + (x * 4 + sx);
          const cov = Math.max(0, Math.min(1, 0.5 - canvas[i])); // 覆盖率
          r += COLOR[0] * cov;
          g += COLOR[1] * cov;
          b += COLOR[2] * cov;
          a += 255 * cov;
        }
      const o = (y * 16 + x) * 4;
      out[o] = Math.round(r / 16);
      out[o + 1] = Math.round(g / 16);
      out[o + 2] = Math.round(b / 16);
      out[o + 3] = Math.round(a / 16);
    }
  fs.writeFileSync(path.join(OUT, name), encodePng(16, 16, out));
  console.log("生成", name);
}

fs.mkdirSync(OUT, { recursive: true });

// ---- 1. 版本（info）：圆环 + i 点/竖线 ----
reset();
strokeCircle(32, 31, 20, 5);
fillCircle(32, 22, 2.6);
strokeLine(32, 27, 32, 39, 4);
render("info.png");

// ---- 2. 显示窗口：圆角矩形描边 + 标题栏 ----
reset();
strokeRoundRect(10, 9, 54, 55, 7, 4);
fillRect(10, 9, 54, 19);
render("window.png");

// ---- 3. 环境：三行滑块 ----
reset();
strokeLine(12, 17, 52, 17, 4);
fillCircle(35, 17, 6);
strokeLine(12, 32, 52, 32, 4);
fillCircle(20, 32, 6);
strokeLine(12, 47, 52, 47, 4);
fillCircle(45, 47, 6);
render("env.png");

// ---- 4. Mock 服务：服务器机架（描边 + 指示灯） ----
reset();
strokeRoundRect(10, 7, 54, 25, 4, 4);
strokeRoundRect(10, 39, 54, 57, 4, 4);
fillCircle(19, 16, 2.6);
fillCircle(29, 16, 2.6);
fillCircle(39, 16, 2.6);
fillCircle(19, 48, 2.6);
fillCircle(29, 48, 2.6);
fillCircle(39, 48, 2.6);
render("mock.png");

// ---- 5. GitHub 仓库：git 分支图 ----
reset();
fillCircle(15, 16, 5.5);
fillCircle(49, 16, 5.5);
fillCircle(40, 50, 5.5);
strokeLine(15, 21, 15, 42, 4.5);
strokeLine(15, 42, 40, 50, 4.5);
render("github.png");

// ---- 6. 提交 Issue：警告三角（描边）+ ! ----
reset();
strokeTriangle([32, 8], [56, 55], [8, 55], 4.5);
fillRect(29.5, 26, 34.5, 42);
fillCircle(32, 49, 2.6);
render("issue.png");

// ---- 7. 退出：电源符号 ----
reset();
powerRing(32, 30, 17, 4.5, 26);
strokeLine(32, 30, 32, 52, 4.5);
render("quit.png");
