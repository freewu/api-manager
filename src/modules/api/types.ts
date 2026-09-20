import { ReactNode } from "react";
import { ApiFile, ApiProtocol, HttpResult, NetResult, ObjectDef, ObjectStore, WsLogEntry } from "../../types";

/**
 * 协议页面渲染上下文：即 `ApiWorkspace` 的全部 props。
 * 各协议目录的 `Index.tsx` 实现页面模块时按需取用（不同协议用到的字段不同）。
 */
export interface ApiPageContext {
  api: ApiFile;
  baseUrl: string;
  /** 右侧接口面包屑：工作区名称 / …/ 分组名称 / 接口名称 */
  breadcrumb?: string[];
  currentVersion?: number;
  /** 示例保存版本号：保存示例成功后自增，用于刷新「示例」角标 */
  exampleVersion?: number;
  enableVersion: boolean;
  enableCodegen: boolean;
  enableMock: boolean;
  codegenLang: string;
  sending: boolean;
  /** 响应面板是否隐藏（Mock/描述/文档/代码 页签下隐藏） */
  hideResponse: boolean;
  editorRatio: number;
  response: HttpResult | null;
  /** TCP / UDP 最近一次收发结果 */
  netResult?: NetResult | null;
  onChange: (a: ApiFile) => void;
  onSend: () => void;
  onSaveExample: (name: string) => void;
  onSaveVersion: () => void;
  onCommit?: () => void;
  onTabChange?: (t: string) => void;
  /** 前置脚本全局变量（即环境变量）被修改后的回调（App 据此刷新环境面板） */
  onEnvChanged?: () => void;
  onStartVResize: (e: React.MouseEvent) => void;
  onResetRatio: () => void;
  // WebSocket 交互记录相关（仅 WS 接口使用）
  wsConnected: boolean;
  wsConnecting: boolean;
  wsEntries: WsLogEntry[];
  onWsDisconnect: () => void;
  /** 已定义对象列表（文档页签 Object 类型可引用） */
  objectsList?: ObjectDef[];
  /** 完整对象仓库（含分组），文档页签 Object 类型弹窗选择对象用（与对象管理一致） */
  objectsStore?: ObjectStore;
}

/**
 * 协议页面模块：每个协议目录的 `Index.tsx` 导出一个，`ApiWorkspace` 按 `api.protocol` 选用。
 * - 编辑区：HTTP / 实时类复用 common 的 Editor，TCP / UDP 用 common 的 NetEditor
 * - 响应区：各协议响应面板 props 不同，由协议模块自行组装
 */
export interface ApiProtocolModule {
  /** 协议标识 */
  protocol: ApiProtocol;
  /** 编辑区（请求 / 消息 / 封包编辑） */
  renderEditor: (ctx: ApiPageContext) => ReactNode;
  /** 响应区（状态码 / 交互记录 / 收发结果） */
  renderResponse: (ctx: ApiPageContext) => ReactNode;
}
