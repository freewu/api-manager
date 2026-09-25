/**
 * MQ（消息队列）代码生成：生成「生产 / 消费」示例代码。
 *
 * 支持的消息队列：
 *   - 原生协议：Kafka / RabbitMQ / RocketMQ / ActiveMQ / ZeroMQ / Pulsar / NATS
 *   - MQTT 系（共用 MQTT 客户端）：EMQX / HiveMQ / Mosquitto / NanoMQ / VerneMQ
 *
 * 与 net.ts（TCP / UDP 封包）不同，MQ 代码不涉及报文字节，而是按语言客户端库给出连接 + 生产 / 消费调用。
 * 已内置代码生成的语言：Bash / Python / JavaScript / TypeScript / Java / Kotlin / Go / C# / PHP / Ruby；
 * 其余语言给出等价命令行提示（避免编造不存在的客户端库用法）。
 */
import { ApiFile, MqConfig, MqKind, emptyMq, mqKindLabel } from "../../types";
import { CodeLang, CodeLibOption } from "./shared";

/** 代码生成方向：生产 / 消费 */
export type MqDirection = "produce" | "consume";

/** 生成代码所需的 MQ 参数 */
export interface MqReq {
  kind: MqKind;
  kindLabel: string;
  host: string;
  port: number;
  /** 主题 / 队列名 */
  topic: string;
  /** 消费组 */
  group: string;
  /** earliest / latest */
  offset: string;
  maxMessages: number;
  timeoutMs: number;
  /** 生产消息内容 */
  body: string;
  /** broker 地址 host:port */
  target: string;
}

/** MQ 语言可选客户端库（当前各语言使用其主流客户端，暂不提供多库切换） */
export const MQ_CODE_LIBS: Partial<Record<CodeLang, CodeLibOption[]>> = {};

export function buildMqReq(api: ApiFile): MqReq {
  const mq: MqConfig = api.mq || emptyMq();
  const host = (mq.host || "127.0.0.1").trim() || "127.0.0.1";
  const port = mq.port || 0;
  const topic = (mq.topic || "").trim() || "test-topic";
  const body = api.body?.raw ?? "";
  return {
    kind: (mq.type || "kafka") as MqKind,
    kindLabel: mqKindLabel(mq.type),
    host,
    port,
    topic,
    group: (mq.group || "").trim() || `${topic}-group`,
    offset: mq.offset === "earliest" ? "earliest" : "latest",
    maxMessages: mq.maxMessages > 0 ? mq.maxMessages : 1,
    timeoutMs: mq.timeoutMs > 0 ? mq.timeoutMs : 3000,
    body,
    target: `${host}:${port}`,
  };
}

// ---------------------------------------------------------------- 通用工具

/** JSON 风格字符串字面量（Python / JS / Java / Go / C# / PHP / Ruby / Kotlin 等均可直接使用） */
const j = (s: string) => JSON.stringify(s);
/** 单引号壳命令行参数（Bash 用） */
const sh = (s: string) => `'${s.replace(/'/g, `'\\''`)}'`;
/** 是否「最早」起始（决定 from-beginning / earliest 等参数） */
const earliest = (r: MqReq) => r.offset === "earliest";
/** 秒级超时（Python / Node 等回调式 API 用） */
const secs = (r: MqReq) => Math.max(1, Math.round(r.timeoutMs / 1000));
/** 去换行（用于单行命令行） */
const oneline = (s: string) => s.replace(/\r?\n/g, " ");

/** MQ 协议族：MQTT 系（EMQX / HiveMQ / Mosquitto / NanoMQ / VerneMQ）共用 MQTT 客户端代码 */
export type MqFamily =
  | "kafka"
  | "rabbitmq"
  | "rocketmq"
  | "activemq"
  | "zeromq"
  | "pulsar"
  | "nats"
  | "mqtt";

const MQ_FAMILY: Partial<Record<MqKind, MqFamily>> = {
  emqx: "mqtt",
  hivemq: "mqtt",
  mosquitto: "mqtt",
  nanomq: "mqtt",
  vernemq: "mqtt",
};

/** 归类到协议族（未列出的类型即自身） */
export function mqFamily(kind: MqKind): MqFamily {
  return MQ_FAMILY[kind] ?? (kind as MqFamily);
}


/** 各消息队列的 CLI 命令：Bash 语言直接输出；其余语言作为兜底提示 */
function cliCommands(r: MqReq, dir: MqDirection): string[] {
  switch (r.kind) {
    case "kafka":
      return dir === "produce"
        ? [
            `kafka-console-producer.sh --bootstrap-server ${r.target} --topic ${r.topic} <<'MSG'`,
            r.body || "",
            "MSG",
          ]
        : [
            `kafka-console-consumer.sh --bootstrap-server ${r.target} --topic ${r.topic} \\`,
            `  --group ${r.group}${earliest(r) ? " --from-beginning" : ""} \\`,
            `  --max-messages ${r.maxMessages} --timeout-ms ${r.timeoutMs}`,
          ];
    case "rabbitmq":
      return dir === "produce"
        ? [
            `rabbitmqadmin publish exchange=amq.default routing_key=${r.topic} \\`,
            `  payload=${sh(oneline(r.body))}`,
          ]
        : [`rabbitmqadmin get queue=${r.topic} ackmode=ack_requeue_false count=${r.maxMessages}`];
    case "rocketmq":
      return dir === "produce"
        ? [`sh mqadmin sendMessage -n ${r.target} -t ${r.topic} -p ${sh(oneline(r.body))}`]
        : [`sh mqadmin consumeMessage -n ${r.target} -t ${r.topic} -c ${r.group}`];
    case "activemq":
      return dir === "produce"
        ? [
            `activemq producer --brokerUrl tcp://${r.target} --destination queue://${r.topic} \\`,
            `  --message ${sh(oneline(r.body))} --messageCount 1`,
          ]
        : [
            `activemq consumer --brokerUrl tcp://${r.target} --destination queue://${r.topic} \\`,
            `  --messageCount ${r.maxMessages}`,
          ];
    case "emqx":
    case "hivemq":
    case "mosquitto":
    case "nanomq":
    case "vernemq":
      // MQTT 系：mosquitto 的 pub/sub 命令行可连接任意 MQTT Broker（含 EMQX / HiveMQ / NanoMQ / VerneMQ）
      return dir === "produce"
        ? [`mosquitto_pub -h ${r.host} -p ${r.port} -t ${r.topic} -m ${sh(oneline(r.body))}`]
        : [
            `mosquitto_sub -h ${r.host} -p ${r.port} -t ${r.topic} \\`,
            `  -C ${r.maxMessages} -W ${secs(r)} -q 1`,
          ];
    case "pulsar":
      return dir === "produce"
        ? [
            `pulsar-client produce ${r.topic} --url pulsar://${r.target} \\`,
            `  --messages ${sh(oneline(r.body))}`,
          ]
        : [
            `pulsar-client consume ${r.topic} --url pulsar://${r.target} \\`,
            `  --subscription-name ${r.group} --num-messages ${r.maxMessages}`,
          ];
    case "nats":
      return dir === "produce"
        ? [`nats pub ${r.topic} ${sh(oneline(r.body))} --server nats://${r.target}`]
        : [`nats sub ${r.topic} --server nats://${r.target} --count ${r.maxMessages}`];
    case "zeromq":
      return dir === "produce"
        ? [
            "python3 - <<'PY'",
            `# Topic: ${r.topic}（ZeroMQ 无 Topic 概念，SUB 端可用前缀过滤）`,
            "import zmq",
            "ctx = zmq.Context()",
            `sock = ctx.socket(zmq.PUSH); sock.connect("tcp://${r.target}")`,
            `sock.send_string(${j(r.body)})`,
            "sock.close(); ctx.term()",
            "PY",
          ]
        : [
            "python3 - <<'PY'",
            `# Topic: ${r.topic}（ZeroMQ 无 Topic 概念，SUB 端可用前缀过滤）`,
            "import zmq",
            "ctx = zmq.Context()",
            `sock = ctx.socket(zmq.PULL); sock.bind("tcp://${r.target}")`,
            `if sock.poll(${r.timeoutMs}): print(sock.recv_string())`,
            "sock.close(); ctx.term()",
            "PY",
          ];
  }
}

// ---------------------------------------------------------------- Bash

function genBash(r: MqReq, dir: MqDirection): string {
  return [
    `# ${r.kindLabel} ${dir === "produce" ? "生产" : "消费"}：topic ${r.topic} @ ${r.target}`,
    ...cliCommands(r, dir),
  ].join("\n");
}

// ---------------------------------------------------------------- Python

function genPython(r: MqReq, dir: MqDirection): string {
  const produce = dir === "produce";
  const body = j(r.body);
  switch (r.kind) {
    case "kafka":
      return produce
        ? [
            "# 依赖：pip install kafka-python",
            "from kafka import KafkaProducer",
            "",
            `producer = KafkaProducer(bootstrap_servers="${r.target}")`,
            `producer.send("${r.topic}", ${body}.encode("utf-8"))`,
            "producer.flush()",
            "producer.close()",
          ].join("\n")
        : [
            "# 依赖：pip install kafka-python",
            "from kafka import KafkaConsumer",
            "",
            "consumer = KafkaConsumer(",
            `    "${r.topic}",`,
            `    bootstrap_servers="${r.target}",`,
            `    group_id="${r.group}",`,
            `    auto_offset_reset="${r.offset}",`,
            `    consumer_timeout_ms=${r.timeoutMs},`,
            ")",
            "for msg in consumer:",
            '    print(msg.value.decode("utf-8"))',
            "    break",
          ].join("\n");
    case "rabbitmq":
      return produce
        ? [
            "# 依赖：pip install pika",
            "import pika",
            "",
            `conn = pika.BlockingConnection(pika.URLParameters("amqp://guest:guest@${r.target}/%2F"))`,
            "ch = conn.channel()",
            `ch.queue_declare(queue="${r.topic}", durable=True)`,
            `ch.basic_publish(exchange="", routing_key="${r.topic}", body=${body})`,
            "conn.close()",
          ].join("\n")
        : [
            "# 依赖：pip install pika",
            "import pika",
            "",
            `conn = pika.BlockingConnection(pika.URLParameters("amqp://guest:guest@${r.target}/%2F"))`,
            "ch = conn.channel()",
            `ch.queue_declare(queue="${r.topic}", durable=True)`,
            "for method, props, msg_body in ch.consume(",
            `    "${r.topic}", inactivity_timeout=${secs(r)}`,
            "):",
            "    if msg_body:",
            '        print(msg_body.decode("utf-8"))',
            "        break",
            "conn.close()",
          ].join("\n");
    case "rocketmq":
      return produce
        ? [
            "# 依赖：pip install rocketmq",
            "from rocketmq.client import Producer, Message",
            "",
            `producer = Producer("${r.group}")`,
            `producer.set_namesrv_addr("${r.target}")`,
            "producer.start()",
            `msg = Message("${r.topic}")`,
            `msg.set_body(${body})`,
            "producer.send_sync(msg)",
            "producer.shutdown()",
          ].join("\n")
        : [
            "# 依赖：pip install rocketmq",
            "from rocketmq.client import PushConsumer",
            "",
            "def on_message(msg):",
            '    print(msg.body.decode("utf-8"))',
            "",
            `consumer = PushConsumer("${r.group}")`,
            `consumer.set_namesrv_addr("${r.target}")`,
            `consumer.subscribe("${r.topic}", on_message)`,
            "consumer.start()",
          ].join("\n");
    case "activemq":
      return produce
        ? [
            "# 依赖：pip install stomp.py",
            "import stomp",
            "",
            `conn = stomp.Connection([("${r.host}", ${r.port})])`,
            'conn.connect("admin", "admin", wait=True)',
            `conn.send(destination="/queue/${r.topic}", body=${body})`,
            "conn.disconnect()",
          ].join("\n")
        : [
            "# 依赖：pip install stomp.py",
            "import stomp, time",
            "",
            "class Listener(stomp.ConnectionListener):",
            "    def on_message(self, frame):",
            "        print(frame.body)",
            "",
            `conn = stomp.Connection([("${r.host}", ${r.port})])`,
            'conn.set_listener("", Listener())',
            'conn.connect("admin", "admin", wait=True)',
            `conn.subscribe(destination="/queue/${r.topic}", id=1, ack="auto")`,
            `time.sleep(${secs(r)})  # 消费为异步回调，按需保持连接`,
            "conn.disconnect()",
          ].join("\n");
    default:
      // 新增消息队列（MQTT 系 / Pulsar / NATS）：由协议族函数生成
      return genMqFamily("python", r, dir);
    case "zeromq":
      return produce
        ? [
            "# 依赖：pip install pyzmq",
            "import zmq",
            "",
            `# Topic: ${r.topic}（ZeroMQ 无 Topic 概念，SUB 端可用前缀过滤）`,
            "ctx = zmq.Context()",
            "sock = ctx.socket(zmq.PUSH)",
            `sock.connect("tcp://${r.target}")`,
            `sock.send_string(${body})`,
            "sock.close()",
            "ctx.term()",
          ].join("\n")
        : [
            "# 依赖：pip install pyzmq",
            "import zmq",
            "",
            `# Topic: ${r.topic}（ZeroMQ 无 Topic 概念，SUB 端可用前缀过滤）`,
            "ctx = zmq.Context()",
            "sock = ctx.socket(zmq.PULL)",
            `sock.bind("tcp://${r.target}")  # ZeroMQ 无 Broker：消费端 bind，生产端 connect`,
            `if sock.poll(${r.timeoutMs}):`,
            "    print(sock.recv_string())",
            "sock.close()",
            "ctx.term()",
          ].join("\n");
  }
}

