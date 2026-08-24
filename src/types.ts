// ---- 与 Rust 后端对应的类型定义 ----

export interface KeyValue {
  key: string;
  value: string;
  enabled: boolean;
  description: string;
  /** 是否文件字段（表单上传用，value 为文件路径） */
  isFile?: boolean;
}

export interface BodyData {
  mode: "none" | "raw" | "json" | "xml" | "form" | "binary";
  raw: string;
  form: KeyValue[];
  /** 二进制模式：本地文件路径（发送时读取文件字节） */
  binaryPath: string;
}

export interface MockConfig {
  enabled: boolean;
  status: number;
  headers: KeyValue[];
  delay: number;
  body: string;
}

/** 响应页签中的一条返回：名称（如 返回成功 / 返回失败）、HTTP 状态码、内容类型与示例体 */
export interface ResponseItem {
  id: string;
  /** 返回名称，可编辑（错误返回可命名为 参数错误 / 未授权 等） */
  name: string;
  /** HTTP 状态码，0 表示未填写 */
  status: number;
  contentType: string;
  /** 响应体示例（JSON / XML / 文本） */
  body: string;
}

export function emptyResponse(name: string, status = 0): ResponseItem {
  return {
    id: `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`,
    name,
    status,
    contentType: "application/json",
    body: "",
  };
}

/** 响应文档字段的 docParams source（resp:<响应条目 id>） */
export function respSource(id: string): DocSource {
  return `resp:${id}` as DocSource;
}

export interface ApiFile {
  uuid: string;
  name: string;
  method: string;
  path: string;
  url: string;
  description: string;
  headers: KeyValue[];
  query: KeyValue[];
  params: KeyValue[];
  body: BodyData;
  mock: MockConfig;
  examples: unknown[];
  /** 响应页签条目：返回成功 / 返回失败 / 自定义错误返回 */
  responses: ResponseItem[];
  /** 入参文档：请求参数的补充说明（类型 / 说明），按 source+key 关联 */
  docParams: DocParam[];
  /** 是否已标记废弃 */
  deprecated: boolean;
  /** 接口协议：http（HTTP 接口）或 websocket（WebSocket 接口） */
  protocol: "http" | "websocket" | "graphql" | "socketio";
}

export type DocSource =
  | "header"
  | "query"
  | "path"
  | "body"
  | "resp_success"
  | "resp_fail"
  | `resp:${string}`;

/** 接口文档字段类型选项 */
export const DOC_TYPES = ["String", "Integer", "Float", "Boolean", "List", "Object"];

export interface DocParam {
  source: DocSource;
  key: string;
  type: string;
  description: string;
  /** List 类型的元素类型 */
  itemType: string;
  /** Object 类型的对象名称 */
  objectName: string;
  /** 下级字段（树状） */
  children: DocParam[];
}

export function emptyDocParam(source: DocSource): DocParam {
  return { source, key: "", type: "", description: "", itemType: "", objectName: "", children: [] };
}

/** 请求示例文件内容（.examples/<接口uuid>/<示例名称hash值>.json） */
export interface ExampleFile {
  name: string;
  /** 保存时间（Unix 秒） */
  time: number;
  method: string;
  url: string;
  reqHeaders: [string, string][];
  /** 路径参数（发送时的取值） */
  reqPath: [string, string][];
  /** Query 参数（发送时的取值） */
  reqQuery: [string, string][];
  reqBody?: string;
  status: number;
  statusText: string;
  respHeaders: [string, string][];
  respBody: string;
  timeMs: number;
  size: number;
  error?: string;
}

/** 示例列表摘要（不含请求/响应全文） */
export interface ExampleSummary {
  name: string;
  /** 文件名（不含目录），用于读取/删除 */
  file: string;
  time: number;
  method: string;
  url: string;
  status: number;
}

export interface InfoJson {
  name?: string;
  description?: string;
  baseUrl?: string;
  mockPort?: number;
  order?: number;
  collapsed?: boolean;
  /** 分组是否已标记废弃 */
  deprecated?: boolean;
}

// ---- 全局环境变量 ----

export interface EnvVariable {
  key: string;
  value: string; // 现有值
  defaultValue: string; // 默认值（现值为空时使用）
  description: string;
  enabled: boolean;
}

export interface Environment {
  name: string;
  variables: EnvVariable[];
}

export interface EnvStore {
  active: string;
  environments: Environment[];
}

export function emptyEnv(): EnvStore {
  return { active: "", environments: [] };
}

export function emptyEnvVariable(): EnvVariable {
  return { key: "", value: "", defaultValue: "", description: "", enabled: true };
}

export interface TreeNode {
  kind: "folder" | "api";
  name: string;
  path: string;
  method?: string;
  endpoint?: string;
  mockEnabled?: boolean;
  description?: string;
  collapsed?: boolean;
  apiCount?: number;
  /** 是否已标记废弃（分组无此字段时默认未废弃） */
  deprecated?: boolean;
  /** 接口协议（http / websocket，分组无此字段） */
  protocol?: "http" | "websocket" | "graphql" | "socketio";
  children?: TreeNode[];
}

