import iconHttp from "../../assets/icon-http.png";
import iconWs from "../../assets/icon-websocket.png";
import iconGql from "../../assets/icon-graphql.png";
import iconSocketIo from "../../assets/icon-socketio.png";
import iconWebdav from "../../../asserts/icon/WebDAV.png";
import iconMcp from "../../../asserts/icon/mcp.png";
import iconTcp from "../../../asserts/icon/TCP.png";
import iconUdp from "../../../asserts/icon/UDP.png";
import iconWebhook from "../../../asserts/icon/webhook.png";
import iconMq from "../../../asserts/icon/MQ.png";
import iconKafka from "../../../asserts/mq/Kafka.png";
import iconRabbitMq from "../../../asserts/mq/RabbitMQ.png";
import iconRocketMq from "../../../asserts/mq/rocketmq.png";
import iconActiveMq from "../../../asserts/mq/ActiveMQ.png";
import iconZeroMq from "../../../asserts/mq/zeromq.png";

/** MQ 类型 → 品牌图标（列表与 MQ 配置栏按所选消息队列展示对应图标） */
export const MQ_ICONS: Record<string, string> = {
  kafka: iconKafka,
  rabbitmq: iconRabbitMq,
  rocketmq: iconRocketMq,
  activemq: iconActiveMq,
  zeromq: iconZeroMq,
};

/** MQ 类型 → 图标 alt / title */
const MQ_LABELS: Record<string, string> = {
  kafka: "Kafka",
  rabbitmq: "RabbitMQ",
  rocketmq: "RocketMQ",
  activemq: "ActiveMQ",
  zeromq: "ZeroMQ",
};

/**
 * 接口协议类型图标：HTTP / Webhook / WebSocket / Socket.IO / GraphQL / WebDAV / MCP / TCP / UDP / MQ。
 * 左侧接口列表、收藏列表、导出弹窗接口树共用，保证图标一致。
 * MQ 接口按消息队列类型（mqType）展示对应品牌图标，未知类型回退到通用 MQ 图标。
 */
export function NodeTypeIcon({
  protocol,
  mqType,
  className = "node-type-icon",
}: {
  protocol?: string;
  /** MQ 接口的消息队列类型（kafka / rabbitmq / rocketmq / activemq / zeromq） */
  mqType?: string;
  className?: string;
}) {
  if (protocol === "mq") {
    const kind = mqType || "kafka";
    return <img className={className} src={MQ_ICONS[kind] || iconMq} alt={MQ_LABELS[kind] || "MQ"} />;
  }
  const src =
    protocol === "webhook"
      ? iconWebhook
      : protocol === "websocket"
        ? iconWs
        : protocol === "socketio"
          ? iconSocketIo
          : protocol === "graphql"
            ? iconGql
            : protocol === "webdav"
              ? iconWebdav
              : protocol === "mcp"
                ? iconMcp
                : protocol === "tcp"
                  ? iconTcp
                  : protocol === "udp"
                    ? iconUdp
                    : iconHttp;
  const alt =
    protocol === "webhook"
      ? "Webhook"
      : protocol === "websocket"
        ? "WebSocket"
        : protocol === "socketio"
          ? "Socket.IO"
          : protocol === "graphql"
            ? "GraphQL"
            : protocol === "webdav"
              ? "WebDAV"
              : protocol === "mcp"
                ? "MCP"
                : protocol === "tcp"
                  ? "TCP"
                  : protocol === "udp"
                    ? "UDP"
                    : "HTTP";
  return <img className={className} src={src} alt={alt} />;
}