// ---------------------------------------------------------------- JavaScript / TypeScript

function genJs(r: MqReq, dir: MqDirection): string {
  const produce = dir === "produce";
  switch (r.kind) {
    case "kafka":
      return produce
        ? [
            "// 依赖：npm i kafkajs",
            'const { Kafka } = require("kafkajs");',
            "",
            `const kafka = new Kafka({ clientId: "api-manager", brokers: ["${r.target}"] });`,
            "const producer = kafka.producer();",
            "",
            "(async () => {",
            "  await producer.connect();",
            `  await producer.send({ topic: "${r.topic}", messages: [{ value: ${j(r.body)} }] });`,
            "  await producer.disconnect();",
            "})();",
          ].join("\n")
        : [
            "// 依赖：npm i kafkajs",
            'const { Kafka } = require("kafkajs");',
            "",
            `const kafka = new Kafka({ clientId: "api-manager", brokers: ["${r.target}"] });`,
            `const consumer = kafka.consumer({ groupId: "${r.group}" });`,
            "",
            "(async () => {",
            "  await consumer.connect();",
            `  await consumer.subscribe({ topic: "${r.topic}", fromBeginning: ${earliest(r)} });`,
            "  await consumer.run({",
            "    eachMessage: async ({ message }) => {",
            '      console.log(message.value?.toString() ?? "");',
            "    },",
            "  });",
            "})();",
          ].join("\n");
    case "rabbitmq":
      return produce
        ? [
            "// 依赖：npm i amqplib",
            'const amqp = require("amqplib");',
            "",
            "(async () => {",
            `  const conn = await amqp.connect("amqp://guest:guest@${r.target}");`,
            "  const ch = await conn.createChannel();",
            `  await ch.assertQueue("${r.topic}", { durable: true });`,
            `  ch.sendToQueue("${r.topic}", Buffer.from(${j(r.body)}));`,
            "  await ch.close();",
            "  await conn.close();",
            "})();",
          ].join("\n")
        : [
            "// 依赖：npm i amqplib",
            'const amqp = require("amqplib");',
            "",
            "(async () => {",
            `  const conn = await amqp.connect("amqp://guest:guest@${r.target}");`,
            "  const ch = await conn.createChannel();",
            `  await ch.assertQueue("${r.topic}", { durable: true });`,
            `  const msg = await ch.get("${r.topic}", { noAck: true });`,
            '  if (msg) console.log(msg.content.toString());',
            "  await ch.close();",
            "  await conn.close();",
            "})();",
          ].join("\n");
    case "rocketmq":
      return produce
        ? [
            "// 依赖：npm i rocketmq-client",
            'const { Producer } = require("rocketmq-client");',
            "",
            "(async () => {",
            `  const producer = new Producer({ groupName: "${r.group}", nameServer: "${r.target}" });`,
            "  await producer.start();",
            `  const res = await producer.send({ topic: "${r.topic}", body: Buffer.from(${j(r.body)}) });`,
            "  console.log(res.messageId);",
            "  await producer.shutdown();",
            "})();",
          ].join("\n")
        : [
            "// 依赖：npm i rocketmq-client",
            'const { PushConsumer, ConsumeStatus } = require("rocketmq-client");',
            "",
            "(async () => {",
            `  const consumer = new PushConsumer({ groupName: "${r.group}", nameServer: "${r.target}" });`,
            `  await consumer.subscribe("${r.topic}", async (msg) => {`,
            '    console.log(msg.body.toString("utf8"));',
            "    return ConsumeStatus.CONSUME_SUCCESS;",
            "  });",
            "  await consumer.start();",
            "})();",
          ].join("\n");
    case "activemq":
      return produce
        ? [
            "// 依赖：npm i stompit",
            'const stompit = require("stompit");',
            "",
            `stompit.connect({ host: "${r.host}", port: ${r.port} }, (err, client) => {`,
            "  if (err) throw err;",
            "  client.send(",
            `    { destination: "/queue/${r.topic}", "content-type": "text/plain" },`,
            `    ${j(r.body)},`,
            "    () => client.disconnect()",
            "  );",
            "});",
          ].join("\n")
        : [
            "// 依赖：npm i stompit",
            'const stompit = require("stompit");',
            "",
            `stompit.connect({ host: "${r.host}", port: ${r.port} }, (err, client) => {`,
            "  if (err) throw err;",
            `  const frame = client.subscribe({ destination: "/queue/${r.topic}", ack: "client" });`,
            '  frame.on("message", (headers, body) => {',
            '    body.readString("utf8", (e, text) => console.log(text));',
            "    client.ack(headers);",
            "    client.disconnect();",
            "  });",
            "});",
          ].join("\n");
    default:
      // 新增消息队列（MQTT 系 / Pulsar / NATS）：由协议族函数生成
      return genMqFamily("javascript", r, dir);
    case "zeromq":
      return produce
        ? [
            "// 依赖：npm i zeromq",
            'const zmq = require("zeromq");',
            "",
            `// Topic: ${r.topic}（ZeroMQ 无 Topic 概念，SUB 端可用前缀过滤）`,
            "(async () => {",
            "  const sock = new zmq.Push();",
            `  await sock.connect("tcp://${r.target}");`,
            `  await sock.send(${j(r.body)});`,
            "  sock.close();",
            "})();",
          ].join("\n")
        : [
            "// 依赖：npm i zeromq",
            'const zmq = require("zeromq");',
            "",
            `// Topic: ${r.topic}（ZeroMQ 无 Topic 概念，SUB 端可用前缀过滤）`,
            "(async () => {",
            "  const sock = new zmq.Pull();",
            `  await sock.bind("tcp://${r.target}"); // ZeroMQ 无 Broker：消费端 bind，生产端 connect`,
            "  const [msg] = await sock.receive();",
            "  console.log(msg.toString());",
            "  sock.close();",
            "})();",
          ].join("\n");
  }
}

// ---------------------------------------------------------------- Java / Kotlin

/** Kotlin 版本：与 Java 使用同一套客户端库，仅语法不同 */
function genKotlin(r: MqReq, dir: MqDirection): string {
  const produce = dir === "produce";
  const body = j(r.body);
  switch (r.kind) {
    case "kafka":
      return produce
        ? [
            "// 依赖（build.gradle.kts）：implementation(\"org.apache.kafka:kafka-clients:3.6.0\")",
            "import org.apache.kafka.clients.producer.KafkaProducer",
            "import org.apache.kafka.clients.producer.ProducerRecord",
            "import java.util.Properties",
            "",
            "fun main() {",
            "    val props = Properties()",
            `    props["bootstrap.servers"] = "${r.target}"`,
            '    props["key.serializer"] = "org.apache.kafka.common.serialization.StringSerializer"',
            '    props["value.serializer"] = "org.apache.kafka.common.serialization.StringSerializer"',
            "    KafkaProducer<String, String>(props).use { producer ->",
            `        producer.send(ProducerRecord("${r.topic}", ${body}))`,
            "        producer.flush()",
            "    }",
            "}",
          ].join("\n")
        : [
            "// 依赖（build.gradle.kts）：implementation(\"org.apache.kafka:kafka-clients:3.6.0\")",
            "import org.apache.kafka.clients.consumer.ConsumerRecords",
            "import org.apache.kafka.clients.consumer.KafkaConsumer",
            "import java.time.Duration",
            "import java.util.Properties",
            "",
            "fun main() {",
            "    val props = Properties()",
            `    props["bootstrap.servers"] = "${r.target}"`,
            `    props["group.id"] = "${r.group}"`,
            `    props["auto.offset.reset"] = "${r.offset}"`,
            '    props["key.deserializer"] = "org.apache.kafka.common.serialization.StringDeserializer"',
            '    props["value.deserializer"] = "org.apache.kafka.common.serialization.StringDeserializer"',
            "    KafkaConsumer<String, String>(props).use { consumer ->",
            `        consumer.subscribe(listOf("${r.topic}"))`,
            `        val records: ConsumerRecords<String, String> = consumer.poll(Duration.ofMillis(${r.timeoutMs}))`,
            "        records.forEach { println(it.value()) }",
            "    }",
            "}",
          ].join("\n");
    case "rabbitmq":
      return [
        '// 依赖（build.gradle.kts）：implementation("com.rabbitmq:amqp-client:5.20.0")',
        "import com.rabbitmq.client.ConnectionFactory",
        "",
        "fun main() {",
        "    val factory = ConnectionFactory()",
        `    factory.host = "${r.host}"`,
        `    factory.port = ${r.port}`,
        "    factory.newConnection().use { conn ->",
        "        conn.createChannel().use { ch ->",
        `            ch.queueDeclare("${r.topic}", true, false, false, null)`,
        ...(produce
          ? [`            ch.basicPublish("", "${r.topic}", null, ${body}.toByteArray())`]
          : [
              `            val msg = ch.basicGet("${r.topic}", true)`,
              '            println(msg?.body?.toString(Charsets.UTF_8) ?: "no message")',
            ]),
        "        }",
        "    }",
        "}",
      ].join("\n");
    case "rocketmq":
      return produce
        ? [
            '// 依赖（build.gradle.kts）：implementation("org.apache.rocketmq:rocketmq-client:5.1.4")',
            "import org.apache.rocketmq.client.producer.DefaultMQProducer",
            "import org.apache.rocketmq.common.message.Message",
            "",
            "fun main() {",
            `    val producer = DefaultMQProducer("${r.group}")`,
            `    producer.namesrvAddr = "${r.target}"`,
            "    producer.start()",
            `    val msg = Message("${r.topic}", ${body}.toByteArray())`,
            "    println(producer.send(msg).msgId)",
            "    producer.shutdown()",
            "}",
          ].join("\n")
        : [
            '// 依赖（build.gradle.kts）：implementation("org.apache.rocketmq:rocketmq-client:5.1.4")',
            "import org.apache.rocketmq.client.consumer.DefaultMQPushConsumer",
            "import org.apache.rocketmq.client.consumer.listener.ConsumeConcurrentlyStatus",
            "import org.apache.rocketmq.client.consumer.listener.MessageListenerConcurrently",
            "import org.apache.rocketmq.common.message.MessageExt",
            "",
            "fun main() {",
            `    val consumer = DefaultMQPushConsumer("${r.group}")`,
            `    consumer.namesrvAddr = "${r.target}"`,
            `    consumer.subscribe("${r.topic}", "*")`,
            "    consumer.registerMessageListener(MessageListenerConcurrently { msgs: List<MessageExt>, _ ->",
            "        msgs.forEach { println(String(it.body)) }",
            "        ConsumeConcurrentlyStatus.CONSUME_SUCCESS",
            "    })",
            "    consumer.start()",
            "}",
          ].join("\n");
    case "activemq":
      return produce
        ? [
            '// 依赖（build.gradle.kts）：implementation("org.apache.activemq:activemq-client:5.18.3")',
            "import org.apache.activemq.ActiveMQConnectionFactory",
            "import javax.jms.Session",
            "",
            "fun main() {",
            `    val factory = ActiveMQConnectionFactory("tcp://${r.target}")`,
            "    factory.createConnection().use { conn ->",
            "        conn.start()",
            "        conn.createSession(false, Session.AUTO_ACKNOWLEDGE).use { session ->",
            `            val queue = session.createQueue("${r.topic}")`,
            "            session.createProducer(queue).use { p ->",
            `                p.send(session.createTextMessage(${body}))`,
            "            }",
            "        }",
            "    }",
            "}",
          ].join("\n")
        : [
            '// 依赖（build.gradle.kts）：implementation("org.apache.activemq:activemq-client:5.18.3")',
            "import org.apache.activemq.ActiveMQConnectionFactory",
            "import javax.jms.Session",
            "import javax.jms.TextMessage",
            "",
            "fun main() {",
            `    val factory = ActiveMQConnectionFactory("tcp://${r.target}")`,
            "    factory.createConnection().use { conn ->",
            "        conn.start()",
            "        conn.createSession(false, Session.AUTO_ACKNOWLEDGE).use { session ->",
            `            val queue = session.createQueue("${r.topic}")`,
            "            session.createConsumer(queue).use { c ->",
            `                val msg = c.receive(${r.timeoutMs})`,
            '                println((msg as? TextMessage)?.text ?: "no message")',
            "            }",
            "        }",
            "    }",
            "}",
          ].join("\n");
    default:
      // 新增消息队列（MQTT 系 / Pulsar / NATS）：由协议族函数生成
      return genMqFamily("kotlin", r, dir);
    case "zeromq":
      return [
        '// 依赖（build.gradle.kts）：implementation("org.zeromq:jeromq:0.5.3")',
        "import org.zeromq.ZMQ",
        "",
        `// Topic: ${r.topic}（ZeroMQ 无 Topic 概念，SUB 端可用前缀过滤）`,
        "fun main() {",
        "    val ctx = ZMQ.context(1)",
        `    val sock = ctx.socket(${produce ? "ZMQ.PUSH" : "ZMQ.PULL"})`,
        `    sock.${produce ? "connect" : "bind"}("tcp://${r.target}")${
          produce ? "" : " // ZeroMQ 无 Broker：消费端 bind，生产端 connect"
        }`,
        ...(produce
          ? [`    sock.send(${body}.toByteArray())`]
          : [`    if (sock.poll(${r.timeoutMs})) println(String(sock.recv()))`]),
        "    sock.close()",
        "    ctx.close()",
        "}",
      ].join("\n");
  }
}

