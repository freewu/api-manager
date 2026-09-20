import { API_MODULES } from "../registry";
import { ApiPageContext } from "../types";

/**
 * 右侧接口工作区：编辑区 + 分隔条 + 响应区。
 * 具体页面由各协议目录的 `Index.tsx` 提供（见 `../registry.ts`），按 `api.protocol` 选用：
 * - Http                ：Editor（请求编辑）+ Response（状态码 / 响应体）
 * - 实时（WS / Socket.IO）：Editor（消息编辑 / 发送）+ WsResponse（实时交互记录）
 * - TCP / UDP           ：NetEditor（封包 / 解包）+ NetResponse（收发字节与解包解析）
 * - GraphQL / WebDAV / MCP：Editor + Response
 */
export function ApiWorkspace(props: ApiPageContext) {
  const { api, hideResponse, onStartVResize, onResetRatio } = props;
  /** 未知 / 历史协议数据回退到 HTTP 页面（与拆分前行为一致） */
  const mod = API_MODULES[api.protocol] ?? API_MODULES.http;

  return (
    <>
      {mod.renderEditor(props)}
      {!hideResponse && (
        <div
          className="v-resizer"
          onMouseDown={onStartVResize}
          onDoubleClick={onResetRatio}
          title=""
        />
      )}
      {!hideResponse && mod.renderResponse(props)}
    </>
  );
}
