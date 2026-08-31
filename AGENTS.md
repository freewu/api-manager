# AGENTS.md

## 工作约定（必须遵守）

- 每次完成一个开发任务后，立即执行 `git commit`，并将代码 `push` 到远程仓库（origin）。
- commit message 用中文，以类型前缀开头（`feat:` 新功能 / `fix:` 修复 / `refactor:` 重构 / `style:` 样式 / `docs:` 文档 / `chore:` 杂项），后面简要描述本次改动内容。示例：`fix: 示例中的 query 现在会包含 URL 上的参数`。
- 提交前先确认改动可构建（`npm run build` / `cargo test` 视改动范围而定）。

## 版本发布（git tag）

### 需要手动修改版本号的位置（全部同步为同一版本号）

| 文件 | 位置 | 说明 |
| --- | --- | --- |
| `package.json` | `version` 字段 | 前端包版本 |
| `src-tauri/tauri.conf.json` | `version` 字段 | 安装包/产物版本 |
| `src-tauri/Cargo.toml` | `version` 字段 | Rust 侧版本（托盘「API Manager vX」、窗口标题、`env!("CARGO_PKG_VERSION")` 均自动取自此） |
| `src-tauri/Cargo.lock` | `version` 字段（3 处） | **不用手动改**，`cargo build` 自动同步 |
| `Update.md` | 顶部新增本次版本更新内容 | 发布时作为 GitHub Release notes（release.yml 自动读取） |
| git tag | `git tag vX.Y.Z && git push origin vX.Y.Z` | 推送后自动触发 release workflow |

### 发布流程（顺序执行）

1. 三处版本号改完（Cargo.lock 由 `cargo build` 自动更新）
2. 更新 `Update.md`：用 `git log <上个版本tag>..HEAD --oneline` 梳理本次更新内容，总结记录在文件顶部
3. 构建验证（`npm run build` / `cargo test`）后提交推送，并打 tag：
  ```bash
  git tag <当前版本tag> && git push origin <当前版本tag>
  ```
4. 版本号已存在时先删除再重打：`git tag -d <当前版本tag> && git push origin :refs/tags/<当前版本tag>`
5. 查看标签列表：`git tag -l`；查看某标签指向的提交：`git show <当前版本tag> --oneline -s`
