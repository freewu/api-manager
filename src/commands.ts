import { invoke } from "@tauri-apps/api/core";
import type {
  ApiFile,
  AppSettings,
  EnvStore,
  ExampleFile,
  ExampleSummary,
  ExportFormat,
  HttpRequestData,
  HttpResult,
  InfoJson,
  MockStatus,
  TreeNode,
  UpdateInfo,
  VersionInfo,
} from "./types";
export type { ExportFormat };

export function getWorkspace(): Promise<string | null> {
  return invoke<string | null>("get_workspace");
}

export function getRecentWorkspaces(): Promise<string[]> {
  return invoke<string[]>("get_recent_workspaces");
}

export function openWorkspace(path: string): Promise<string> {
  return invoke<string>("open_workspace", { path });
}

export function pickWorkspace(): Promise<string | null> {
  return invoke<string | null>("pick_workspace");
}

export function workspaceIsEmpty(): Promise<boolean> {
  return invoke<boolean>("workspace_is_empty");
}

/** 工作区根目录是否已有 __info.json（判断是否为全新工作目录） */
export function hasWorkspaceInfo(): Promise<boolean> {
  return invoke<boolean>("has_workspace_info");
}

export function createDemo(): Promise<void> {
  return invoke<void>("create_demo");
}

export interface VcsInfo {
  vcs: "git" | "svn" | null;
}

export function vcsInfo(): Promise<VcsInfo> {
  return invoke<VcsInfo>("vcs_info");
}

/** 同步（git pull / svn update）；remote=false 时仅 git fetch */
export function vcsSync(remote: boolean): Promise<string> {
  return invoke<string>("vcs_sync", { remote });
}

/** 提交并推送远程；remote=false 时只提交不推送 */
export function vcsCommitPush(remote: boolean): Promise<string> {
  return invoke<string>("vcs_commit_push", { remote });
}

export interface PostmanImportResult {
  folder: string;
  env: string;
  vars: number;
}

export function importPostman(): Promise<PostmanImportResult | null> {
  return invoke<PostmanImportResult | null>("import_postman");
}

export interface OpenApiImportResult {
  folder: string;
  count: number;
}

export function importOpenApi(): Promise<OpenApiImportResult | null> {
  return invoke<OpenApiImportResult | null>("import_openapi");
}

export interface MarkdownDoc {
  name: string;
  md: string;
  html: string;
}

/** 渲染接口的 Markdown 文档（含 HTML 预览） */
export function renderApiMarkdown(path: string): Promise<MarkdownDoc> {
  return invoke<MarkdownDoc>("render_api_markdown", { path });
}

/** 渲染分组（含其下全部接口）为单个 Markdown 文档（含 HTML 预览） */
export function renderGroupMarkdown(path: string): Promise<MarkdownDoc> {
  return invoke<MarkdownDoc>("render_group_markdown", { path });
}

/** 导出接口为 .md / .html 文件（弹窗选目录），返回保存路径或 null（用户取消） */
export function exportApiMarkdown(path: string, format: "md" | "html"): Promise<string | null> {
  return invoke<string | null>("export_api_markdown", { path, format });
}

export interface MarkdownImportResult {
  folder: string;
  count: number;
}

/** 导入 Markdown 接口文档（弹窗选 .md 文件） */
export function importMarkdown(): Promise<MarkdownImportResult | null> {
  return invoke<MarkdownImportResult | null>("import_markdown");
}

/** 导出选中接口/分组为 Postman / OpenAPI / Docsify 格式，返回保存路径或 null（用户取消） */
export function exportSelection(paths: string[], format: ExportFormat): Promise<string | null> {
  return invoke<string | null>("export_selection", { paths, format });
}

export function readTree(): Promise<TreeNode> {
  return invoke<TreeNode>("read_tree");
}

export function readApi(path: string): Promise<ApiFile> {
  return invoke<ApiFile>("read_api", { path });
}

export function saveApi(path: string, data: ApiFile): Promise<string> {
  return invoke<string>("save_api", { path, data });
}

export function saveApiVersion(data: ApiFile): Promise<string> {
  return invoke<string>("save_api_version", { data });
}

export function listVersions(uuid: string): Promise<VersionInfo[]> {
  return invoke<VersionInfo[]>("list_versions", { uuid });
}

export function getCurrentVersion(uuid: string): Promise<number> {
  return invoke<number>("get_current_version", { uuid });
}

export function readApiVersion(path: string): Promise<string> {
  return invoke<string>("read_api_version", { path });
}

/** 恢复到指定历史版本：后端先自动备份当前状态为新版本，再写回主文件，返回主文件路径 */
export function restoreApiVersion(versionPath: string, uuid: string): Promise<string> {
  return invoke<string>("restore_api_version", { versionPath, uuid });
}

export function loadSettings(): Promise<AppSettings> {
  return invoke<AppSettings>("load_settings");
}

export function saveSettings(settings: AppSettings): Promise<void> {
  return invoke<void>("save_settings", { settings });
}

