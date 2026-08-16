/* API Manager 官网中英双语切换 */
(function () {
  "use strict";

  const I18N = {
    zh: {
      "nav.features": "功能特性",
      "nav.structure": "目录结构",
      "nav.stack": "技术栈",
      "nav.download": "下载",
      "hero.sub": "API 文档 · 接口测试 · Mock 服务 —— 一个工具全搞定",
      "hero.desc":
        "基于 Tauri 2 开发的桌面应用，目录即集合、一个接口一个 JSON 文件，天然支持 Git 版本管理。",
      "hero.download": "下载安装",
      "hero.github": "查看源码",
      "features.title": "功能特性",
      "features.desc": "Postman 风格布局，为个人与团队打造轻量高效的 API 工作台",
      "f1.title": "目录即集合",
      "f1.desc": "选择一个工作目录，目录结构即接口集合结构；一个接口一个 JSON 文件，天然支持 Git 版本管理。",
      "f2.title": "接口测试",
      "f2.desc": "发送 GET / POST / PUT / DELETE / PATCH 请求，查看状态、耗时、大小、Headers 与 Body（JSON 语法高亮）。",
      "f3.title": "Mock 服务",
      "f3.desc": "一键启动本地 Mock 服务，自动扫描启用 Mock 的接口，支持路径参数、延迟、模板变量与环境变量。",
      "f4.title": "演示接口",
      "f4.desc": "内置演示 API 集合（用户管理 / 订单管理），开箱即用，快速体验全部功能。",
      "f5.title": "一键导出",
      "f5.desc": "导出 Postman Collection、OpenAPI 3.0 或 Docsify 文档站点，方便共享与发布。",
      "f6.title": "代码生成",
      "f6.desc": "一键生成 20+ 种语言 / 框架的请求代码：curl、JavaScript、Python、Java、Go、Rust……",
      "f7.title": "全局环境变量",
      "f7.desc": "开发 / 测试 / 生产多环境一键切换，{{变量名}} 自动替换到 URL、Headers、Query、Body 与 Mock 响应。",
      "f8.title": "请求历史",
      "f8.desc": "自动记录历史请求，随时回看、一键回填重发，调试效率翻倍。",
      "f9.title": "多格式导入",
      "f9.desc": "支持 Postman Collection、OpenAPI (Swagger) 协议、Markdown 文档导入，接口与集合变量一键迁移。",
      "f10.title": "版本管理",
      "f10.desc": "接口可保存多个版本，左右对比差异，随时回退到任意历史版本。",
      "f11.title": "统计",
      "f11.desc": "分组 / 工作区维度统计接口数、Mock 启用数与请求方法分布，一目了然。",
      "f12.title": "灵活设置",
      "f12.desc": "丰富的设置项（工作区、代码生成、导出格式等），打造个性化工作环境。",
      "structure.title": "目录结构约定",
      "structure.desc": "分组即目录，接口即文件 —— 工作区本身就是可读的文档",
      "code.root": "# 根目录描述（集合信息）",
      "code.env": "# 全局环境变量（可选）",
      "code.group": "# 分组 = 目录",
      "code.ginfo": "# 分组描述",
      "code.api": "# 一个接口 = 一个 JSON 文件",
      "structure.note":
        "Mock 模板变量：{{path.id}} 路径参数、{{query.page}} Query 参数、{{method}}、{{path}}、{{变量名}} 全局环境变量。",
      "stack.title": "技术栈",
      "stack.react.desc": "前端框架",
      "stack.ts.desc": "类型安全的 JavaScript",
      "stack.vite.desc": "前端构建工具",
      "stack.backend": "后端 · Axum 提供 Mock 服务，reqwest 发送测试请求",
      "stack.just.desc": "命令运行器 · 统一开发与构建命令",
      "cap.main": "主界面",
      "cap.mock": "Mock 服务",
      "cap.export": "一键导出",
      "cap.codegen": "代码生成",
      "cap.env": "全局环境变量",
      "cap.history": "请求历史",
      "cap.import": "多种格式导入",
      "cap.examples": "目录即集合",
      "cap.demo": "演示接口",
      "cap.version": "版本对比",
      "cap.stat": "统计",
      "cap.setting": "设置",
      "cta.title": "现在就试试 API Manager",
      "cta.desc": "轻量 · 高效 · 开源 —— API 开发调试从未如此简单",
      "footer.text": "基于 Tauri 2 的 API 文档、测试与 Mock 工具 · 开源项目",
      "footer.releases": "发布版本",
    },
    en: {
      "nav.features": "Features",
      "nav.structure": "Structure",
      "nav.stack": "Tech Stack",
      "nav.download": "Download",
      "hero.sub": "API Docs · Testing · Mock — one tool for everything",
      "hero.desc":
        "A Tauri 2 desktop app where directories are collections and each API is a single JSON file — Git-friendly by nature.",
      "hero.download": "Download",
      "hero.github": "Source Code",
      "features.title": "Features",
      "features.desc": "A Postman-style layout — a lightweight, efficient API workbench for individuals and teams",
      "f1.title": "Directories as Collections",
      "f1.desc": "Pick a working directory; its structure is your API collection. One JSON file per API — Git-friendly by nature.",
      "f2.title": "Request Testing",
      "f2.desc": "Send GET / POST / PUT / DELETE / PATCH requests and inspect status, latency, size, headers, and body (JSON syntax highlighting).",
      "f3.title": "Mock Server",
      "f3.desc": "Start a local Mock server with one click; automatically serves every API with Mock enabled — path params, delay, template variables, and env vars supported.",
      "f4.title": "Demo APIs",
      "f4.desc": "Built-in demo collection (user management / order management) so you can try every feature right away.",
      "f5.title": "One-click Export",
      "f5.desc": "Export as Postman Collection, OpenAPI 3.0, or a Docsify documentation site for easy sharing and publishing.",
      "f6.title": "Code Generation",
      "f6.desc": "Generate request code for 20+ languages / frameworks: curl, JavaScript, Python, Java, Go, Rust…",
      "f7.title": "Environment Variables",
      "f7.desc": "Switch dev / test / prod environments instantly; {{variable}} placeholders resolve in URL, Headers, Query, Body, and Mock responses.",
      "f8.title": "Request History",
      "f8.desc": "Every request is recorded automatically; revisit or re-run past requests with one click.",
      "f9.title": "Multi-format Import",
      "f9.desc": "Import Postman Collection, OpenAPI (Swagger), or Markdown docs — APIs and collection variables migrate in one click.",
      "f10.title": "Versioning",
      "f10.desc": "Save multiple versions of an API, diff them side by side, and roll back to any historical version.",
      "f11.title": "Statistics",
      "f11.desc": "Per-group and per-workspace stats for API counts, Mock usage, and request-method distribution at a glance.",
      "f12.title": "Flexible Settings",
      "f12.desc": "Rich settings (workspace, code generation, export format, …) to personalize your workbench.",
      "structure.title": "Directory Conventions",
      "structure.desc": "Groups are directories, APIs are files — the workspace itself is readable documentation",
      "code.root": "# root description (collection info)",
      "code.env": "# global environment variables (optional)",
      "code.group": "# group = directory",
      "code.ginfo": "# group description",
      "code.api": "# one API = one JSON file",
      "structure.note":
        "Mock template variables: {{path.id}} path param, {{query.page}} query param, {{method}}, {{path}}, {{variable}} global env var.",
      "stack.title": "Tech Stack",
      "stack.react.desc": "Frontend framework",
      "stack.ts.desc": "Type-safe JavaScript",
      "stack.vite.desc": "Frontend build tool",
      "stack.backend": "Backend · Axum powers the Mock server, reqwest sends test requests",
      "stack.just.desc": "Command runner · unified dev & build commands",
      "cap.main": "Main UI",
      "cap.mock": "Mock Server",
      "cap.export": "One-click Export",
      "cap.codegen": "Code Generation",
      "cap.env": "Environment Variables",
      "cap.history": "Request History",
      "cap.import": "Multi-format Import",
      "cap.examples": "Directories as Collections",
      "cap.demo": "Demo APIs",
      "cap.version": "Version Diff",
      "cap.stat": "Statistics",
      "cap.setting": "Settings",
      "cta.title": "Try API Manager Now",
      "cta.desc": "Lightweight · Efficient · Open source — API development and debugging made simple",
      "footer.text": "A Tauri 2 based API documentation, testing & Mock tool · Open source",
      "footer.releases": "Releases",
    },
  };

  const KEY = "api-manager-site-lang";
  const detect = () =>
    (navigator.language || "zh").toLowerCase().startsWith("zh") ? "zh" : "en";
  let lang = localStorage.getItem(KEY) || detect();

  function apply(l) {
    lang = l;
    document.documentElement.lang = l;
    const dict = I18N[l] || I18N.zh;
    document.querySelectorAll("[data-i18n]").forEach((el) => {
      const key = el.dataset.i18n;
      if (dict[key] !== undefined) el.textContent = dict[key];
    });
    document.title =
      l === "zh"
        ? "API Manager - API 文档 · 测试 · Mock"
        : "API Manager - API Docs · Testing · Mock";
    document.querySelectorAll(".lang-btn").forEach((b) => {
      b.classList.toggle("active", b.dataset.lang === l);
    });
    localStorage.setItem(KEY, l);
  }

  document.querySelectorAll(".lang-btn").forEach((btn) => {
    btn.addEventListener("click", () => apply(btn.dataset.lang));
  });

  apply(lang);

  // ---------- Hero 轮播（12 张截图自动切换 + 左右按钮 + 圆点指示） ----------
  const track = document.getElementById("carouselTrack");
  const dotsBox = document.getElementById("carouselDots");
  const box = document.getElementById("heroCarousel");
  const slides = track ? Array.from(track.children) : [];
  let idx = 0;
  let timer = null;

  function show(i) {
    if (!slides.length) return;
    idx = (i + slides.length) % slides.length;
    track.style.transform = "translateX(-" + idx * 100 + "%)";
    if (dotsBox) {
      Array.from(dotsBox.children).forEach((d, k) => {
        d.classList.toggle("active", k === idx);
      });
    }
  }

  function restart() {
    if (timer) clearInterval(timer);
    timer = setInterval(() => show(idx + 1), 4000);
  }

  if (slides.length && dotsBox) {
    slides.forEach((_, k) => {
      const dot = document.createElement("button");
      dot.type = "button";
      dot.className = "carousel-dot" + (k === 0 ? " active" : "");
      dot.setAttribute("aria-label", "第 " + (k + 1) + " 张");
      dot.addEventListener("click", () => {
        show(k);
        restart();
      });
      dotsBox.appendChild(dot);
    });
    const prev = document.querySelector(".carousel-prev");
    const next = document.querySelector(".carousel-next");
    if (prev) prev.addEventListener("click", () => { show(idx - 1); restart(); });
    if (next) next.addEventListener("click", () => { show(idx + 1); restart(); });
    if (box) {
      box.addEventListener("mouseenter", () => timer && clearInterval(timer));
      box.addEventListener("mouseleave", restart);
    }
    restart();
  }
})();
