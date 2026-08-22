/**
 * 独立加载页脚本（loading.html）：
 * 1. 全屏加载动画由 loading.html 的 CSS 提供（无 React 依赖，秒开）；
 * 2. 本脚本预取主应用（index.html）及其全部 JS/CSS 资源；
 * 3. 资源加载完成后跳转到主应用页面。
 */

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
