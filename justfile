# API Manager 开发命令
# 工具链：~/bin 下有 node / cargo 包装脚本（指向 Windows 的 node.exe / cargo.exe）
export PATH := env_var("PATH") + ":" + env_var("HOME") + "/bin"

# 列出全部可用命令
default:
    @just --list

# ========== 开发 ==========

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

# 构建 release 可执行文件（快速，不含安装程序）
exe:
    npm run build
    cd src-tauri && cargo build --release

# 完整打包：release exe + NSIS / MSI 安装程序
build:
    npm run tauri build

# 重新生成应用图标（群青主题 #2E59A7）
icon:
    node scripts/gen-icon.cjs app-icon.png
    npx tauri icon app-icon.png

# 清理构建产物
clean:
    cd src-tauri && cargo clean
    rm -rf dist node_modules/.vite

# ========== Git ==========

# 提交并推送：just push "提交信息"
push message="update":
    git add -A
    git commit -m "{{message}}"
    git push origin main
