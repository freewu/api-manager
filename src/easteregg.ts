/**
 * 彩蛋（本地私有，不提交到 git）：
 * URL 输入框包含 bluefrog（不区分大小写）时，点击「发送」触发 10 秒
 * 黑客帝国风格字符雨全屏效果（字符为接口协议 / method）。
 */
(() => {
  const WORDS = [
    "GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS",
    "HTTP", "HTTPS", "WS", "WSS", "SOCKET.IO", "GRAPHQL", "REST",
    "TCP", "UDP", "QUERY", "HEADER", "BODY", "MOCK", "JSON", "XML",
  ];
  let overlay: HTMLDivElement | null = null;
  let cleanup: (() => void) | null = null;

  function showRain(): void {
    if (overlay) return;
    overlay = document.createElement("div");
    overlay.style.cssText =
      "position:fixed;inset:0;z-index:99999;background:#000;pointer-events:none;";
    const canvas = document.createElement("canvas");
    canvas.style.cssText = "width:100%;height:100%;display:block;";
    overlay.appendChild(canvas);
    const brand = document.createElement("div");
    brand.textContent = "🐸 bluefrog";
    brand.style.cssText =
      "position:absolute;left:50%;top:44%;transform:translate(-50%,-50%);" +
      "color:#4ade80;font-size:36px;font-weight:700;letter-spacing:3px;" +
      "text-shadow:0 0 14px rgba(74,222,128,.6),0 0 44px rgba(34,197,94,.35);" +
      "font-family:system-ui,-apple-system,'Segoe UI','Microsoft YaHei',sans-serif;";
    overlay.appendChild(brand);
    const hint = document.createElement("div");
    hint.textContent = "wow, you found me 🐸";
    hint.style.cssText =
      "position:absolute;left:50%;top:calc(44% + 52px);transform:translateX(-50%);" +
      "color:#22c55e;font-size:13px;letter-spacing:2px;opacity:.7;" +
      "font-family:system-ui,sans-serif;";
    overlay.appendChild(hint);
    document.body.appendChild(overlay);

    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const FONT_SIZE = 16;
    let cols = 0;
    let drops: { y: number; word: string; off: number; speed: number }[] = [];
    const randWord = () => WORDS[Math.floor(Math.random() * WORDS.length)];
    const resize = () => {
      canvas.width = window.innerWidth;
      canvas.height = window.innerHeight;
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
    const FRAME = 1000 / 28;
    let raf = 0;
    const tick = (now: number) => {
      acc += now - last;
      last = now;
      if (acc >= FRAME) {
        acc = 0;
        ctx.fillStyle = "rgba(0,0,0,0.1)";
        ctx.fillRect(0, 0, canvas.width, canvas.height);
        ctx.font = `${FONT_SIZE}px Consolas, "Courier New", monospace`;
        ctx.textBaseline = "top";
        for (let c = 0; c < cols; c++) {
          const d = drops[c];
          const py = Math.floor(d.y) * FONT_SIZE;
          if (py > canvas.height + FONT_SIZE) {
            d.y = Math.floor(Math.random() * -30);
            d.word = randWord();
            d.off = Math.floor(Math.random() * 12);
            continue;
          }
          const idx = d.off + Math.floor(d.y);
          ctx.fillStyle = "#22c55e";
          ctx.fillText(d.word.charAt(idx % d.word.length), c * FONT_SIZE, py);
          const head = d.word.charAt((idx + 1) % d.word.length);
          ctx.fillStyle = "#d1fae5";
          ctx.fillText(head, c * FONT_SIZE, py + FONT_SIZE);
          d.y += d.speed;
        }
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);

    cleanup = () => {
      cancelAnimationFrame(raf);
      window.removeEventListener("resize", resize);
      overlay?.remove();
      overlay = null;
      cleanup = null;
    };
    window.setTimeout(() => cleanup?.(), 10000);
  }

  document.addEventListener(
    "click",
    (e) => {
      const target = e.target as HTMLElement;
      if (!target.closest(".send-btn")) return;
      const input = document.querySelector(".url-input") as HTMLInputElement | null;
      const v = (input?.value || "").toLowerCase();
      if (!v.includes("bluefrog")) return;
      // 彩蛋：不真正发送请求，仅触发矩阵雨效果（capture 阶段阻止事件到达 React）
      e.preventDefault();
      e.stopPropagation();
      showRain();
    },
    true,
  );

  // URL 输入框内按 Enter 同样触发发送：命中彩蛋时也只播放效果
  document.addEventListener(
    "keydown",
    (e) => {
      if (e.key !== "Enter") return;
      const active = document.activeElement as HTMLElement | null;
      if (!active || !active.classList.contains("url-input")) return;
      const v = (active as HTMLInputElement).value.toLowerCase();
      if (!v.includes("bluefrog")) return;
      e.preventDefault();
      e.stopPropagation();
      showRain();
    },
    true,
  );
})();
