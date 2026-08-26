# ============================================================
# API Manager 开发命令（跨平台：Windows / WSL / macOS / Linux）
#
# 环境要求（Windows）：node、npm、cargo、just 在系统 PATH 中
# 环境要求（WSL）：   ~/bin 下有 node/cargo 包装脚本指向 Windows 可执行文件
#
# 说明：
#   - Windows 使用 cmd 作为 shell，确保在任何环境都能运行
#   - 其他平台使用 bash
# ============================================================

# Windows 使用 cmd.exe
[windows]
set shell := ["cmd.exe", "/c"]

# Unix 系（WSL / macOS / Linux）使用 bash
[unix]
set shell := ["bash", "-uc"]

# WSL/Linux 下需要把 ~/bin（node/cargo 包装脚本）加入 PATH；
# Windows 上直接使用系统 PATH（node/cargo 已在 PATH 中）
export PATH := env_var("PATH") + if os() == "windows" { "" } else { ":" + env_var_or_default("HOME", "") + "/bin" }

# 列出全部可用命令
default:
    @just --list

# ========== 开发 ==========

# 安装开发环境依赖（Rust 工具链 + Node.js），支持 Windows / macOS / Linux
init:
    @just init-rust
    @just init-node

# 安装 Rust 工具链（rustup + stable），已安装则跳过
[windows]
init-rust:
    rustc --version >nul 2>nul && (echo Rust already installed: & rustc --version) || (echo Installing Rust via rustup... & winget install --id Rustlang.Rustup -e --accept-source-agreements --accept-package-agreements & rustup default stable)
    rustc --version >nul 2>nul || echo Please reopen a terminal so PATH picks up rustc/cargo from %%USERPROFILE%%\.cargo\bin

[unix]
init-rust:
    if command -v cargo >/dev/null 2>&1; then echo "Rust already installed: $(cargo --version)"; else echo "Installing Rust via rustup..."; curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; fi
    if ! command -v cargo >/dev/null 2>&1 && [ -f "$HOME/.cargo/env" ]; then . "$HOME/.cargo/env"; echo "cargo activated via ~/.cargo/env"; fi

# 安装 Node.js（Windows 用 winget 装 LTS；macOS 用 Homebrew；Linux 用发行版包管理器）
[windows]
init-node:
    node --version >nul 2>nul && (echo Node already installed: & node --version) || (echo Installing Node.js LTS via winget... & winget install --id OpenJS.NodeJS.LTS -e --accept-source-agreements --accept-package-agreements)
    node --version >nul 2>nul || echo Please reopen a terminal so PATH picks up node/npm

[unix]
init-node:
    if command -v node >/dev/null 2>&1; then echo "Node already installed: $(node --version)"; else if command -v brew >/dev/null 2>&1; then echo "Installing Node.js via Homebrew..."; brew install node; elif command -v apt-get >/dev/null 2>&1; then echo "Installing Node.js via apt..."; sudo apt-get update && sudo apt-get install -y nodejs npm; elif command -v dnf >/dev/null 2>&1; then echo "Installing Node.js via dnf..."; sudo dnf install -y nodejs npm; elif command -v pacman >/dev/null 2>&1; then echo "Installing Node.js via pacman..."; sudo pacman -Sy --noconfirm nodejs npm; else echo "No supported package manager found. Please install Node.js manually: https://nodejs.org"; fi; fi

# 开发模式运行（前端热更新 + Rust dev 模式）
dev:
    npm run tauri dev

# 前端类型检查
tsc:
    npx tsc --noEmit

# 前端构建（vite → dist/）
ui:
    npm run build

# Rust 编译检查
check:
    cd src-tauri && cargo check

# 运行全部测试（Rust 单测 + 前端类型检查 + 前端构建）
test: tsc ui
    cd src-tauri && cargo test

# ========== 打包 ==========

# 构建 release 可执行文件（快速，不含安装程序；启用 custom-protocol 内嵌前端资源）
exe:
    npm run build
    cd src-tauri && cargo build --release --features custom-protocol

# 完整打包：release exe + NSIS / MSI 安装程序（可选，需要安装包时使用）
build:
    npm run tauri build

# 打包并复制到 ./release：仅单体可执行文件（无需安装，拷走即用）
release:
    npm run build
    cd src-tauri && cargo build --release --features custom-protocol
    @just release-collect

# 将单体可执行文件收集到 ./release（Windows）
[windows]
release-collect:
    if exist release rmdir /s /q release
    mkdir release
    copy /y "src-tauri\target\release\api-manager.exe" release\api-manager.exe
    dir release

# 将单体可执行文件收集到 ./release（Unix / WSL；若工具链为 Windows MSVC，产物为 .exe）
[unix]
release-collect:
    rm -rf release
    mkdir -p release
    for f in src-tauri/target/release/api-manager src-tauri/target/release/api-manager.exe; do if [ -f "$f" ]; then cp -f "$f" release/; fi; done
    ls -la release

# 重新生成应用图标（群青主题 #2E59A7）
icon:
    node scripts/gen-icon.cjs app-icon.png
    npx tauri icon app-icon.png

# 清理构建产物
clean: clean-dist
    cd src-tauri && cargo clean

# 清理前端产物（Windows / Unix 各自实现）
[windows]
clean-dist:
    if exist dist rmdir /s /q dist
    if exist node_modules\.vite rmdir /s /q node_modules\.vite

[unix]
clean-dist:
    rm -rf dist node_modules/.vite

# ========== Git ==========

# 提交并推送：just push "提交信息"
push message="update":
    git add -A
    git commit -m "{{message}}"
    git push origin main
