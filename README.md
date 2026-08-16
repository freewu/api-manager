# API Manager

使用 **Tauri 2** 开发的 API 接口文档、接口测试与 Mock 工具，界面布局参考 Postman。

## 功能特性

- 📁 **目录即集合**：打开应用时选择一个工作目录，目录结构即接口集合结构
- 📄 **一个接口一个 JSON 文件**：接口定义、请求参数、Mock 数据全部以 JSON 文件形式管理，天然支持 Git 版本管理
- 🧪 **接口测试**：发送 GET/POST/PUT/DELETE/PATCH 等请求，查看响应状态、耗时、大小、Headers、Body（带 JSON 语法高亮）
- 🎭 **Mock 服务**：一键启动本地 Mock 服务（默认端口 5050），自动扫描所有启用 Mock 的接口，支持路径参数、延迟、模板变量、全局环境变量
- 📝 **接口文档**：内置接口文档页，自动从请求配置与 Mock 响应体推导参数（类型、嵌套字段、说明），支持手动补充与修正；可导出 Markdown 文档
- 📤 **一键导出**：支持 Postman Collection / OpenAPI 3.0 / Docsify 文档站点三种格式导出
- 💻 **代码生成**：一键生成 20+ 种语言 / 框架的请求代码（curl、JavaScript、Python、Java、Go、Rust 等）
- 🌐 **全局环境变量**：多环境配置（开发/测试/生产）一键切换，请求时 `{{变量名}}` 自动替换到 URL / Headers / Query / Body / Mock 响应体
- 🖥️ **系统托盘**：关闭窗口最小化到托盘，托盘菜单可显示/隐藏窗口、快速启停 Mock、退出
- 🗂️ **Postman 风格布局**：左侧接口树（支持搜索、分组、增删改、复制）、中间请求编辑器、下方响应面板
- 🌐 **路径参数 / Query / Headers / Body** 完整支持
- 🧩 接口支持 `{参数名}` 路径模板、模板变量 `{{path.id}}`、`{{query.page}}`
- 📥 **Postman 导入**：一键导入 Postman Collection 全部接口，集合级 `variable` 自动合并到环境变量集

## 目录结构约定

```
工作目录/
├── __info.json                  # 根目录描述（集合信息）
│                                # 字段：name, description, baseUrl, mockPort
├── __envs.json                  # 全局环境变量（可选）
│                                # 字段：active, environments[{name, variables[]}]
├── 用户管理/                    # 分组 = 目录
│   ├── __info.json              # 分组描述：name, description, order
│   ├── 获取用户信息.json         # 一个接口 = 一个 JSON 文件
│   └── 创建用户.json
└── 订单管理/
    ├── __info.json
    └── 获取订单列表.json
```

### 接口 JSON 文件格式

```json
{
  "name": "获取用户信息",
  "method": "GET",
  "path": "/api/users/{id}",
  "url": "",
  "description": "根据用户 ID 获取用户信息",
  "headers": [{ "key": "Authorization", "value": "Bearer xxx", "enabled": true, "description": "" }],
  "query": [{ "key": "page", "value": "1", "enabled": true, "description": "" }],
  "params": [{ "key": "id", "value": "1001", "enabled": true, "description": "用户 ID" }],
  "body": { "mode": "json", "raw": "{ ... }", "form": [] },
  "mock": {
    "enabled": true,
    "status": 200,
    "headers": [],
    "delay": 200,
    "body": "{ \"code\": 0, \"data\": { \"id\": \"{{path.id}}\" } }"
  },
  "examples": []
}
```

> `url` 为空时，请求地址 = 根 `__info.json` 的 `baseUrl` + `path`；
> 填写 `url` 则优先使用完整地址。

### Mock 模板变量

