import { NetEditor } from "../common/NetEditor";
import { NetResponse } from "../common/NetResponse";
import { ApiProtocolModule } from "../types";

/** UDP 接口页面模块：封包编辑与收发结果面板复用 common 下的通用网络组件 */
export const udpModule: ApiProtocolModule = {
  protocol: "udp",
  renderEditor: (ctx) => (
    <NetEditor
      style={{ height: ctx.hideResponse ? "100%" : `${ctx.editorRatio * 100}%` }}
      api={ctx.api}
      baseUrl={ctx.baseUrl}
      breadcrumb={ctx.breadcrumb}
      currentVersion={ctx.currentVersion}
      onChange={ctx.onChange}
      onSend={ctx.onSend}
      onSaveVersion={ctx.onSaveVersion}
      enableVersion={ctx.enableVersion}
      sending={ctx.sending}
      onCommit={ctx.onCommit}
      enableCodegen={ctx.enableCodegen}
      codegenLang={ctx.codegenLang}
      onTabChange={ctx.onTabChange}
    />
  ),
  renderResponse: (ctx) => (
    <NetResponse
      result={ctx.netResult ?? null}
      sending={ctx.sending}
      fields={ctx.api.unpack || []}
      onSaveExample={ctx.onSaveExample}
    />
  ),
};
