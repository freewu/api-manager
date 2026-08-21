/** Python（urllib / http.client / requests / websockets / websocket-client）代码生成 */

import { esc, Req, WsReq } from "./shared";
export function genPython(r: Req): string {
  const out: string[] = [];
  out.push("import requests");
  if (r.bodyKind === "json") out.push("import json");
  out.push("");
  out.push(`url = "${esc(r.url, '"')}"`);
  if (r.headers.length) {
    out.push("");
    out.push("headers = {");
    for (const h of r.headers) out.push(`    "${esc(h.key, '"')}": "${esc(h.value, '"')}",`);
    out.push("}");
  }
  if (r.body) {
    out.push("");
    out.push(`payload = """${esc(r.body, '"')}"""`);
  }
  if (r.files.length) {
    out.push("");
    if (r.formText.length) {
      out.push("data = {");
      for (const t of r.formText) out.push(`    "${esc(t.key, '"')}": "${esc(t.value, '"')}",`);
      out.push("}");
    }
    out.push("files = {");
    for (const f of r.files) out.push(`    "${esc(f.key, '"')}": open("${esc(f.path, '"')}", "rb"),`);
    out.push("}");
  }
  out.push("");
  const m = r.method.toLowerCase();
  const args: string[] = [];
  if (r.headers.length) args.push("headers=headers");
  if (r.bodyKind === "json") args.push("json=json.loads(payload)");
  else if (r.bodyKind === "text") args.push("data=payload");
  if (r.files.length) args.push("files=files");
  if (r.files.length && r.formText.length) {
    // 用 data 字典而不是 urlencoded payload
    args.splice(args.indexOf("data=payload"), 1, "data=data");
  }
  out.push(`response = requests.${m}(url${args.length ? ", " + args.join(", ") : ""})`);
  out.push("");
  out.push("print(response.status_code)");
  out.push("print(response.text)");
  return out.join("\n");
}

export function genPythonHttpClient(r: Req): string {
  const out: string[] = [];
  out.push("import http.client");
  out.push("import json");
  out.push("from urllib.parse import urlparse");
  out.push("");
  out.push(`url = "${esc(r.url, '"')}"`);
  if (r.headers.length) {
    out.push("");
    out.push("headers = {");
    for (const h of r.headers) out.push(`    "${esc(h.key, '"')}": "${esc(h.value, '"')}",`);
    out.push("}");
  }
  if (r.body) {
    out.push("");
    out.push(`payload = """${esc(r.body, '"')}"""`);
  }
  out.push("");
  out.push("u = urlparse(url)");
  out.push('conn = (http.client.HTTPSConnection if u.scheme == "https" else http.client.HTTPConnection)(');
  out.push('    u.hostname, u.port or (443 if u.scheme == "https" else 80))');
  out.push('path = u.path + (("?" + u.query) if u.query else "")');
  const args: string[] = [`"${r.method}"`, "path"];
  if (r.headers.length) args.push("headers=headers");
  if (r.body) args.push("payload");
  out.push(`conn.request(${args.join(", ")})`);
  out.push("res = conn.getresponse()");
  out.push("print(res.status, res.reason)");
  out.push('print(res.read().decode("utf-8"))');
  out.push("conn.close()");
  return out.join("\n");
}

export function genWsPython(r: WsReq): string {
  const out: string[] = [];
  out.push("# WebSocket 客户端示例（websockets：异步 asyncio，现代首选，支持 ws/wss）");
  out.push("# 官网: https://websockets.readthedocs.io/");
  out.push("# 安装: pip install websockets");
  out.push("# 运行: python ws_client.py");
  out.push("");
  out.push("import asyncio");
  out.push("import json");
  out.push("import websockets");
  out.push("");
  out.push("");
  out.push("async def main():");
  if (r.headers.length) {
    out.push(`    headers = ${JSON.stringify(Object.fromEntries(r.headers.map((h) => [h.key, h.value])))}`);
    out.push(`    async with websockets.connect(${JSON.stringify(r.url)}, additional_headers=headers) as ws:`);
  } else {
    out.push(`    async with websockets.connect(${JSON.stringify(r.url)}) as ws:`);
  }
  if (r.message) {
    out.push(`    message = ${JSON.stringify(r.message)}`);
    out.push("    print('>>> 发送:', message)");
    out.push("    await ws.send(message)");
    out.push("");
    out.push("    # 循环接收服务器回传的信息（path / query / header / message）");
    out.push("    try:");
    out.push("        while True:");
    out.push("            reply = await asyncio.wait_for(ws.recv(), timeout=5)");
    out.push("            print('<<< 接收:', reply)");
    out.push("    except asyncio.TimeoutError:");
    out.push("        pass");
  } else {
    out.push("    # 建立连接后即可收发消息");
    out.push("    async for reply in ws:");
    out.push("        print('<<< 接收:', reply)");
  }
  out.push("");
  out.push("");
  out.push("asyncio.run(main())");
  return out.join("\n");
}

export function genWsPythonDispatch(r: WsReq, lib?: string): string {
  switch (lib) {
    case "sync":
      return genWsPythonSync(r);
    default:
      return genWsPython(r);
  }
}

export function genWsPythonSync(r: WsReq): string {
  const out: string[] = [];
  out.push("# WebSocket 客户端示例（websocket-client：同步阻塞，简单脚本用）");
  out.push("# 官网: https://github.com/websocket-client/websocket-client");
  out.push("# 安装: pip install websocket-client");
  out.push("# 运行: python ws_client.py");
  out.push("");
  out.push("from websocket import create_connection");
  out.push("");
  if (r.headers.length) {
    out.push("# 自定义请求头（握手时发送）");
    out.push(`headers = ${JSON.stringify(Object.fromEntries(r.headers.map((h) => [h.key, h.value])))}`);
    out.push("");
  }
  if (r.headers.length) {
    out.push(`ws = create_connection(${JSON.stringify(r.url)}, header=headers, timeout=10)`);
  } else {
    out.push(`ws = create_connection(${JSON.stringify(r.url)}, timeout=10)`);
  }
  out.push("print('>>> 连接成功')");
  out.push("");
  if (r.message) {
    out.push(`message = ${JSON.stringify(r.message)}`);
    out.push("print('>>> 发送:', message)");
    out.push("ws.send(message)");
    out.push("");
    out.push("# 接收回显");
    out.push("reply = ws.recv()");
    out.push("print('<<< 接收:', reply)");
  } else {
    out.push("# 接收消息");
    out.push("reply = ws.recv()");
    out.push("print('<<< 接收:', reply)");
  }
  out.push("");
  out.push("ws.close()");
  return out.join("\n");
}