export interface VersionInfo {
  version: number;
  name: string;
  path: string;
  modified: number;
  method?: string;
  endpoint?: string;
}

export type ExportFormat =
  | "postman"
  | "openapi"
  | "docsify"
  | "markdown"
  | "html"
  | "apifox"
  | "apipost"
  | "raml"
  | "wadl"
  | "yapi"
  | "eolink"
  | "insomnia"
  | "jmeter"
  | "apidoc"
  | "apidog"
  | "bruno"
  | "apizza"
  | "nei"
  | "doclever"
  | "io-docs"
  | "easydoc"
  | "docway"
  | "hoppscotch"
  | "metersphere"
  | "rap2-project";

/** 主页「导入」菜单支持的格式 */
export type ImportFormat =
  | "postman"
  | "openapi"
  | "markdown"
  | "apifox"
  | "apipost"
  | "raml"
  | "wadl"
  | "har"
  | "yapi"
  | "eolink"
  | "insomnia"
  | "jmeter"
  | "apidoc"
  | "apidog"
  | "bruno"
  | "apizza"
  | "nei"
  | "doclever"
  | "io-docs"
  | "easydoc"
  | "docway"
  | "hoppscotch"
  | "metersphere"
  | "rap2"
  | "curl";

/** 导入格式中必选（不可关闭）的类型 */
export const REQUIRED_IMPORT_FORMATS: ImportFormat[] = ["postman", "openapi"];

/** 导出格式中必选（不可关闭）的类型 */
export const REQUIRED_EXPORT_FORMATS: ExportFormat[] = [
  "postman",
  "openapi",
  "docsify",
  "markdown",
  "html",
];

export interface AppSettings {
  displayMode: string; // "dark" | "light" | "system"
  enableVersion: boolean;
  enableMock: boolean;
  mockPort: number;
  syncRemote: boolean;
  enableCodegen: boolean;
  codegenLang: string; // 代码生成默认语言（bash / python / c / cpp / java / csharp / ...）
  /** 是否启用默认 Header（新增接口时自动附带） */
  enableDefaultHeaders: boolean;
  /** 默认 Header 列表 */
  defaultHeaders: KeyValue[];
  /** 导出默认格式 */
  exportFormat: ExportFormat;
  /** 主页导入按钮总开关（false 时隐藏「导入」按钮） */
  importEnabled: boolean;
  /** 主页导出按钮总开关（false 时隐藏「导出」按钮） */
  exportEnabled: boolean;
  /** 主页导入菜单展示的格式开关（postman/openapi 必选不可关闭） */
  importTypes: Record<ImportFormat, boolean>;
  /** 导出弹窗格式下拉展示的格式开关（postman/openapi/docsify/markdown/html 必选不可关闭） */
  exportTypes: Record<ExportFormat, boolean>;
  /** HTML 文档悬浮导航栏位置（off 关闭 / left 左侧 / right 右侧） */
  htmlNav: "off" | "left" | "right";
  /** 界面语言（zh / en） */
  language: "zh" | "zh-tw" | "en";
  /** 最近打开的工作目录数量上限（最少 3） */
  recentLimit: number;
}

export const defaultSettings = (): AppSettings => ({
  displayMode: "system",
  enableVersion: true,
  enableMock: true,
  mockPort: 5050,
  syncRemote: true,
  enableCodegen: true,
  codegenLang: "bash",
  enableDefaultHeaders: false,
  defaultHeaders: [],
  exportFormat: "postman",
  importEnabled: true,
  exportEnabled: true,
  // 默认仅开启 apifox / apipost（其余需在设置中手动开启；必选格式保持开启）
  importTypes: {
    postman: true,
    openapi: true,
    markdown: false,
    apifox: true,
    apipost: true,
    raml: false,
    wadl: false,
    har: false,
    yapi: false,
    eolink: false,
    insomnia: false,
    jmeter: false,
    apidoc: false,
    apidog: false,
    bruno: false,
    apizza: false,
    nei: false,
    doclever: false,
    "io-docs": false,
    easydoc: false,
    docway: false,
    hoppscotch: false,
    metersphere: false,
    rap2: false,
    curl: false,
  },
  exportTypes: {
    postman: true,
    openapi: true,
    apifox: true,
    apipost: true,
    docsify: true,
    markdown: true,
    html: true,
    raml: false,
    wadl: false,
    yapi: false,
    eolink: false,
    insomnia: false,
    jmeter: false,
    apidoc: false,
    apidog: false,
    bruno: false,
    apizza: false,
    nei: false,
    doclever: false,
    "io-docs": false,
    easydoc: false,
    docway: false,
    hoppscotch: false,
    metersphere: false,
    "rap2-project": false,
  },
  htmlNav: "right",
  language: "zh",
  recentLimit: 5,
});