function genJava(r: MqReq, dir: MqDirection): string {
  const produce = dir === "produce";
  const body = j(r.body);
  const cls = `Mq${produce ? "Producer" : "Consumer"}`;
  switch (r.kind) {
    case "kafka":
      return produce
        ? [
            "// Kafka 生产（Maven：org.apache.kafka:kafka-clients:3.6.0）",
            "import org.apache.kafka.clients.producer.KafkaProducer;",
            "import org.apache.kafka.clients.producer.Producer;",
            "import org.apache.kafka.clients.producer.ProducerRecord;",
            "import java.util.Properties;",
            "",
            `public class ${cls} {`,
            "    public static void main(String[] args) {",
            "        Properties props = new Properties();",
            `        props.put("bootstrap.servers", "${r.target}");`,
            '        props.put("key.serializer", "org.apache.kafka.common.serialization.StringSerializer");',
            '        props.put("value.serializer", "org.apache.kafka.common.serialization.StringSerializer");',
            "        try (Producer<String, String> producer = new KafkaProducer<>(props)) {",
            `            producer.send(new ProducerRecord<>("${r.topic}", ${body}));`,
            "            producer.flush();",
            "        }",
            "    }",
            "}",
          ].join("\n")
        : [
            "// Kafka 消费（Maven：org.apache.kafka:kafka-clients:3.6.0）",
            "import org.apache.kafka.clients.consumer.Consumer;",
            "import org.apache.kafka.clients.consumer.ConsumerRecords;",
            "import org.apache.kafka.clients.consumer.KafkaConsumer;",
            "import java.time.Duration;",
            "import java.util.Collections;",
            "import java.util.Properties;",
            "",
            `public class ${cls} {`,
            "    public static void main(String[] args) {",
            "        Properties props = new Properties();",
            `        props.put("bootstrap.servers", "${r.target}");`,
            `        props.put("group.id", "${r.group}");`,
            `        props.put("auto.offset.reset", "${r.offset}");`,
            '        props.put("key.deserializer", "org.apache.kafka.common.serialization.StringDeserializer");',
            '        props.put("value.deserializer", "org.apache.kafka.common.serialization.StringDeserializer");',
            "        try (Consumer<String, String> consumer = new KafkaConsumer<>(props)) {",
            `            consumer.subscribe(Collections.singletonList("${r.topic}"));`,
            `            ConsumerRecords<String, String> records = consumer.poll(Duration.ofMillis(${r.timeoutMs}));`,
            "            records.forEach(record -> System.out.println(record.value()));",
            "        }",
            "    }",
            "}",
          ].join("\n");
    case "rabbitmq":
      return [
        `// RabbitMQ ${produce ? "生产" : "消费"}（Maven：com.rabbitmq:amqp-client:5.20.0）`,
        "import com.rabbitmq.client.Channel;",
        "import com.rabbitmq.client.Connection;",
        "import com.rabbitmq.client.ConnectionFactory;",
        "import java.nio.charset.StandardCharsets;",
        "",
        `public class ${cls} {`,
        "    public static void main(String[] args) throws Exception {",
        "        ConnectionFactory factory = new ConnectionFactory();",
        `        factory.setHost("${r.host}");`,
        `        factory.setPort(${r.port});`,
        "        try (Connection conn = factory.newConnection(); Channel ch = conn.createChannel()) {",
        `            ch.queueDeclare("${r.topic}", true, false, false, null);`,
        ...(produce
          ? [
              `            ch.basicPublish("", "${r.topic}", null, ${body}.getBytes(StandardCharsets.UTF_8));`,
            ]
          : [
              `            var msg = ch.basicGet("${r.topic}", true);`,
              '            System.out.println(msg == null ? "no message" : new String(msg.getBody(), StandardCharsets.UTF_8));',
            ]),
        "        }",
        "    }",
        "}",
      ].join("\n");
    case "rocketmq":
      return produce
        ? [
            "// RocketMQ 生产（Maven：org.apache.rocketmq:rocketmq-client:5.1.4）",
            "import org.apache.rocketmq.client.producer.DefaultMQProducer;",
            "import org.apache.rocketmq.common.message.Message;",
            "import java.nio.charset.StandardCharsets;",
            "",
            `public class ${cls} {`,
            "    public static void main(String[] args) throws Exception {",
            `        DefaultMQProducer producer = new DefaultMQProducer("${r.group}");`,
            `        producer.setNamesrvAddr("${r.target}");`,
            "        producer.start();",
            `        Message msg = new Message("${r.topic}", ${body}.getBytes(StandardCharsets.UTF_8));`,
            "        System.out.println(producer.send(msg).getMsgId());",
            "        producer.shutdown();",
            "    }",
            "}",
          ].join("\n")
        : [
            "// RocketMQ 消费（Maven：org.apache.rocketmq:rocketmq-client:5.1.4）",
            "import org.apache.rocketmq.client.consumer.DefaultMQPushConsumer;",
            "import org.apache.rocketmq.client.consumer.listener.ConsumeConcurrentlyStatus;",
            "import org.apache.rocketmq.client.consumer.listener.MessageListenerConcurrently;",
            "import java.nio.charset.StandardCharsets;",
            "",
            `public class ${cls} {`,
            "    public static void main(String[] args) throws Exception {",
            `        DefaultMQPushConsumer consumer = new DefaultMQPushConsumer("${r.group}");`,
            `        consumer.setNamesrvAddr("${r.target}");`,
            `        consumer.subscribe("${r.topic}", "*");`,
            "        consumer.registerMessageListener((MessageListenerConcurrently) (msgs, ctx) -> {",
            "            msgs.forEach(m -> System.out.println(new String(m.getBody(), StandardCharsets.UTF_8)));",
            "            return ConsumeConcurrentlyStatus.CONSUME_SUCCESS;",
            "        });",
            "        consumer.start();",
            "    }",
            "}",
          ].join("\n");
    case "activemq":
      return [
        `// ActiveMQ ${produce ? "生产" : "消费"}（Maven：org.apache.activemq:activemq-client:5.18.3）`,
        "import org.apache.activemq.ActiveMQConnectionFactory;",
        "import javax.jms.*;",
        "",
        `public class ${cls} {`,
        "    public static void main(String[] args) throws Exception {",
        `        ConnectionFactory factory = new ActiveMQConnectionFactory("tcp://${r.target}");`,
        "        try (Connection conn = factory.createConnection()) {",
        "            conn.start();",
        "            Session session = conn.createSession(false, Session.AUTO_ACKNOWLEDGE);",
        `            Queue queue = session.createQueue("${r.topic}");`,
        ...(produce
          ? [
              "            try (MessageProducer producer = session.createProducer(queue)) {",
              `                producer.send(session.createTextMessage(${body}));`,
              "            }",
            ]
          : [
              "            try (MessageConsumer consumer = session.createConsumer(queue)) {",
              `                Message msg = consumer.receive(${r.timeoutMs});`,
              '                System.out.println(msg instanceof TextMessage ? ((TextMessage) msg).getText() : "no message");',
              "            }",
            ]),
        "        }",
        "    }",
        "}",
      ].join("\n");
    default:
      // 新增消息队列（MQTT 系 / Pulsar / NATS）：由协议族函数生成
      return genMqFamily("java", r, dir);
    case "zeromq":
      return [
        `// ZeroMQ ${produce ? "生产" : "消费"}（Maven：org.zeromq:jeromq:0.5.3）`,
        "import org.zeromq.ZMQ;",
        "import java.nio.charset.StandardCharsets;",
        "",
        `// Topic: ${r.topic}（ZeroMQ 无 Topic 概念，SUB 端可用前缀过滤）`,
        `public class ${cls} {`,
        "    public static void main(String[] args) {",
        "        ZMQ.Context ctx = ZMQ.context(1);",
        `        ZMQ.Socket sock = ctx.socket(${produce ? "ZMQ.PUSH" : "ZMQ.PULL"});`,
        `        sock.${produce ? "connect" : "bind"}("tcp://${r.target}");${
          produce ? "" : " // ZeroMQ 无 Broker：消费端 bind，生产端 connect"
        }`,
        ...(produce
          ? [`        sock.send(${body}.getBytes(StandardCharsets.UTF_8));`]
          : [
              `        if (sock.poll(${r.timeoutMs})) {`,
              "            System.out.println(new String(sock.recv(), StandardCharsets.UTF_8));",
              "        }",
            ]),
        "        sock.close();",
        "        ctx.close();",
        "    }",
        "}",
      ].join("\n");
  }
}

// ---------------------------------------------------------------- Go

