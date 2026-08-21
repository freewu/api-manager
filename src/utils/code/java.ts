/** Java（OkHttp / Unirest / WebClient / Retrofit2 / HttpClient5；JSR-356 / Spring / Netty）代码生成 */

import { esc, Req, WsReq } from "./shared";
export function genJava(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("// 该表单包含文件上传（multipart/form-data），Java 请使用 MultipartBody.Builder 构造请求");
  }
  out.push("import java.net.URI;");
  out.push("import java.net.http.HttpClient;");
  out.push("import java.net.http.HttpRequest;");
  out.push("import java.net.http.HttpResponse;");
  out.push("");
  out.push("public class Main {");
  out.push("    public static void main(String[] args) throws Exception {");
  out.push("        HttpClient client = HttpClient.newHttpClient();");
  out.push("");
  out.push("        HttpRequest request = HttpRequest.newBuilder()");
  out.push(`            .uri(URI.create("${esc(r.url, '"')}"))`);
  for (const h of r.headers) {
    out.push(`            .header("${esc(h.key, '"')}", "${esc(h.value, '"')}")`);
  }
  if (r.body) {
    out.push(`            .method("${r.method}", HttpRequest.BodyPublishers.ofString("${esc(r.body, '"')}"))`);
  } else {
    out.push(`            .method("${r.method}", HttpRequest.BodyPublishers.noBody())`);
  }
  out.push("            .build();");
  out.push("");
  out.push("        HttpResponse<String> response = client.send(request, HttpResponse.BodyHandlers.ofString());");
  out.push("        System.out.println(response.body());");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