export interface HttpRequestData {
  method: string;
  url: string;
  headers: KeyValue[];
  body?: string;
  /** 二进制模式：本地文件路径，存在时按原始字节发送 */
  bodyFile?: string | null;
  /** 表单字段（含文件字段 isFile=true，值为文件路径），存在时按 multipart/form-data 发送 */
  form?: KeyValue[] | null;
  timeoutMs: number;
}

export interface HttpResult {
  ok: boolean;
  status: number;
  statusText: string;
  headers: [string, string][];
  body: string;
  timeMs: number;
  size: number;
  url: string;
  error?: string;
}

/** WebSocket 交互记录（连接事件 / 发送 / 接收 / 错误） */
export interface WsLogEntry {
  /** sent=已发送 / recv=已接收 / info=连接事件 / error=错误 */
  dir: "sent" | "recv" | "info" | "error";
  text: string;
  time: number;
}

/** 更新检查结果（来自 GitHub Releases） */
export interface UpdateInfo {
  latest: string;
  current: string;
  hasUpdate: boolean;
  url: string;
}

export interface MockStatus {
  running: boolean;
  url?: string;
  port?: number;
  routeCount: number;
}

// ==================== 对象管理 ====================

/** 对象分组（分组名支持 "父级/子级" 斜杠实现多级） */
export interface ObjectGroup {
  id: string;
  name: string;
  /** 已废弃标记（展示用，不影响功能） */
  deprecated: boolean;
}

/** 对象属性类型 */
export const PROP_KINDS = ["String", "Integer", "Float", "Boolean", "Datetime", "Date", "Time", "List", "Object", "Any"] as const;

export interface ObjectProp {
  key: string;
  /** string / number / boolean / object / list / any */
  kind: string;
  /** list 的元素类型（string / number / boolean / datetime / date / time / object / any） */
  itemKind: string;
  /** object / list(object) 引用的对象 hash */
  refHash: string;
  description: string;
  /** mock 值（示例数据，不参与结构 hash） */
  mock: string;
}

export interface ObjectDef {
  /** 稳定标识（不随属性变化，用于版本管理 .object_version/<uuid>/） */
  uuid: string;
  /** 唯一标识：属性按 key 排序拼接后的 SHA-256 前 12 位 */
  hash: string;
  /** 英文标识名（字母开头，仅字母数字，即文件名 <名称>.obj.json） */
  name: string;
  /** 代码生成类名（可空；不设置则不生成代码，格式：字母开头，仅字母/数字/下划线） */
  object_name?: string;
  /** Java 包名（可空；生成 Java 代码时输出 package 语句，格式：小写字母开头，点分隔） */
  package_name?: string;
  /** 显示名称（展示用，可为中文等任意文本；为空时回退显示 name） */
  displayName?: string;
  /** 所属分组 id（空串为未分组） */
  group: string;
  /** 已废弃标记（展示用，不影响功能） */
  deprecated: boolean;
  description: string;
  properties: ObjectProp[];
  createdAt: number;
  updatedAt: number;
}

export interface ObjectStore {
  groups: ObjectGroup[];
  objects: ObjectDef[];
}

/** 对象版本信息（.object_version/<uuid>/<n>.json） */
export interface ObjectVersionInfo {
  version: number;
  savedAt: number;
  name: string;
  description: string;
  propCount: number;
  hash: string;
}

/** 对象被接口文档引用的统计（接口数量 + 引用接口列表） */
export interface ObjectUsageApi {
  name: string;
  method: string;
  path: string;
  protocol: string;
}

export interface ObjectUsageItem {
  hash: string;
  apiCount: number;
  apis: ObjectUsageApi[];
}

/** JSON 导入结果 */
export interface ObjectImportResult {
  objects: ObjectDef[];
  created: string[];
  reused: string[];
  /** 顶层对象 hash（复用场景下指向已有对象） */
  topHash: string;
}

export const METHODS = [
  "GET",
  "POST",
  "PUT",
  "DELETE",
  "PATCH",
  "HEAD",
  "OPTIONS",
] as const;

export const BODY_MODES = ["none", "raw", "json", "xml", "form", "binary"] as const;

export function emptyKV(): KeyValue {
  return { key: "", value: "", enabled: true, description: "" };
}

export function emptyBody(): BodyData {
  return { mode: "none", raw: "", form: [], binaryPath: "" };
}

export function emptyMock(): MockConfig {
  return { enabled: false, status: 200, headers: [], delay: 0, body: "" };
}

export function emptyApi(): ApiFile {
  return {
    uuid: "",
    name: "未命名接口",
    method: "GET",
    path: "/",
    url: "",
    description: "",
    headers: [],
    query: [],
    params: [],
    body: emptyBody(),
    mock: emptyMock(),
    examples: [],
    responses: [emptyResponse("返回成功", 200), emptyResponse("返回失败", 400)],
    docParams: [],
    deprecated: false,
    protocol: "http",
  };
}
