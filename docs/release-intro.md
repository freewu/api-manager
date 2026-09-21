# API Manager 应用功能介绍（应用发布用）

> 版本：v1.0.0 · 平台：Windows 10/11 · macOS · Linux · 语言：简体中文 / 繁體中文 / English

---

## 一、一句话简介（Slogan）

**API 文档 · 接口测试 · Mock 服务 —— 一个工具全搞定。**

A lightweight API workbench for documentation, testing & Mock — all in one.

---

## 二、简短描述（应用商店列表用，约 100 字）

**简体中文：**
基于 Tauri 2 的轻量 API 调试工具，目录即集合、一个接口一个 JSON 文件，天然支持 Git 版本管理；支持 HTTP / WebSocket / Socket.IO / GraphQL / WebDAV / MCP / TCP / UDP 多协议，内置一键 Mock 服务、代码生成（20+ 语言）、26 种导入 / 28 种导出格式、全局环境变量、全局快捷键与接口版本管理。开源免费，单文件分发，开箱即用。

**English:**
A lightweight Tauri 2 API workbench where directories are collections and every API is a single Git-friendly JSON file. Supports HTTP / WebSocket / Socket.IO / GraphQL / WebDAV / MCP / TCP / UDP, with a built-in one-click Mock server, code generation for 20+ languages, 26 import / 28 export formats, environment variables, global shortcuts and API versioning. Open source, free, single-file distribution.

---

## 三、核心功能详细介绍

### 🧪 接口测试
- 支持 GET / POST / PUT / DELETE / PATCH / HEAD / OPTIONS 等常用方法
- 完整支持**路径参数 / Query / Headers / Body**，Body 支持 raw / JSON / XML / 表单 / **二进制文件**五种模式
- JSON / XML **一键格式化**，响应查看状态码、耗时、大小、Headers 与 Body（JSON 语法高亮）
- **请求历史**自动记录，随时回看、一键回填重发；**请求示例**可保存请求 + 响应快照，一键回填或导出 `.http`
- **收藏**：接口右键即可收藏，收藏视图支持拖动排序
- **批量编辑**：Query / Headers / Body（表单）支持 `key: value` 每行一条批量录入

### 🔌 多协议接口
- 新建接口支持 **HTTP / WebSocket / Socket.IO / GraphQL / WebDAV / MCP / TCP / UDP** 八种协议，一套工作区统一管理
- **WebSocket / Socket.IO**：连接后实时收发消息，响应区查看帧 / 事件日志
- **GraphQL**：查询语句 + 变量编辑器
- **WebDAV**：额外提供 PROPFIND / PROPPATCH / MKCOL / COPY / MOVE / LOCK / UNLOCK / REPORT 方法
- **MCP**：以 HTTP 方式调用 MCP 端点
- **TCP / UDP**：报文编辑器自定义字段（名称 / 类型 / 长度 / 值）并内置报文解析

### 🎭 Mock 服务
- **一键启动本地 Mock 服务**（默认端口 5050），自动扫描所有启用 Mock 的接口
- 支持状态码、响应头、请求延迟、路径参数、模板变量与全局环境变量
- 无需后端联调，前端开发即刻起步

### 📁 目录即集合
- 打开应用选择一个工作目录，**目录结构即接口集合结构**，一个接口一个 JSON 文件
- 天然支持 **Git / SVN 版本管理**，标题栏一键同步 / 提交并 Push；接口定义、参数、Mock 数据全部可追溯、可协作
- 接口支持多版本保存、**左右对比差异**、随时回退到任意历史版本

### 🌐 全局环境变量
- 开发 / 测试 / 生产多环境一键切换，`{{变量名}}` 自动替换到 URL / Headers / Query / Body / Mock 响应
- 环境变量集支持拖动排序、右键编辑 / 复制 / 删除；每个变量含现有值、默认值与说明

### 💻 代码生成与导入导出
- **一键生成 20+ 种语言 / 框架的请求代码**（curl、JavaScript、Python、Java、Go、Rust 等）
- 一键导出 **28 种格式**：Postman Collection、**OpenAPI 3.0（JSON / YAML，默认 YAML）**、Apifox、Apipost、Apizza、Bruno、Insomnia、Hoppscotch、YApi、Eolink、NEI、Doclever、EasyDoc、Docway、io-docs、MeterSphere、Rap2、JMeter、apiDoc、Apidog、RAML、WADL、HAR，以及 Docsify / MkDocs / Markdown / HTML 文档站点
- 一键导入 **26 种格式**：Postman Collection、OpenAPI / Swagger（JSON 与 YAML）、Markdown、Apifox、Apipost、cURL 等，集合级变量自动迁移

### 📝 接口文档与描述
- 内置接口文档页，自动从请求配置与 Mock 响应体推导参数（类型、嵌套字段、说明）
- 支持手动补充修正，可导出 Markdown 文档；支持查看 apiDoc 注释
- **接口描述页签内置 Vditor Markdown 编辑器**：所见即所得 + 高亮只读预览，全部资源本地化打包，**完全离线可用**

### 📦 对象管理与数据生成
- 以分组 + 对象管理数据结构，属性支持类型 / 引用对象 / Mock 值 / 描述
- 支持从 JSON 或 SQL 建表语句导入，一键生成多语言代码与 MySQL 建表语句
- 按对象属性批量生成测试数据（JSON / SQL / CSV），生成记录展示耗时、文件大小并可一键重新生成

