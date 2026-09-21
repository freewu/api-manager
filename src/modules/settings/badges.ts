/**
 * 关于页「技术栈徽章」：把项目实际使用的技术栈渲染成 shields.io 徽章。
 * 徽章地址为静态徽章（/badge/标签-版本-颜色），由 shields.io 动态生成；
 * 离线或网络不可用时由组件自动降级为文字标签，不影响功能。
 */
import type { DepGroup } from "./deps";

export interface StackBadge {
  /** 徽章左半部分文字（组件名 / 技术名） */
  label: string;
  /** 徽章右半部分文字（版本） */
  version: string;
  /** 右半部分背景色（十六进制，不带 #） */
  color: string;
  /** simple-icons 图标 slug（可选，取不到图标时自动省略） */
  logo?: string;
  /** 图标颜色，默认白色 */
  logoColor?: string;
  /** 点击跳转地址 */
  url: string;
}

/**
 * shields.io 静态徽章转义规则：
 * `-` → `--`、`_` → `__`、空格 → `_`，其余字符按 URL 编码（`+` → %2B、`/` → %2F）。
 */
function esc(text: string): string {
  return encodeURIComponent(text.replace(/-/g, "--").replace(/_/g, "__").replace(/ /g, "_"));
}

/** 生成 shields.io 静态徽章地址 */
export function badgeUrl(badge: StackBadge): string {
  const params = new URLSearchParams({ style: "flat-square" });
  if (badge.logo) {
    params.set("logo", badge.logo);
    params.set("logoColor", badge.logoColor ?? "white");
  }
  return `https://img.shields.io/badge/${esc(badge.label)}-${esc(badge.version)}-${badge.color}?${params.toString()}`;
}

/** 组件名 → simple-icons 图标（只保留 shields.io 能取到的图标，取不到就不带图标） */
const LOGOS: Record<string, string> = {
  tauri: "tauri",
  "tauri-plugin-dialog": "tauri",
  "tauri-plugin-opener": "tauri",
  "@tauri-apps/api": "tauri",
  "@tauri-apps/cli": "tauri",
  "@tauri-apps/plugin-dialog": "tauri",
  react: "react",
  "react-dom": "react",
  "@types/react": "react",
  "@types/react-dom": "react",
  "@vitejs/plugin-react": "vite",
  vite: "vite",
  typescript: "typescript",
  "socket.io-client": "socketdotio",
  serde_json: "json",
  serde_yaml: "yaml",
  tokio: "tokio",
};

/** 取组件对应的图标：优先精确匹配，其次用分组兜底图标（如 Rust 生态统一用 rust） */
export function logoFor(name: string, fallback?: string): string | undefined {
  return LOGOS[name] ?? fallback;
}

/** 依赖分组 → 技术栈徽章列表 */
export function groupBadges(group: DepGroup): StackBadge[] {
  return group.deps.map((dep) => ({
    label: dep.name,
    version: dep.version,
    color: group.color,
    logo: logoFor(dep.name, group.logo),
    url: dep.url,
  }));
}

/** 核心栈徽章：项目技术选型概览（列表中的依赖仍会在下方分组中完整列出） */
export const CORE_STACK: StackBadge[] = [
  { label: "Tauri", version: "2", color: "24C8DB", logo: "tauri", url: "https://tauri.app" },
  { label: "Rust", version: "1.77+", color: "B7410E", logo: "rust", url: "https://www.rust-lang.org" },
  { label: "React", version: "18", color: "61DAFB", logo: "react", logoColor: "black", url: "https://react.dev" },
  { label: "TypeScript", version: "5", color: "3178C6", logo: "typescript", url: "https://www.typescriptlang.org" },
  { label: "Vite", version: "5", color: "646CFF", logo: "vite", url: "https://vitejs.dev" },
  { label: "Vditor", version: "4", color: "4285F4", url: "https://b3log.org/vditor/" },
  { label: "highlight.js", version: "11", color: "1E293B", url: "https://highlightjs.org" },
  { label: "Socket.IO", version: "4", color: "010101", logo: "socketdotio", url: "https://socket.io" },
  { label: "axum", version: "0.7", color: "B7410E", logo: "rust", url: "https://github.com/tokio-rs/axum" },
  { label: "reqwest", version: "0.12", color: "B7410E", logo: "rust", url: "https://github.com/seanmonstar/reqwest" },
  { label: "Tokio", version: "1", color: "B7410E", logo: "tokio", url: "https://tokio.rs" },
  { label: "serde", version: "1", color: "B7410E", logo: "rust", url: "https://serde.rs" },
];
