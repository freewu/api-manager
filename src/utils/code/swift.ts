/** Swift（URLSession；URLSession WebSocket / Starscream / Network.framework）代码生成 */

import { esc, Req, WsReq } from "./shared";
export function genSwift(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("// 该表单包含文件上传（multipart/form-data），请使用 URLSession uploadTask 配合 multipart body 构造请求");
  }
  out.push("import Foundation");
  out.push("");
  out.push(`let url = URL(string: "${esc(r.url, '"')}")!`);
  out.push("var request = URLRequest(url: url)");
  out.push(`request.httpMethod = "${r.method}"`);
  for (const h of r.headers) {
    out.push(`request.setValue("${esc(h.value, '"')}", forHTTPHeaderField: "${esc(h.key, '"')}")`);
  }
  if (r.body) {
    out.push(`request.httpBody = "${esc(r.body, '"')}".data(using: .utf8)`);
  }
  out.push("");
  out.push("let semaphore = DispatchSemaphore(value: 0)");
  out.push("let task = URLSession.shared.dataTask(with: request) { data, response, error in");
  out.push("    if let error = error {");
  out.push("        print(\"Error: \\(error)\")");
  out.push("    } else if let data = data {");
  out.push("        print(String(data: data, encoding: .utf8) ?? \"\")");
  out.push("    }");
  out.push("    semaphore.signal()");
  out.push("}");
  out.push("task.resume()");
  out.push("semaphore.wait()");
  return out.join("\n");
}

export function genWsSwiftDispatch(r: WsReq, lib?: string): string {
  switch (lib) {
    case "starscream":
      return genWsSwiftStarscream(r);
    case "network":
      return genWsSwiftNetwork(r);
    default:
      return genWsSwiftUrlSession(r);
  }
}

export function genWsSwiftUrlSession(r: WsReq): string {
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（URLSession WebSocket：系统原生，Swift 5.5+，iOS 13+ / macOS 10.15+）");
  out.push(" * 官网: https://developer.apple.com/documentation/foundation/urlsessionwebsockettask");
  out.push(" * 无需第三方库，系统自带");
  out.push(" */");
  out.push("import Foundation");
  out.push("");
  out.push("// 1. 发起连接（支持自定义请求头）");
  out.push(`var request = URLRequest(url: URL(string: ${JSON.stringify(r.url)})!)`);
  for (const h of r.headers) out.push(`request.setValue(${JSON.stringify(h.value)}, forHTTPHeaderField: ${JSON.stringify(h.key)})`);
  out.push("let session = URLSession(configuration: .default)");
  out.push("let wsTask = session.webSocketTask(with: request)");
  out.push("wsTask.resume()");
  out.push("print(\">>> 连接中...\")");
  out.push("");
  out.push("// 2. 发送一条文本消息");
  out.push(`let msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")}`);
  out.push("wsTask.send(.string(msg)) { error in");
  out.push("    if let error = error {");
  out.push("        print(\"发送失败: \" + error.localizedDescription)");
  out.push("    } else {");
  out.push("        print(\">>> 发送: \" + msg)");
  out.push("    }");
  out.push("}");
  out.push("");
  out.push("// 3. 循环接收消息");
  out.push("func receive() {");
  out.push("    wsTask.receive { result in");
  out.push("        switch result {");
  out.push("        case .success(let message):");
  out.push("            switch message {");
  out.push("            case .string(let text):");
  out.push("                print(\"<<< 接收: \" + text)");
  out.push("            case .data(let data):");
  out.push("                print(\"<<< 接收(binary): \" + data.base64EncodedString())");
  out.push("            @unknown default:");
  out.push("                break");
  out.push("            }");
  out.push("            receive()   // 继续接收下一条");
  out.push("        case .failure(let error):");
  out.push("            print(\"接收失败: \" + error.localizedDescription)");
  out.push("            exit(0)");
  out.push("        }");
  out.push("    }");
  out.push("}");
  out.push("receive()");
  out.push("");
  out.push("// 4. 保持主线程运行");
  out.push("dispatchMain()");
  return out.join("\n");
}

