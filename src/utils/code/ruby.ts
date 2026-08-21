/** Ruby（net/http；faye-websocket / websocket-ruby / Sinatra / ActionCable）代码生成 */

import { esc, parseWsUrl, Req, WsReq } from "./shared";
export const RUBY_CLASSES: Record<string, string> = {
  GET: "Net::HTTP::Get",
  POST: "Net::HTTP::Post",
  PUT: "Net::HTTP::Put",
  DELETE: "Net::HTTP::Delete",
  PATCH: "Net::HTTP::Patch",
  HEAD: "Net::HTTP::Head",
  OPTIONS: "Net::HTTP::Options",
};

export function genRuby(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("# 该表单包含文件上传（multipart/form-data），请使用 Net::HTTP 配合 multipart 构造请求");
  }
  out.push("require 'net/http'");
  out.push("require 'uri'");
  if (r.bodyKind === "json") out.push("require 'json'");
  out.push("");
  out.push(`uri = URI.parse("${esc(r.url, '"')}")`);
  out.push("http = Net::HTTP.new(uri.host, uri.port)");
  out.push("http.use_ssl = uri.scheme == 'https'");
  out.push("");
  out.push(`request = ${RUBY_CLASSES[r.method] ?? "Net::HTTP::Get"}.new(uri.request_uri)`);
  for (const h of r.headers) {
    out.push(`request['${esc(h.key, "'")}'] = '${esc(h.value, "'")}'`);
  }
  if (r.body) {
    out.push(`request.body = '${esc(r.body, "'")}'`);
    out.push(`request.content_type = '${r.bodyKind === "json" ? "application/json" : "text/plain"}'`);
  }
  out.push("");
  out.push("response = http.request(request)");
  out.push("puts response.code");
  out.push("puts response.body");
  return out.join("\n");
}

export function genWsRuby(r: WsReq, lib?: string): string {
  switch (lib) {
    case "websocket-ruby":
      return genWsRubyWebsocket(r);
    case "sinatra":
      return genWsRubySinatra(r);
    case "actioncable":
      return genWsRubyActionCable(r);
    default:
      return genWsRubyFaye(r);
  }
}

export function genWsRubyFaye(r: WsReq): string {
  const out: string[] = [];
  out.push("# WebSocket 客户端示例（faye-websocket：最经典，配合 EventMachine，可做服务端 + 客户端）");
  out.push("# 官网: https://github.com/faye/faye-websocket-ruby");
  out.push("# 安装: gem install faye-websocket eventmachine");
  out.push("# 运行: ruby ws_client.rb");
  out.push("");
  out.push("require 'faye/websocket'");
  out.push("require 'eventmachine'");
  out.push("");
  out.push("EM.run do");
  if (r.headers.length) {
    out.push(`  ws = Faye::WebSocket::Client.new(${JSON.stringify(r.url)}, [], headers: {`);
    for (const h of r.headers) out.push(`    ${JSON.stringify(h.key)} => ${JSON.stringify(h.value)},`);
    out.push("  })");
  } else {
    out.push(`  ws = Faye::WebSocket::Client.new(${JSON.stringify(r.url)})`);
  }
  out.push("");
  out.push("  ws.on :open do |_event|");
  out.push(`    msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")}`);
  out.push("    puts \">>> 发送: #{msg}\"");
  out.push("    ws.send(msg)");
  out.push("  end");
  out.push("");
  out.push("  ws.on :message do |event|");
  out.push("    puts \"<<< 接收: #{event.data}\"");
  out.push("    ws.close");
  out.push("  end");
  out.push("");
  out.push("  ws.on :error do |event|");
  out.push("    puts \"连接失败: #{event.message}\"");
  out.push("  end");
  out.push("");
  out.push("  ws.on :close do |_event|");
  out.push("    puts \"连接已关闭\"");
  out.push("    EM.stop");
  out.push("  end");
  out.push("end");
  return out.join("\n");
}