function genGo(r: MqReq, dir: MqDirection): string {
  const produce = dir === "produce";
  /** Go 原始字符串（内容含反引号时退化为普通字符串字面量） */
  const raw = (s: string) => (s.includes("`") ? j(s) : "`" + s + "`");
  switch (r.kind) {
    case "kafka":
      return produce
        ? [
            "// 依赖：go get github.com/segmentio/kafka-go",
            "package main",
            "",
            "import (",
            '\t"context"',
            '\t"fmt"',
            "",
            '\t"github.com/segmentio/kafka-go"',
            ")",
            "",
            "func main() {",
            "\tw := &kafka.Writer{",
            `\t\tAddr: kafka.TCP("${r.target}"),`,
            `\t\tTopic: "${r.topic}",`,
            "\t\tBalancer: &kafka.LeastBytes{},",
            "\t}",
            "\tdefer w.Close()",
            "\terr := w.WriteMessages(context.Background(), kafka.Message{Value: []byte(" + raw(r.body) + ")})",
            "\tif err != nil {",
            "\t\tpanic(err)",
            "\t}",
            '\tfmt.Println("sent")',
            "}",
          ].join("\n")
        : [
            "// 依赖：go get github.com/segmentio/kafka-go",
            "package main",
            "",
            "import (",
            '\t"context"',
            '\t"fmt"',
            '\t"time"',
            "",
            '\t"github.com/segmentio/kafka-go"',
            ")",
            "",
            "func main() {",
            "\tr := kafka.NewReader(kafka.ReaderConfig{",
            `\t\tBrokers: []string{"${r.target}"},`,
            `\t\tTopic: "${r.topic}",`,
            `\t\tGroupID: "${r.group}",`,
            "\t})",
            "\tdefer r.Close()",
            `\tctx, cancel := context.WithTimeout(context.Background(), ${r.timeoutMs}*time.Millisecond)`,
            "\tdefer cancel()",
            "\tm, err := r.ReadMessage(ctx)",
            "\tif err != nil {",
            "\t\tpanic(err)",
            "\t}",
            "\tfmt.Println(string(m.Value))",
            "}",
          ].join("\n");
    case "rabbitmq":
      return produce
        ? [
            "// 依赖：go get github.com/rabbitmq/amqp091-go",
            "package main",
            "",
            "import (",
            '\t"context"',
            '\t"fmt"',
            "",
            '\tamqp "github.com/rabbitmq/amqp091-go"',
            ")",
            "",
            "func main() {",
            `\tconn, err := amqp.Dial("amqp://guest:guest@${r.target}/")`,
            "\tif err != nil {",
            "\t\tpanic(err)",
            "\t}",
            "\tdefer conn.Close()",
            "\tch, err := conn.Channel()",
            "\tif err != nil {",
            "\t\tpanic(err)",
            "\t}",
            "\tdefer ch.Close()",
            `\tq, _ := ch.QueueDeclare("${r.topic}", true, false, false, false, nil)`,
            "\terr = ch.PublishWithContext(context.Background(), \"\", q.Name, false, false, amqp.Publishing{",
            '\t\tContentType: "text/plain",',
            "\t\tBody:        []byte(" + raw(r.body) + "),",
            "\t})",
            "\tif err != nil {",
            "\t\tpanic(err)",
            "\t}",
            '\tfmt.Println("sent")',
            "}",
          ].join("\n")
        : [
            "// 依赖：go get github.com/rabbitmq/amqp091-go",
            "package main",
            "",
            "import (",
            '\t"fmt"',
            "",
            '\tamqp "github.com/rabbitmq/amqp091-go"',
            ")",
            "",
            "func main() {",
            `\tconn, _ := amqp.Dial("amqp://guest:guest@${r.target}/")`,
            "\tdefer conn.Close()",
            "\tch, _ := conn.Channel()",
            "\tdefer ch.Close()",
            `\tq, _ := ch.QueueDeclare("${r.topic}", true, false, false, false, nil)`,
            "\tmsg, ok, err := ch.Get(q.Name, false)",
            "\tif err != nil || !ok {",
            '\t\tfmt.Println("no message")',
            "\t\treturn",
            "\t}",
            "\tfmt.Println(string(msg.Body))",
            "\t_ = msg.Ack(false)",
            "}",
          ].join("\n");
    case "rocketmq":
      return produce
        ? [
            "// 依赖：go get github.com/apache/rocketmq-client-go/v2",
            "package main",
            "",
            "import (",
            '\t"context"',
            '\t"fmt"',
            "",
            '\t"github.com/apache/rocketmq-client-go/v2"',
            '\t"github.com/apache/rocketmq-client-go/v2/primitive"',
            '\t"github.com/apache/rocketmq-client-go/v2/producer"',
            ")",
            "",
            "func main() {",
            `\tp, err := rocketmq.NewProducer(producer.WithNameServer([]string{"${r.target}"}), producer.WithGroupName("${r.group}"))`,
            "\tif err != nil {",
            "\t\tpanic(err)",
            "\t}",
            "\t_ = p.Start()",
            "\tdefer p.Shutdown()",
            `\tres, err := p.SendSync(context.Background(), &primitive.Message{Topic: "${r.topic}", Body: []byte(` +
              raw(r.body) +
              ")})",
            "\tif err != nil {",
            "\t\tpanic(err)",
            "\t}",
            "\tfmt.Println(res.MsgID)",
            "}",
          ].join("\n")
        : [
            "// 依赖：go get github.com/apache/rocketmq-client-go/v2",
            "package main",
            "",
            "import (",
            '\t"context"',
            '\t"fmt"',
            "",
            '\t"github.com/apache/rocketmq-client-go/v2"',
            '\t"github.com/apache/rocketmq-client-go/v2/consumer"',
            '\t"github.com/apache/rocketmq-client-go/v2/primitive"',
            ")",
            "",
            "func main() {",
            `\tc, err := rocketmq.NewPushConsumer(consumer.WithNameServer([]string{"${r.target}"}), consumer.WithGroupName("${r.group}"))`,
            "\tif err != nil {",
            "\t\tpanic(err)",
            "\t}",
            `\t_ = c.Subscribe("${r.topic}", consumer.MessageSelector{}, func(ctx context.Context, msgs ...*primitive.MessageExt) (consumer.ConsumeResult, error) {`,
            "\t\tfor _, m := range msgs {",
            "\t\t\tfmt.Println(string(m.Body))",
            "\t\t}",
            "\t\treturn consumer.ConsumeSuccess, nil",
            "\t})",
            "\t_ = c.Start()",
            "\tdefer c.Shutdown()",
            "\tselect {}",
            "}",
          ].join("\n");
    case "activemq":
      return produce
        ? [
            "// 依赖：go get github.com/go-stomp/stomp/v3",
            "package main",
            "",
            "import (",
            '\t"fmt"',
            "",
            '\t"github.com/go-stomp/stomp/v3"',
            ")",
            "",
            "func main() {",
            `\tconn, err := stomp.Dial("tcp", "${r.target}")`,
            "\tif err != nil {",
            "\t\tpanic(err)",
            "\t}",
            "\tdefer conn.Disconnect()",
            `\terr = conn.Send("/queue/${r.topic}", "text/plain", []byte(` + raw(r.body) + "))",
            "\tif err != nil {",
            "\t\tpanic(err)",
            "\t}",
            '\tfmt.Println("sent")',
            "}",
          ].join("\n")
        : [
            "// 依赖：go get github.com/go-stomp/stomp/v3",
            "package main",
            "",
            "import (",
            '\t"fmt"',
            "",
            '\t"github.com/go-stomp/stomp/v3"',
            ")",
            "",
            "func main() {",
            `\tconn, err := stomp.Dial("tcp", "${r.target}")`,
            "\tif err != nil {",
            "\t\tpanic(err)",
            "\t}",
            "\tdefer conn.Disconnect()",
            `\tsub, err := conn.Subscribe("/queue/${r.topic}", stomp.AckAuto)`,
            "\tif err != nil {",
            "\t\tpanic(err)",
            "\t}",
            "\tmsg := <-sub.C",
            "\tfmt.Println(string(msg.Body))",
            "}",
          ].join("\n");
    default:
      // 新增消息队列（MQTT 系 / Pulsar / NATS）：由协议族函数生成
      return genMqFamily("go", r, dir);
    case "zeromq":
      return produce
        ? [
            "// 依赖：go get github.com/pebbe/zmq4",
            "package main",
            "",
            "import (",
            '\t"fmt"',
            "",
            '\t"github.com/pebbe/zmq4"',
            ")",
            "",
            `// Topic: ${r.topic}（ZeroMQ 无 Topic 概念，SUB 端可用前缀过滤）`,
            "func main() {",
            "\tctx, _ := zmq4.NewContext()",
            "\tsock, _ := ctx.NewSocket(zmq4.PUSH)",
            "\tdefer sock.Close()",
            `\t_ = sock.Connect("tcp://${r.target}")`,
            "\t_, err := sock.Send(" + raw(r.body) + ", 0)",
            "\tif err != nil {",
            "\t\tpanic(err)",
            "\t}",
            '\tfmt.Println("sent")',
            "}",
          ].join("\n")
        : [
            "// 依赖：go get github.com/pebbe/zmq4",
            "package main",
            "",
            "import (",
            '\t"fmt"',
            '\t"time"',
            "",
            '\t"github.com/pebbe/zmq4"',
            ")",
            "",
            `// Topic: ${r.topic}（ZeroMQ 无 Topic 概念，SUB 端可用前缀过滤）`,
            "func main() {",
            "\tctx, _ := zmq4.NewContext()",
            "\tsock, _ := ctx.NewSocket(zmq4.PULL)",
            "\tdefer sock.Close()",
            `\t_ = sock.Bind("tcp://${r.target}") // ZeroMQ 无 Broker：消费端 bind，生产端 connect`,
            "\tpoller := zmq4.NewPoller()",
            "\tpoller.Add(sock, zmq4.POLLIN)",
            `\tevents, _ := poller.Poll(${r.timeoutMs} * time.Millisecond)`,
            "\tif len(events) > 0 {",
            "\t\tmsg, _ := sock.Recv(0)",
            "\t\tfmt.Println(msg)",
            "\t}",
            "}",
          ].join("\n");
  }
}

// ---------------------------------------------------------------- C#

function genCsharp(r: MqReq, dir: MqDirection): string {
  const produce = dir === "produce";
  const body = j(r.body);
  switch (r.kind) {
    case "kafka":
      return produce
        ? [
            "// 依赖：dotnet add package Confluent.Kafka",
            "using Confluent.Kafka;",
            "",
            `var config = new ProducerConfig { BootstrapServers = "${r.target}" };`,
            "using var producer = new ProducerBuilder<Null, string>(config).Build();",
            `var result = await producer.ProduceAsync("${r.topic}", new Message<Null, string> { Value = ${body} });`,
            'Console.WriteLine($"delivered to {result.TopicPartitionOffset}");',
          ].join("\n")
        : [
            "// 依赖：dotnet add package Confluent.Kafka",
            "using Confluent.Kafka;",
            "",
            "var config = new ConsumerConfig",
            "{",
            `    BootstrapServers = "${r.target}",`,
            `    GroupId = "${r.group}",`,
            `    AutoOffsetReset = AutoOffsetReset.${earliest(r) ? "Earliest" : "Latest"},`,
            "};",
            "using var consumer = new ConsumerBuilder<Ignore, string>(config).Build();",
            `consumer.Subscribe("${r.topic}");`,
            `var cr = consumer.Consume(TimeSpan.FromMilliseconds(${r.timeoutMs}));`,
            'Console.WriteLine(cr?.Message.Value ?? "no message");',
          ].join("\n");
    case "rabbitmq":
      return produce
        ? [
            "// 依赖：dotnet add package RabbitMQ.Client",
            "using System.Text;",
            "using RabbitMQ.Client;",
            "",
            `var factory = new ConnectionFactory { HostName = "${r.host}", Port = ${r.port} };`,
            "using var conn = factory.CreateConnection();",
            "using var ch = conn.CreateModel();",
            `ch.QueueDeclare("${r.topic}", durable: true, exclusive: false, autoDelete: false);`,
            `ch.BasicPublish("", "${r.topic}", null, Encoding.UTF8.GetBytes(${body}));`,
            'Console.WriteLine("sent");',
          ].join("\n")
        : [
            "// 依赖：dotnet add package RabbitMQ.Client",
            "using System.Text;",
            "using RabbitMQ.Client;",
            "",
            `var factory = new ConnectionFactory { HostName = "${r.host}", Port = ${r.port} };`,
            "using var conn = factory.CreateConnection();",
            "using var ch = conn.CreateModel();",
            `ch.QueueDeclare("${r.topic}", durable: true, exclusive: false, autoDelete: false);`,
            `var msg = ch.BasicGet("${r.topic}", autoAck: true);`,
            'Console.WriteLine(msg == null ? "no message" : Encoding.UTF8.GetString(msg.Body.ToArray()));',
          ].join("\n");
    case "rocketmq":
      return produce
        ? [
            "// 依赖：dotnet add package Apache.Rocketmq",
            "using Org.Apache.Rocketmq;",
            "",
            `var client = await Client.Builder().SetEndpoints("${r.target}").Build();`,
            `var producer = await Producer.Builder().SetClient(client).SetTopics("${r.topic}").Build();`,
            `var message = new Message.Builder().SetTopic("${r.topic}").SetBody(${body}).Build();`,
            "var receipt = await producer.Send(message);",
            "Console.WriteLine(receipt.MessageId);",
          ].join("\n")
        : [
            "// 依赖：dotnet add package Apache.Rocketmq",
            "using Org.Apache.Rocketmq;",
            "",
            `var client = await Client.Builder().SetEndpoints("${r.target}").Build();`,
            "var consumer = await SimpleConsumer.Builder()",
            "    .SetClient(client)",
            `    .SetConsumerGroup("${r.group}")`,
            `    .SetSubscriptionExpression("${r.topic}")`,
            "    .Build();",
            "var messages = await consumer.Receive(32, TimeSpan.FromSeconds(" + secs(r) + "));",
            "foreach (var message in messages) {",
            "    Console.WriteLine(message.Body);",
            "    await consumer.Ack(message);",
            "}",
          ].join("\n");
    case "activemq":
      return produce
        ? [
            "// 依赖：dotnet add package Apache.NMS.ActiveMQ",
            "using Apache.NMS;",
            "using Apache.NMS.ActiveMQ;",
            "",
            `var factory = new ConnectionFactory("tcp://${r.target}");`,
            "using var conn = factory.CreateConnection();",
            "conn.Start();",
            "using var session = conn.CreateSession();",
            `var queue = session.GetQueue("${r.topic}");`,
            "using var producer = session.CreateProducer(queue);",
            `producer.Send(session.CreateTextMessage(${body}));`,
          ].join("\n")
        : [
            "// 依赖：dotnet add package Apache.NMS.ActiveMQ",
            "using Apache.NMS;",
            "using Apache.NMS.ActiveMQ;",
            "",
            `var factory = new ConnectionFactory("tcp://${r.target}");`,
            "using var conn = factory.CreateConnection();",
            "conn.Start();",
            "using var session = conn.CreateSession();",
            `var queue = session.GetQueue("${r.topic}");`,
            "using var consumer = session.CreateConsumer(queue);",
            `var msg = consumer.Receive(TimeSpan.FromMilliseconds(${r.timeoutMs})) as ITextMessage;`,
            'Console.WriteLine(msg?.Text ?? "no message");',
          ].join("\n");
    default:
      // 新增消息队列（MQTT 系 / Pulsar / NATS）：由协议族函数生成
      return genMqFamily("csharp", r, dir);
    case "zeromq":
      return produce
        ? [
            "// 依赖：dotnet add package NetMQ",
            "using NetMQ;",
            "using NetMQ.Sockets;",
            "",
            `// Topic: ${r.topic}（ZeroMQ 无 Topic 概念，SUB 端可用前缀过滤）`,
            "using var sock = new PushSocket();",
            `sock.Connect("tcp://${r.target}");`,
            `sock.SendFrame(${body});`,
          ].join("\n")
        : [
            "// 依赖：dotnet add package NetMQ",
            "using NetMQ;",
            "using NetMQ.Sockets;",
            "",
            `// Topic: ${r.topic}（ZeroMQ 无 Topic 概念，SUB 端可用前缀过滤）`,
            "using var sock = new PullSocket();",
            `sock.Bind("tcp://${r.target}"); // ZeroMQ 无 Broker：消费端 bind，生产端 connect`,
            `if (sock.TryReceiveFrameString(TimeSpan.FromMilliseconds(${r.timeoutMs}), out var msg))`,
            "    Console.WriteLine(msg);",
          ].join("\n");
  }
}

