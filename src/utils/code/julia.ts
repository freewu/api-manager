/** Julia（HTTP.jl / WebSocket.jl）代码生成 */

import { esc, Req, WsReq } from "./shared";
export function genJulia(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("# 该表单包含文件上传（multipart/form-data），请使用 HTTP.Multipart 构造请求");
  }
  out.push("using HTTP");
  out.push("");
  out.push(`url = "${esc(r.url, '"')}"`);
  if (r.headers.length) {
    out.push("");
    out.push("headers = [");
    for (const h of r.headers) out.push(`    "${esc(h.key, '"')}" => "${esc(h.value, '"')}",`);
    out.push("]");
  }
  if (r.body) {
    out.push("");
    out.push(`body = "${esc(r.body, '"')}"`);
  }
  out.push("");
  const args: string[] = [`"${r.method}"`, "url"];
  if (r.headers.length) args.push("headers");
  if (r.body) args.push("body");
  out.push(`resp = HTTP.request(${args.join(", ")})`);
  out.push("println(resp.status)");
  out.push("println(String(resp.body))");
  return out.join("\n");
}

export function genWsJulia(r: WsReq): string {
  const out: string[] = [];
  out.push("# WebSocket 客户端示例（WebSocket.jl：Julia 生态的 WebSocket 库）");
  out.push("# 官网: https://github.com/JuliaWeb/WebSocket.jl");
  out.push("# 安装: Pkg.add(\"WebSockets\")   （包名 WebSockets，仓库名 WebSocket.jl）");
  out.push("# 运行: julia ws_client.jl");
  out.push("using WebSockets");
  out.push("");
  if (r.headers.length) {
    out.push("# 自定义请求头（握手时发送）");
    out.push("headers = Dict{String,String}(");
    for (const h of r.headers) out.push(`    ${JSON.stringify(h.key)} => ${JSON.stringify(h.value)},`);
    out.push(")");
  } else {
    out.push("headers = Dict{String,String}()");
  }
  out.push("");
  out.push("# 建立连接（握手完成后回调 do 块，connect 返回后台任务）");
  out.push(`task = WebSockets.connect(${JSON.stringify(r.url)}, headers) do ws`);
  out.push("    println(\">>> 连接成功\")");
  out.push("");
  out.push("    # 发送一条文本消息");
  out.push(`    msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")}`);
  out.push("    send(ws, msg)");
  out.push("    println(\">>> 发送: \", msg)");
  out.push("");
  out.push("    # 循环接收消息");
  out.push("    while isopen(ws)");
  out.push("        message = receive(ws)");
  out.push("        if message.opcode == WebSockets.TEXT");
  out.push("            println(\"<<< 接收: \", String(message.data))");
  out.push("        end");
  out.push("    end");
  out.push("    println(\"连接已关闭\")");
  out.push("end");
  out.push("");
  out.push("# 等待连接任务结束");
  out.push("wait(task)");
  return out.join("\n");
}
