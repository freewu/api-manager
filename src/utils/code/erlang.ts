/** Erlang（httpc / inets；gun）代码生成 */

import { esc, parseWsUrl, Req, WsReq } from "./shared";
export function genErlang(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("%% 该表单包含文件上传（multipart/form-data），请使用 httpc multipart 或 ibrowse 构造请求");
  }
  out.push("-module(request).");
  out.push("-export([main/0]).");
  out.push("");
  out.push("main() ->");
  out.push("    inets:start(),");
  out.push("    ssl:start(),");
  out.push(`    URL = "${esc(r.url, '"')}",`);
  out.push("    Headers = [");
  for (const h of r.headers) {
    out.push(`        {"${esc(h.key, '"')}", "${esc(h.value, '"')}"},`);
  }
  out.push("    ],");
  if (r.body) {
    out.push(`    Body = "${esc(r.body, '"')}",`);
    out.push(`    ContentType = "${r.bodyKind === "json" ? "application/json" : "text/plain"}",`);
  }
  out.push(`    Method = ${r.method.toLowerCase()},`);
  if (r.body) {
    out.push("    Request = {URL, Headers, ContentType, Body},");
  } else {
    out.push("    Request = {URL, Headers},");
  }
  out.push("    case httpc:request(Method, Request, [], []) of");
  out.push("        {ok, {{_, Status, _}, _, RespBody}} ->");
  out.push('            io:format("~p~n", [Status]),');
  out.push('            io:format("~s~n", [RespBody]);');
  out.push("        {error, Reason} ->");
  out.push('            io:format("Error: ~p~n", [Reason])');
  out.push("    end.");
  return out.join("\n");
}

export function genWsErlang(r: WsReq): string {
  const u = parseWsUrl(r.url);
  const out: string[] = [];
  out.push("%% WebSocket 客户端示例（gun：http/https/ws/wss，高性能，工业级，支持 HTTP2，erlang 生态首选）");
  out.push("%% 官网: https://ninenines.eu/docs/en/gun/2.1/");
  out.push("%% GitHub: https://github.com/ninenines/gun");
  out.push("%% 依赖（rebar3）: {deps, [{gun, \"2.1.0\"}]}");
  out.push("%% 编译运行: rebar3 compile && rebar3 shell");
  out.push("-module(ws_client).");
  out.push("-export([main/0]).");
  out.push("");
  out.push("main() ->");
  out.push(`    %% 1. 建立 TCP/TLS 连接（wss:// 时加 #{transport => tls}）`);
  out.push(`    {ok, Conn} = gun:open(${JSON.stringify(u.host)}, ${u.port}),`);
  out.push("");
  out.push("    %% 2. 发起 WebSocket 握手升级（可携带自定义请求头）");
  if (r.headers.length) {
    out.push(`    StreamRef = gun:ws_upgrade(Conn, ${JSON.stringify(u.path)}, #{`);
    for (const h of r.headers) out.push(`        ${JSON.stringify(h.key)} => ${JSON.stringify(h.value)},`);
    out.push("    }),");
  } else {
    out.push(`    StreamRef = gun:ws_upgrade(Conn, ${JSON.stringify(u.path)}),`);
  }
  out.push("");
  out.push("    receive");
  out.push("        {gun_upgrade, Conn, StreamRef, _} ->");
  out.push("            io:format(\">>> 连接成功~n\")");
  out.push("            %% 3. 发送一条文本消息");
  out.push(`            gun:ws_send(Conn, StreamRef, {text, ${JSON.stringify(r.message || "hello, this is a websocket echo message")}}),`);
  out.push("            io:format(\">>> 发送完成~n\")");
  out.push("            receive_loop(Conn, StreamRef);");
  out.push("        {gun_response, Conn, StreamRef, fin, Status, _Headers} ->");
  out.push("            io:format(\"连接失败: HTTP ~p~n\", [Status]);");
  out.push("        {gun_error, Conn, StreamRef, Reason} ->");
  out.push("            io:format(\"连接失败: ~p~n\", [Reason])");
  out.push("    after 5000 ->");
  out.push("        io:format(\"连接超时~n\")");
  out.push("    end.");
  out.push("");
  out.push("receive_loop(Conn, StreamRef) ->");
  out.push("    receive");
  out.push("        {gun_ws, Conn, StreamRef, {text, Data}} ->");
  out.push("            io:format(\"<<< 接收: ~s~n\", [Data]),");
  out.push("            gun:close(Conn);");
  out.push("        {gun_ws, Conn, StreamRef, {binary, Data}} ->");
  out.push("            io:format(\"<<< 接收(binary): ~s~n\", [Data]),");
  out.push("            receive_loop(Conn, StreamRef);");
  out.push("        {gun_ws, Conn, StreamRef, {close, Code, Reason}} ->");
  out.push("            io:format(\"连接已关闭: ~p ~s~n\", [Code, Reason]);");
  out.push("        {gun_down, Conn, _Reason, _} ->");
  out.push("            io:format(\"连接已关闭~n\")");
  out.push("    end.");
  return out.join("\n");
}
