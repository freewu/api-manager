# API Manager 应用功能介绍（应用发布用）

> 版本：v0.1.6 · 平台：Windows 10/11（x64）· 语言：简体中文 / 繁體中文 / English

---

## 一、一句话简介（Slogan）

**API 文档 · 接口测试 · Mock 服务 —— 一个工具全搞定。**

A lightweight API workbench for documentation, testing & Mock — all in one.

---

## 二、简短描述（应用商店列表用，约 100 字）

**简体中文：**
基于 Tauri 2 的轻量 API 调试工具。目录即集合、一个接口一个 JSON 文件，天然支持 Git 版本管理；内置一键 Mock 服务、代码生成（20+ 语言）、多格式导入导出、全局环境变量与接口版本管理。开源免费，单文件分发，开箱即用。

**English:**
A lightweight Tauri 2 API workbench where directories are collections and every API is a single Git-friendly JSON file. Built-in one-click Mock server, code generation for 20+ languages, Postman/OpenAPI/Markdown import & export, environment variables, and API versioning. Open source, free, single-file distribution.

---

## 三、核心功能详细介绍

### 🧪 接口测试
- 支持 GET / POST / PUT / DELETE / PATCH 等常用方法
- 完整支持**路径参数 / Query / Headers / Body**，Body 支持 raw / JSON / XML / 表单 / **二进制文件**五种模式
- JSON / XML **一键格式化**，响应查看状态码、耗时、大小、Headers 与 Body（JSON 语法高亮）
- **请求历史**自动记录，随时回看、一键回填重发

### 🎭 Mock 服务
- **一键启动本地 Mock 服务**（默认端口 5050），自动扫描所有启用 Mock 的接口
- 支持路径参数、请求延迟、模板变量与全局环境变量
- 无需后端联调，前端开发即刻起步

### 📁 目录即集合
- 打开应用选择一个工作目录，**目录结构即接口集合结构**，一个接口一个 JSON 文件
- 天然支持 **Git 版本管理**，接口定义、参数、Mock 数据全部可追溯、可协作
- 接口支持多版本保存、**左右对比差异**、随时回退到任意历史版本

### 🌐 全局环境变量
- 开发 / 测试 / 生产多环境一键切换，`{{变量名}}` 自动替换到 URL / Headers / Query / Body / Mock 响应
- 环境变量集支持拖动排序、右键编辑/复制/删除；每个变量含现有值、默认值与说明

### 💻 代码生成与导出
- **一键生成 20+ 种语言 / 框架的请求代码**（curl、JavaScript、Python、Java、Go、Rust 等）
- 一键导出 Postman Collection / **OpenAPI 3.0** / Docsify 文档站点
- 一键导入 Postman Collection / OpenAPI (Swagger) / Markdown 接口文档，集合级变量自动迁移

### 📝 接口文档
- 内置接口文档页，自动从请求配置与 Mock 响应体推导参数（类型、嵌套字段、说明）
- 支持手动补充修正，可导出 Markdown 文档

### 📊 统计与演示
- 分组 / 工作区维度统计接口数、Mock 启用数与请求方法分布
- 内置**演示工作区**（用户管理 / 订单管理），开箱即用体验全部功能

### 🖥️ 系统托盘与更新
- 关闭窗口最小化到托盘，托盘菜单快速显示/隐藏窗口、启停 Mock、**切换语言**、**检查更新**
- 启动时自动检查 GitHub Releases，发现新版本弹窗提醒并可一键跳转下载

### 🗂️ 界面与体验
- Postman 风格布局：左侧接口树（搜索 / 分组 / 增删改 / 复制）、中间请求编辑器、下方响应面板
- 三语界面（简体中文 / 繁體中文 / English）随时切换
- 轻量原生（Tauri 2 + Rust），**单文件分发，无需安装**，拷贝即用

---

## 四、适用场景

| 场景 | 说明 |
| --- | --- |
| 后端接口开发 | 编写、调试、Mock 联调，无需等待前端 |
| 前端开发 | 一键 Mock + 代码生成，快速接入接口 |
| 接口文档维护 | 目录即文档，Git 可版本化、可评审 |
| 团队协作 | 工作区纳入 Git，接口定义随代码一起管理 |
| 接口迁移 | Postman / Swagger / Markdown 一键导入导出 |

---

## 五、Release 发布说明模板（GitHub Releases）

```markdown
## API Manager v0.1.6

轻量级 API 文档 · 测试 · Mock 工作台（Tauri 2 / React / Rust）。

### ✨ 功能亮点
- 🧪 接口测试：GET/POST/PUT/DELETE/PATCH，JSON/XML 一键格式化，支持二进制文件上传
- 🎭 一键本地 Mock 服务：路径参数、延迟、模板变量、环境变量
- 📁 目录即集合：一个接口一个 JSON 文件，天然 Git 版本管理
- 🌐 多环境变量：{{变量名}} 自动替换到请求各处
- 💻 代码生成 20+ 语言 / 框架，一键导出 Postman / OpenAPI / Docsify
- 📥 多格式导入：Postman / Swagger / Markdown
- 📝 内置接口文档页，可导出 Markdown
- 📊 接口统计、请求历史、版本对比回退
- 🖥️ 系统托盘 + 新版本自动检查提醒
- 🌍 三语界面：简体中文 / 繁體中文 / English

### 📦 下载
- 单文件版（免安装，拷走即用）：release/api-manager.exe
- NSIS 安装包 / MSI：见 Assets

### 🖼️ 截图
见 docs/images/{cn,en,tc}/
```

---

## 六、English Release Notes (GitHub Releases)

```markdown
## API Manager v0.1.6

A lightweight API documentation · testing · Mock workbench (Tauri 2 / React / Rust).

### ✨ Highlights
- 🧪 Request testing: GET/POST/PUT/DELETE/PATCH, one-click JSON/XML formatting, binary file upload
- 🎭 One-click local Mock server: path params, delay, template & environment variables
- 📁 Directories as collections: one API per JSON file, Git-friendly by nature
- 🌐 Multiple environments: {{variable}} resolved across URL / Headers / Query / Body / Mock
- 💻 Code generation for 20+ languages; export to Postman / OpenAPI / Docsify
- 📥 Import from Postman / Swagger / Markdown
- 📝 Built-in API docs page, exportable as Markdown
- 📊 Statistics, request history, version diff & rollback
- 🖥️ System tray + automatic update check
- 🌍 Trilingual UI: 简体中文 / 繁體中文 / English

### 📦 Downloads
- Standalone (no install, copy & run): release/api-manager.exe
- NSIS / MSI installers: see Assets

### 🖼️ Screenshots
See docs/images/{cn,en,tc}/
```