export function genJavaOkHttp(r: Req): string {
  const out: string[] = [];
  out.push("import okhttp3.MediaType;");
  out.push("import okhttp3.OkHttpClient;");
  out.push("import okhttp3.Request;");
  out.push("import okhttp3.RequestBody;");
  out.push("import okhttp3.Response;");
  if (r.files.length) {
    out.push("import okhttp3.MultipartBody;");
    out.push("import java.io.File;");
  }
  out.push("");
  out.push("public class Main {");
  out.push("    public static void main(String[] args) throws Exception {");
  out.push("        OkHttpClient client = new OkHttpClient();");
  out.push("");
  if (r.files.length) {
    out.push("        // 文件上传：使用 MultipartBody.Builder 构造请求体");
    out.push("        RequestBody body = new MultipartBody.Builder()");
    out.push("                .setType(MultipartBody.FORM)");
    for (const t of r.formText) out.push(`                .addFormDataPart("${esc(t.key, '"')}", "${esc(t.value, '"')}")`);
    for (const f of r.files) {
      const fname = (f.path.split(/[\\/]/).pop() || "file").replace(/"/g, "");
      out.push(`                .addFormDataPart("${esc(f.key, '"')}", "${esc(fname, '"')}", RequestBody.create(new File("${esc(f.path, '"')}"), MediaType.parse("application/octet-stream")))`);
    }
    out.push("                .build();");
  } else if (r.body) {
    const mt = r.bodyKind === "json" ? "application/json; charset=utf-8" : "text/plain; charset=utf-8";
    out.push(`        RequestBody body = RequestBody.create("${esc(r.body, '"')}", MediaType.parse("${mt}"));`);
  } else {
    out.push("        RequestBody body = null;");
  }
  out.push("");
  out.push("        Request request = new Request.Builder()");
  out.push(`                .url("${esc(r.url, '"')}")`);
  for (const h of r.headers) out.push(`                .addHeader("${esc(h.key, '"')}", "${esc(h.value, '"')}")`);
  out.push(`                .method("${r.method}", body)`);
  out.push("                .build();");
  out.push("");
  out.push("        try (Response response = client.newCall(request).execute()) {");
  out.push("            System.out.println(response.body().string());");
  out.push("        }");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

export function genJavaUnirest(r: Req): string {
  const out: string[] = [];
  out.push("import kong.unirest.HttpResponse;");
  out.push("import kong.unirest.Unirest;");
  if (r.files.length) out.push("import java.io.File;");
  out.push("");
  out.push("public class Main {");
  out.push("    public static void main(String[] args) {");
  out.push(`        HttpResponse<String> response = Unirest.${r.method.toLowerCase()}(${JSON.stringify(r.url)})`);
  for (const h of r.headers) out.push(`                .header("${esc(h.key, '"')}", "${esc(h.value, '"')}")`);
  if (r.files.length) {
    for (const t of r.formText) out.push(`                .field("${esc(t.key, '"')}", "${esc(t.value, '"')}")`);
    for (const f of r.files) out.push(`                .field("${esc(f.key, '"')}", new File("${esc(f.path, '"')}"))`);
  } else if (r.body) {
    out.push(`                .body("${esc(r.body, '"')}")`);
  }
  out.push("                .asString();");
  out.push("        System.out.println(response.getBody());");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

export function genJavaWebClient(r: Req): string {
  const out: string[] = [];
  out.push("import org.springframework.web.reactive.function.client.WebClient;");
  out.push("");
  out.push("public class Main {");
  out.push("    public static void main(String[] args) {");
  out.push("        WebClient client = WebClient.builder().build();");
  out.push("");
  out.push(`        String response = client.${r.method.toLowerCase()}()`);
  out.push(`                .uri("${esc(r.url, '"')}")`);
  for (const h of r.headers) out.push(`                .header("${esc(h.key, '"')}", "${esc(h.value, '"')}")`);
  if (r.body) out.push(`                .bodyValue("${esc(r.body, '"')}")`);
  out.push("                .retrieve()");
  out.push("                .bodyToMono(String.class)");
  out.push("                .block();");
  out.push("        System.out.println(response);");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

export function genJavaRetrofit2(r: Req): string {
  let basePart = r.url;
  let pathPart = r.url;
  try {
    const u = new URL(r.url);
    pathPart = u.pathname + u.search;
    basePart = u.origin + "/";
  } catch {
    /* URL 解析失败时保留原始值 */
  }
  const out: string[] = [];
  out.push("import retrofit2.Call;");
  out.push("import retrofit2.Response;");
  out.push("import retrofit2.Retrofit;");
  out.push("import retrofit2.converter.scalars.ScalarsConverterFactory;");
  out.push("import retrofit2.http.Body;");
  out.push("import retrofit2.http.DELETE;");
  out.push("import retrofit2.http.GET;");
  out.push("import retrofit2.http.Header;");
  out.push("import retrofit2.http.POST;");
  out.push("import retrofit2.http.PUT;");
  out.push("");
  out.push("public interface ApiService {");
  out.push(`    @${r.method}("${pathPart}")`);
  const params: string[] = [];
  r.headers.forEach((h, i) => params.push(`@Header("${esc(h.key, '"')}") String h${i}`));
  if (r.body) params.push("@Body String body");
  out.push(`    Call<String> request(${params.join(", ")});`);
  out.push("}");
  out.push("");
  out.push("// 使用示例");
  out.push("public class Main {");
  out.push("    public static void main(String[] args) throws Exception {");
  out.push("        Retrofit retrofit = new Retrofit.Builder()");
  out.push(`                .baseUrl("${basePart}")`);
  out.push("                .addConverterFactory(ScalarsConverterFactory.create())");
  out.push("                .build();");
  out.push("        ApiService service = retrofit.create(ApiService.class);");
  const callArgs: string[] = [];
  for (const h of r.headers) callArgs.push(JSON.stringify(h.value));
  if (r.body) callArgs.push(r.bodyKind === "json" ? r.body : JSON.stringify(r.body));
  out.push(`        Call<String> call = service.request(${callArgs.join(", ")});`);
  out.push("        Response<String> response = call.execute();");
  out.push("        System.out.println(response.body());");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

export function genJavaHttpClient5(r: Req): string {
  const M5: Record<string, string> = {
    GET: "HttpGet",
    POST: "HttpPost",
    PUT: "HttpPut",
    DELETE: "HttpDelete",
    PATCH: "HttpPatch",
    HEAD: "HttpHead",
    OPTIONS: "HttpOptions",
  };
  const cls = M5[r.method] || "HttpPost";
  const out: string[] = [];
  out.push(`import org.apache.hc.client5.http.classic.methods.${cls};`);
  out.push("import org.apache.hc.client5.http.impl.classic.CloseableHttpClient;");
  out.push("import org.apache.hc.client5.http.impl.classic.HttpClients;");
  out.push("import org.apache.hc.core5.http.io.entity.EntityUtils;");
  if (r.body) out.push("import org.apache.hc.core5.http.io.entity.StringEntity;");
  out.push("");
  out.push("public class Main {");
  out.push("    public static void main(String[] args) throws Exception {");
  out.push("        try (CloseableHttpClient client = HttpClients.createDefault()) {");
  out.push(`            ${cls} request = new ${cls}(${JSON.stringify(r.url)});`);
  for (const h of r.headers) out.push(`            request.setHeader("${esc(h.key, '"')}", "${esc(h.value, '"')}");`);
  if (r.body) out.push(`            request.setEntity(new StringEntity(${JSON.stringify(r.body)}));`);
  out.push("            client.execute(request, response -> {");
  out.push("                System.out.println(EntityUtils.toString(response.getEntity()));");
  out.push("                return null;");
  out.push("            });");
  out.push("        }");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

export function genWsJavaDispatch(r: WsReq, lib?: string): string {
  switch (lib) {
    case "spring":
      return genWsJavaSpring(r);
    case "netty":
      return genWsJavaNetty(r);
    case "okhttp":
      return genWsJavaOkhttp(r);
    default:
      return genWsJavaJsr356(r);
  }
}

export function genWsJavaJsr356(r: WsReq): string {
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（JSR-356：Java 标准 WebSocket API，JavaEE/JakartaEE 标准）");
  out.push(" * 规范官网: https://jakarta.ee/specifications/websocket/");
  out.push(" * 教程: https://javaee.github.io/tutorial/websocket.html");
  out.push(" * 容器内置: Tomcat（tomcat-websocket）、Jetty（jetty-websocket）");
  out.push(" * 依赖（以 Tomcat 为例，Maven）:");
  out.push(" *   <dependency>");
  out.push(" *     <groupId>org.apache.tomcat</groupId>");
  out.push(" *     <artifactId>tomcat-websocket</artifactId>");
  out.push(" *     <version>9.0.102</version>");
  out.push(" *   </dependency>");
  out.push(" * 注意: JakartaEE 9+ 将包名 javax.websocket 改为 jakarta.websocket");
  out.push(" */");
  out.push("import javax.websocket.*;");
  out.push("import java.net.URI;");
  out.push("import java.util.List;");
  out.push("import java.util.Map;");
  out.push("");
  out.push("@ClientEndpoint");
  out.push("public class Main {");
  out.push("    public static void main(String[] args) throws Exception {");
  out.push("        WebSocketContainer container = ContainerProvider.getWebSocketContainer();");
  if (r.headers.length) {
    out.push("        ClientEndpointConfig config = ClientEndpointConfig.Builder.create()");
    out.push("            .configurator(new ClientEndpointConfig.Configurator() {");
    out.push("                @Override");
    out.push("                public void beforeRequest(Map<String, List<String>> headers) {");
    for (const h of r.headers) out.push(`                    headers.put(${JSON.stringify(h.key)}, List.of(${JSON.stringify(h.value)}));`);
    out.push("                }");
    out.push("            }).build();");
    out.push(`        Session session = container.connectToServer(Main.class, config, URI.create(${JSON.stringify(r.url)}));`);
  } else {
    out.push(`        Session session = container.connectToServer(Main.class, URI.create(${JSON.stringify(r.url)}));`);
  }
  out.push("        // 保持主线程存活，等待回调");
  out.push("        Thread.sleep(5000);");
  out.push("        session.close();");
  out.push("    }");
  out.push("");
  out.push("    @OnOpen");
  out.push("    public static void onOpen(Session session) {");
  out.push("        System.out.println(\">>> 连接成功\");");
  out.push(`        String msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")};`);
  out.push("        System.out.println(\">>> 发送: \" + msg);");
  out.push("        try { session.getBasicRemote().sendText(msg); } catch (Exception e) { }");
  out.push("    }");
  out.push("");
  out.push("    @OnMessage");
  out.push("    public static void onMessage(String message, Session session) {");
  out.push("        System.out.println(\"<<< 接收: \" + message);");
  out.push("        try { session.close(); } catch (Exception e) { }");
  out.push("    }");
  out.push("");
  out.push("    @OnError");
  out.push("    public static void onError(Session session, Throwable t) {");
  out.push("        System.out.println(\"连接失败: \" + t.getMessage());");
  out.push("    }");
  out.push("");
  out.push("    @OnClose");
  out.push("    public static void onClose(Session session, CloseReason reason) {");
  out.push("        System.out.println(\"连接已关闭: \" + reason.getReasonPhrase());");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

export function genWsJavaSpring(r: WsReq): string {
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（Spring WebSocket：SpringBoot 项目首选，底层可用 JSR356 / Netty / Jetty）");
  out.push(" * 官网: https://spring.io/projects/spring-websocket");
  out.push(" * 文档: https://docs.spring.io/spring-framework/reference/web/websocket.html");
  out.push(" * 依赖: spring-boot-starter-websocket（或 spring-websocket）");
  out.push(" * 说明: 底层客户端可切换 StandardWebSocketClient（JSR-356）/ JettyWebSocketClient /");
  out.push(" *       ReactorNettyWebSocketClient（WebFlux）；如需 STOMP 子协议，");
  out.push(" *       改用 spring-messaging 的 WebSocketStompClient + StompSession");
  out.push(" */");
  out.push("import org.springframework.web.socket.*;");
  out.push("import org.springframework.web.socket.client.WebSocketClient;");
  out.push("import org.springframework.web.socket.client.standard.StandardWebSocketClient;");
  out.push("import java.util.concurrent.CompletableFuture;");
  out.push("");
  out.push("public class Main {");
  out.push("    public static void main(String[] args) throws Exception {");
  out.push("        WebSocketClient client = new StandardWebSocketClient();");
  if (r.headers.length) {
    out.push("        HttpHeaders headers = new HttpHeaders();");
    for (const h of r.headers) out.push(`        headers.set(${JSON.stringify(h.key)}, ${JSON.stringify(h.value)});`);
    out.push(`        CompletableFuture<WebSocketSession> future = client.execute(handler(), ${JSON.stringify(r.url)}, headers);`);
  } else {
    out.push(`        CompletableFuture<WebSocketSession> future = client.execute(handler(), ${JSON.stringify(r.url)});`);
  }
  out.push("        WebSocketSession session = future.get();");
  out.push("        Thread.sleep(5000);");
  out.push("        session.close();");
  out.push("    }");
  out.push("");
  out.push("    private static WebSocketHandler handler() {");
  out.push("        return new WebSocketHandler() {");
  out.push("            @Override");
  out.push("            public void afterConnectionEstablished(WebSocketSession session) throws Exception {");
  out.push("                System.out.println(\">>> 连接成功\");");
  out.push(`                session.sendMessage(new TextMessage(${JSON.stringify(r.message || "hello, this is a websocket echo message")}));`);
  out.push("                System.out.println(\">>> 发送完成\");");
  out.push("            }");
  out.push("");
  out.push("            @Override");
  out.push("            public void handleMessage(WebSocketSession session, WebSocketMessage<?> message) throws Exception {");
  out.push("                if (message instanceof TextMessage) {");
  out.push("                    System.out.println(\"<<< 接收: \" + ((TextMessage) message).getPayload());");
  out.push("                }");
  out.push("            }");
  out.push("");
  out.push("            @Override");
  out.push("            public void handleTransportError(WebSocketSession session, Throwable exception) throws Exception {");
  out.push("                System.out.println(\"连接失败: \" + exception.getMessage());");
  out.push("            }");
  out.push("");
  out.push("            @Override");
  out.push("            public void afterConnectionClosed(WebSocketSession session, CloseStatus closeStatus) throws Exception {");
  out.push("                System.out.println(\"连接已关闭: \" + closeStatus);");
  out.push("            }");
  out.push("");
  out.push("            @Override");
  out.push("            public boolean supportsPartialMessages() { return false; }");
  out.push("        };");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

export function genWsJavaNetty(r: WsReq): string {
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（Netty 原生：高性能底层，高并发网关、IM 服务）");
  out.push(" * 官网: https://netty.io/");
  out.push(" * 依赖（Maven）:");
  out.push(" *   <dependency>");
  out.push(" *     <groupId>io.netty</groupId>");
  out.push(" *     <artifactId>netty-codec-http</artifactId>");
  out.push(" *     <version>4.1.115.Final</version>");
  out.push(" *   </dependency>");
  out.push(" *   （wss:// 时另需 netty-handler 与 netty-tcnative/OpenSSL）");
  out.push(" */");
  out.push("import io.netty.bootstrap.Bootstrap;");
  out.push("import io.netty.channel.*;");
  out.push("import io.netty.channel.nio.NioEventLoopGroup;");
  out.push("import io.netty.channel.socket.SocketChannel;");
  out.push("import io.netty.channel.socket.nio.NioSocketChannel;");
  out.push("import io.netty.handler.codec.http.DefaultHttpHeaders;");
  out.push("import io.netty.handler.codec.http.HttpClientCodec;");
  out.push("import io.netty.handler.codec.http.HttpObjectAggregator;");
  out.push("import io.netty.handler.codec.http.websocketx.*;");
  out.push("import io.netty.handler.ssl.SslContextBuilder;");
  out.push("import java.net.URI;");
  out.push("");
  out.push("public class Main {");
  out.push("    public static void main(String[] args) throws Exception {");
  out.push(`        URI uri = new URI(${JSON.stringify(r.url)});`);
  out.push("        String scheme = uri.getScheme();");
  out.push("        boolean ssl = \"wss\".equals(scheme);");
  out.push("        String host = uri.getHost();");
  out.push("        int port = uri.getPort();");
  out.push("        if (port == -1) port = ssl ? 443 : 80;");
  out.push("");
  if (r.headers.length) {
    out.push("        DefaultHttpHeaders headers = new DefaultHttpHeaders();");
    for (const h of r.headers) out.push(`        headers.add(${JSON.stringify(h.key)}, ${JSON.stringify(h.value)});`);
  } else {
    out.push("        DefaultHttpHeaders headers = new DefaultHttpHeaders();");
  }
  out.push(`        WebSocketClientHandshaker handshaker = WebSocketClientHandshakerFactory`);
  out.push(`                .newHandshaker(uri, WebSocketVersion.V13, null, true, headers);`);
  out.push("");
  out.push("        EventLoopGroup group = new NioEventLoopGroup();");
  out.push("        try {");
  out.push("            Bootstrap bootstrap = new Bootstrap();");
  out.push("            bootstrap.group(group)");
  out.push("                .channel(NioSocketChannel.class)");
  out.push("                .handler(new ChannelInitializer<SocketChannel>() {");
  out.push("                    @Override");
  out.push("                    protected void initChannel(SocketChannel ch) {");
  out.push("                        ChannelPipeline p = ch.pipeline();");
  out.push("                        if (ssl) {");
  out.push("                            p.addLast(SslContextBuilder.forClient().build()");
  out.push("                                    .newHandler(ch.alloc(), host, port));");
  out.push("                        }");
  out.push("                        p.addLast(new HttpClientCodec());");
  out.push("                        p.addLast(new HttpObjectAggregator(8192));");
  out.push("                        p.addLast(new WebSocketClientProtocolHandler(handshaker, true));");
  out.push("                        p.addLast(new SimpleChannelInboundHandler<WebSocketFrame>() {");
  out.push("                            @Override");
  out.push("                            protected void channelRead0(ChannelHandlerContext ctx, WebSocketFrame frame) {");
  out.push("                                if (frame instanceof TextWebSocketFrame) {");
  out.push("                                    System.out.println(\"<<< 接收: \" + ((TextWebSocketFrame) frame).text());");
  out.push("                                    ctx.close();");
  out.push("                                }");
  out.push("                            }");
  out.push("");
  out.push("                            @Override");
  out.push("                            public void exceptionCaught(ChannelHandlerContext ctx, Throwable cause) {");
  out.push("                                System.out.println(\"连接失败: \" + cause.getMessage());");
  out.push("                                ctx.close();");
  out.push("                            }");
  out.push("                        });");
  out.push("                    }");
  out.push("                });");
  out.push("");
  out.push("            Channel ch = bootstrap.connect(host, port).sync().channel();");
  out.push("            handshaker.handshakeFuture().sync();   // 等待握手完成");
  out.push("            System.out.println(\">>> 连接成功\");");
  out.push(`            String msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")};`);
  out.push("            System.out.println(\">>> 发送: \" + msg);");
  out.push("            ch.writeAndFlush(new TextWebSocketFrame(msg));");
  out.push("            ch.closeFuture().sync();");
  out.push("        } finally {");
  out.push("            group.shutdownGracefully();");
  out.push("        }");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

export function genWsJavaOkhttp(r: WsReq): string {
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（OkHttp：轻量易用，Android 与 JVM 通用）");
  out.push(" * 官网: https://square.github.io/okhttp/");
  out.push(" * GitHub: https://github.com/square/okhttp");
  out.push(" * 依赖（Maven）:");
  out.push(" *   <dependency>");
  out.push(" *     <groupId>com.squareup.okhttp3</groupId>");
  out.push(" *     <artifactId>okhttp</artifactId>");
  out.push(" *     <version>4.12.0</version>");
  out.push(" *   </dependency>");
  out.push(" */");
  out.push("import okhttp3.*;");
  out.push("import okio.ByteString;");
  out.push("");
  out.push("public class Main {");
  out.push("    public static void main(String[] args) throws Exception {");
  out.push("        OkHttpClient client = new OkHttpClient();");
  out.push("        Request request = new Request.Builder()");
  out.push(`            .url(${JSON.stringify(r.url)})`);
  for (const h of r.headers) out.push(`            .addHeader(${JSON.stringify(h.key)}, ${JSON.stringify(h.value)})`);
  out.push("            .build();");
  out.push("");
  out.push("        WebSocket ws = client.newWebSocket(request, new WebSocketListener() {");
  out.push("            @Override");
  out.push("            public void onOpen(WebSocket webSocket, Response response) {");
  out.push("                System.out.println(\">>> 连接成功\");");
  out.push(`                webSocket.send(${JSON.stringify(r.message || "hello, this is a websocket echo message")});`);
  out.push("                System.out.println(\">>> 发送完成\");");
  out.push("            }");
  out.push("");
  out.push("            @Override");
  out.push("            public void onMessage(WebSocket webSocket, String text) {");
  out.push("                System.out.println(\"<<< 接收: \" + text);");
  out.push("                webSocket.close(1000, \"bye\");");
  out.push("            }");
  out.push("");
  out.push("            @Override");
  out.push("            public void onMessage(WebSocket webSocket, ByteString bytes) {");
  out.push("                System.out.println(\"<<< 接收(binary): \" + bytes.hex());");
  out.push("            }");
  out.push("");
  out.push("            @Override");
  out.push("            public void onFailure(WebSocket webSocket, Throwable t, Response response) {");
  out.push("                System.out.println(\"连接失败: \" + t.getMessage());");
  out.push("            }");
  out.push("");
  out.push("            @Override");
  out.push("            public void onClosed(WebSocket webSocket, int code, String reason) {");
  out.push("                System.out.println(\"连接已关闭: \" + reason);");
  out.push("            }");
  out.push("        });");
  out.push("");
  out.push("        // 保持主线程存活");
  out.push("        Thread.sleep(5000);");
  out.push("        client.dispatcher().executorService().shutdown();");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

export function genJavaDispatch(lib: string | undefined, r: Req): string {
  switch (lib) {
    case "okhttp":
      return genJavaOkHttp(r);
    case "unirest":
      return genJavaUnirest(r);
    case "webclient":
      return genJavaWebClient(r);
    case "retrofit2":
      return genJavaRetrofit2(r);
    case "httpclient5":
      return genJavaHttpClient5(r);
    default:
      return genJava(r);
  }
}
