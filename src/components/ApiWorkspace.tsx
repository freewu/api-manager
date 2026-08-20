import { ApiFile, HttpResult, WsLogEntry } from "../types";
import { Editor } from "./Editor";
import { Response } from "./Response";
import { WsResponse } from "./WsResponse";

interface Props {
  api: ApiFile;
  baseUrl: string;
  currentVersion?: number;
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
}

/**
 * 右侧渲染按接口类型拆分为 Http / WebSocket 两种：
 * - Http      ：Editor（请求编辑）+ Response（状态码 / 响应体）
 * - WebSocket ：Editor（消息编辑 / 发送）+ WsResponse（实时交互记录）
 * 根据 api.protocol 加载不同的组件。
 */
export function ApiWorkspace({
  api,
  baseUrl,
  currentVersion = 0,
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
}: Props) {
  const isWs = api.protocol === "websocket";

  const editor = (
    <Editor
      style={{ height: hideResponse ? "100%" : `${editorRatio * 100}%` }}
      api={api}
      baseUrl={baseUrl}
      currentVersion={currentVersion}
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
          />
        ) : (
          <Response result={response} sending={sending} onSaveExample={onSaveExample} />
        ))}
    </>
  );
}