/** 切换界面语言（保存设置 + 刷新托盘菜单 + 通知前端） */
export function setLanguage(lang: "zh" | "zh-tw" | "en"): Promise<void> {
  return invoke<void>("set_language", { lang });
}

export function createApi(dir: string, name: string, protocol: "http" | "websocket" = "http"): Promise<string> {
  return invoke<string>("create_api", { dir, name, protocol });
}

export function createFolder(parent: string, name: string): Promise<string> {
  return invoke<string>("create_folder", { parent, name });
}

export function renameEntry(path: string, newName: string): Promise<void> {
  return invoke<void>("rename_entry", { path, newName });
}

export function deleteEntry(path: string): Promise<void> {
  return invoke<void>("delete_entry", { path });
}

export function copyEntry(path: string): Promise<string> {
  return invoke<string>("copy_entry", { path });
}

export function moveEntry(src: string, dstDir: string): Promise<string> {
  return invoke<string>("move_entry", { src, dstDir });
}

export function readInfo(path: string): Promise<InfoJson> {
  return invoke<InfoJson>("read_info", { path });
}

export function saveInfo(path: string, data: InfoJson): Promise<void> {
  return invoke<void>("save_info", { path, data });
}

/** 标记 / 取消标记“已废弃”（接口或分组），返回新的废弃状态 */
export function toggleDeprecated(path: string): Promise<boolean> {
  return invoke<boolean>("toggle_deprecated", { path });
}

export function readEnv(): Promise<EnvStore> {
  return invoke<EnvStore>("read_envs");
}

export function saveEnv(data: EnvStore): Promise<void> {
  return invoke<void>("save_envs", { data });
}

export function updateTrayEnv(name: string): Promise<void> {
  return invoke<void>("update_tray_env", { name });
}

export function sendRequest(req: HttpRequestData): Promise<HttpResult> {
  return invoke<HttpResult>("send_request", { req });
}

/** 弹出系统文件选择框，返回文件路径（取消时返回 null） */
export function pickFile(): Promise<string | null> {
  return invoke<string | null>("pick_file");
}

// ---- 请求历史 ----

export interface HistorySummary {
  id: string;
  time: number;
  method: string;
  url: string;
  ok: boolean;
  status: number;
  statusText: string;
  timeMs: number;
  size: number;
  error?: string;
}

export interface HistoryDetail {
  id: string;
  time: number;
  method: string;
  url: string;
  ok: boolean;
  status: number;
  statusText: string;
  timeMs: number;
  size: number;
  error?: string;
  reqHeaders: [string, string][];
  reqBody?: string;
  respHeaders: [string, string][];
  respBody: string;
}

export interface HistoryInput {
  method: string;
  url: string;
  reqHeaders: [string, string][];
  reqBody?: string;
  ok: boolean;
  status: number;
  statusText: string;
  respHeaders: [string, string][];
  respBody: string;
  timeMs: number;
  size: number;
  error?: string;
}

export interface HistoryDay {
  day: string;
  count: number;
}

export function saveHistory(input: HistoryInput): Promise<string> {
  return invoke<string>("save_history", { input });
}

export function historyRecords(offset: number, limit: number): Promise<HistorySummary[]> {
  return invoke<HistorySummary[]>("history_records", { offset, limit });
}

export function historyDetail(id: string): Promise<HistoryDetail> {
  return invoke<HistoryDetail>("history_detail", { id });
}

export function historyDays(): Promise<HistoryDay[]> {
  return invoke<HistoryDay[]>("history_days");
}

export function historyClear(): Promise<void> {
  return invoke<void>("history_clear");
}

export function saveExample(
  uuid: string,
  name: string,
  data: ExampleFile
): Promise<string> {
  return invoke<string>("save_example", { uuid, name, data });
}

export function listExamples(uuid: string): Promise<ExampleSummary[]> {
  return invoke<ExampleSummary[]>("list_examples", { uuid });
}

export function readExample(uuid: string, file: string): Promise<ExampleFile> {
  return invoke<ExampleFile>("read_example", { uuid, file });
}

export function deleteExample(uuid: string, file: string): Promise<void> {
  return invoke("delete_example", { uuid, file });
}

export function mockStart(port: number): Promise<MockStatus> {
  return invoke<MockStatus>("mock_start", { port });
}

export function mockStop(): Promise<MockStatus> {
  return invoke<MockStatus>("mock_stop");
}

export function mockStatus(): Promise<MockStatus> {
  return invoke<MockStatus>("mock_status");
}

export function mockReload(): Promise<MockStatus> {
  return invoke<MockStatus>("mock_reload");
}

export function getAppVersion(): Promise<string> {
  return invoke<string>("get_app_version");
}

/** 异步检查 GitHub Releases 是否有新版本 */
export function checkUpdate(): Promise<UpdateInfo> {
  return invoke<UpdateInfo>("check_update");
}

/** 在系统浏览器中打开外部链接 */
export function openExternal(url: string): Promise<void> {
  return invoke("plugin:opener|open_url", { url });
}
