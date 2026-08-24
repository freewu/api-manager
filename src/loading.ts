/**
 * 独立加载页脚本（loading.html）：
 * 1. 黑客帝国风格字符雨动画（协议 / method 字符掉落，canvas 实现）；
 * 2. 预取主应用（index.html）及其全部 JS/CSS 资源；
 * 3. 资源加载完成后跳转到主应用页面。
 */

/** 掉落字符池：各种接口协议与 method */
const RAIN_WORDS = [
  "GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS",
  "HTTP", "HTTPS", "WS", "WSS", "SOCKET.IO", "GRAPHQL", "REST",
  "TCP", "UDP", "QUERY", "HEADER", "BODY", "MOCK", "JSON", "XML",
];

/** 黑客帝国风格字符雨：每列绑定一个协议/method 词，字符循环下落 */
function initMatrixRain(): void {
  const canvas = document.getElementById("rain") as HTMLCanvasElement | null;
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  const FONT_SIZE = 16;
  let cols = 0;
  interface Drop {
    y: number; // 列内字符位置（字符为单位）
    word: string; // 该列绑定的协议/method 词
    off: number; // 从词中哪个字符开始
    speed: number;
  }
  let drops: Drop[] = [];

  const randWord = () => RAIN_WORDS[Math.floor(Math.random() * RAIN_WORDS.length)];
  // 闭包内引用（非空已在上方检查）
  const cvs = canvas;
  const c2d = ctx;

  const resize = () => {
    cvs.width = window.innerWidth;
    cvs.height = window.innerHeight;
    cols = Math.max(1, Math.ceil(canvas.width / FONT_SIZE));
    drops = Array.from({ length: cols }, () => ({
      y: Math.floor(Math.random() * -40),
      word: randWord(),
      off: Math.floor(Math.random() * 12),
      speed: 0.4 + Math.random() * 0.7,
    }));
  };
  resize();
  window.addEventListener("resize", resize);

  let last = performance.now();
  let acc = 0;
  const FRAME = 1000 / 28; // 每帧间隔

  function tick(now: number) {
    acc += now - last;
    last = now;
    if (acc >= FRAME) {
      acc = 0;
      // 半透明黑覆盖形成拖尾
      c2d.fillStyle = "rgba(0, 0, 0, 0.1)";
      c2d.fillRect(0, 0, cvs.width, cvs.height);
      c2d.font = `${FONT_SIZE}px Consolas, "Courier New", monospace`;
      c2d.textBaseline = "top";
      for (let c = 0; c < cols; c++) {
        const d = drops[c];
        const py = Math.floor(d.y) * FONT_SIZE;
        if (py > cvs.height + FONT_SIZE) {
          // 落到底部后重新开始
          d.y = Math.floor(Math.random() * -30);
          d.word = randWord();
          d.off = Math.floor(Math.random() * 12);
          continue;
        }
        const idx = d.off + Math.floor(d.y);
        // 主体字符（绿色）
        c2d.fillStyle = "#22c55e";
        c2d.fillText(d.word.charAt(idx % d.word.length), c * FONT_SIZE, py);
        // 头部字符（亮白绿）
        const head = d.word.charAt((idx + 1) % d.word.length);
        c2d.fillStyle = "#d1fae5";
        c2d.fillText(head, c * FONT_SIZE, py + FONT_SIZE);
        d.y += d.speed;
      }
    }
    requestAnimationFrame(tick);
  }
  requestAnimationFrame(tick);
}

/** 预取主应用 HTML 中引用的全部 js/css 资源 */
async function warmUpMainApp(): Promise<void> {
  const res = await fetch("index.html", { cache: "no-cache" });
  const html = await res.text();
  const urls: string[] = [];
  const re = /(?:src|href)="([^"]+\.(?:js|css))"/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(html))) {
    urls.push(m[1]);
  }
  await Promise.all(urls.map((u) => fetch(u, { cache: "force-cache" })));
}

initMatrixRain();

const MIN_ANIM_MS = 600; // 动画最短展示时长，避免闪烁
const start = Date.now();

warmUpMainApp()
  .catch((e) => {
    // 预取失败不阻塞跳转（index.html 内置兜底动画）
    console.warn("[loading] preload failed:", e);
  })
  .finally(() => {
    const elapsed = Date.now() - start;
    const wait = Math.max(0, MIN_ANIM_MS - elapsed);
    setTimeout(() => {
      location.replace("index.html");
    }, wait);
  });
