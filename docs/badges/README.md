# 本地徽章（`docs/badges`）

README（[英文](../README.md) / [简体中文](../README.cn.md) / [繁體中文](../README.tc.md)）和 GitHub Pages 首页 [docs/index.html](../index.html) 里的徽章都是**本地静态 SVG**：图标与文字全部内联在文件里，不请求任何网络资源，因此断网、离线打开页面或离线预览 README 时都能正常显示。

这些文件是 [shields.io](https://shields.io) flat-square 静态徽章（`/badge/标签-内容-颜色`）的快照，样式与线上完全一致。

| 文件 | 内容 | 来源（`https://img.shields.io/` 之后的路径与参数） |
| --- | --- | --- |
| `tauri-2.svg` | Tauri: 2 | `badge/Tauri-2-24C8DB?style=flat-square&logo=tauri&logoColor=white` |
| `rust-1.77.svg` | Rust: 1.77+ | `badge/Rust-1.77%2B-000000?style=flat-square&logo=rust&logoColor=white` |
| `react-18.svg` | React: 18 | `badge/React-18-61DAFB?style=flat-square&logo=react&logoColor=black` |
| `typescript-5.svg` | TypeScript: 5 | `badge/TypeScript-5-3178C6?style=flat-square&logo=typescript&logoColor=white` |
| `vite-5.svg` | Vite: 5 | `badge/Vite-5-646CFF?style=flat-square&logo=vite&logoColor=white` |
| `axum-0.7.svg` | Axum: 0.7 | `badge/Axum-0.7-000000?style=flat-square&logo=rust&logoColor=white` |
| `reqwest-0.12.svg` | reqwest: 0.12 | `badge/reqwest-0.12-000000?style=flat-square&logo=rust&logoColor=white` |
| `tokio-1.svg` | Tokio: 1 | `badge/Tokio-1-000000?style=flat-square&logo=rust&logoColor=white` |
| `tower-http-0.6.svg` | tower-http: 0.6 | `badge/tower--http-0.6-000000?style=flat-square&logo=rust&logoColor=white` |
| `serde-1.svg` | serde: 1 | `badge/serde-1-000000?style=flat-square&logo=rust&logoColor=white` |
| `serde_json-1.svg` | serde_json: 1 | `badge/serde__json-1-000000?style=flat-square&logo=rust&logoColor=white` |
| `serde_yaml-0.9.svg` | serde_yaml: 0.9 | `badge/serde__yaml-0.9-000000?style=flat-square&logo=rust&logoColor=white` |
| `roxmltree-0.20.svg` | roxmltree: 0.20 | `badge/roxmltree-0.20-000000?style=flat-square&logo=rust&logoColor=white` |
| `socket.io-client-4.svg` | socket.io-client: 4 | `badge/socket.io--client-4-010101?style=flat-square&logo=socketdotio&logoColor=white` |
| `vditor-4.svg` | Vditor: 4 | `badge/Vditor-4-4285F4?style=flat-square` |
| `highlight.js-11.svg` | highlight.js: 11 | `badge/highlight.js-11-1E293B?style=flat-square` |
| `just-command-runner.svg` | just: command runner | `badge/just-command%20runner-7B68EE?style=flat-square` |
| `tauri-cli-2.svg` | tauri-cli: 2 | `badge/tauri--cli-2-24C8DB?style=flat-square&logo=tauri&logoColor=white` |
| `tauri-plugin-dialog-2.svg` | tauri-plugin-dialog: 2 | `badge/tauri--plugin--dialog-2-24C8DB?style=flat-square&logo=tauri&logoColor=white` |
| `tauri-plugin-opener-2.svg` | tauri-plugin-opener: 2 | `badge/tauri--plugin--opener-2-24C8DB?style=flat-square&logo=tauri&logoColor=white` |
| `platform-windows-macos-linux.svg` | platform: Windows | macOS | Linux | `badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-2E59A7?style=flat-square` |
| `license-mit.svg` | license: MIT | `badge/license-MIT-2E59A7?style=flat-square` |

## 更新方式

技术栈或版本变化时，重新抓取同名徽章覆盖即可（无网络时跳过更新，不影响显示）：

```bash
curl -sSfL -o docs/badges/tauri-2.svg \
  "https://img.shields.io/badge/Tauri-2-24C8DB?style=flat-square&logo=tauri&logoColor=white"
```

注意 shields.io 的转义规则：`--` 表示字面 `-`，`__` 表示字面 `_`，`_` 表示空格，`+` 需写成 `%2B`。

## 仍然是远程的动态徽章

- `Release` 版本号 —— 依赖 GitHub Release 实时数据
- `Downloads` 下载量 —— 依赖 GitHub Release 资产统计

这两个只能保持远程（例如 `https://img.shields.io/github/v/release/freewu/api-manager`）；离线时会显示为空白/裂图，不影响其它徽章与正文。

应用内的「设置 → 关于」技术栈徽章不读取这些文件，而是由前端 `src/modules/settings/BadgeSvg.tsx` 现场绘制（同样离线可用）。
