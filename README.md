# API Manager

使用 **Tauri 2** 开发的 API 接口文档、接口测试与 Mock 工具，界面布局参考 Postman。

## 功能特性

- 📁 **目录即集合**：打开应用时选择一个工作目录，目录结构即接口集合结构
- 📄 **一个接口一个 JSON 文件**：接口定义、请求参数、Mock 数据全部以 JSON 文件形式管理，天然支持 Git 版本管理
- 🧪 **接口测试**：发送 GET/POST/PUT/DELETE/PATCH 等请求，查看响应状态、耗时、大小、Headers、Body（带 JSON 语法高亮）
- 🎭 **Mock 服务**：一键启动本地 Mock 服务（默认端口 5050），自动扫描所有启用 Mock 的接口，支持路径参数、延迟、模板变量
- 🗂️ **Postman 风格布局**：左侧接口树（支持搜索、分组、增删改）、中间请求编辑器、下方响应面板
- 🌐 **路径参数 / Query / Headers / Body** 完整支持
- 🧩 接口支持 `{参数名}` 路径模板、模板变量 `{{path.id}}`、`{{query.page}}`

## 目录结构约定

```
工作目录/
├── __info.json                  # 根目录描述（集合信息）
│                                # 字段：name, description, baseUrl, mockPort
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

## 开发与构建

### 环境要求

- Node.js ≥ 18（本机位于 `D:\env\nodejs`）
- Rust（Windows MSVC 工具链，`C:\Users\24358\.cargo`）
- WebView2（Windows 10/11 自带）
- [just](https://github.com/casey/just)（命令运行器，本机已安装）

### 常用命令（just）

```bash
just dev         # 开发模式运行（前端热更新 + Rust dev）
just test        # 运行全部测试（Rust 单测 + 前端类型检查 + 前端构建）
just build       # 完整打包：exe + NSIS / MSI 安装程序
just exe         # 仅构建 release 可执行文件（快速）
just tsc         # 前端类型检查
just check       # Rust 编译检查
just icon        # 重新生成应用图标（群青主题）
just clean       # 清理构建产物
just push "信息" # 提交并推送到远程
just             # 列出全部命令
```

> 说明：WSL 环境下 `node` / `cargo` 由 `~/bin` 下的包装脚本指向 Windows 可执行文件，`just` 已自动处理 PATH。

### 构建产物

- `src-tauri/target/release/api-manager.exe` — 可执行文件
- `src-tauri/target/release/bundle/nsis/API Manager_0.1.0_x64-setup.exe` — 安装包
- `src-tauri/target/release/bundle/msi/API Manager_0.1.0_x64_en-US.msi` — MSI

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
