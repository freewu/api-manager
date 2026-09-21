# API Manager

**English** · [简体中文](README.cn.md) · [繁體中文](README.tc.md)

[![Release](https://img.shields.io/github/v/release/freewu/api-manager?style=flat-square&color=2E59A7&label=release&sort=semver)](https://github.com/freewu/api-manager/releases)
[![License](https://img.shields.io/github/license/freewu/api-manager?style=flat-square&color=2E59A7)](LICENSE)
[![Downloads](https://img.shields.io/github/downloads/freewu/api-manager/total?style=flat-square&color=2E59A7)](https://github.com/freewu/api-manager/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-2E59A7?style=flat-square)](#download)

[![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?style=flat-square&logo=tauri&logoColor=white)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-1.77%2B-000000?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![React](https://img.shields.io/badge/React-18-61DAFB?style=flat-square&logo=react&logoColor=black)](https://react.dev)
[![TypeScript](https://img.shields.io/badge/TypeScript-5-3178C6?style=flat-square&logo=typescript&logoColor=white)](https://www.typescriptlang.org)
[![Vite](https://img.shields.io/badge/Vite-5-646CFF?style=flat-square&logo=vite&logoColor=white)](https://vitejs.dev)

**API Manager** is an API documentation, testing, and Mock workbench built with **Tauri 2** (Rust + React) and a Postman-inspired layout. **Directories are collections and every API is a single JSON file**, so your interface definitions can be reviewed, versioned and shared with Git right next to your code.

> 🌍 Trilingual UI (简体中文 / 繁體中文 / English) · 🖥️ Windows / macOS / Linux · 📦 Open source (MIT)

## Screenshots

| | |
| --- | --- |
| ![Start page](docs/images/en/start.png) | ![Main UI](docs/images/en/main.png) |
| ![Mock server](docs/images/en/mock.png) | ![One-click export](docs/images/en/export.png) |
| ![Code generation](docs/images/en/code-generator.png) | ![Environment variables](docs/images/en/global-env.png) |
| ![Request history](docs/images/en/history.png) | ![Multi-format import](docs/images/en/import.png) |
| ![Directories as collections](docs/images/en/examples.png) | ![Demo APIs](docs/images/en/demo.png) |
| ![Version diff](docs/images/en/version-diff.png) | ![Statistics](docs/images/en/stat.png) |
| ![Object manager](docs/images/en/object-manager.png) | ![New API](docs/images/en/api-add.png) |
| ![Data generator](docs/images/en/data-generator.png) | ![Generation logs](docs/images/en/data-generate-log.png) |
| ![Settings](docs/images/en/setting.png) | |

## Features

### 🗂️ Workspace & Collections

- 📁 **Directories as collections** — pick a working directory when launching the app; the folder tree *is* your API collection, one group per directory
- 📄 **One API per JSON file** — API definition, request parameters, Mock data and examples are all plain JSON files, Git-friendly by nature
- 🕘 **Versioning** — save multiple snapshots per API, diff two versions side by side, and roll back to any historical version
- 🔀 **VCS aware** — detects `.git` / `.svn` in the workspace and offers one-click **sync** / **commit & push** from the title bar (remotes are configured in *Settings → Remote Sync*)
- 🔍 **Search & advanced search** — find APIs by name or path, or narrow down by protocol type and HTTP method
- ⭐ **Favorites** — right-click an API → *Favorite* to collect it in the Favorites view (drag-and-drop ordering, dedicated editor pane)

### 🧪 Request Testing

- 🧪 **Send requests** — GET / POST / PUT / DELETE / PATCH / HEAD / OPTIONS; inspect status code, latency, size, headers and body, with JSON syntax highlighting and one-click JSON / XML formatting
- 🌐 **path params / Query / Headers / Body** fully supported (Body modes: raw / JSON / XML / form / **binary file**)
- 🧩 **Template variables** — `{param}` path templates plus `{{path.id}}`, `{{query.page}}`, `{{method}}`, `{{path}}` and `{{variable}}` globals, resolved in URL / Headers / Query / Body / Mock response
- 🕘 **Request history** — every request is recorded automatically; re-run it or refill it into the editor with one click
- 📋 **Request examples** — save a request + response snapshot as a named example, apply it back onto the API, or export all examples of one API as a single `.http` file
- ✏️ **Batch edit** — Query / Headers / Body(form) tabs accept `key: value` lines, one per line, and preserve the enabled state of matching rows

### 🔌 Multi-protocol APIs

- 🔌 Create APIs over **HTTP / WebSocket / Socket.IO / GraphQL / WebDAV / MCP / TCP / UDP** — one workspace, one tree, one history for every protocol
- 🔁 **WebSocket / Socket.IO** — connect, send messages and watch the incoming frame/event log in real time
- 🧮 **GraphQL** — query document plus variables editor
- 🗂️ **WebDAV** — the WebDAV methods `PROPFIND` / `PROPPATCH` / `MKCOL` / `COPY` / `MOVE` / `LOCK` / `UNLOCK` / `REPORT` are available next to the standard HTTP methods
- 🤖 **MCP** — call an MCP endpoint over HTTP with the same request builder and response viewer
- 📦 **TCP / UDP** — packet editor with field definitions (name / type / length / value) and a built-in packet inspector

### 🎭 Mock Server

- 🎭 **One-click local Mock server** (default port 5050) — automatically scans every API with Mock enabled and serves it locally
- ⚙️ **Per-API Mock config** — status code, response headers, delay, and a template body
- 🧩 **Template support** — `{{path.id}}` / `{{query.page}}` / `{{method}}` / `{{path}}` and global `{{variable}}` placeholders are resolved per request
- 🚀 **No backend needed** — front-end work can start before the real API exists

### 📝 Documentation, Import & Export

- 📝 **API docs** — a built-in documentation view that derives parameters (types, nested fields, descriptions) from the request config and Mock body, supports manual overrides, and can be exported as Markdown
- ✍️ **Markdown description editor (Vditor)** — the API *Description* tab is a full Markdown editor with instant preview and a highlighted read mode; **all Vditor assets are bundled locally, so it works completely offline**
- 🏷️ **apiDoc comments** — inspect the apiDoc annotations of an API, and preview Markdown documents inside the workspace
- 📤 **One-click export (28 formats)** — Postman Collection, **OpenAPI 3.0 (JSON *and* YAML — YAML is the default format)**, Apifox, Apipost, Apizza, Bruno, Insomnia, Hoppscotch, YApi, Eolink, NEI, Doclever, EasyDoc, Docway, io-docs, MeterSphere, Rap2, JMeter, apiDoc, Apidog, RAML, WADL, HAR, and documentation sites (Docsify / MkDocs / Markdown / HTML)
- 📥 **Multi-format import (26 formats)** — Postman Collection, OpenAPI / Swagger (JSON & YAML), Markdown, Apifox, Apipost, cURL and many more; which entries appear in the import menu is configurable in *Settings → Import*
- 🔄 **Collection variables migrate automatically** — importing a Postman Collection merges its top-level `variable` array into an environment set with the same name

### 📦 Object Manager & Data Generation

- 📦 **Object manager** — manage data structures as groups + objects; properties support type, referenced object, Mock value and description; import from JSON or SQL `CREATE TABLE`; generate code in multiple languages and MySQL DDL in one click
- 🎲 **Data generation** — batch-generate test data from an object's properties (JSON / SQL / CSV) with a custom table name, record count and export directory
- 🧾 **Generation logs** — every run records elapsed time and file size, and can be re-generated with one click
- 🧬 **Code generation** — request code for 20+ languages / frameworks (curl, JavaScript, Python, Java, Go, Rust, …)

### ⚙️ Productivity & Appearance

- 🌐 **Global environment variables** — switch dev / test / prod with one click; `{{variable}}` placeholders are replaced in URL / Headers / Query / Body / Mock responses (see [Global Environment Variables](#global-environment-variables))
- ⌨️ **Global shortcuts** — 9 fully rebindable shortcuts for view switching, import/export, settings and environments (see [Keyboard Shortcuts](#keyboard-shortcuts)); record a new key combination in *Settings → Shortcuts*
- 🗂️ **Postman-style layout** — API tree on the left (search, groups, CRUD, duplicate), request editor in the middle, response panel below, all panels resizable
- 📊 **Statistics** — API counts, Mock usage and request-method distribution per group / workspace
- 🖥️ **System tray** — closing the window minimizes to tray; the tray menu can show/hide the window, start/stop Mock, switch the display mode and language, check for updates, or quit
- 🔔 **Update check** — checks GitHub Releases on startup and offers a one-click jump to the download page
- ℹ️ **About tab** — *Settings → About* renders the whole tech stack (Tauri / Rust / React / TypeScript / Vite …) as shields.io badges plus the full open-source component list (name, version, jump link); badges degrade to text labels when offline
- 🎨 **Themes** — dark / light / follow system, applied instantly
- 🌍 **Trilingual UI** — 简体中文 / 繁體中文 / English, switchable at any time

## Download

All packages are built by GitHub Actions for every release and published on the [Releases](https://github.com/freewu/api-manager/releases) page.

| Platform | Packages |
| --- | --- |
| Windows | `API.Manager_1.0.0_x64-setup.exe` (NSIS) · `API.Manager_1.0.0_x64_en-US.msi` · `API.Manager_1.0.0_x64_portable.exe` (standalone, no install) |
| macOS | `API.Manager_1.0.0_x64.dmg` (Intel) · `API.Manager_1.0.0_aarch64.dmg` (Apple Silicon) |
| Linux | `API.Manager_1.0.0_amd64.AppImage` · `API.Manager_1.0.0_amd64.deb` · `API.Manager-1.0.0-1.x86_64.rpm` |

> On Windows you can also build a single portable `api-manager.exe` yourself — see [Development & Build](#development--build).

## Keyboard Shortcuts

Defaults (all of them can be rebound in *Settings → Shortcuts*; on macOS use <kbd>Cmd</kbd> instead of <kbd>Ctrl</kbd>):

| Shortcut | Action |
| --- | --- |
| <kbd>Ctrl</kbd>+<kbd>A</kbd> | Switch to the **API management** view |
| <kbd>Ctrl</kbd>+<kbd>O</kbd> | Switch to the **Object management** view |
| <kbd>Ctrl</kbd>+<kbd>H</kbd> | Switch to the **Request history** view |
| <kbd>Ctrl</kbd>+<kbd>G</kbd> | Switch to the **Data generation logs** view |
| <kbd>Ctrl</kbd>+<kbd>F</kbd> | Switch to the **Favorites** view |
| <kbd>Ctrl</kbd>+<kbd>E</kbd> | Open the **Export** dialog (switches to the API view first) |
| <kbd>Ctrl</kbd>+<kbd>I</kbd> | Open the **Import** menu (switches to the API view first) |
| <kbd>Ctrl</kbd>+<kbd>S</kbd> | Open the **Settings** dialog (press again to close) |
| <kbd>Ctrl</kbd>+<kbd>M</kbd> | Open the **Environment management** dialog (press again to close) |

- Shortcuts are ignored while typing in an input / textarea / contenteditable element, and never switch the view behind an open dialog
- Recording a new shortcut: open *Settings → Shortcuts*, click the key button of a row and press the combination; <kbd>Esc</kbd> cancels, <kbd>Backspace</kbd> / <kbd>Delete</kbd> clears the binding, `↺` restores the default of one row, and **Restore all defaults** resets everything
- Conflicting bindings are flagged with a ⚠ badge; the first matching action wins

## Directory Conventions

```
workspace/
├── __info.json                  # root description (collection info)
│                                # fields: name, description, baseUrl, mockPort
├── __envs.json                  # global environment variables (optional)
│                                # fields: active, environments[{name, variables[]}]
├── user-management/             # group = directory
│   ├── __info.json              # group description: name, description, order
│   ├── get-user.json            # one API = one JSON file
│   └── create-user.json
└── order-management/
    ├── __info.json
    └── list-orders.json
```

### API JSON File Format

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

> When `url` is empty, the request URL = root `__info.json` `baseUrl` + `path`;
> when `url` is set, it takes priority.

### Mock Template Variables

- `{{path.id}}` — path parameter (matches `{id}` in path)
- `{{query.page}}` — Query parameter
- `{{method}}` — request method
- `{{path}}` — full path
- `{{variable}}` — global environment variable (from the active environment; `path` / `method` / `path.*` / `query.*` are reserved for system variables)

### Global Environment Variables

The **environment** dropdown in the toolbar switches the active environment; click 🌐 to open environment management, split into **two dialogs**:

**① Environment set management**: create full variable sets, reorder by **drag-and-drop**; **right-click** a set name to edit (rename) / duplicate / delete — multiple sets (e.g. dev / test / prod) can coexist.
**② Variable value management**: select a set, click "✏ 管理变量值" to add / edit / delete variables within it. Each variable has a **current value**, a **default value** (used automatically when the current value is empty), and a **description**.

- When sending requests, `{{variable}}` in URL / Headers / Query / Body is replaced with the active environment's value
- Mock response bodies support `{{variable}}` too (applied after starting/refreshing the Mock server)
- Configuration is stored in `__envs.json` at the workspace root and can be version-controlled with Git

Example: set `baseUrl` in `__info.json` to `{{baseUrl}}` and the request target changes when you switch environments.

> **Postman import**: when importing a Collection, the top-level `variable` array is merged by key into the environment set with the same name (created if missing, named after the collection; auto-activated if no environment is active) — no manual entry needed.

## Settings at a Glance

*Settings* (<kbd>Ctrl</kbd>+<kbd>S</kbd>) is organized into 12 tabs:

| Tab | What it configures |
| --- | --- |
| 📁 Workspace | Workspace name and path |
| 🌐 Language | 简体中文 / 繁體中文 / English |
| 🎨 Appearance | Dark / light / follow system, panel preview |
| 📦 Versions | Version snapshot toggle |
| 🛡️ Mock Server | Local Mock switch and port |
| 💻 Code Generation | Default language for generated request code |
| 📤 Export | Default export format + which formats show up in the export dialog |
| 📥 Import | Which formats show up in the import menu |
| 🧾 Default Headers | Headers automatically attached to new APIs |
| ⌨️ Shortcuts | Record / reset the global shortcuts |
| 🔄 Remote Sync | Git / SVN remote sync options |
| ℹ️ About | App version, project links, tech-stack badges and the open-source component list |

## System Tray

- Clicking the window close button hides the app to the tray (the app keeps running)
- Left-click the tray icon: show the window; right-click: menu
- Tray menu: show window / hide window / start·stop Mock server / check for updates / display mode / language (single submenu, check to switch) / quit
- The Mock menu item text updates with the Mock server status

## Development & Build

### Prerequisites

- Node.js ≥ 18
- Rust (Windows MSVC toolchain)
- WebView2 (bundled with Windows 10/11)
- [just](https://github.com/casey/just) (command runner)

### Common Commands ([justfile](justfile))

```bash
just init        # install dev prerequisites (Rust toolchain + Node.js)
just dev         # run in dev mode (frontend HMR + Rust dev)
just test        # run all tests (Rust unit tests + frontend type check + frontend build)
just build       # full package: exe + NSIS / MSI installer (optional)
just release     # build the standalone executable and collect it into ./release/ (no install needed)
just exe         # build only the release executable (fast)
just tsc         # frontend type check
just check       # Rust compile check
just icon        # regenerate app icons (ultramarine theme)
just clean       # clean build artifacts
just push "msg"  # commit and push to remote
just             # list all commands
```

> The justfile supports both Windows (cmd) and WSL (bash). If Chinese arguments look garbled on a Windows console, run `chcp 65001` first to switch to UTF-8.

### Build Artifacts

- `just release` output: `release/api-manager.exe` — a standalone executable; no installation needed, copy it to any Windows machine and run (Win10/11 ships the WebView2 runtime)
- `just build` (optional) output: `src-tauri/target/release/bundle/nsis/API Manager_1.0.0_x64-setup.exe` — NSIS installer; `bundle/msi/API Manager_1.0.0_x64_en-US.msi` — MSI

### Running Tests

```bash
just test
```

### Sample Workspace

`examples/demo-workspace/` provides a complete example with two groups: user management and order management.
Open the app and pick this directory to try every feature (built-in Mock is enabled).

## Tech Stack

| Layer | Stack |
| --- | --- |
| Desktop shell | [![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?style=flat-square&logo=tauri&logoColor=white)](https://tauri.app) [![Rust](https://img.shields.io/badge/Rust-1.77%2B-000000?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org) |
| Frontend | [![React](https://img.shields.io/badge/React-18-61DAFB?style=flat-square&logo=react&logoColor=black)](https://react.dev) [![TypeScript](https://img.shields.io/badge/TypeScript-5-3178C6?style=flat-square&logo=typescript&logoColor=white)](https://www.typescriptlang.org) [![Vite](https://img.shields.io/badge/Vite-5-646CFF?style=flat-square&logo=vite&logoColor=white)](https://vitejs.dev) |
| Backend (Rust) | [![Axum](https://img.shields.io/badge/Axum-0.7-000000?style=flat-square&logo=rust&logoColor=white)](https://github.com/tokio-rs/axum) [![reqwest](https://img.shields.io/badge/reqwest-0.12-000000?style=flat-square&logo=rust&logoColor=white)](https://github.com/seanmonstar/reqwest) [![Tokio](https://img.shields.io/badge/Tokio-1-000000?style=flat-square&logo=rust&logoColor=white)](https://tokio.rs) [![tower-http](https://img.shields.io/badge/tower--http-0.6-000000?style=flat-square&logo=rust&logoColor=white)](https://github.com/tower-rs/tower-http) |
| Data & protocols | [![serde](https://img.shields.io/badge/serde-1-000000?style=flat-square&logo=rust&logoColor=white)](https://serde.rs) [![serde_json](https://img.shields.io/badge/serde__json-1-000000?style=flat-square&logo=rust&logoColor=white)](https://github.com/serde-rs/json) [![serde_yaml](https://img.shields.io/badge/serde__yaml-0.9-000000?style=flat-square&logo=rust&logoColor=white)](https://github.com/dtolnay/serde-yaml) [![roxmltree](https://img.shields.io/badge/roxmltree-0.20-000000?style=flat-square&logo=rust&logoColor=white)](https://github.com/RazrFalcon/roxmltree) [![socket.io-client](https://img.shields.io/badge/socket.io--client-4-010101?style=flat-square&logo=socketdotio&logoColor=white)](https://socket.io) |
| Editor & highlighting | [![Vditor](https://img.shields.io/badge/Vditor-4-4285F4?style=flat-square)](https://b3log.org/vditor/) [![highlight.js](https://img.shields.io/badge/highlight.js-11-1E293B?style=flat-square)](https://highlightjs.org) |
| Tooling | [![just](https://img.shields.io/badge/just-command%20runner-7B68EE?style=flat-square)](https://just.systems) [![Tauri CLI](https://img.shields.io/badge/tauri--cli-2-24C8DB?style=flat-square&logo=tauri&logoColor=white)](https://tauri.app) [![tauri-plugin-dialog](https://img.shields.io/badge/tauri--plugin--dialog-2-24C8DB?style=flat-square&logo=tauri&logoColor=white)](https://github.com/tauri-apps/plugins-workspace) [![tauri-plugin-opener](https://img.shields.io/badge/tauri--plugin--opener-2-24C8DB?style=flat-square&logo=tauri&logoColor=white)](https://github.com/tauri-apps/plugins-workspace) |

- **Theme color**: Ultramarine `#2E59A7`
- Every dependency and its declared version is also listed inside the app: *Settings → About*

## License

[MIT](LICENSE) © bluefrog