export function genWsSwiftStarscream(r: WsReq): string {
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（Starscream：最流行的第三方 WebSocket 库，老系统兼容）");
  out.push(" * 官网: https://github.com/daltoniam/Starscream");
  out.push(" * 安装（Swift Package Manager）:");
  out.push(" *   dependencies: [.package(url: \"https://github.com/daltoniam/Starscream.git\", from: \"4.0.6\")]");
  out.push(" * 支持 iOS 8+ / macOS 10.10+，老系统也可用");
  out.push(" */");
  out.push("import Starscream");
  out.push("");
  out.push(`let url = URL(string: ${JSON.stringify(r.url)})!`);
  out.push("var request = URLRequest(url: url)");
  for (const h of r.headers) out.push(`request.setValue(${JSON.stringify(h.value)}, forHTTPHeaderField: ${JSON.stringify(h.key)})`);
  out.push("");
  out.push("let socket = WebSocket(request: request)");
  out.push("socket.onEvent = { event in");
  out.push("    switch event {");
  out.push("    case .connected(let headers):");
  out.push("        print(\">>> 连接成功\")");
  out.push(`        socket.write(string: ${JSON.stringify(r.message || "hello, this is a websocket echo message")})`);
  out.push("        print(\">>> 发送完成\")");
  out.push("    case .text(let text):");
  out.push("        print(\"<<< 接收: \" + text)");
  out.push("        socket.disconnect()");
  out.push("    case .binary(let data):");
  out.push("        print(\"<<< 接收(binary): \" + data.base64EncodedString())");
  out.push("    case .error(let error):");
  out.push("        print(\"连接失败: \" + (error?.localizedDescription ?? \"unknown\"))");
  out.push("    case .disconnected(let reason, let code):");
  out.push("        print(\"连接已关闭: \" + reason + \" (\" + String(code) + \")\")");
  out.push("    default:");
  out.push("        break");
  out.push("    }");
  out.push("}");
  out.push("socket.connect()");
  out.push("");
  out.push("// 保持主线程运行");
  out.push("dispatchMain()");
  return out.join("\n");
}

export function genWsSwiftNetwork(r: WsReq): string {
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（Network.framework NWWebSocket：Apple Network 框架，底层高性能）");
  out.push(" * 官网: https://developer.apple.com/documentation/network/nwprotocolwebsocket");
  out.push(" * 系统自带（iOS 13+ / macOS 10.15+），基于 nw_connection，性能高");
  out.push(" */");
  out.push("import Network");
  out.push("import Foundation");
  out.push("");
  out.push("// 1. 解析 URL 并配置参数");
  out.push(`let url = URL(string: ${JSON.stringify(r.url)})!`);
  out.push("let params = NWParameters(url: url)!");
  out.push("params.allowLocalEndpointReuse = true");
  out.push("");
  if (r.headers.length) {
    out.push("// 2. 附加自定义请求头（WebSocket metadata）");
    out.push("let handshake = NWProtocolWebSocket.Metadata()");
    out.push("handshake.setAdditionalHeaders([" + r.headers.map((h) => `(${JSON.stringify(h.key)}, ${JSON.stringify(h.value)})`).join(", ") + "])");
    out.push("params.defaultProtocolStack.applicationProtocols.insert(handshake, at: 0)");
    out.push("");
  }
  out.push("// 3. 建立连接");
  out.push("let connection = NWConnection(to: .url(url), using: params)");
  out.push("");
  out.push("// 4. 循环接收消息");
  out.push("func receive() {");
  out.push("    connection.receiveMessage { content, context, isComplete, error in");
  out.push("        if let content = content, let text = String(data: content, encoding: .utf8) {");
  out.push("            print(\"<<< 接收: \" + text)");
  out.push("            connection.cancel()");
  out.push("        } else if let error = error {");
  out.push("            print(\"接收失败: \" + error.localizedDescription)");
  out.push("            connection.cancel()");
  out.push("        }");
  out.push("    }");
  out.push("}");
  out.push("");
  out.push("connection.stateUpdateHandler = { state in");
  out.push("    switch state {");
  out.push("    case .ready:");
  out.push("        print(\">>> 连接成功\")");
  out.push("        // 发送一条文本消息");
  out.push(`        let msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")}`);
  out.push("        let content = Data(msg.utf8)");
  out.push("        let meta = NWProtocolWebSocket.Metadata(opcode: .text)");
  out.push("        let context = NWConnection.ContentContext(identifier: \"text\", metadata: [meta])");
  out.push("        connection.send(content: content, contentContext: context, isComplete: true) { error in");
  out.push("            if let error = error {");
  out.push("                print(\"发送失败: \" + error.localizedDescription)");
  out.push("            } else {");
  out.push("                print(\">>> 发送: \" + msg)");
  out.push("            }");
  out.push("        }");
  out.push("        receive()");
  out.push("    case .waiting(let error):");
  out.push("        print(\"等待连接: \" + error.localizedDescription)");
  out.push("    case .failed(let error):");
  out.push("        print(\"连接失败: \" + error.localizedDescription)");
  out.push("    default:");
  out.push("        break");
  out.push("    }");
  out.push("}");
  out.push("connection.start(queue: .main)");
  out.push("");
  out.push("// 5. 保持主线程运行");
  out.push("dispatchMain()");
  return out.join("\n");
}
