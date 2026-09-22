import { ApiProtocol } from "../../types";
import { graphqlModule } from "./graphql/Index";
import { httpModule } from "./http/Index";
import { mcpModule } from "./mcp/Index";
import { socketioModule } from "./socketio/Index";
import { tcpModule } from "./tcp/Index";
import { ApiProtocolModule } from "./types";
import { udpModule } from "./udp/Index";
import { webdavModule } from "./webdav/Index";
import { webhookModule } from "./webhook/Index";
import { websocketModule } from "./websocket/Index";

/**
 * 协议页面模块注册表（key 为 api.protocol）。
 * 新增协议时：在协议目录下实现 Index.tsx，然后在此登记。
 */
export const API_MODULES: Record<ApiProtocol, ApiProtocolModule> = {
  http: httpModule,
  webhook: webhookModule,
  websocket: websocketModule,
  socketio: socketioModule,
  graphql: graphqlModule,
  mcp: mcpModule,
  webdav: webdavModule,
  tcp: tcpModule,
  udp: udpModule,
};