- `{{path.id}}` — 路径参数（对应 path 中的 `{id}`）
- `{{query.page}}` — Query 参数
- `{{method}}` — 请求方法
- `{{path}}` — 完整路径
- `{{变量名}}` — 全局环境变量（来自激活环境，`path`/`method`/`path.*`/`query.*` 保留给系统变量）

### 全局环境变量

工具栏的**环境**下拉框可快速切换当前环境；点击 🌐 进入环境变量管理，分**两个弹出框**：

**① 环境变量集管理**：新增整套变量配置，支持**拖动排序**；**右键**集名称弹出菜单可 编辑（重命名）/ 复制 / 删除，多套配置（如 开发 / 测试 / 生产）并存；
**② 环境变量值管理**：选中具体的环境变量集后，点击「✏ 管理变量值」打开，维护该集合内的变量（新增 / 编辑 / 删除）。每个变量包含**现有值**、**默认值**（现值为空时自动使用）和**描述说明**。

- 请求发送时，URL / Headers / Query / Body 中的 `{{变量名}}` 会被替换为激活环境的值
- Mock 响应体同样支持 `{{变量名}}`（启动/刷新 Mock 后生效）
- 配置保存在工作区根目录 `__envs.json`，可纳入 Git 管理

示例：`__info.json` 的 `baseUrl` 可设为 `{{baseUrl}}`，在不同环境间切换时请求目标随之变化。

> **Postman 导入**：导入 Collection 时，文件顶层的 `variable` 数组会按 key 合并到同名环境变量集（不存在则新建，命名同集合名；无激活环境时自动激活），无需手动逐个录入。

### 系统托盘

- 点击窗口关闭按钮：隐藏到托盘（应用继续运行）
- 托盘图标左键单击：显示窗口；右键：菜单
- 托盘菜单：显示窗口 / 隐藏窗口 / 启动·停止 Mock 服务 / 退出
- 托盘菜单中的 Mock 项文字会随 Mock 服务状态自动更新

## 开发与构建

### 环境要求

- Node.js ≥ 18
- Rust（Windows MSVC 工具链）
- WebView2（Windows 10/11 自带）
- [just](https://github.com/casey/just)（命令运行器）

### 常用命令（just）

```bash
just dev         # 开发模式运行（前端热更新 + Rust dev）
just test        # 运行全部测试（Rust 单测 + 前端类型检查 + 前端构建）
just build       # 完整打包：exe + NSIS / MSI 安装程序（可选）
just release     # 仅生成单体可执行文件并收集到 ./release/（无需安装，拷走即用）
just exe         # 仅构建 release 可执行文件（快速，release 的构建部分）
just tsc         # 前端类型检查
just check       # Rust 编译检查
just icon        # 重新生成应用图标（群青主题）
just clean       # 清理构建产物
just push "信息" # 提交并推送到远程
just             # 列出全部命令
```

> justfile 同时支持 Windows（cmd）与 WSL（bash）环境；Windows 命令行下中文参数乱码时，可先执行 `chcp 65001` 切换 UTF-8 代码页。

### 构建产物

- `just release` 产物：`release/api-manager.exe` — 单体可执行文件，无需安装，拷到任意 Windows 机器直接运行（Win10/11 自带 WebView2 运行时）
- `just build`（可选）产物：`src-tauri/target/release/bundle/nsis/API Manager_0.1.5_x64-setup.exe` — NSIS 安装包；`bundle/msi/API Manager_0.1.5_x64_en-US.msi` — MSI

### 运行测试

```bash
just test
```

### 示例工作区

`examples/demo-workspace/` 提供了一个完整示例，包含用户管理、订单管理两个分组。
启动应用后选择该目录即可体验全部功能（内置 Mock 已启用）。

## 技术栈

- **前端**：React 18 + TypeScript + Vite 5
- **后端**：Tauri 2（Rust），Axum 提供 Mock 服务，reqwest 发送测试请求
- **主题色**：群青 `#2E59A7`
- **插件**：`tauri-plugin-dialog`（目录选择）
