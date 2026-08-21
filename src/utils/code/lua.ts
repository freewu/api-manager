/** Lua（luasocket / lua-curl / lua-resty-httpclient；lua-resty-websocket / lua-websocket）代码生成 */

import { esc, Req, WsReq } from "./shared";
export function genLua(r: Req): string {
  const out: string[] = [];
  out.push('local http = require("socket.http")');
  out.push('local ltn12 = require("ltn12")');
  out.push("");
  out.push(`local url = "${esc(r.url, '"')}"`);
  if (r.body) {
    out.push("");
    out.push(`local payload = "${esc(r.body, '"')}"`);
  }
  out.push("");
  out.push("local response_body = {}");
  out.push("local res, code, headers = http.request{");
  out.push("    url = url,");
  out.push(`    method = "${r.method}",`);
  if (r.headers.length || r.bodyKind === "json") {
    out.push("    headers = {");
    for (const h of r.headers) out.push(`        ["${esc(h.key, '"')}"] = "${esc(h.value, '"')}",`);
    if (r.bodyKind === "json" && !r.headers.some((h) => h.key.toLowerCase() === "content-type")) {
      out.push('        ["Content-Type"] = "application/json",');
    }
    out.push("    },");
  }
  if (r.body) out.push("    source = ltn12.source.string(payload),");
  out.push("    sink = ltn12.sink.table(response_body),");
  out.push("}");
  out.push("");
  out.push("print(code)");
  out.push("print(table.concat(response_body))");
  return out.join("\n");
}

export function genLuaCurl(r: Req): string {
  const out: string[] = [];
  out.push('local curl = require("lcurl")');
  out.push("");
  out.push("local c = curl.easy()");
  out.push(`c:setopt(curl.OPT_URL, "${esc(r.url, '"')}")`);
  out.push(`c:setopt(curl.OPT_CUSTOMREQUEST, "${r.method}")`);
  if (r.headers.length || r.bodyKind === "json") {
    out.push("c:setopt(curl.OPT_HTTPHEADER, {");
    for (const h of r.headers) out.push(`    "${esc(h.key, '"')}: ${esc(h.value, '"')}",`);
    if (r.bodyKind === "json" && !r.headers.some((h) => h.key.toLowerCase() === "content-type")) {
      out.push('    "Content-Type: application/json",');
    }
    out.push("})");
  }
  if (r.files.length) {
    out.push("// 注意：lua-curl 文件上传请使用 OPT_HTTPPOST / OPT_MIMEPOST 构造 multipart");
    for (const f of r.files) out.push(`c:setopt(curl.OPT_HTTPPOST, { "${esc(f.key, '"')}" = { file = "${esc(f.path, '"')}" } })`);
  } else if (r.body) {
    out.push(`c:setopt(curl.OPT_POSTFIELDS, "${esc(r.body, '"')}")`);
  }
  out.push("c:setopt(curl.OPT_WRITEFUNCTION, function(buffer)");
  out.push("    io.write(buffer)");
  out.push("    return #buffer");
  out.push("end)");
  out.push("");
  out.push("local ok, err = c:perform()");
  out.push("if not ok then");
  out.push('    io.stderr:write(err .. "\\n")');
  out.push("end");
  out.push("c:close()");
  return out.join("\n");
}

export function genLuaResty(r: Req): string {
  const out: string[] = [];
  out.push("-- 需要 OpenResty / Nginx Lua 环境（ngx_lua + lua-resty-http）");
  out.push('local http = require("resty.http")');
  out.push("local httpc = http.new()");
  out.push("");
  out.push(`local res, err = httpc:request_uri("${esc(r.url, '"')}", {`);
  out.push(`    method = "${r.method}",`);
  if (r.headers.length || r.bodyKind === "json") {
    out.push("    headers = {");
    for (const h of r.headers) out.push(`        ["${esc(h.key, '"')}"] = "${esc(h.value, '"')}",`);
    if (r.bodyKind === "json" && !r.headers.some((h) => h.key.toLowerCase() === "content-type")) {
      out.push('        ["Content-Type"] = "application/json",');
    }
    out.push("    },");
  }
  if (r.body) out.push(`    body = "${esc(r.body, '"')}",`);
  out.push("})");
  out.push("");
  out.push("if not res then");
  out.push("    ngx.log(ngx.ERR, err)");
  out.push("    return");
  out.push("end");
  out.push("ngx.say(res.status)");
  out.push("ngx.say(res.body)");
  out.push("httpc:close()");
  return out.join("\n");
}

export function genWsLuaDispatch(r: WsReq, lib?: string): string {
  switch (lib) {
    case "standalone":
      return genWsLuaStandalone(r);
    default:
      return genWsLuaResty(r);
  }
}

