import { invoke } from "@tauri-apps/api/core";
import { migrateDocParamsToRows } from "./utils/docParams";
import type {
  ApiFile,
  AppSettings,
  CustomMock,
  EnvStore,
  ExampleFile,
  ExampleSummary,
  ExportFormat,
  HttpRequestData,
  HttpResult,
  InfoJson,
  KeyValue,
  MockStatus,
  ObjectDef,
  ObjectImportResult,
  ObjectStore,
  ObjectUsageItem,
  ObjectVersionInfo,
  PrescriptResult,
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
  /** 导入统计 */
  http?: number;
  ws?: number;
  graphql?: number;
  socketio?: number;
  objects?: number;
  failed?: number;
  duplicated?: number;
}

export function importPostman(): Promise<PostmanImportResult | null> {
  return invoke<PostmanImportResult | null>("import_postman");
}

export interface OpenApiImportResult {
  folder: string;
  count: number;
  /** 导入统计 */
  http?: number;
  ws?: number;
  graphql?: number;
  socketio?: number;
  objects?: number;
  failed?: number;
  duplicated?: number;
}

export function importOpenApi(): Promise<OpenApiImportResult | null> {
  return invoke<OpenApiImportResult | null>("import_openapi");
}

/** 导入 Apifox 项目（弹窗选 .json 文件） */
export function importApifox(): Promise<OpenApiImportResult | null> {
  return invoke<OpenApiImportResult | null>("import_apifox");
}

/** 导入 Apipost 项目（弹窗选 .json 文件） */
export function importApipost(): Promise<OpenApiImportResult | null> {
  return invoke<OpenApiImportResult | null>("import_apipost");
}

/** 导入 RAML 文档（弹窗选 .raml 文件） */
export function importRaml(): Promise<OpenApiImportResult | null> {
  return invoke<OpenApiImportResult | null>("import_raml");
}

/** 导入 WADL 文档（弹窗选 .wadl 文件） */
export function importWadl(): Promise<OpenApiImportResult | null> {
  return invoke<OpenApiImportResult | null>("import_wadl");
}

/** 导入 HAR 抓包文件（弹窗选 .har 文件） */
export function importHar(): Promise<OpenApiImportResult | null> {
  return invoke<OpenApiImportResult | null>("import_har");
}

/** 导入 YApi 导出文件（弹窗选 .json；自动识别 Swagger / YApi 原生树） */
export function importYapi(): Promise<OpenApiImportResult | null> {
  return invoke<OpenApiImportResult | null>("import_yapi");
}

/** 导入 Eolink 导出文件（弹窗选 .json） */
export function importEolink(): Promise<OpenApiImportResult | null> {
  return invoke<OpenApiImportResult | null>("import_eolink");
}

/** 导入 Insomnia 导出文件（弹窗选 .yml/.json） */
export function importInsomnia(): Promise<OpenApiImportResult | null> {
  return invoke<OpenApiImportResult | null>("import_insomnia");
}

export async function importJmeter(): Promise<OpenApiImportResult | null> {
  return invoke<OpenApiImportResult | null>("import_jmeter");
}

export async function importApiDoc(): Promise<OpenApiImportResult | null> {
  return invoke<OpenApiImportResult | null>("import_apidoc");
}

/** 导入扩展格式（apidog/bruno/apizza/nei/doclever/io-docs/easydoc/docway/hoppscotch/metersphere） */
export async function importExtra(format: string): Promise<OpenApiImportResult | null> {
  return invoke<OpenApiImportResult | null>("import_extra", { format });
}

export interface MarkdownDoc {
  name: string;
  md: string;
  html: string;
}

