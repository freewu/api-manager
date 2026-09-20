import { Editor } from "../common/Editor";
import { WsResponse } from "./WsResponse";
import { ApiProtocolModule } from "../types";

/** WebSocket 接口页面模块：消息编辑复用 common 下的通用编辑器，响应面板为本目录的实时交互记录 */
export const websocketModule: ApiProtocolModule = {
  protocol: "websocket",
  renderEditor: (ctx) => (
    <Editor
      style={{ height: ctx.hideResponse ? "100%" : `${ctx.editorRatio * 100}%` }}
      api={ctx.api}
      baseUrl={ctx.baseUrl}
      breadcrumb={ctx.breadcrumb}
      currentVersion={ctx.currentVersion}
      exampleVersion={ctx.exampleVersion}
      onChange={ctx.onChange}
      onSend={ctx.onSend}
      onSaveVersion={ctx.onSaveVersion}
      enableVersion={ctx.enableVersion}
      sending={ctx.sending}
      onCommit={ctx.onCommit}
      enableCodegen={ctx.enableCodegen}
      enableMock={ctx.enableMock}
      codegenLang={ctx.codegenLang}
      onTabChange={ctx.onTabChange}
      onEnvChanged={ctx.onEnvChanged}
      objectsList={ctx.objectsList}
      objectsStore={ctx.objectsStore}
    />
  ),
  renderResponse: (ctx) => (
    <WsResponse
      connected={ctx.wsConnected}
      connecting={ctx.wsConnecting}
      entries={ctx.wsEntries}
      onDisconnect={ctx.onWsDisconnect}
      onSaveExample={ctx.onSaveExample}
    />
  ),
};
