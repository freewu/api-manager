// ---- 与 Rust 后端对应的类型定义 ----
import { DEFAULT_SHORTCUTS, ShortcutAction } from "./utils/shortcuts";

/** 接口协议类型：http=HTTP / websocket·socketio=实时 / graphql·webdav·mcp=HTTP 形态 / tcp·udp=网络封包 / mq=消息队列 */
export type ApiProtocol =
  | "http"
  | "webhook"
  | "websocket"
  | "graphql"
  | "socketio"
  | "webdav"
  | "mcp"
  | "tcp"
  | "udp"
  | "mq";

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

/** 报文字段类型：fixed=固定值 / var=变量 / varlen=不定长变量 */
export type PacketFieldKind = "fixed" | "var" | "varlen";

/** TCP / UDP 报文字段定义（封包 / 解包共用同一结构） */
export interface PacketField {
  /** 字段英文标识 */
  key: string;
  kind: PacketFieldKind;
  /** 位数：占几个字节；不定长变量忽略此值 */
  bytes: number;
  /** 值：0x 开头按 hex 解析，否则按 UTF-8 文本编码（不定长变量为其内容） */
  value: string;
  description: string;
  /** 不定长变量：长度取自第几个字段（0 基下标），缺省表示前一个字段 */
  lenFrom?: number;
}

/** TCP / UDP 连接配置（ip:port） */
export interface NetConfig {
  host: string;
  port: number;
  /** 超时（毫秒） */
  timeoutMs: number;
}

/** TCP / UDP 一次收发的原始字节与统计信息 */
export interface NetResult {
  ok: boolean;
  /** 接收到的字节（hex，空格分隔） */
  hex: string;
  /** 接收到的字节（可打印字符文本） */
  text: string;
  size: number;
  /** 发送的字节（hex，空格分隔） */
  sentHex: string;
  sentSize: number;
  timeMs: number;
  /** UDP 对端地址 */
  from?: string;
  error?: string;
}

export function emptyPacketField(kind: PacketFieldKind = "var", bytes = 1): PacketField {
  return { key: "", kind, bytes, value: "", description: "" };
}

export function emptyNet(): NetConfig {
  return { host: "127.0.0.1", port: 0, timeoutMs: 3000 };
}

/**
 * 消息队列类型：
 *   - 原生协议：kafka / rabbitmq / rocketmq / activemq / zeromq / pulsar / nats
 *   - MQTT 系：emqx / hivemq / mosquitto / nanomq / vernemq
 */
export type MqKind =
  | "kafka"
  | "rabbitmq"
  | "rocketmq"
  | "activemq"
  | "zeromq"
  | "pulsar"
  | "emqx"
  | "hivemq"
  | "mosquitto"
  | "nanomq"
  | "nats"
  | "vernemq";

/** MQ 类型清单（含默认端口，切换类型时用于填充默认端口；MQTT 系默认 1883） */
export const MQ_KINDS: { value: MqKind; label: string; port: number }[] = [
  { value: "kafka", label: "Kafka", port: 9092 },
  { value: "rabbitmq", label: "RabbitMQ", port: 5672 },
  { value: "rocketmq", label: "RocketMQ", port: 9876 },
  { value: "activemq", label: "ActiveMQ", port: 61616 },
  { value: "zeromq", label: "ZeroMQ", port: 5555 },
  { value: "pulsar", label: "Pulsar", port: 6650 },
  { value: "emqx", label: "EMQX", port: 1883 },
  { value: "hivemq", label: "HiveMQ", port: 1883 },
  { value: "mosquitto", label: "Mosquitto", port: 1883 },
  { value: "nanomq", label: "NanoMQ", port: 1883 },
  { value: "nats", label: "NATS", port: 4222 },
  { value: "vernemq", label: "VerneMQ", port: 1883 },
];

/** MQ（消息队列）连接与消费配置 */
export interface MqConfig {
  /** 消息队列类型 */
  type: MqKind;
  host: string;
  port: number;
  /** 主题 / 队列名称 */
  topic: string;
  /** 消费组 */
  group: string;
  /** 消费起始位置：earliest（最早）/ latest（最新） */
  offset: "earliest" | "latest";
  /** 单次消费最多拉取的消息条数 */
  maxMessages: number;
  /** 超时（毫秒） */
  timeoutMs: number;
}