/** 将 Markdown 文本渲染为 HTML 片段（接口描述预览） */
export function renderMarkdown(text: string): Promise<string> {
  return invoke<string>("render_markdown", { text });
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
export function exportApiMarkdown(
  path: string,
  format: "md" | "html",
  nav: "off" | "left" | "right" = "right"
): Promise<string | null> {
  return invoke<string | null>("export_api_markdown", { path, format, nav });
}

export interface MarkdownImportResult {
  folder: string;
  count: number;
  /** 导入统计 */
  http?: number;
  ws?: number;
  graphql?: number;
  socketio?: number;
}

/** 导入 Markdown 接口文档（弹窗选 .md 文件） */
export function importMarkdown(): Promise<MarkdownImportResult | null> {
  return invoke<MarkdownImportResult | null>("import_markdown");
}

/** 导出选中接口/分组为 Postman / OpenAPI / Docsify 格式，返回保存路径或 null（用户取消） */
export function exportSelection(
  paths: string[],
  format: ExportFormat,
  nav: "off" | "left" | "right" = "right"
): Promise<string | null> {
  return invoke<string | null>("export_selection", { paths, format, nav });
}

export function readTree(): Promise<TreeNode> {
  return invoke<TreeNode>("read_tree");
}

export function readApi(path: string): Promise<ApiFile> {
  // 说明字段统一迁移：docParams 旧说明搬入 KeyValue.description（接口文档 tab 与请求页签共用一行内字段）
  return invoke<ApiFile>("read_api", { path }).then(migrateDocParamsToRows);
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

export function createApi(dir: string, name: string, protocol: "http" | "websocket" | "graphql" | "socketio" = "http"): Promise<string> {
  return invoke<string>("create_api", { dir, name, protocol });
}

export function createFolder(parent: string, name: string, collapsed?: boolean): Promise<string> {
  return invoke<string>("create_folder", { parent, name, collapsed: collapsed ?? null });
}

/** 拖动排序：按有序子项路径列表保存父分组 __info.json 的 dirs / apis 顺序列表 */
export function reorderChildren(parent: string, paths: string[]): Promise<void> {
  return invoke<void>("reorder_children", { parent, paths });
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

/** 记录分组目录的开闭状态（写入目录 __info.json 的 collapsed 字段） */
export function setFolderCollapsed(path: string, collapsed: boolean): Promise<void> {
  return invoke<void>("set_folder_collapsed", { path, collapsed });
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
  /** 所属接口 uuid（Diff 比对限定同接口；旧记录可能为空字符串） */
  apiUuid?: string;
  apiName?: string;
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
  apiUuid?: string;
  apiName?: string;
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
  /** 所属接口 uuid（发送请求时从当前接口带出，用于 Diff 比对） */
  apiUuid?: string;
  apiName?: string;
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

export function listCustomMocks(): Promise<CustomMock[]> {
  return invoke<CustomMock[]>("list_custom_mocks");
}

export function saveCustomMock(input: CustomMock, oldName?: string): Promise<void> {
  return invoke("save_custom_mock", { input, oldName });
}

export function deleteCustomMock(name: string): Promise<void> {
  return invoke("delete_custom_mock", { name });
}

// ==================== 对象管理 ====================

export function listObjects(): Promise<ObjectStore> {
  return invoke<ObjectStore>("list_objects");
}

export function saveObjects(store: ObjectStore): Promise<string> {
  return invoke<string>("save_objects", { store });
}

export function importJsonObject(
  name: string,
  group: string,
  json: string
): Promise<ObjectImportResult> {
  return invoke<ObjectImportResult>("import_json_object", { name, group, json });
}

export function importDdl(group: string, ddl: string): Promise<ObjectImportResult> {
  return invoke<ObjectImportResult>("import_ddl", { group, ddl });
}

export function objectUsage(store: ObjectStore): Promise<ObjectUsageItem[]> {
  return invoke<ObjectUsageItem[]>("object_usage", { store });
}

export function saveObjectVersion(uuid: string, snapshot: ObjectDef): Promise<string> {
  return invoke<string>("save_object_version", { uuid, snapshot });
}

export function listObjectVersions(uuid: string): Promise<ObjectVersionInfo[]> {
  return invoke<ObjectVersionInfo[]>("list_object_versions", { uuid });
}

export function readObjectVersion(uuid: string, version: number): Promise<ObjectDef> {
  return invoke<ObjectDef>("read_object_version", { uuid, version });
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

/** 在系统文件管理器中打开路径（目录/文件） */
export function openPath(path: string): Promise<void> {
  return invoke("plugin:opener|open_path", { path });
}

/** 数据生成结果 */
export interface GenDataResult {
  file: string;
  dir: string;
  count: number;
  elapsed_ms: number;
}

/** 数据生成提交的属性配置（写入 .gen_log 记录） */
export interface GenPropItem {
  key: string;
  kind: string;
  mock: string;
  enabled: boolean;
  desc?: string;
}

/** .gen_log 单条生成记录 */
export interface GenLogItem {
  file: string;
  time: number;
  time_str: string;
  object_uuid: string;
  object_name: string;
  dir: string;
  format: string;
  table: string;
  count: number;
  elapsed_ms: number;
  /** 生成文件大小（字节）；旧记录可能缺失 */
  file_size?: number;
  props: GenPropItem[];
}

/** 写入生成的数据文件并记录 .gen_log */
export function genData(p: {
  dir: string;
  fileName: string;
  content: string;
  format: string;
  table: string;
  count: number;
  elapsedMs: number;
  objectUuid: string;
  objectName: string;
  props: GenPropItem[];
}): Promise<GenDataResult> {
  return invoke<GenDataResult>("gen_data", {
    dir: p.dir,
    fileName: p.fileName,
    content: p.content,
    format: p.format,
    table: p.table,
    count: p.count,
    elapsedMs: p.elapsedMs,
    objectUuid: p.objectUuid,
    objectName: p.objectName,
    props: p.props,
  });
}

/** 读取 .gen_log 全部生成记录（按时间倒序） */
export function listGenLogs(): Promise<GenLogItem[]> {
  return invoke<GenLogItem[]>("list_gen_logs");
}

// ---------- 前置脚本 / 全局变量 ----------

/** 测试执行前置脚本，返回 console 日志 / 返回值 / 更新后的全局变量 */
export function runPrescript(p: {
  code: string;
  query: KeyValue[];
  path: KeyValue[];
  headers: KeyValue[];
  body: string;
  globals: Record<string, string>;
}): Promise<PrescriptResult> {
  return invoke<PrescriptResult>("run_prescript", {
    code: p.code,
    query: p.query,
    path: p.path,
    headers: p.headers,
    body: p.body,
    globals: p.globals,
  });
}

/** 读取工作区全局变量 */
export function getGlobalVars(): Promise<Record<string, string>> {
  return invoke<Record<string, string>>("get_global_vars");
}

/** 保存工作区全局变量 */
export function setGlobalVars(vars: Record<string, string>): Promise<void> {
  return invoke<void>("set_global_vars", { vars });
}
