import { ApiFile, HttpResult, ObjectDef, ObjectStore, WsLogEntry } from "../types";
import { Editor } from "./Editor";
import { Response } from "./Response";
import { WsResponse } from "./WsResponse";

interface Props {
  api: ApiFile;
  baseUrl: string;
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
  onChange: (a: ApiFile) => void;
  onSend: () => void;
  onSaveExample: (name: string) => void;
  onSaveVersion: () => void;
  onCommit?: () => void;
  onTabChange?: (t: string) => void;
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
 * 右侧渲染按接口类型拆分为 Http / 实时（WebSocket、Socket.IO）两种：
 * - Http      ：Editor（请求编辑）+ Response（状态码 / 响应体）
 * - 实时       ：Editor（消息编辑 / 发送）+ WsResponse（实时交互记录）
 * 根据 api.protocol 加载不同的组件。
 */
export function ApiWorkspace({
  api,
  baseUrl,
  currentVersion = 0,
  exampleVersion = 0,
  enableVersion,
  enableCodegen,
  enableMock,
  codegenLang,
  sending,
  hideResponse,
  editorRatio,
  response,
  onChange,
  onSend,
  onSaveExample,
  onSaveVersion,
  onCommit,
  onTabChange,
  onStartVResize,
  onResetRatio,
  wsConnected,
  wsConnecting,
  wsEntries,
  onWsDisconnect,
  objectsList,
  objectsStore,
}: Props) {
  /** 实时类接口（WebSocket / Socket.IO）：右侧渲染为消息编辑 + 实时交互记录 */
  const isWs = api.protocol === "websocket" || api.protocol === "socketio";

  const editor = (
    <Editor
      style={{ height: hideResponse ? "100%" : `${editorRatio * 100}%` }}
      api={api}
      baseUrl={baseUrl}
      currentVersion={currentVersion}
      exampleVersion={exampleVersion}
      onChange={onChange}
      onSend={onSend}
      onSaveVersion={onSaveVersion}
      enableVersion={enableVersion}
      sending={sending}
      onCommit={onCommit}
      enableCodegen={enableCodegen}
      enableMock={enableMock}
      codegenLang={codegenLang}
      onTabChange={onTabChange}
      objectsList={objectsList}
      objectsStore={objectsStore}
    />
  );

  return (
    <>
      {editor}
      {!hideResponse && (
        <div
          className="v-resizer"
          onMouseDown={onStartVResize}
          onDoubleClick={onResetRatio}
          title=""
        />
      )}
      {!hideResponse &&
        (isWs ? (
          <WsResponse
            connected={wsConnected}
            connecting={wsConnecting}
            entries={wsEntries}
            onDisconnect={onWsDisconnect}
            onSaveExample={onSaveExample}
          />
        ) : (
          <Response result={response} sending={sending} onSaveExample={onSaveExample} />
        ))}
    </>
  );
}