// ---------------------------------------------------------------- PHP

function genPhp(r: MqReq, dir: MqDirection): string {
  const produce = dir === "produce";
  const body = j(r.body);
  switch (r.kind) {
    case "kafka":
      return produce
        ? [
            "<?php",
            "// 依赖：pecl install rdkafka",
            '$producer = new RdKafka\\Producer();',
            `$producer->addBrokers("${r.target}");`,
            `$topic = $producer->newTopic("${r.topic}");`,
            `$topic->produce(RD_KAFKA_PARTITION_UA, 0, ${body});`,
            "$producer->flush(3000);",
          ].join("\n")
        : [
            "<?php",
            "// 依赖：pecl install rdkafka",
            "$conf = new RdKafka\\Conf();",
            `$conf->set("metadata.broker.list", "${r.target}");`,
            `$conf->set("group.id", "${r.group}");`,
            `$conf->set("auto.offset.reset", "${r.offset}");`,
            "$consumer = new RdKafka\\KafkaConsumer($conf);",
            `$consumer->subscribe(["${r.topic}"]);`,
            `$msg = $consumer->consume(${r.timeoutMs});`,
            "if ($msg && $msg->err === RD_KAFKA_RESP_ERR_NO_ERROR) {",
            "    echo $msg->payload, PHP_EOL;",
            "}",
          ].join("\n");
    case "rabbitmq":
      return produce
        ? [
            "<?php",
            "// 依赖：composer require php-amqplib/php-amqplib",
            "require __DIR__ . '/vendor/autoload.php';",
            "",
            "use PhpAmqpLib\\Connection\\AMQPStreamConnection;",
            "use PhpAmqpLib\\Message\\AMQPMessage;",
            "",
            `$conn = new AMQPStreamConnection("${r.host}", ${r.port}, "guest", "guest");`,
            "$ch = $conn->channel();",
            `$ch->queue_declare("${r.topic}", false, true, false, false);`,
            `$ch->basic_publish(new AMQPMessage(${body}), "", "${r.topic}");`,
            "$ch->close();",
            "$conn->close();",
          ].join("\n")
        : [
            "<?php",
            "// 依赖：composer require php-amqplib/php-amqplib",
            "require __DIR__ . '/vendor/autoload.php';",
            "",
            "use PhpAmqpLib\\Connection\\AMQPStreamConnection;",
            "",
            `$conn = new AMQPStreamConnection("${r.host}", ${r.port}, "guest", "guest");`,
            "$ch = $conn->channel();",
            `$ch->queue_declare("${r.topic}", false, true, false, false);`,
            `$msg = $ch->basic_get("${r.topic}", false);`,
            'echo $msg ? $msg->body : "no message", PHP_EOL;',
            "$ch->close();",
            "$conn->close();",
          ].join("\n");
    case "rocketmq":
      return produce
        ? [
            "<?php",
            "// 依赖：composer require apache/rocketmq-client-php",
            "require __DIR__ . '/vendor/autoload.php';",
            "",
            "use Apache\\RocketMQ\\Producer;",
            "",
            `$producer = new Producer("${r.group}");`,
            `$producer->setNamesrvAddr("${r.target}");`,
            "$producer->start();",
            `$result = $producer->sendMessage("${r.topic}", ${body});`,
            "echo $result->getMessageId(), PHP_EOL;",
            "$producer->shutdown();",
          ].join("\n")
        : [
            "<?php",
            "// 依赖：composer require apache/rocketmq-client-php",
            "require __DIR__ . '/vendor/autoload.php';",
            "",
            "use Apache\\RocketMQ\\PushConsumer;",
            "",
            `$consumer = new PushConsumer("${r.group}");`,
            `$consumer->setNamesrvAddr("${r.target}");`,
            `$consumer->subscribe("${r.topic}", function ($msg) {`,
            "    echo $msg->getBody(), PHP_EOL;",
            "});",
            "$consumer->start();",
          ].join("\n");
    case "activemq":
      return produce
        ? [
            "<?php",
            "// 依赖：composer require stomp-php/stomp-php",
            "require __DIR__ . '/vendor/autoload.php';",
            "",
            "use Stomp\\Client;",
            "",
            `$stomp = new Client("tcp://${r.target}");`,
            '$stomp->connect("admin", "admin");',
            `$stomp->send("/queue/${r.topic}", ${body});`,
            "$stomp->disconnect();",
          ].join("\n")
        : [
            "<?php",
            "// 依赖：composer require stomp-php/stomp-php",
            "require __DIR__ . '/vendor/autoload.php';",
            "",
            "use Stomp\\Client;",
            "",
            `$stomp = new Client("tcp://${r.target}");`,
            '$stomp->connect("admin", "admin");',
            `$stomp->subscribe("/queue/${r.topic}");`,
            "if ($stomp->hasFrame()) {",
            "    $frame = $stomp->readFrame();",
            "    echo $frame->body, PHP_EOL;",
            "    $stomp->ack($frame);",
            "}",
            "$stomp->disconnect();",
          ].join("\n");
    default:
      // 新增消息队列（MQTT 系 / Pulsar / NATS）：由协议族函数生成
      return genMqFamily("php", r, dir);
    case "zeromq":
      return produce
        ? [
            "<?php",
            "// 依赖：pecl install zmq",
            `// Topic: ${r.topic}（ZeroMQ 无 Topic 概念，SUB 端可用前缀过滤）`,
            "$ctx = new ZMQContext();",
            "$sock = $ctx->getSocket(ZMQ::SOCKET_PUSH);",
            `$sock->connect("tcp://${r.target}");`,
            `$sock->send(${body});`,
            "$sock->setSockOpt(ZMQ::SOCKOPT_LINGER, 0);",
          ].join("\n")
        : [
            "<?php",
            "// 依赖：pecl install zmq",
            `// Topic: ${r.topic}（ZeroMQ 无 Topic 概念，SUB 端可用前缀过滤）`,
            "$ctx = new ZMQContext();",
            "$sock = $ctx->getSocket(ZMQ::SOCKET_PULL);",
            `$sock->bind("tcp://${r.target}");`,
            "$read = new ZMQPoll();",
            "$read->add($sock, ZMQ::POLL_IN);",
            "$readable = $writable = [];",
            `if ($read->poll($readable, $writable, ${r.timeoutMs}) > 0) {`,
            "    echo $sock->recv(), PHP_EOL;",
            "}",
          ].join("\n");
  }
}

// ---------------------------------------------------------------- Ruby

function genRuby(r: MqReq, dir: MqDirection): string {
  const produce = dir === "produce";
  const body = j(r.body);
  switch (r.kind) {
    case "kafka":
      return produce
        ? [
            "# 依赖：gem install ruby-kafka",
            'require "kafka"',
            "",
            `kafka = Kafka.new(seed_brokers: ["${r.target}"])`,
            `kafka.deliver_message(${body}, topic: "${r.topic}")`,
          ].join("\n")
        : [
            "# 依赖：gem install ruby-kafka",
            'require "kafka"',
            "",
            `kafka = Kafka.new(seed_brokers: ["${r.target}"])`,
            `consumer = kafka.consumer(group_id: "${r.group}")`,
            `consumer.subscribe("${r.topic}")`,
            "consumer.each_message do |message|",
            "  puts message.value",
            "  break",
            "end",
          ].join("\n");
    case "rabbitmq":
      return produce
        ? [
            "# 依赖：gem install bunny",
            'require "bunny"',
            "",
            `conn = Bunny.new(host: "${r.host}", port: ${r.port})`,
            "conn.start",
            "ch = conn.create_channel",
            `q = ch.queue("${r.topic}", durable: true)`,
            `ch.default_exchange.publish(${body}, routing_key: q.name)`,
            "conn.close",
          ].join("\n")
        : [
            "# 依赖：gem install bunny",
            'require "bunny"',
            "",
            `conn = Bunny.new(host: "${r.host}", port: ${r.port})`,
            "conn.start",
            "ch = conn.create_channel",
            `q = ch.queue("${r.topic}", durable: true)`,
            "delivery_info, _props, payload = q.pop",
            'puts payload || "no message"',
            "conn.close",
          ].join("\n");
    case "rocketmq":
      return produce
        ? [
            "# 依赖：gem install rocketmq-client",
            'require "rocketmq"',
            "",
            `producer = RocketMQ::Producer.new("${r.group}")`,
            `producer.namesrv_addr = "${r.target}"`,
            "producer.start",
            `producer.send_message("${r.topic}", ${body})`,
            "producer.shutdown",
          ].join("\n")
        : [
            "# 依赖：gem install rocketmq-client",
            'require "rocketmq"',
            "",
            `consumer = RocketMQ::PushConsumer.new("${r.group}")`,
            `consumer.namesrv_addr = "${r.target}"`,
            `consumer.subscribe("${r.topic}") { |msg| puts msg.body }`,
            "consumer.start",
          ].join("\n");
    case "activemq":
      return produce
        ? [
            "# 依赖：gem install stomp",
            'require "stomp"',
            "",
            "client = Stomp::Client.new",
            `client.publish("/queue/${r.topic}", ${body})`,
            "client.close",
            "",
            '# 连接参数：client = Stomp::Client.new("admin", "admin", "' + r.host + '", ' + r.port + ")",
          ].join("\n")
        : [
            "# 依赖：gem install stomp",
            'require "stomp"',
            "",
            "client = Stomp::Client.new",
            `client.subscribe("/queue/${r.topic}", ack: :auto) do |msg|`,
            "  puts msg.body",
            "  break",
            "end",
            "client.close",
            "",
            '# 连接参数：client = Stomp::Client.new("admin", "admin", "' + r.host + '", ' + r.port + ")",
          ].join("\n");
    default:
      // 新增消息队列（MQTT 系 / Pulsar / NATS）：由协议族函数生成
      return genMqFamily("ruby", r, dir);
    case "zeromq":
      return produce
        ? [
            "# 依赖：gem install ffi-rzmq",
            'require "ffi-rzmq"',
            "",
            `# Topic: ${r.topic}（ZeroMQ 无 Topic 概念，SUB 端可用前缀过滤）`,
            'ctx = ZMQ::Context.new',
            "sock = ctx.socket(ZMQ::PUSH)",
            `sock.connect("tcp://${r.target}")`,
            `sock.send_string(${body})`,
            "sock.close",
            "ctx.terminate",
          ].join("\n")
        : [
            "# 依赖：gem install ffi-rzmq",
            'require "ffi-rzmq"',
            "",
            `# Topic: ${r.topic}（ZeroMQ 无 Topic 概念，SUB 端可用前缀过滤）`,
            "ctx = ZMQ::Context.new",
            "sock = ctx.socket(ZMQ::PULL)",
            `sock.bind("tcp://${r.target}")`,
            "sock.recv_string(msg = '')",
            "puts msg",
            "sock.close",
            "ctx.terminate",
          ].join("\n");
  }
}

// ---------------------------------------------------------------- MQTT 系（EMQX / HiveMQ / Mosquitto / NanoMQ / VerneMQ）
//
// 这几种消息队列都使用标准 MQTT 协议，所以共用同一套 MQTT 客户端代码，仅注释里标注具体 Broker。

