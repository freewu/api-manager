import { MqEditor } from "../common/MqEditor";
import { ApiProtocolModule } from "../types";

/**
 * MQ（消息队列）接口页面模块：连接配置（MQ 类型 / ip / port / topic）与生产 / 消费代码生成。
 * MQ 接口不直接连接 Broker，因此没有「发送」按钮与响应面板。
 */
export const mqModule: ApiProtocolModule = {
  protocol: "mq",
  renderEditor: (ctx) => (
    <MqEditor
      style={{ height: "100%" }}
      api={ctx.api}
      baseUrl={ctx.baseUrl}
      breadcrumb={ctx.breadcrumb}
      currentVersion={ctx.currentVersion}
      onChange={ctx.onChange}
      onSaveVersion={ctx.onSaveVersion}
      enableVersion={ctx.enableVersion}
      onCommit={ctx.onCommit}
      enableCodegen={ctx.enableCodegen}
      codegenLang={ctx.codegenLang}
      onTabChange={ctx.onTabChange}
    />
  ),
  /** MQ 无响应面板（不直连 Broker） */
  renderResponse: () => null,
};
