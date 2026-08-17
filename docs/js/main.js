/* API Manager 官网三语切换（简体中文 / 繁體中文 / English） */
(function () {
  "use strict";

  const I18N = {
    zh: {
      "nav.features": "功能特性",
      "nav.structure": "目录结构",
      "nav.stack": "技术栈",
      "nav.download": "下载",
      "hero.sub": "API 文档 · 接口测试 · Mock 服务 —— 一个工具全搞定",
      "hero.desc": "基于 Tauri 2 开发的桌面应用，目录即集合、一个接口一个 JSON 文件，天然支持 Git 版本管理。",
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
      "structure.note": "Mock 模板变量：{{path.id}} 路径参数、{{query.page}} Query 参数、{{method}}、{{path}}、{{变量名}} 全局环境变量。",
      "stack.title": "技术栈",
      "stack.react.desc": "前端框架",
      "stack.ts.desc": "类型安全的 JavaScript",
      "stack.vite.desc": "前端构建工具",
      "stack.backend": "后端 · Axum Mock 服务 + reqwest 请求",
      "stack.just.desc": "命令运行器",
      "cap.main": "主界面 —— Postman 风格布局：左侧接口树、中间请求编辑器、下方响应面板",
      "cap.start": "开始页 —— 最近打开工作区、示例工作区与快捷入口",
      "cap.mock": "Mock 服务 —— 一键启动本地 Mock，支持路径参数、延迟与模板变量",
      "cap.export": "一键导出 —— Postman Collection / OpenAPI / Docsify 三种格式",
      "cap.codegen": "代码生成 —— 20+ 种语言 / 框架的请求代码一键生成",
      "cap.env": "全局环境变量 —— 多环境切换，{{变量名}} 自动替换到请求各处",
      "cap.history": "请求历史 —— 自动记录，一键回填重发",
      "cap.import": "多格式导入 —— Postman / OpenAPI (Swagger) / Markdown 一键导入",
      "cap.examples": "目录即集合 —— 目录结构即接口集合，天然支持 Git 管理",
      "cap.demo": "演示接口 —— 内置演示集合，开箱即用体验全部功能",
      "cap.version": "版本对比 —— 多版本保存、差异对比、随时回退",
      "cap.stat": "统计 —— 接口数 / Mock 启用 / 请求方法分布一览",
      "cap.setting": "设置 —— 工作区、代码生成、导出格式等个性化配置",
      "cta.title": "现在就试试 API Manager",
      "cta.desc": "轻量 · 高效 · 开源 —— API 开发调试从未如此简单",
      "footer.text": "基于 Tauri 2 的 API 文档、测试与 Mock 工具 · 开源项目",
      "footer.releases": "发布版本",
      "carousel.aria": "第 {n} 张",
    },
    tw: {
      "nav.features": "功能特性",
      "nav.structure": "目錄結構",
      "nav.stack": "技術棧",
      "nav.download": "下載",
      "hero.sub": "API 文檔 · 接口測試 · Mock 服務 —— 一個工具全搞定",
      "hero.desc": "基於 Tauri 2 開發的桌面應用，目錄即集合、一個接口一個 JSON 文件，天然支持 Git 版本管理。",
      "hero.download": "下載安裝",
      "hero.github": "查看源碼",
      "features.title": "功能特性",
      "features.desc": "Postman 風格佈局，為個人與團隊打造輕量高效的 API 工作臺",
      "f1.title": "目錄即集合",
      "f1.desc": "選擇一個工作目錄，目錄結構即接口集合結構；一個接口一個 JSON 文件，天然支持 Git 版本管理。",
      "f2.title": "接口測試",
      "f2.desc": "發送 GET / POST / PUT / DELETE / PATCH 請求，查看狀態、耗時、大小、Headers 與 Body（JSON 語法高亮）。",
      "f3.title": "Mock 服務",
      "f3.desc": "一鍵啟動本地 Mock 服務，自動掃描啟用 Mock 的接口，支持路徑參數、延遲、模板變量與環境變量。",
      "f4.title": "演示接口",
      "f4.desc": "內置演示 API 集合（用戶管理 / 訂單管理），開箱即用，快速體驗全部功能。",
      "f5.title": "一鍵導出",
      "f5.desc": "導出 Postman Collection、OpenAPI 3.0 或 Docsify 文檔站點，方便共享與發佈。",
      "f6.title": "代碼生成",
      "f6.desc": "一鍵生成 20+ 種語言 / 框架的請求代碼：curl、JavaScript、Python、Java、Go、Rust……",
      "f7.title": "全局環境變量",
      "f7.desc": "開發 / 測試 / 生產多環境一鍵切換，{{變量名}} 自動替換到 URL、Headers、Query、Body 與 Mock 響應。",
      "f8.title": "請求歷史",
      "f8.desc": "自動記錄歷史請求，隨時回看、一鍵回填重發，調試效率翻倍。",
      "f9.title": "多格式導入",
      "f9.desc": "支持 Postman Collection、OpenAPI (Swagger) 協議、Markdown 文檔導入，接口與集合變量一鍵遷移。",
      "f10.title": "版本管理",
      "f10.desc": "接口可保存多個版本，左右對比差異，隨時回退到任意歷史版本。",
      "f11.title": "統計",
      "f11.desc": "分組 / 工作區維度統計接口數、Mock 啟用數與請求方法分佈，一目瞭然。",
      "f12.title": "靈活設置",
      "f12.desc": "豐富的設置項（工作區、代碼生成、導出格式等），打造個性化工作環境。",
      "structure.title": "目錄結構約定",
      "structure.desc": "分組即目錄，接口即文件 —— 工作區本身就是可讀的文檔",
      "code.root": "# 根目錄描述（集合信息）",
      "code.env": "# 全局環境變量（可選）",
      "code.group": "# 分組 = 目錄",
      "code.ginfo": "# 分組描述",
      "code.api": "# 一個接口 = 一個 JSON 文件",
      "structure.note": "Mock 模板變量：{{path.id}} 路徑參數、{{query.page}} Query 參數、{{method}}、{{path}}、{{變量名}} 全局環境變量。",
      "stack.title": "技術棧",
      "stack.react.desc": "前端框架",
      "stack.ts.desc": "類型安全的 JavaScript",
      "stack.vite.desc": "前端構建工具",
      "stack.backend": "後端 · Axum Mock 服務 + reqwest 請求",
      "stack.just.desc": "命令運行器",
      "cap.main": "主界面 —— Postman 風格佈局：左側接口樹、中間請求編輯器、下方響應面板",
      "cap.start": "開始頁 —— 最近打開工作區、示例工作區與快捷入口",
      "cap.mock": "Mock 服務 —— 一鍵啟動本地 Mock，支持路徑參數、延遲與模板變量",
      "cap.export": "一鍵導出 —— Postman Collection / OpenAPI / Docsify 三種格式",
      "cap.codegen": "代碼生成 —— 20+ 種語言 / 框架的請求代碼一鍵生成",
      "cap.env": "全局環境變量 —— 多環境切換，{{變量名}} 自動替換到請求各處",
      "cap.history": "請求歷史 —— 自動記錄，一鍵回填重發",
      "cap.import": "多格式導入 —— Postman / OpenAPI (Swagger) / Markdown 一鍵導入",
      "cap.examples": "目錄即集合 —— 目錄結構即接口集合，天然支持 Git 管理",
      "cap.demo": "演示接口 —— 內置演示集合，開箱即用體驗全部功能",
      "cap.version": "版本對比 —— 多版本保存、差異對比、隨時回退",
      "cap.stat": "統計 —— 接口數 / Mock 啟用 / 請求方法分佈一覽",
      "cap.setting": "設置 —— 工作區、代碼生成、導出格式等個性化配置",
      "cta.title": "現在就試試 API Manager",
      "cta.desc": "輕量 · 高效 · 開源 —— API 開發調試從未如此簡單",
      "footer.text": "基於 Tauri 2 的 API 文檔、測試與 Mock 工具 · 開源項目",
      "footer.releases": "發佈版本",
      "carousel.aria": "第 {n} 張",
    },
    en: {
      "nav.features": "Features",
      "nav.structure": "Structure",
      "nav.stack": "Tech Stack",
      "nav.download": "Download",
      "hero.sub": "API Docs · Testing · Mock — one tool for everything",
      "hero.desc": "A Tauri 2 desktop app where directories are collections and each API is a single JSON file — Git-friendly by nature.",
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
      "structure.note": "Mock template variables: {{path.id}} path param, {{query.page}} query param, {{method}}, {{path}}, {{variable}} global env var.",
      "stack.title": "Tech Stack",
      "stack.react.desc": "Frontend framework",
      "stack.ts.desc": "Type-safe JavaScript",
      "stack.vite.desc": "Frontend build tool",
      "stack.backend": "Backend · Axum Mock server + reqwest",
      "stack.just.desc": "Command runner",
      "cap.main": "Main UI — Postman-style layout: API tree, request editor, response panel",
      "cap.start": "Start Page — recent workspaces, sample workspace & quick actions",
      "cap.mock": "Mock Server — one-click local Mock with path params, delay & template variables",
      "cap.export": "One-click Export — Postman Collection / OpenAPI / Docsify formats",
      "cap.codegen": "Code Generation — request code for 20+ languages & frameworks",
      "cap.env": "Environment Variables — switch environments, {{variable}} resolved everywhere",
      "cap.history": "Request History — auto-recorded, re-run with one click",
      "cap.import": "Multi-format Import — Postman / OpenAPI (Swagger) / Markdown",
      "cap.examples": "Directories as Collections — your folder structure is the collection, Git-friendly",
      "cap.demo": "Demo APIs — built-in demo collection to try every feature",
      "cap.version": "Version Diff — save versions, diff & roll back anytime",
      "cap.stat": "Statistics — API counts, Mock usage & method distribution",
      "cap.setting": "Settings — workspace, codegen & export preferences",
      "cta.title": "Try API Manager Now",
      "cta.desc": "Lightweight · Efficient · Open source — API development and debugging made simple",
      "footer.text": "A Tauri 2 based API documentation, testing & Mock tool · Open source",
      "footer.releases": "Releases",
      "carousel.aria": "Slide {n}",
    },
  };
  const KEY = "api-manager-site-lang";
  // 官网默认英语；用户手动切换后记住选择
  let lang = localStorage.getItem(KEY) || "en";

  function apply(l) {
    lang = l;
    document.documentElement.lang = l;
    const dict = I18N[l] || I18N.zh;
    document.querySelectorAll("[data-i18n]").forEach((el) => {
      const key = el.dataset.i18n;
      if (dict[key] !== undefined) el.textContent = dict[key];
    });
    // 截图按语言切换：cn 简体 / tc 繁体 / en 英文
    const LANG_DIR = { zh: "cn", tw: "tc", en: "en" };
    document.querySelectorAll("[data-shot]").forEach((img) => {
      img.src = "images/" + (LANG_DIR[l] || "en") + "/" + img.dataset.shot + ".png";
    });
    document.title =
      l === "zh"
        ? "API Manager - API 文档 · 测试 · Mock"
        : l === "tw"
          ? "API Manager - API 文檔 · 測試 · Mock"
          : "API Manager - API Docs · Testing · Mock";
    const sel = document.getElementById("langSelect");
    if (sel) sel.value = l;
    document.querySelectorAll(".carousel-dot").forEach((d, k) => {
      d.setAttribute(
        "aria-label",
        (dict["carousel.aria"] || "Slide {n}").replace("{n}", String(k + 1))
      );
    });
    localStorage.setItem(KEY, l);
  }

  const sel = document.getElementById("langSelect");
  if (sel) sel.addEventListener("change", () => apply(sel.value));

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
      dot.setAttribute("aria-label", "Slide " + (k + 1));
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