/** MQTT 系提示：MQTT 无消费组 / offset 概念（消费组 / 位点由 QoS、会话与 retained 消息替代） */
const mqttNote = (r: MqReq) =>
  `${r.kindLabel} 使用标准 MQTT 协议：无消费组 / offset 概念（示例用 QoS 1）`;

/** Pulsar topic 规范：未带协议前缀时补默认租户 / 命名空间 */
const pulsarTopic = (t: string) => (t.includes("://") ? t : `persistent://public/default/${t}`);

/** NATS 提示：Core NATS 无持久化语义 */
const natsNote = (r: MqReq) =>
  `${r.kindLabel} Core NATS：无消费组 / offset 概念（需要持久化 / 重放可改用 JetStream）`;

function mqttPython(r: MqReq, dir: MqDirection): string {
  return dir === "produce"
    ? [
        '# 依赖：pip install "paho-mqtt>=2.0"',
        `# ${mqttNote(r)}`,
        "import paho.mqtt.client as mqtt",
        "",
        "client = mqtt.Client(mqtt.CallbackAPIVersion.VERSION2)",
        `client.connect("${r.host}", ${r.port}, 10)`,
        `client.publish("${r.topic}", ${j(r.body)}, qos=1)`,
        "client.disconnect()",
      ].join("\n")
    : [
        '# 依赖：pip install "paho-mqtt>=2.0"',
        `# ${mqttNote(r)}`,
        "import paho.mqtt.client as mqtt",
        "",
        "received = 0",
        "",
        "",
        "def on_message(client, userdata, msg):",
        "    global received",
        '    print(msg.payload.decode("utf-8"))',
        "    received += 1",
        `    if received >= ${r.maxMessages}:`,
        "        client.disconnect()",
        "",
        "",
        "client = mqtt.Client(mqtt.CallbackAPIVersion.VERSION2)",
        "client.on_message = on_message",
        `client.connect("${r.host}", ${r.port}, 10)`,
        `client.subscribe("${r.topic}", qos=1)`,
        "client.loop_forever()  # 收到指定条数后自动断开",
      ].join("\n");
}

function mqttJs(r: MqReq, dir: MqDirection): string {
  return dir === "produce"
    ? [
        "// 依赖：npm i mqtt",
        `// ${mqttNote(r)}`,
        'const mqtt = require("mqtt");',
        "",
        `const client = mqtt.connect("mqtt://${r.target}");`,
        'client.on("connect", () => {',
        `  client.publish("${r.topic}", ${j(r.body)}, { qos: 1 }, () => client.end());`,
        "});",
      ].join("\n")
    : [
        "// 依赖：npm i mqtt",
        `// ${mqttNote(r)}`,
        'const mqtt = require("mqtt");',
        "",
        "let received = 0;",
        `const client = mqtt.connect("mqtt://${r.target}");`,
        `client.on("connect", () => client.subscribe("${r.topic}", { qos: 1 }));`,
        'client.on("message", (_topic, payload) => {',
        "  console.log(payload.toString());",
        `  if (++received >= ${r.maxMessages}) client.end();`,
        "});",
      ].join("\n");
}

function mqttKotlin(r: MqReq, dir: MqDirection): string {
  return dir === "produce"
    ? [
        '// 依赖（build.gradle.kts）：implementation("org.eclipse.paho:org.eclipse.paho.client.mqttv3:1.2.5")',
        "import org.eclipse.paho.client.mqttv3.MqttClient",
        "import org.eclipse.paho.client.mqttv3.MqttMessage",
        "",
        `// ${mqttNote(r)}`,
        "fun main() {",
        `    val client = MqttClient("tcp://${r.target}", MqttClient.generateClientId())`,
        "    client.connect()",
        `    client.publish("${r.topic}", MqttMessage(${j(r.body)}.toByteArray()).apply { qos = 1 })`,
        "    client.disconnect()",
        "}",
      ].join("\n")
    : [
        '// 依赖（build.gradle.kts）：implementation("org.eclipse.paho:org.eclipse.paho.client.mqttv3:1.2.5")',
        "import org.eclipse.paho.client.mqttv3.IMqttMessageListener",
        "import org.eclipse.paho.client.mqttv3.MqttClient",
        "",
        `// ${mqttNote(r)}`,
        "fun main() {",
        `    val client = MqttClient("tcp://${r.target}", MqttClient.generateClientId())`,
        "    client.connect()",
        `    client.subscribe("${r.topic}", 1, IMqttMessageListener { _, msg -> println(String(msg.payload)) })`,
        `    Thread.sleep(${r.timeoutMs})`,
        "    client.disconnect()",
        "}",
      ].join("\n");
}

function mqttJava(r: MqReq, dir: MqDirection): string {
  const produce = dir === "produce";
  const cls = `Mq${produce ? "Producer" : "Consumer"}`;
  return produce
    ? [
        "// 依赖（Maven）：org.eclipse.paho:org.eclipse.paho.client.mqttv3:1.2.5",
        "import java.nio.charset.StandardCharsets;",
        "import org.eclipse.paho.client.mqttv3.MqttClient;",
        "import org.eclipse.paho.client.mqttv3.MqttMessage;",
        "",
        `// ${mqttNote(r)}`,
        `public class ${cls} {`,
        "    public static void main(String[] args) throws Exception {",
        `        MqttClient client = new MqttClient("tcp://${r.target}", MqttClient.generateClientId());`,
        "        client.connect();",
        `        MqttMessage message = new MqttMessage(${j(r.body)}.getBytes(StandardCharsets.UTF_8));`,
        "        message.setQos(1);",
        `        client.publish("${r.topic}", message);`,
        "        client.disconnect();",
        "    }",
        "}",
      ].join("\n")
    : [
        "// 依赖（Maven）：org.eclipse.paho:org.eclipse.paho.client.mqttv3:1.2.5",
        "import java.nio.charset.StandardCharsets;",
        "import org.eclipse.paho.client.mqttv3.MqttClient;",
        "",
        `// ${mqttNote(r)}`,
        `public class ${cls} {`,
        "    public static void main(String[] args) throws Exception {",
        `        MqttClient client = new MqttClient("tcp://${r.target}", MqttClient.generateClientId());`,
        "        client.connect();",
        `        client.subscribe("${r.topic}", 1, (topic, message) ->`,
        "            System.out.println(new String(message.getPayload(), StandardCharsets.UTF_8)));",
        `        Thread.sleep(${r.timeoutMs});`,
        "        client.disconnect();",
        "    }",
        "}",
      ].join("\n");
}

function mqttGo(r: MqReq, dir: MqDirection): string {
  return dir === "produce"
    ? [
        "// 依赖：go get github.com/eclipse/paho.mqtt.golang",
        "package main",
        "",
        "import (",
        '\t"time"',
        "",
        '\tmqtt "github.com/eclipse/paho.mqtt.golang"',
        ")",
        "",
        `// ${mqttNote(r)}`,
        "func main() {",
        `\topts := mqtt.NewClientOptions().AddBroker("tcp://${r.target}").SetClientID("api-manager")`,
        "\tclient := mqtt.NewClient(opts)",
        "\tif token := client.Connect(); token.Wait() && token.Error() != nil {",
        "\t\tpanic(token.Error())",
        "\t}",
        `\tif token := client.Publish("${r.topic}", 1, false, ${j(r.body)}); token.Wait() && token.Error() != nil {`,
        "\t\tpanic(token.Error())",
        "\t}",
        "\ttime.Sleep(300 * time.Millisecond) // 等待发送完成",
        "\tclient.Disconnect(250)",
        "}",
      ].join("\n")
    : [
        "// 依赖：go get github.com/eclipse/paho.mqtt.golang",
        "package main",
        "",
        "import (",
        '\t"fmt"',
        '\t"time"',
        "",
        '\tmqtt "github.com/eclipse/paho.mqtt.golang"',
        ")",
        "",
        `// ${mqttNote(r)}`,
        "func main() {",
        `\topts := mqtt.NewClientOptions().AddBroker("tcp://${r.target}").SetClientID("api-manager")`,
        "\tclient := mqtt.NewClient(opts)",
        "\tif token := client.Connect(); token.Wait() && token.Error() != nil {",
        "\t\tpanic(token.Error())",
        "\t}",
        `\tif token := client.Subscribe("${r.topic}", 1, func(_ mqtt.Client, msg mqtt.Message) {`,
        "\t\tfmt.Println(string(msg.Payload()))",
        "\t}); token.Wait() && token.Error() != nil {",
        "\t\tpanic(token.Error())",
        "\t}",
        `\ttime.Sleep(${r.timeoutMs} * time.Millisecond)`,
        `\tclient.Unsubscribe("${r.topic}")`,
        "\tclient.Disconnect(250)",
        "}",
      ].join("\n");
}

function mqttCsharp(r: MqReq, dir: MqDirection): string {
  return dir === "produce"
    ? [
        "// 依赖：dotnet add package MQTTnet",
        "using MQTTnet;",
        "using MQTTnet.Client;",
        "using MQTTnet.Protocol;",
        "",
        `// ${mqttNote(r)}`,
        "var factory = new MqttFactory();",
        "using var client = factory.CreateMqttClient();",
        `await client.ConnectAsync(new MqttClientOptionsBuilder().WithTcpServer("${r.host}", ${r.port}).Build());`,
        "var message = new MqttApplicationMessageBuilder()",
        `    .WithTopic("${r.topic}")`,
        `    .WithPayload(${j(r.body)})`,
        "    .WithQualityOfServiceLevel(MqttQualityOfServiceLevel.AtLeastOnce)",
        "    .Build();",
        "await client.PublishAsync(message);",
        "await client.DisconnectAsync();",
      ].join("\n")
    : [
        "// 依赖：dotnet add package MQTTnet",
        "using MQTTnet;",
        "using MQTTnet.Client;",
        "using MQTTnet.Protocol;",
        "",
        `// ${mqttNote(r)}`,
        "var factory = new MqttFactory();",
        "using var client = factory.CreateMqttClient();",
        "client.ApplicationMessageReceivedAsync += e =>",
        "{",
        "    Console.WriteLine(e.ApplicationMessage.ConvertPayloadToString());",
        "    return Task.CompletedTask;",
        "};",
        `await client.ConnectAsync(new MqttClientOptionsBuilder().WithTcpServer("${r.host}", ${r.port}).Build());`,
        "await client.SubscribeAsync(new MqttTopicFilterBuilder()",
        `    .WithTopic("${r.topic}")`,
        "    .WithQualityOfServiceLevel(MqttQualityOfServiceLevel.AtLeastOnce)",
        "    .Build());",
        `await Task.Delay(${r.timeoutMs});`,
        "await client.DisconnectAsync();",
      ].join("\n");
}

function mqttPhp(r: MqReq, dir: MqDirection): string {
  return dir === "produce"
    ? [
        "<?php",
        "// 依赖：composer require php-mqtt/client",
        "use PhpMqtt\\Client\\ConnectionSettings;",
        "use PhpMqtt\\Client\\MqttClient;",
        "",
        `// ${mqttNote(r)}`,
        `$client = new MqttClient("${r.host}", ${r.port}, "api-manager");`,
        "$client->connect((new ConnectionSettings())->setKeepAliveInterval(30));",
        `$client->publish("${r.topic}", ${j(r.body)}, 1);`,
        "$client->disconnect();",
      ].join("\n")
    : [
        "<?php",
        "// 依赖：composer require php-mqtt/client",
        "use PhpMqtt\\Client\\ConnectionSettings;",
        "use PhpMqtt\\Client\\MqttClient;",
        "",
        `// ${mqttNote(r)}`,
        `$client = new MqttClient("${r.host}", ${r.port}, "api-manager");`,
        "$client->connect((new ConnectionSettings())->setKeepAliveInterval(30));",
        `$client->subscribe("${r.topic}", function (string $topic, string $message) {`,
        "    echo $message . PHP_EOL;",
        "}, 1);",
        "$client->loop(true); // 常驻订阅，收到消息后按需 Ctrl+C 结束",
      ].join("\n");
}

function mqttRuby(r: MqReq, dir: MqDirection): string {
  return dir === "produce"
    ? [
        "# 依赖：gem install mqtt",
        'require "mqtt"',
        "",
        `# ${mqttNote(r)}`,
        `MQTT::Client.connect(host: "${r.host}", port: ${r.port}) do |client|`,
        `  client.publish("${r.topic}", ${j(r.body)}, qos: 1)`,
        "end",
      ].join("\n")
    : [
        "# 依赖：gem install mqtt",
        'require "mqtt"',
        "",
        `# ${mqttNote(r)}`,
        `MQTT::Client.connect(host: "${r.host}", port: ${r.port}) do |client|`,
        `  topic, message = client.get("${r.topic}")`,
        '  puts "#{topic}: #{message}"',
        "end",
      ].join("\n");
}