export function genWsRubyWebsocket(r: WsReq): string {
  const u = parseWsUrl(r.url);
  const out: string[] = [];
  out.push("# WebSocket 客户端示例（websocket-ruby：轻量、纯 Ruby，负责帧编解码与握手，socket 层自实现）");
  out.push("# 官网: https://github.com/imanel/websocket-ruby");
  out.push("# 安装: gem install websocket");
  out.push("# 运行: ruby ws_client.rb");
  out.push("");
  out.push("require 'socket'");
  out.push("require 'websocket'");
  out.push("");
  out.push(`host = ${JSON.stringify(u.host)}`);
  out.push(`port = ${u.port}`);
  out.push(`url = ${JSON.stringify(r.url)}`);
  out.push("");
  out.push("# 1. 建立 TCP 连接");
  out.push("socket = TCPSocket.new(host, port)");
  out.push("");
  out.push("# 2. HTTP Upgrade 握手");
  if (r.headers.length) {
    out.push("handshake = WebSocket::ClientHandshake.new(url: url, headers: {");
    for (const h of r.headers) out.push(`  ${JSON.stringify(h.key)} => ${JSON.stringify(h.value)},`);
    out.push("})");
  } else {
    out.push("handshake = WebSocket::ClientHandshake.new(url: url)");
  }
  out.push("socket.write(handshake.to_s)");
  out.push("until handshake.finished?");
  out.push("  handshake << socket.readpartial(4096)");
  out.push("end");
  out.push("unless handshake.valid?");
  out.push("  puts \"握手失败: #{handshake.error}\"");
  out.push("  exit 1");
  out.push("end");
  out.push("puts \">>> 握手成功\"");
  out.push("");
  out.push("# 3. 发送一条文本消息");
  out.push(`msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")}`);
  out.push("frame = WebSocket::Frame::Outgoing::Client.new(version: handshake.version, data: msg, type: :text).to_s");
  out.push("socket.write(frame)");
  out.push("puts \">>> 发送: #{msg}\"");
  out.push("");
  out.push("# 4. 接收回显");
  out.push("incoming = WebSocket::Frame::Incoming::Client.new(version: handshake.version)");
  out.push("loop do");
  out.push("  incoming << socket.readpartial(4096)");
  out.push("  while (f = incoming.next)");
  out.push("    if %i[text binary].include?(f.type)");
  out.push("      puts \"<<< 接收: #{f.to_s}\"");
  out.push("      socket.close");
  out.push("      exit 0");
  out.push("    end");
  out.push("  end");
  out.push("end");
  return out.join("\n");
}

export function genWsRubySinatra(r: WsReq): string {
  const u = parseWsUrl(r.url);
  const endpoint = (u.path.split("?")[0] || "/");
  const out: string[] = [];
  out.push("# WebSocket 服务端集成示例（Sinatra + faye-websocket：web 项目集成，与普通 HTTP 路由共存）");
  out.push("# Sinatra 官网: https://sinatrarb.com/");
  out.push("# faye-websocket 官网: https://github.com/faye/faye-websocket-ruby");
  out.push("# 安装: gem install sinatra faye-websocket puma");
  out.push("# 运行: ruby ws_server.rb   （启动后监听 4567 端口）");
  out.push("# 客户端连接: ws://127.0.0.1:4567" + endpoint);
  out.push("");
  out.push("require 'sinatra'");
  out.push("require 'faye/websocket'");
  out.push("");
  out.push("set :server, :puma");
  out.push("Faye::WebSocket.load_adapter('puma')");
  out.push("");
  out.push(`get ${JSON.stringify(endpoint)} do`);
  out.push("  if Faye::WebSocket.websocket?(request.env)");
  out.push("    ws = Faye::WebSocket.new(request.env, [], ping: 15)");
  out.push("");
  out.push("    ws.on :open do |_event|");
  out.push("      puts \"客户端已连接\"");
  out.push("    end");
  out.push("");
  out.push("    ws.on :message do |event|");
  out.push("      puts \">>> 接收: #{event.data}\"");
  out.push("      ws.send(event.data)   # 回显");
  out.push("    end");
  out.push("");
  out.push("    ws.on :close do |_event|");
  out.push("      puts \"客户端已断开\"");
  out.push("      ws = nil");
  out.push("    end");
  out.push("");
  out.push("    ws.rack_response");
  out.push("  else");
  out.push(`    \"WebSocket 服务已启动，请使用 ws://127.0.0.1:4567${endpoint} 连接\\n\"`);
  out.push("  end");
  out.push("end");
  return out.join("\n");
}

export function genWsRubyActionCable(r: WsReq): string {
  const out: string[] = [];
  out.push("# Rails ActionCable 示例（Rails 框架内置 WebSocket，生产常用）");
  out.push("# 官网: https://guides.rubyonrails.org/action_cable_overview.html");
  out.push("# 内置（Rails 5+ 自带 ActionCable），无需额外安装 gem");
  out.push("# 1) 生成 Channel:  bin/rails generate channel Echo");
  out.push("# 2) 挂载路由:      config/routes.rb 中需有 mount ActionCable.server => '/cable'");
  out.push("# 3) 前端连接:      ActionCable 自带 consumer（app/javascript/channels/consumer.js）");
  out.push("");
  out.push("# ==== 服务端: app/channels/echo_channel.rb ====");
  out.push("class EchoChannel < ApplicationCable::Channel");
  out.push("  def subscribed");
  out.push(`    stream_from ${JSON.stringify("echo_channel")}`);
  out.push("  end");
  out.push("");
  out.push("  def echo(data)");
  out.push("    ActionCable.server.broadcast \"echo_channel\", data");
  out.push("  end");
  out.push("end");
  out.push("");
  out.push("# ==== 客户端: app/javascript/channels/echo_channel.js ====");
  out.push("import consumer from \"./consumer\"");
  out.push("");
  out.push("consumer.subscriptions.create(\"EchoChannel\", {");
  out.push("  received(data) {");
  out.push("    console.log(\"<<< 接收:\", data)");
  out.push("  },");
  out.push("");
  out.push("  echo(message) {");
  out.push(`    this.perform(\"echo\", { message: ${JSON.stringify(r.message || "hello, this is a websocket echo message")} })`);
  out.push("  }");
  out.push("});");
  return out.join("\n");
}
