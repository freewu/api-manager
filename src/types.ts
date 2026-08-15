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
  mode: "none" | "raw" | "json" | "form";
  raw: string;
  form: KeyValue[];
}

export interface MockConfig {
  enabled: boolean;
  status: number;
  headers: KeyValue[];
  delay: number;
  body: string;
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
  /** 入参文档：请求参数的补充说明（类型 / 说明），按 source+key 关联 */
  docParams: DocParam[];
}

export interface DocParam {
  source: "query" | "path" | "body";
  key: string;
  type: string;
  description: string;
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
});

export interface HttpRequestData {
  method: string;
  url: string;
  headers: KeyValue[];
  body?: string;
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

export interface MockStatus {
  running: boolean;
  url?: string;
  port?: number;
  routeCount: number;
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

export const BODY_MODES = ["none", "raw", "json", "form"] as const;

export function emptyKV(): KeyValue {
  return { key: "", value: "", enabled: true, description: "" };
}

export function emptyBody(): BodyData {
  return { mode: "none", raw: "", form: [] };
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
    docParams: [],
  };
}