// ---------------------------------------------------------------- Pulsar

function pulsarPython(r: MqReq, dir: MqDirection): string {
  const topic = pulsarTopic(r.topic);
  return dir === "produce"
    ? [
        "# 依赖：pip install pulsar-client",
        "import pulsar",
        "",
        `# Pulsar 生产：topic ${topic}`,
        `client = pulsar.Client("pulsar://${r.target}")`,
        `producer = client.create_producer("${topic}")`,
        `producer.send(${j(r.body)}.encode("utf-8"))`,
        "producer.close()",
        "client.close()",
      ].join("\n")
    : [
        "# 依赖：pip install pulsar-client",
        "import pulsar",
        "",
        `# Pulsar 消费：topic ${topic}`,
        `client = pulsar.Client("pulsar://${r.target}")`,
        "consumer = client.subscribe(",
        `    "${topic}",`,
        `    subscription_name="${r.group}",`,
        `    initial_position=pulsar.InitialPosition.${earliest(r) ? "Earliest" : "Latest"},`,
        ")",
        `for _ in range(${r.maxMessages}):`,
        `    msg = consumer.receive(timeout_millis=${r.timeoutMs})  # 超时未收到会抛异常`,
        '    print(msg.data().decode("utf-8"))',
        "    consumer.acknowledge(msg)",
        "consumer.close()",
        "client.close()",
      ].join("\n");
}

function pulsarJs(r: MqReq, dir: MqDirection): string {
  const topic = pulsarTopic(r.topic);
  return dir === "produce"
    ? [
        "// 依赖：npm i pulsar-client",
        'const Pulsar = require("pulsar-client");',
        "",
        `// Pulsar 生产：topic ${topic}`,
        "(async () => {",
        `  const client = new Pulsar.Client({ serviceUrl: "pulsar://${r.target}" });`,
        `  const producer = await client.createProducer({ topic: "${topic}" });`,
        `  await producer.send({ data: Buffer.from(${j(r.body)}) });`,
        "  await producer.close();",
        "  await client.close();",
        "})();",
      ].join("\n")
    : [
        "// 依赖：npm i pulsar-client",
        'const Pulsar = require("pulsar-client");',
        "",
        `// Pulsar 消费：topic ${topic}`,
        "(async () => {",
        `  const client = new Pulsar.Client({ serviceUrl: "pulsar://${r.target}" });`,
        "  const consumer = await client.subscribe({",
        `    topic: "${topic}",`,
        `    subscription: "${r.group}",`,
        `    subscriptionInitialPosition: "${earliest(r) ? "Earliest" : "Latest"}",`,
        "  });",
        `  for (let i = 0; i < ${r.maxMessages}; i++) {`,
        `    const msg = await consumer.receive(${r.timeoutMs}); // 超时抛错`,
        "    console.log(msg.getData().toString());",
        "    await consumer.acknowledge(msg);",
        "  }",
        "  await consumer.close();",
        "  await client.close();",
        "})();",
      ].join("\n");
}

function pulsarJava(r: MqReq, dir: MqDirection): string {
  const produce = dir === "produce";
  const cls = `Mq${produce ? "Producer" : "Consumer"}`;
  const topic = pulsarTopic(r.topic);
  return produce
    ? [
        "// 依赖（Maven）：org.apache.pulsar:pulsar-client:3.3.0",
        "import java.nio.charset.StandardCharsets;",
        "import org.apache.pulsar.client.api.Producer;",
        "import org.apache.pulsar.client.api.PulsarClient;",
        "",
        `// Pulsar 生产：topic ${topic}`,
        `public class ${cls} {`,
        "    public static void main(String[] args) throws Exception {",
        `        try (PulsarClient client = PulsarClient.builder().serviceUrl("pulsar://${r.target}").build();`,
        `             Producer<byte[]> producer = client.newProducer().topic("${topic}").create()) {`,
        `            producer.send(${j(r.body)}.getBytes(StandardCharsets.UTF_8));`,
        "        }",
        "    }",
        "}",
      ].join("\n")
    : [
        "// 依赖（Maven）：org.apache.pulsar:pulsar-client:3.3.0",
        "import java.nio.charset.StandardCharsets;",
        "import java.util.concurrent.TimeUnit;",
        "import org.apache.pulsar.client.api.Consumer;",
        "import org.apache.pulsar.client.api.Message;",
        "import org.apache.pulsar.client.api.PulsarClient;",
        "import org.apache.pulsar.client.api.SubscriptionInitialPosition;",
        "",
        `// Pulsar 消费：topic ${topic}`,
        `public class ${cls} {`,
        "    public static void main(String[] args) throws Exception {",
        `        try (PulsarClient client = PulsarClient.builder().serviceUrl("pulsar://${r.target}").build();`,
        "             Consumer<byte[]> consumer = client.newConsumer()",
        `                 .topic("${topic}")`,
        `                 .subscriptionName("${r.group}")`,
        `                 .subscriptionInitialPosition(SubscriptionInitialPosition.${earliest(r) ? "Earliest" : "Latest"})`,
        "                 .subscribe()) {",
        `            for (int i = 0; i < ${r.maxMessages}; i++) {`,
        `                Message<byte[]> msg = consumer.receive(${secs(r)}, TimeUnit.SECONDS);`,
        "                System.out.println(new String(msg.getData(), StandardCharsets.UTF_8));",
        "                consumer.acknowledge(msg);",
        "            }",
        "        }",
        "    }",
        "}",
      ].join("\n");
}

function pulsarKotlin(r: MqReq, dir: MqDirection): string {
  const topic = pulsarTopic(r.topic);
  return dir === "produce"
    ? [
        '// 依赖（build.gradle.kts）：implementation("org.apache.pulsar:pulsar-client:3.3.0")',
        "import org.apache.pulsar.client.api.PulsarClient",
        "",
        `// Pulsar 生产：topic ${topic}`,
        "fun main() {",
        `    val client = PulsarClient.builder().serviceUrl("pulsar://${r.target}").build()`,
        "    client.newProducer()",
        `        .topic("${topic}")`,
        "        .create()",
        "        .use { producer ->",
        `            producer.send(${j(r.body)}.toByteArray())`,
        "        }",
        "    client.close()",
        "}",
      ].join("\n")
    : [
        '// 依赖（build.gradle.kts）：implementation("org.apache.pulsar:pulsar-client:3.3.0")',
        "import java.util.concurrent.TimeUnit",
        "import org.apache.pulsar.client.api.PulsarClient",
        "import org.apache.pulsar.client.api.SubscriptionInitialPosition",
        "",
        `// Pulsar 消费：topic ${topic}`,
        "fun main() {",
        `    val client = PulsarClient.builder().serviceUrl("pulsar://${r.target}").build()`,
        "    client.newConsumer()",
        `        .topic("${topic}")`,
        `        .subscriptionName("${r.group}")`,
        `        .subscriptionInitialPosition(SubscriptionInitialPosition.${earliest(r) ? "Earliest" : "Latest"})`,
        "        .subscribe()",
        "        .use { consumer ->",
        `            repeat(${r.maxMessages}) {`,
        `                val msg = consumer.receive(${secs(r)}, TimeUnit.SECONDS)`,
        "                println(String(msg.data))",
        "                consumer.acknowledge(msg)",
        "            }",
        "        }",
        "    client.close()",
        "}",
      ].join("\n");
}

function pulsarGo(r: MqReq, dir: MqDirection): string {
  const topic = pulsarTopic(r.topic);
  return dir === "produce"
    ? [
        "// 依赖：go get github.com/apache/pulsar-client-go/pulsar",
        "package main",
        "",
        "import (",
        '\t"context"',
        "",
        '\t"github.com/apache/pulsar-client-go/pulsar"',
        ")",
        "",
        `// Pulsar 生产：topic ${topic}`,
        "func main() {",
        `\tclient, err := pulsar.NewClient(pulsar.ClientOptions{URL: "pulsar://${r.target}"})`,
        "\tif err != nil {",
        "\t\tpanic(err)",
        "\t}",
        "\tdefer client.Close()",
        "",
        `\tproducer, err := client.CreateProducer(pulsar.ProducerOptions{Topic: "${topic}"})`,
        "\tif err != nil {",
        "\t\tpanic(err)",
        "\t}",
        "\tdefer producer.Close()",
        "",
        `\tif _, err := producer.Send(context.Background(), &pulsar.ProducerMessage{Payload: []byte(${j(r.body)})}); err != nil {`,
        "\t\tpanic(err)",
        "\t}",
        "}",
      ].join("\n")
    : [
        "// 依赖：go get github.com/apache/pulsar-client-go/pulsar",
        "package main",
        "",
        "import (",
        '\t"context"',
        '\t"fmt"',
        '\t"time"',
        "",
        '\t"github.com/apache/pulsar-client-go/pulsar"',
        ")",
        "",
        `// Pulsar 消费：topic ${topic}`,
        "func main() {",
        `\tclient, err := pulsar.NewClient(pulsar.ClientOptions{URL: "pulsar://${r.target}"})`,
        "\tif err != nil {",
        "\t\tpanic(err)",
        "\t}",
        "\tdefer client.Close()",
        "",
        "\tconsumer, err := client.Subscribe(pulsar.ConsumerOptions{",
        `\t\tTopic:            "${topic}",`,
        `\t\tSubscriptionName: "${r.group}",`,
        `\t\tSubscriptionInitialPosition: pulsar.SubscriptionPosition${earliest(r) ? "Earliest" : "Latest"},`,
        "\t})",
        "\tif err != nil {",
        "\t\tpanic(err)",
        "\t}",
        "\tdefer consumer.Close()",
        "",
        `\tctx, cancel := context.WithTimeout(context.Background(), ${r.timeoutMs}*time.Millisecond)`,
        "\tdefer cancel()",
        "",
        `\tfor i := 0; i < ${r.maxMessages}; i++ {`,
        "\t\tmsg, err := consumer.Receive(ctx)",
        "\t\tif err != nil {",
        "\t\t\tpanic(err)",
        "\t\t}",
        "\t\tfmt.Println(string(msg.Payload()))",
        "\t\tconsumer.Ack(msg)",
        "\t}",
        "}",
      ].join("\n");
}

function pulsarCsharp(r: MqReq, dir: MqDirection): string {
  const topic = pulsarTopic(r.topic);
  return dir === "produce"
    ? [
        "// 依赖：dotnet add package DotPulsar",
        "using System.Text;",
        "using DotPulsar;",
        "using DotPulsar.Extensions;",
        "",
        `// Pulsar 生产：topic ${topic}`,
        `await using var client = PulsarClient.Builder().ServiceUrl(new Uri("pulsar://${r.target}")).Build();`,
        "await using var producer = client.NewProducer()",
        `    .Topic("${topic}")`,
        "    .Create();",
        `await producer.Send(Encoding.UTF8.GetBytes(${j(r.body)}));`,
      ].join("\n")
    : [
        "// 依赖：dotnet add package DotPulsar",
        "using System.Text;",
        "using DotPulsar;",
        "using DotPulsar.Extensions;",
        "",
        `// Pulsar 消费：topic ${topic}`,
        `await using var client = PulsarClient.Builder().ServiceUrl(new Uri("pulsar://${r.target}")).Build();`,
        "await using var consumer = client.NewConsumer()",
        `    .SubscriptionName("${r.group}")`,
        `    .Topic("${topic}")`,
        `    .InitialPosition(SubscriptionInitialPosition.${earliest(r) ? "Earliest" : "Latest"})`,
        "    .Create();",
        "",
        `using var cts = new CancellationTokenSource(TimeSpan.FromMilliseconds(${r.timeoutMs}));`,
        `for (var i = 0; i < ${r.maxMessages}; i++)`,
        "{",
        "    var message = await consumer.Receive(cts.Token);",
        "    Console.WriteLine(Encoding.UTF8.GetString(message.Data.ToArray()));",
        "    await consumer.Acknowledge(message, cts.Token);",
        "}",
      ].join("\n");
}

/** PHP / Ruby 暂无成熟的 Pulsar 官方客户端，给出命令行等价写法（避免编造库用法） */
function pulsarFallback(lang: CodeLang, r: MqReq, dir: MqDirection): string {
  return genFallback(lang, r, dir);
}

// ---------------------------------------------------------------- NATS

