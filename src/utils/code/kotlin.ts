/** Kotlin（OkHttp；OkHttp WebSocket / Java-WebSocket）代码生成 */

import { esc, Req, WsReq } from "./shared";
export function genKotlin(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("// 该表单包含文件上传（multipart/form-data），请使用 MultipartBody.Builder 构造请求");
  }
  out.push("import okhttp3.*");
  out.push("import okhttp3.MediaType.Companion.toMediaType");
  out.push("import okhttp3.RequestBody.Companion.toRequestBody");
  out.push("");
  out.push("fun main() {");
  out.push("    val client = OkHttpClient()");
  if (r.body) {
    const contentType = r.bodyKind === "json" ? "application/json; charset=utf-8" : "text/plain; charset=utf-8";
    out.push(`    val mediaType = "${contentType}".toMediaType()`);
    out.push(`    val body = "${esc(r.body, '"')}".toRequestBody(mediaType)`);
  }
  out.push("");
  out.push("    val request = Request.Builder()");
  out.push(`        .url("${esc(r.url, '"')}")`);
  for (const h of r.headers) {
    out.push(`        .header("${esc(h.key, '"')}", "${esc(h.value, '"')}")`);
  }
  if (r.body) {
    out.push(`        .method("${r.method}", body)`);
  } else if (r.method === "GET") {
    out.push("        .get()");
  } else {
    out.push(`        .method("${r.method}", ByteArray(0).toRequestBody(null))`);
  }
  out.push("        .build()");
  out.push("");
  out.push("    client.newCall(request).execute().use { resp ->");
  out.push("        println(resp.code)");
  out.push("        println(resp.body?.string())");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

export function genWsKotlinDispatch(r: WsReq, lib?: string): string {
  switch (lib) {
    case "java-websocket":
      return genWsKotlinJavaWebSocket(r);
    default:
      return genWsKotlinOkhttp(r);
  }
}

export function genWsKotlinOkhttp(r: WsReq): string {
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（OkHttp：最常用，Android / JVM 后端通用，生产首选）");
  out.push(" * 官网: https://square.github.io/okhttp/");
  out.push(" * GitHub: https://github.com/square/okhttp");
  out.push(" * 依赖（Gradle）:");
  out.push(" *   implementation(\"com.squareup.okhttp3:okhttp:4.12.0\")");
  out.push(" */");
  out.push("import okhttp3.OkHttpClient");
  out.push("import okhttp3.Request");
  out.push("import okhttp3.Response");
  out.push("import okhttp3.WebSocket");
  out.push("import okhttp3.WebSocketListener");
  out.push("import okio.ByteString");
  out.push("");
  out.push("fun main() {");
  out.push("    val client = OkHttpClient()");
  out.push("    val request = Request.Builder()");
  out.push(`        .url(${JSON.stringify(r.url)})`);
  for (const h of r.headers) out.push(`        .addHeader(${JSON.stringify(h.key)}, ${JSON.stringify(h.value)})`);
  out.push("        .build()");
  out.push("");
  out.push("    val ws = client.newWebSocket(request, object : WebSocketListener() {");
  out.push("        override fun onOpen(webSocket: WebSocket, response: Response) {");
  out.push("            println(\">>> 连接成功\")");
  out.push(`            webSocket.send(${JSON.stringify(r.message || "hello, this is a websocket echo message")})`);
  out.push("            println(\">>> 发送完成\")");
  out.push("        }");
  out.push("");
  out.push("        override fun onMessage(webSocket: WebSocket, text: String) {");
  out.push("            println(\"<<< 接收: \" + text)");
  out.push("            webSocket.close(1000, \"bye\")");
  out.push("        }");
  out.push("");
  out.push("        override fun onMessage(webSocket: WebSocket, bytes: ByteString) {");
  out.push("            println(\"<<< 接收(binary): \" + bytes.hex())");
  out.push("        }");
  out.push("");
  out.push("        override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {");
  out.push("            println(\"连接失败: \" + t.message)");
  out.push("        }");
  out.push("");
  out.push("        override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {");
  out.push("            println(\"连接已关闭: \" + reason)");
  out.push("        }");
  out.push("    })");
  out.push("");
  out.push("    // 保持主线程存活（命令行场景）");
  out.push("    Thread.sleep(5000)");
  out.push("    client.dispatcher.executorService.shutdown()");
  out.push("}");
  return out.join("\n");
}

export function genWsKotlinJavaWebSocket(r: WsReq): string {
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（Java-WebSocket：独立 websocket 库，不依赖 okhttp）");
  out.push(" * 官网: https://github.com/TooTallNate/Java-WebSocket");
  out.push(" * 依赖（Gradle）:");
  out.push(" *   implementation(\"org.java-websocket:Java-WebSocket:1.5.7\")");
  out.push(" */");
  out.push("import org.java_websocket.client.WebSocketClient");
  out.push("import org.java_websocket.handshake.ServerHandshake");
  out.push("import java.net.URI");
  out.push("");
  out.push("fun main() {");
  out.push("    val client = object : WebSocketClient(URI(\"" + r.url + "\")) {");
  out.push("        override fun onOpen(handshakedata: ServerHandshake) {");
  out.push("            println(\">>> 连接成功\")");
  out.push(`            send(${JSON.stringify(r.message || "hello, this is a websocket echo message")})`);
  out.push("            println(\">>> 发送完成\")");
  out.push("        }");
  out.push("");
  out.push("        override fun onMessage(message: String) {");
  out.push("            println(\"<<< 接收: \" + message)");
  out.push("            close()");
  out.push("        }");
  out.push("");
  out.push("        override fun onClose(code: Int, reason: String, remote: Boolean) {");
  out.push("            println(\"连接已关闭: \" + reason)");
  out.push("        }");
  out.push("");
  out.push("        override fun onError(ex: Exception) {");
  out.push("            println(\"连接失败: \" + ex.message)");
  out.push("        }");
  out.push("    }");
  out.push("");
  if (r.headers.length) {
    out.push("    // 自定义请求头（握手前设置）");
    out.push("    val headers = mapOf(" + r.headers.map((h) => `"${h.key}" to "${h.value}"`).join(", ") + ")");
    out.push("    headers.forEach { (k, v) -> client.addHeader(k, v) }");
    out.push("");
  }
  out.push("    client.connect()");
  out.push("    // 保持主线程存活");
  out.push("    Thread.sleep(5000)");
  out.push("}");
  return out.join("\n");
}