export function genWsLuaResty(r: WsReq): string {
  const out: string[] = [];
  out.push("-- WebSocket 客户端示例（lua-resty-websocket：OpenResty 生产首选，OpenResty 专属）");
  out.push("-- 官网: https://github.com/openresty/lua-resty-websocket");
  out.push("-- OpenResty 官网: https://openresty.org/");
  out.push("-- 适用: OpenResty（Nginx + LuaJIT）环境，运行于 content_by_lua / access 阶段");
  out.push('local client = require "resty.websocket.client"');
  out.push("");
  if (r.headers.length) {
    out.push("-- 自定义请求头（握手时发送）");
    out.push("local headers = {");
    for (const h of r.headers) out.push(`    [${JSON.stringify(h.key)}] = ${JSON.stringify(h.value)},`);
    out.push("}");
    out.push("");
  }
  out.push("local wb, err = client:new()");
  out.push("if not wb then");
  out.push("    ngx.log(ngx.ERR, \"创建失败: \", err)");
  out.push("    return");
  out.push("end");
  out.push("");
  out.push("-- 建立连接（wss:// 自动使用 TLS）");
  if (r.headers.length) {
    out.push(`local ok, err = wb:connect(${JSON.stringify(r.url)}, { headers = headers })`);
  } else {
    out.push(`local ok, err = wb:connect(${JSON.stringify(r.url)})`);
  }
  out.push("if not ok then");
  out.push("    ngx.log(ngx.ERR, \"连接失败: \", err)");
  out.push("    return");
  out.push("end");
  out.push("ngx.say(\">>> 连接成功\")");
  out.push("");
  out.push("-- 发送一条文本消息");
  out.push(`local bytes, err = wb:send_text(${JSON.stringify(r.message || "hello, this is a websocket echo message")})`);
  out.push("if not bytes then");
  out.push("    ngx.log(ngx.ERR, \"发送失败: \", err)");
  out.push("    return");
  out.push("end");
  out.push("ngx.say(\">>> 发送完成\")");
  out.push("");
  out.push("-- 接收回显（text 帧）");
  out.push("while true do");
  out.push("    local data, typ, err = wb:recv_frame()");
  out.push("    if not data then");
  out.push("        ngx.log(ngx.ERR, \"接收失败: \", err)");
  out.push("        break");
  out.push("    end");
  out.push("    if typ == \"text\" then");
  out.push("        ngx.say(\"<<< 接收: \", data)");
  out.push("        break");
  out.push("    elseif typ == \"close\" then");
  out.push("        ngx.say(\"连接已关闭\")");
  out.push("        break");
  out.push("    end");
  out.push("end");
  out.push("");
  out.push("wb:close()");
  return out.join("\n");
}

export function genWsLuaStandalone(r: WsReq): string {
  const out: string[] = [];
  out.push("-- WebSocket 客户端示例（lua-websocket：基于 luasocket 的纯 Lua 实现，独立脚本）");
  out.push("-- 官网: https://github.com/lipp/lua-websocket");
  out.push("-- 安装: luarocks install lua-websocket   （依赖 luasocket: luarocks install luasocket）");
  out.push("-- 运行: lua ws_client.lua   （Lua 5.1+ / LuaJIT）");
  out.push('local client = require "websocket.client"');
  out.push("");
  if (r.headers.length) {
    out.push("-- 自定义请求头（握手时发送，connect 第三个参数）");
    out.push("local headers = {");
    for (const h of r.headers) out.push(`    [${JSON.stringify(h.key)}] = ${JSON.stringify(h.value)},`);
    out.push("}");
    out.push("");
  }
  out.push("local ws, err = client.new()");
  out.push("if not ws then");
  out.push("    print(\"创建失败: \" .. tostring(err))");
  out.push("    return");
  out.push("end");
  out.push("");
  out.push("-- 建立连接（wss:// 时 client.new({ tls = true })）");
  if (r.headers.length) {
    out.push(`local ok, err = ws:connect(${JSON.stringify(r.url)}, nil, headers)`);
  } else {
    out.push(`local ok, err = ws:connect(${JSON.stringify(r.url)})`);
  }
  out.push("if not ok then");
  out.push("    print(\"连接失败: \" .. tostring(err))");
  out.push("    return");
  out.push("end");
  out.push("print(\">>> 连接成功\")");
  out.push("");
  out.push("-- 发送一条文本消息");
  out.push(`local ok, err = ws:send(${JSON.stringify(r.message || "hello, this is a websocket echo message")})`);
  out.push("if not ok then");
  out.push("    print(\"发送失败: \" .. tostring(err))");
  out.push("    return");
  out.push("end");
  out.push("print(\">>> 发送完成\")");
  out.push("");
  out.push("-- 接收回显（opcode 1 = text）");
  out.push("while true do");
  out.push("    local message, opcode, err = ws:receive()");
  out.push("    if not message then");
  out.push("        print(\"接收失败: \" .. tostring(err))");
  out.push("        break");
  out.push("    end");
  out.push("    if opcode == 1 then");
  out.push("        print(\"<<< 接收: \" .. message)");
  out.push("        break");
  out.push("    end");
  out.push("end");
  out.push("");
  out.push("ws:close()");
  return out.join("\n");
}

export function genLuaDispatch(lib: string | undefined, r: Req): string {
  switch (lib) {
    case "luacurl":
      return genLuaCurl(r);
    case "resty":
      return genLuaResty(r);
    default:
      return genLua(r);
  }
}