function natsPython(r: MqReq, dir: MqDirection): string {
  return dir === "produce"
    ? [
        "# 依赖：pip install nats-py",
        "import asyncio",
        "",
        "import nats",
        "",
        `# ${natsNote(r)}`,
        "",
        "async def main():",
        `    nc = await nats.connect("nats://${r.target}")`,
        `    await nc.publish("${r.topic}", ${j(r.body)}.encode("utf-8"))`,
        "    await nc.flush()",
        "    await nc.close()",
        "",
        "",
        "asyncio.run(main())",
      ].join("\n")
    : [
        "# 依赖：pip install nats-py",
        "import asyncio",
        "",
        "import nats",
        "",
        `# ${natsNote(r)}`,
        "",
        "",
        "async def main():",
        `    nc = await nats.connect("nats://${r.target}")`,
        "    received = 0",
        "",
        "    async def on_message(msg):",
        "        nonlocal received",
        '        print(msg.data.decode("utf-8"))',
        "        received += 1",
        `        if received >= ${r.maxMessages}:`,
        "            await nc.close()",
        "",
        `    await nc.subscribe("${r.topic}", cb=on_message)`,
        "    await nc.flush()",
        `    await asyncio.sleep(${secs(r)})  # 收到指定条数后连接会自动关闭`,
        "",
        "",
        "asyncio.run(main())",
      ].join("\n");
}

function natsJs(r: MqReq, dir: MqDirection): string {
  return dir === "produce"
    ? [
        "// 依赖：npm i nats",
        'const { connect } = require("nats");',
        "",
        `// ${natsNote(r)}`,
        "(async () => {",
        `  const nc = await connect({ servers: "nats://${r.target}" });`,
        `  nc.publish("${r.topic}", Buffer.from(${j(r.body)}));`,
        "  await nc.flush();",
        "  await nc.drain();",
        "})();",
      ].join("\n")
    : [
        "// 依赖：npm i nats",
        'const { connect } = require("nats");',
        "",
        `// ${natsNote(r)}`,
        "(async () => {",
        `  const nc = await connect({ servers: "nats://${r.target}" });`,
        `  const sub = nc.subscribe("${r.topic}");`,
        "  for await (const msg of sub) {",
        "    console.log(msg.string());",
        "    break;",
        "  }",
        "  await nc.drain();",
        "})();",
      ].join("\n");
}

function natsJava(r: MqReq, dir: MqDirection): string {
  const produce = dir === "produce";
  const cls = `Mq${produce ? "Producer" : "Consumer"}`;
  return produce
    ? [
        "// 依赖（Maven）：io.nats:jnats:2.20.5",
        "import io.nats.client.Connection;",
        "import io.nats.client.Nats;",
        "import java.nio.charset.StandardCharsets;",
        "import java.time.Duration;",
        "",
        `// ${natsNote(r)}`,
        `public class ${cls} {`,
        "    public static void main(String[] args) throws Exception {",
        `        try (Connection nc = Nats.connect("nats://${r.target}")) {`,
        `            nc.publish("${r.topic}", ${j(r.body)}.getBytes(StandardCharsets.UTF_8));`,
        "            nc.flush(Duration.ofSeconds(3));",
        "        }",
        "    }",
        "}",
      ].join("\n")
    : [
        "// 依赖（Maven）：io.nats:jnats:2.20.5",
        "import io.nats.client.Connection;",
        "import io.nats.client.Dispatcher;",
        "import io.nats.client.Nats;",
        "import java.nio.charset.StandardCharsets;",
        "",
        `// ${natsNote(r)}`,
        `public class ${cls} {`,
        "    public static void main(String[] args) throws Exception {",
        `        try (Connection nc = Nats.connect("nats://${r.target}")) {`,
        "            Dispatcher dispatcher = nc.createDispatcher(message ->",
        "                System.out.println(new String(message.getData(), StandardCharsets.UTF_8)));",
        `            dispatcher.subscribe("${r.topic}");`,
        `            Thread.sleep(${r.timeoutMs});`,
        "        }",
        "    }",
        "}",
      ].join("\n");
}

function natsKotlin(r: MqReq, dir: MqDirection): string {
  return dir === "produce"
    ? [
        '// 依赖（build.gradle.kts）：implementation("io.nats:jnats:2.20.5")',
        "import io.nats.client.Nats",
        "import java.time.Duration",
        "",
        `// ${natsNote(r)}`,
        "fun main() {",
        `    Nats.connect("nats://${r.target}").use { nc ->`,
        `        nc.publish("${r.topic}", ${j(r.body)}.toByteArray())`,
        "        nc.flush(Duration.ofSeconds(3))",
        "    }",
        "}",
      ].join("\n")
    : [
        '// 依赖（build.gradle.kts）：implementation("io.nats:jnats:2.20.5")',
        "import io.nats.client.Nats",
        "",
        `// ${natsNote(r)}`,
        "fun main() {",
        `    Nats.connect("nats://${r.target}").use { nc ->`,
        `        val dispatcher = nc.createDispatcher { msg -> println(String(msg.data)) }`,
        `        dispatcher.subscribe("${r.topic}")`,
        `        Thread.sleep(${r.timeoutMs})`,
        "    }",
        "}",
      ].join("\n");
}

function natsGo(r: MqReq, dir: MqDirection): string {
  return dir === "produce"
    ? [
        "// 依赖：go get github.com/nats-io/nats.go",
        "package main",
        "",
        'import "github.com/nats-io/nats.go"',
        "",
        `// ${natsNote(r)}`,
        "func main() {",
        `\tnc, err := nats.Connect("nats://${r.target}")`,
        "\tif err != nil {",
        "\t\tpanic(err)",
        "\t}",
        "\tdefer nc.Close()",
        "",
        `\tif err := nc.Publish("${r.topic}", []byte(${j(r.body)})); err != nil {`,
        "\t\tpanic(err)",
        "\t}",
        "\t_ = nc.Flush()",
        "}",
      ].join("\n")
    : [
        "// 依赖：go get github.com/nats-io/nats.go",
        "package main",
        "",
        "import (",
        '\t"fmt"',
        '\t"time"',
        "",
        '\t"github.com/nats-io/nats.go"',
        ")",
        "",
        `// ${natsNote(r)}`,
        "func main() {",
        `\tnc, err := nats.Connect("nats://${r.target}")`,
        "\tif err != nil {",
        "\t\tpanic(err)",
        "\t}",
        "\tdefer nc.Close()",
        "",
        `\tsub, err := nc.SubscribeSync("${r.topic}")`,
        "\tif err != nil {",
        "\t\tpanic(err)",
        "\t}",
        "\t_ = nc.Flush()",
        "",
        `\tfor i := 0; i < ${r.maxMessages}; i++ {`,
        `\t\tmsg, err := sub.NextMsg(${r.timeoutMs} * time.Millisecond)`,
        "\t\tif err != nil {",
        "\t\t\tpanic(err)",
        "\t\t}",
        "\t\tfmt.Println(string(msg.Data))",
        "\t}",
        "}",
      ].join("\n");
}

function natsCsharp(r: MqReq, dir: MqDirection): string {
  return dir === "produce"
    ? [
        "// 依赖：dotnet add package NATS.Client.Core",
        "using NATS.Client.Core;",
        "",
        `// ${natsNote(r)}`,
        `await using var client = new NatsClient("nats://${r.target}");`,
        `await client.PublishAsync("${r.topic}", ${j(r.body)});`,
      ].join("\n")
    : [
        "// 依赖：dotnet add package NATS.Client.Core",
        "using NATS.Client.Core;",
        "",
        `// ${natsNote(r)}`,
        `await using var client = new NatsClient("nats://${r.target}");`,
        `await foreach (var msg in client.SubscribeAsync<string>("${r.topic}"))`,
        "{",
        "    Console.WriteLine(msg.Data);",
        "    break;",
        "}",
      ].join("\n");
}

function natsPhp(r: MqReq, dir: MqDirection): string {
  return dir === "produce"
    ? [
        "<?php",
        "// 依赖：composer require basis-company/nats",
        "use Basis\\Nats\\Client;",
        "",
        `// ${natsNote(r)}`,
        `$client = new Client(["host" => "${r.host}", "port" => ${r.port}]);`,
        `$client->publish("${r.topic}", ${j(r.body)});`,
      ].join("\n")
    : [
        "<?php",
        "// 依赖：composer require basis-company/nats",
        "use Basis\\Nats\\Client;",
        "",
        `// ${natsNote(r)}`,
        `$client = new Client(["host" => "${r.host}", "port" => ${r.port}]);`,
        `$client->subscribe("${r.topic}", function ($message) {`,
        '    echo $message->payload . PHP_EOL;',
        "});",
        `$client->process(${r.maxMessages}); // 处理指定条数后返回`,
      ].join("\n");
}

function natsRuby(r: MqReq, dir: MqDirection): string {
  return dir === "produce"
    ? [
        "# 依赖：gem install nats-pure",
        'require "nats"',
        "",
        `# ${natsNote(r)}`,
        `nc = NATS.connect("nats://${r.target}")`,
        `nc.publish("${r.topic}", ${j(r.body)})`,
        "nc.flush",
        "nc.close",
      ].join("\n")
    : [
        "# 依赖：gem install nats-pure",
        'require "nats"',
        "",
        `# ${natsNote(r)}`,
        `nc = NATS.connect("nats://${r.target}")`,
        `sub = nc.subscribe("${r.topic}")`,
        `msg = sub.next_msg(timeout: ${secs(r)})`,
        "puts msg.data",
        "nc.close",
      ].join("\n");
}

/** 协议族生成函数表：语言 → 协议族（MQTT 系 / Pulsar / NATS） */
const FAMILY_GENERATORS: Partial<
  Record<CodeLang, Partial<Record<MqFamily, (r: MqReq, dir: MqDirection) => string>>>
> = {
  python: { mqtt: mqttPython, pulsar: pulsarPython, nats: natsPython },
  javascript: { mqtt: mqttJs, pulsar: pulsarJs, nats: natsJs },
  typescript: { mqtt: mqttJs, pulsar: pulsarJs, nats: natsJs },
  java: { mqtt: mqttJava, pulsar: pulsarJava, nats: natsJava },
  kotlin: { mqtt: mqttKotlin, pulsar: pulsarKotlin, nats: natsKotlin },
  go: { mqtt: mqttGo, pulsar: pulsarGo, nats: natsGo },
  csharp: { mqtt: mqttCsharp, pulsar: pulsarCsharp, nats: natsCsharp },
  php: { mqtt: mqttPhp, pulsar: (r, dir) => pulsarFallback("php", r, dir), nats: natsPhp },
  ruby: { mqtt: mqttRuby, pulsar: (r, dir) => pulsarFallback("ruby", r, dir), nats: natsRuby },
};

/** 生成新增消息队列（MQTT 系 / Pulsar / NATS）的代码；该语言暂无内置实现时退回命令行提示 */
function genMqFamily(lang: CodeLang, r: MqReq, dir: MqDirection): string {
  const gen = FAMILY_GENERATORS[lang]?.[mqFamily(r.kind)];
  return gen ? gen(r, dir) : genFallback(lang, r, dir);
}

// ---------------------------------------------------------------- 兜底：无内置客户端的语言

/** 各种语言的注释前缀（兜底提示用） */
const COMMENT_PREFIX: Partial<Record<CodeLang, string>> = {
  c: "//",
  cpp: "//",
  rust: "//",
  swift: "//",
  perl: "#",
  objectivec: "//",
  julia: "#",
  r: "#",
  delphi: "//",
  erlang: "%",
  lua: "--",
  powershell: "#",
};

function genFallback(lang: CodeLang, r: MqReq, dir: MqDirection): string {
  const p = COMMENT_PREFIX[lang] || "//";
  const lines = [
    `${p} ${r.kindLabel} ${dir === "produce" ? "生产" : "消费"}：topic ${r.topic} @ ${r.target}`,
    `${p} 暂未内置 ${lang} 客户端示例，可直接使用命令行客户端完成同样的操作：`,
    `${p}`,
    ...cliCommands(r, dir).map((l) => `${p} ${l}`),
    `${p}`,
    `${p} 也可安装官方客户端库后参考「Bash」示例的参数自行实现。`,
  ];
  return lines.join("\n");
}

// ---------------------------------------------------------------- 对外入口

/** 按语言 / 方向生成 MQ 生产或消费代码 */
export function generateMqCode(
  lang: CodeLang,
  api: ApiFile,
  direction: MqDirection = "produce"
): string {
  const r = buildMqReq(api);
  switch (lang) {
    case "curl":
    case "bash":
      return genBash(r, direction);
    case "python":
      return genPython(r, direction);
    case "javascript":
    case "typescript":
      return genJs(r, direction);
    case "java":
      return genJava(r, direction);
    case "kotlin":
      return genKotlin(r, direction);
    case "go":
      return genGo(r, direction);
    case "csharp":
      return genCsharp(r, direction);
    case "php":
      return genPhp(r, direction);
    case "ruby":
      return genRuby(r, direction);
    default:
      return genFallback(lang, r, direction);
  }
}
