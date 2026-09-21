/**
 * 关于页「开源组件」数据：列出项目实际使用的依赖（名称 / 版本 / 跳转链接）。
 * 前端取自 package.json，后端取自 src-tauri/Cargo.toml，版本号为清单中声明的版本，
 * 新增依赖后无需改动本文件即可自动出现在列表中。
 */
import pkg from "../../../package.json";
import cargoToml from "../../../src-tauri/Cargo.toml?raw";

export interface ProjectDep {
  /** 组件名（npm 包名 / crate 名） */
  name: string;
  /** 声明版本 */
  version: string;
  /** 跳转链接（npm / crates.io 包页） */
  url: string;
}

export interface DepGroup {
  /** 分组标题 i18n key */
  titleKey: string;
  /** 分组徽章颜色（shields.io 右侧背景色，十六进制不带 #） */
  color: string;
  /** 分组兼置图标（组件名无精确图标时使用） */
  logo?: string;
  deps: ProjectDep[];
}

/** 去掉 npm 版本范围前缀（^ / ~ / >= 等），只保留版本数字 */
function cleanVersion(range: string): string {
  return range.replace(/^[\s^~>=<v]+/, "").trim();
}

/** 前端依赖：package.json 的 dependencies + devDependencies（跳过私有/本地依赖） */
function frontendDeps(): ProjectDep[] {
  const all: Record<string, string> = {
    ...(pkg.dependencies ?? {}),
    ...(pkg.devDependencies ?? {}),
  };
  return Object.entries(all)
    .filter(([, range]) => typeof range === "string")
    .map(([name, range]) => ({
      name,
      version: cleanVersion(range),
      url: `https://www.npmjs.com/package/${name}`,
    }));
}

/** 后端依赖：解析 Cargo.toml 的 [dependencies] / [build-dependencies] / [dev-dependencies] */
function rustDeps(): ProjectDep[] {
  const out: ProjectDep[] = [];
  let inDeps = false;
  for (const raw of cargoToml.split(/\r?\n/)) {
    const line = raw.trim();
    if (line.startsWith("[")) {
      inDeps = /^\[(dependencies|build-dependencies|dev-dependencies)\]$/.test(line);
      continue;
    }
    if (!inDeps || !line || line.startsWith("#")) continue;
    const m = /^([A-Za-z0-9_-]+)\s*=\s*(.+)$/.exec(line);
    if (!m) continue;
    const [, name, value] = m;
    // 形如 serde = "1" 或 serde = { version = "1", features = [...] }
    const version = value.trim().startsWith("{")
      ? /version\s*=\s*"([^"]+)"/.exec(value)?.[1]
      : value.replace(/["']/g, "").trim();
    if (!version) continue; // 没有声明版本的依赖（如 path 依赖）不展示
    out.push({ name, version, url: `https://crates.io/crates/${name}` });
  }
  return out;
}

/** 开源组件分组：前端（npm）+ 后端（Cargo） */
export const DEP_GROUPS: DepGroup[] = [
  { titleKey: "settings.aboutDepsFrontend", color: "2E59A7", deps: frontendDeps() },
  { titleKey: "settings.aboutDepsRust", color: "B7410E", logo: "rust", deps: rustDeps() },
];
