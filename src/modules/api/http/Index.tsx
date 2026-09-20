import { Editor } from "../common/Editor";
import { Response } from "./Response";
import { ApiProtocolModule } from "../types";

/** HTTP / HTTPS 接口页面模块：请求编辑复用 common 下的通用编辑器，响应面板为本目录的 HTTP 响应视图 */
export const httpModule: ApiProtocolModule = {
  protocol: "http",
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
    <Response result={ctx.response} sending={ctx.sending} onSaveExample={ctx.onSaveExample} />
  ),
};