export function emptyMq(kind: MqKind = "kafka"): MqConfig {
  const preset = MQ_KINDS.find((k) => k.value === kind);
  return {
    type: kind,
    host: "127.0.0.1",
    port: preset?.port ?? 9092,
    topic: "",
    group: "",
    offset: "latest",
    maxMessages: 1,
    timeoutMs: 3000,
  };
}

/** MQ 类型显示名 */
export function mqKindLabel(kind?: string): string {
  return MQ_KINDS.find((k) => k.value === kind)?.label ?? "MQ";
}

/** 是否为 MQ（消息队列）接口协议（无 HTTP 方法 / path / Mock 概念，也不直连发送） */
export function isMqProtocol(protocol?: string): boolean {
  return protocol === "mq";
}

/** 是否为 TCP / UDP 接口协议（无 HTTP 方法 / path / Mock 概念） */
export function isNetProtocol(protocol?: string): boolean {
  return protocol === "tcp" || protocol === "udp";
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
  /** HTTP 接口前置脚本（发送请求前执行的 JS，可读写全局变量 / 计算签名） */
  prescript: string;
  examples: unknown[];
  /** 响应页签条目：返回成功 / 返回失败 / 自定义错误返回 */
  responses: ResponseItem[];
  /** 入参文档：请求参数的补充说明（类型 / 说明），按 source+key 关联 */
  docParams: DocParam[];
  /** 是否已标记废弃 */
  deprecated: boolean;
  /** 接口协议：http / webhook / websocket / socketio / graphql / webdav / mcp / tcp / udp */
  protocol: ApiProtocol;
  /** 封包字段定义（TCP / UDP） */
  pack?: PacketField[];
  /** 解包字段定义（TCP / UDP） */
  unpack?: PacketField[];
  /** TCP / UDP 连接配置（ip:port） */
  net?: NetConfig;
  /** MQ（消息队列）连接与消费配置 */
  mq?: MqConfig;
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
export const DOC_TYPES = [
  "String",
  "Integer",
  "Float",
  "Datetime",
  "Date",
  "Time",
  "Boolean",
  "List",
  "Object",
  "Any",
];

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
  /** TCP / UDP 示例：协议（tcp / udp） */
  protocol?: "tcp" | "udp" | "mq";
  /** TCP / UDP 示例：连接配置（保存时的目标地址） */
  net?: NetConfig;
  /** TCP / UDP 示例：封包字段定义（用于还原请求报文） */
  pack?: PacketField[];
  /** TCP / UDP 示例：解包字段定义（用于解析响应报文） */
  unpack?: PacketField[];
  /** MQ 示例：连接与消费配置（保存时的配置） */
  mq?: MqConfig;
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
  collapsed?: boolean;
  /** 分组是否已标记废弃 */
  deprecated?: boolean;
  /** 最近一次选中的接口（相对工作区根目录的路径），重开工作区时默认选中 */
  selectedApi?: string;
  /** 收藏的接口 uuid 列表（按显示顺序，保存在工作区根目录 __info.json） */
  favorites?: string[];
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
  /** 接口协议（http / websocket / ... / mcp / tcp / udp / mq，分组无此字段） */
  protocol?: ApiProtocol;
  /** 接口 uuid（仅接口节点有，用于收藏等按 uuid 关联的场景） */
  uuid?: string;
  /** MQ 接口的消息队列类型（kafka / rabbitmq / rocketmq / activemq / zeromq / pulsar / nats / emqx / hivemq / mosquitto / nanomq / vernemq） */
  mqType?: string;
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
  | "openapi-yaml"
  | "docsify"
  | "mkdocs"
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

/**
 * 支持导出 TCP / UDP 接口的格式：仅文档类（HTML / Markdown / MkDocs / Docsify），
 * 其余格式（Postman / OpenAPI / YApi …）没有报文概念，导出时这些接口置灰不可选。
 */
export const NET_EXPORT_FORMATS: ExportFormat[] = ["html", "markdown", "mkdocs", "docsify"];

/** 当前导出格式是否支持 TCP / UDP 接口 */
export function supportsNetExport(format: ExportFormat): boolean {
  return NET_EXPORT_FORMATS.includes(format);
}

/** 导入格式中必选（不可关闭）的类型 */
export const REQUIRED_IMPORT_FORMATS: ImportFormat[] = ["postman", "openapi"];

/** 导出格式中必选（不可关闭）的类型 */
export const REQUIRED_EXPORT_FORMATS: ExportFormat[] = [
  "postman",
  "openapi",
  "openapi-yaml",
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
  /** 导出默认格式（默认 OpenAPI 3.0 YAML） */
  exportFormat: ExportFormat;
  /** 主页导入按钮总开关（false 时隐藏「导入」按钮） */
  importEnabled: boolean;
  /** 主页导出按钮总开关（false 时隐藏「导出」按钮） */
  exportEnabled: boolean;
  /** 主页导入菜单展示的格式开关（postman/openapi 必选不可关闭） */
  importTypes: Record<ImportFormat, boolean>;
  /** 导出弹窗格式下拉展示的格式开关（postman/openapi/openapi-yaml/docsify/markdown/html 必选不可关闭） */
  exportTypes: Record<ExportFormat, boolean>;
  /** HTML 文档悬浮导航栏位置（off 关闭 / left 左侧 / right 右侧） */
  htmlNav: "off" | "left" | "right";
  /** 界面语言（zh / en） */
  language: "zh" | "zh-tw" | "en";
  /** 新建分组默认开合状态（expanded / collapsed，接口管理与对象管理共用） */
  defaultFolderState: "expanded" | "collapsed";
  /** 最近打开的工作目录数量上限（最少 3） */
  recentLimit: number;
  /** 全局快捷键（动作 → 归一化按键，如 ctrl+a；"" 表示未绑定） */
  shortcuts: Record<ShortcutAction, string>;
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
  exportFormat: "openapi-yaml",
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
    "openapi-yaml": true,
    apifox: true,
    apipost: true,
    docsify: true,
    mkdocs: true,
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
  defaultFolderState: "expanded",
  recentLimit: 5,
  shortcuts: { ...DEFAULT_SHORTCUTS },
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

/** 自定义 Mock 占位符（保存在工作目录 .mock/<name>.js，name 不含 @） */
export interface CustomMock {
  /** 占位符标识（不含 @），使用时写作 @name */
  name: string;
  /** 是否启用（未启用不展示、不参与生成） */
  enabled: boolean;
  /** 说明文字 */
  desc: string;
  /** JS 代码：(ctx) => 返回值，ctx 提供 randInt/pick/random 等工具 */
  code: string;
}

// ==================== 对象管理 ====================

/** 对象分组（分组名支持 "父级/子级" 斜杠实现多级） */
export interface ObjectGroup {
  id: string;
  name: string;
  /** 已废弃标记（展示用，不影响功能） */
  deprecated: boolean;
  /** 同级排序序号（越小越靠前） */
  order?: number;
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
  /** 同级排序序号（越小越靠前） */
  order?: number;
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

/** 前置脚本运行结果（console 日志 / 返回值 / 更新后的全局变量） */
export interface PrescriptResult {
  logs: string[];
  result: string;
  globals: Record<string, string>;
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

/** Webhook 仅支持 GET / POST 两种请求方法 */
export const WEBHOOK_METHODS = ["GET", "POST"] as const;

/** WebDAV 扩展方法（除标准 HTTP 方法外的 WebDAV 专用协议） */
export const WEBDAV_METHODS = [
  "PROPFIND",
  "PROPPATCH",
  "MKCOL",
  "COPY",
  "MOVE",
  "LOCK",
  "UNLOCK",
  "REPORT",
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
    prescript: "",
    examples: [],
    responses: [emptyResponse("返回成功", 200), emptyResponse("返回失败", 400)],
    docParams: [],
    deprecated: false,
    protocol: "http",
  };
}
