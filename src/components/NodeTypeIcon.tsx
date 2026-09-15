import iconHttp from "../assets/icon-http.png";
import iconWs from "../assets/icon-websocket.png";
import iconGql from "../assets/icon-graphql.png";
import iconSocketIo from "../assets/icon-socketio.png";
import iconWebdav from "../../asserts/icon/WebDAV.png";
import iconTcp from "../../asserts/icon/TCP.png";
import iconUdp from "../../asserts/icon/UDP.png";

/**
 * 接口协议类型图标：HTTP / WebSocket / Socket.IO / GraphQL / WebDAV / TCP / UDP
 * 左侧接口列表、收藏列表、导出弹窗接口树共用，保证图标一致。
 */
export function NodeTypeIcon({
  protocol,
  className = "node-type-icon",
}: {
  protocol?: string;
  className?: string;
}) {
  const src =
    protocol === "websocket"
      ? iconWs
      : protocol === "socketio"
        ? iconSocketIo
        : protocol === "graphql"
          ? iconGql
          : protocol === "webdav"
            ? iconWebdav
            : protocol === "tcp"
              ? iconTcp
              : protocol === "udp"
                ? iconUdp
                : iconHttp;
  const alt =
    protocol === "websocket"
      ? "WebSocket"
      : protocol === "socketio"
        ? "Socket.IO"
        : protocol === "graphql"
          ? "GraphQL"
          : protocol === "webdav"
            ? "WebDAV"
            : protocol === "tcp"
              ? "TCP"
              : protocol === "udp"
                ? "UDP"
                : "HTTP";
  return <img className={className} src={src} alt={alt} />;
}