### ⚙️ 效率与体验
- **全局快捷键**：9 个可自定义快捷键（接口 / 对象 / 历史 / 生成记录 / 收藏视图切换、导入、导出、设置、环境管理），支持录制改键、冲突提示与一键恢复默认
- **设置中心**（12 个页签）：工作区、语言、外观、接口版本、Mock 服务、代码生成、导出、导入、默认 Header、快捷键、同步远程、关于
- **关于页签**：汇总应用依赖的全部开源组件（名称、版本、跳转链接）
- **统计**：分组 / 工作区维度统计接口数、Mock 启用数与请求方法分布
- 内置**演示工作区**（用户管理 / 订单管理），开箱即用体验全部功能

### 🖥️ 系统托盘与更新
- 关闭窗口最小化到托盘，托盘菜单快速显示 / 隐藏窗口、启停 Mock、切换显示模式与语言、**检查更新**
- 启动时自动检查 GitHub Releases，发现新版本弹窗提醒并可一键跳转下载
- 三语界面（简体中文 / 繁體中文 / English）随时切换；深色 / 浅色 / 跟随系统主题
- 轻量原生（Tauri 2 + Rust），**单文件分发，无需安装**，拷贝即用

---

## 四、适用场景

| 场景 | 说明 |
| --- | --- |
| 后端接口开发 | 编写、调试、Mock 联调，无需等待前端 |
| 前端开发 | 一键 Mock + 代码生成，快速接入接口 |
| 接口文档维护 | 目录即文档，Git 可版本化、可评审 |
| 团队协作 | 工作区纳入 Git，接口定义随代码一起管理 |
| 接口迁移 | 26 种格式导入 / 28 种格式导出，Postman / Swagger / Markdown 一键互转 |
| 多协议调试 | HTTP、WebSocket、Socket.IO、GraphQL、WebDAV、MCP、TCP、UDP 一套工具覆盖 |

---

## 五、Release 发布说明模板（GitHub Releases）

```markdown
## API Manager v1.0.0

轻量级 API 文档 · 测试 · Mock 工作台（Tauri 2 / React / Rust）。

### ✨ 功能亮点
- 🧪 接口测试：GET/POST/PUT/DELETE/PATCH，JSON/XML 一键格式化，支持二进制文件上传
- 🔌 多协议：HTTP / WebSocket / Socket.IO / GraphQL / WebDAV / MCP / TCP / UDP
- 🎭 一键本地 Mock 服务：路径参数、延迟、模板变量、环境变量
- 📁 目录即集合：一个接口一个 JSON 文件，天然 Git 版本管理
- 🌐 多环境变量：{{变量名}} 自动替换到请求各处
- 💻 代码生成 20+ 语言 / 框架，28 种导出格式（OpenAPI 3.0 JSON / YAML 等）
- 📥 多格式导入：Postman / Swagger（JSON / YAML）/ Markdown 等 26 种格式
- ✍️ Vditor Markdown 接口描述编辑器（资源本地化，离线可用）
- ⌨️ 9 个可自定义全局快捷键
- 📦 对象管理与数据生成（JSON / SQL / CSV）
- 📊 接口统计、请求历史、版本对比回退
- 🖥️ 系统托盘 + 新版本自动检查提醒
- 🌍 三语界面：简体中文 / 繁體中文 / English

### 📦 下载
- 单文件版（免安装，拷走即用）：release/api-manager.exe
- NSIS 安装包 / MSI / DMG / AppImage / deb / rpm：见 Assets

### 🖼️ 截图
见 docs/images/{cn,en,tc}/
```

---

## 六、English Release Notes (GitHub Releases)

```markdown
## API Manager v1.0.0

A lightweight API documentation · testing · Mock workbench (Tauri 2 / React / Rust).

### ✨ Highlights
- 🧪 Request testing: GET/POST/PUT/DELETE/PATCH, one-click JSON/XML formatting, binary file upload
- 🔌 Multi-protocol: HTTP / WebSocket / Socket.IO / GraphQL / WebDAV / MCP / TCP / UDP
- 🎭 One-click local Mock server: path params, delay, template & environment variables
- 📁 Directories as collections: one API per JSON file, Git-friendly by nature
- 🌐 Multiple environments: {{variable}} resolved across URL / Headers / Query / Body / Mock
- 💻 Code generation for 20+ languages; 28 export formats (incl. OpenAPI 3.0 JSON / YAML)
- 📥 Import from 26 formats: Postman / Swagger (JSON & YAML) / Markdown and more
- ✍️ Vditor Markdown API description editor (assets bundled locally, fully offline)
- ⌨️ 9 customizable global shortcuts
- 📦 Object manager & data generation (JSON / SQL / CSV)
- 📊 Statistics, request history, version diff & rollback
- 🖥️ System tray + automatic update check
- 🌍 Trilingual UI: 简体中文 / 繁體中文 / English

### 📦 Downloads
- Standalone (no install, copy & run): release/api-manager.exe
- NSIS / MSI / DMG / AppImage / deb / rpm: see Assets

### 🖼️ Screenshots
See docs/images/{cn,en,tc}/
```
