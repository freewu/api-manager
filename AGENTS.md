# AGENTS.md

## 工作约定（必须遵守）

- 每次完成一个开发任务后，立即执行 `git commit`，并将代码 `push` 到远程仓库（origin）。
- commit message 用中文，以类型前缀开头（`feat:` 新功能 / `fix:` 修复 / `refactor:` 重构 / `style:` 样式 / `docs:` 文档 / `chore:` 杂项），后面简要描述本次改动内容。示例：`fix: 示例中的 query 现在会包含 URL 上的参数`。
- 提交前先确认改动可构建（`npm run build` / `cargo test` 视改动范围而定）。

## 版本发布（git tag）

- **每次修改版本号后，必须更新 `Update.md`**：将本次版本的更新内容总结记录在文件顶部（用 `git log <上个版本tag>..HEAD --oneline` 梳理），作为 GitHub Release 的 Release notes（`.github/workflows/release.yml` 发布时自动读取 Update.md）。
- 每次发布（打包安装程序）完成后，执行一条指令即可提交 git tag 并推送到远程（版本号与 `tauri.conf.json` 中的 version 保持一致）：
  ```bash
  git tag v0.5.2 && git push origin v0.5.2
  ```
- **版本号需同步修改三处**：`package.json` 的 `version`、`src-tauri/tauri.conf.json` 的 `version`、`src-tauri/Cargo.toml` 的 `version`（Cargo.lock 由 cargo build 自动更新），当前均为 v0.5.2。修改版本号时三处要一起改。
- 版本号已存在时先删除再重打：`git tag -d v0.5.2 && git push origin :refs/tags/v0.5.2`
- 查看标签列表：`git tag -l`；查看某标签指向的提交：`git show v0.1.6 --oneline -s`
