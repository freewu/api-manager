/** C++（libcurl / Boost.Beast / libwebsockets / uWebSockets / Qt）代码生成 */

import { esc, parseWsUrl, Req, WsReq } from "./shared";
export function genCpp(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("// 该表单包含文件上传（multipart/form-data），请使用 curl_formadd() 构造 multipart 请求");
  }
  out.push("#include <iostream>");
  out.push("#include <string>");
  out.push("#include <curl/curl.h>");
  out.push("");
  out.push("static size_t write_cb(void *ptr, size_t size, size_t nmemb, void *userdata) {");
  out.push("    (void)userdata;");
  out.push("    std::cout.write(static_cast<const char *>(ptr), size * nmemb);");
  out.push("    return size * nmemb;");
  out.push("}");
  out.push("");
  out.push("int main() {");
  out.push("    CURL *curl = curl_easy_init();");
  out.push("    if (!curl) return 1;");
  out.push("    struct curl_slist *headers = NULL;");
  for (const h of r.headers) {
    out.push(`    headers = curl_slist_append(headers, "${esc(`${h.key}: ${h.value}`, '"')}");`);
  }
  out.push(`    curl_easy_setopt(curl, CURLOPT_URL, "${esc(r.url, '"')}");`);
  out.push(`    curl_easy_setopt(curl, CURLOPT_CUSTOMREQUEST, "${r.method}");`);
  if (r.headers.length) out.push("    curl_easy_setopt(curl, CURLOPT_HTTPHEADER, headers);");
  if (r.body) {
    out.push(`    std::string body = "${esc(r.body, '"')}";`);
    out.push("    curl_easy_setopt(curl, CURLOPT_POSTFIELDS, body.c_str());");
    out.push("    curl_easy_setopt(curl, CURLOPT_POSTFIELDSIZE, (long)body.size());");
  }
  out.push("    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, write_cb);");
  out.push("    CURLcode res = curl_easy_perform(curl);");
  out.push("    curl_slist_free_all(headers);");
  out.push("    curl_easy_cleanup(curl);");
  out.push("    return res != CURLE_OK;");
  out.push("}");
  return out.join("\n");
}

export function genWsCpp(r: WsReq, lib?: string): string {
  switch (lib) {
    case "libwebsockets":
      return genWsCppLibwebsockets(r);
    case "uwebsockets":
      return genWsCppUwebsockets(r);
    case "qt":
      return genWsCppQt(r);
    default:
      return genWsCppBeast(r);
  }
}

export function genWsCppBeast(r: WsReq): string {
  const u = parseWsUrl(r.url);
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（Boost.Beast，C++ 原生，工业级，推荐）");
  out.push(" * 官网: https://www.boost.org/doc/libs/release/libs/beast/");
  out.push(" * GitHub: https://github.com/boostorg/beast");
  out.push(" * 安装: sudo apt install libboost-all-dev   (Ubuntu/Debian)");
  out.push(" * 编译: g++ -std=c++17 -o ws_client ws_client.cpp -lboost_system -lpthread");
  out.push(" *       （或 CMake: find_package(Boost) / find_package(Threads)）");
  out.push(" *       （wss:// 需改用 ssl::stream<tcp::socket> 并链接 OpenSSL）");
  out.push(" *");
  for (const h of r.headers) out.push(` * 请求头: ${h.key}: ${h.value}（通过 set_option(decorator) 设置）`);
  out.push(" */");
  out.push("#include <boost/beast/core.hpp>");
  out.push("#include <boost/beast/websocket.hpp>");
  out.push("#include <boost/asio/connect.hpp>");
  out.push("#include <boost/asio/ip/tcp.hpp>");
  out.push("#include <cstdlib>");
  out.push("#include <iostream>");
  out.push("#include <string>");
  out.push("");
  out.push("namespace beast = boost::beast;");
  out.push("namespace http = beast::http;");
  out.push("namespace websocket = beast::websocket;");
  out.push("namespace net = boost::asio;");
  out.push("using tcp = net::ip::tcp;");
  out.push("");
  out.push("int main() {");
  out.push("    try {");
  out.push(`        const std::string host = ${JSON.stringify(u.host)};`);
  out.push(`        const std::string port = ${JSON.stringify(String(u.port))};`);
  out.push(`        const std::string path = ${JSON.stringify(u.path)};`);
  out.push(`        const std::string msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")};`);
  out.push("");
  out.push("        net::io_context ioc;");
  out.push("        tcp::resolver resolver{ioc};");
  out.push("        auto const results = resolver.resolve(host, port);");
  out.push("        websocket::stream<tcp::socket> ws{ioc};");
  if (r.headers.length) {
    out.push("        ws.set_option(websocket::stream_base::decorator([](websocket::request_type &req) {");
    for (const h of r.headers) out.push(`            req.set(${JSON.stringify(h.key)}, ${JSON.stringify(h.value)});`);
    out.push("        }));");
  }
  out.push("        auto ep = net::connect(ws.next_layer(), results);");
  out.push("        (void)ep;");
  out.push("        ws.handshake(host, path);");
  out.push("");
  out.push("        ws.write(net::buffer(msg));");
  out.push("        std::cout << \">>> 发送: \" << msg << std::endl;");
  out.push("");
  out.push("        beast::flat_buffer buffer;");
  out.push("        ws.read(buffer);");
  out.push("        std::cout << \"<<< 接收: \" << beast::make_printable(buffer.data()) << std::endl;");
  out.push("");
  out.push("        ws.close(websocket::close_code::normal);");
  out.push("    } catch (std::exception const &e) {");
  out.push("        std::cerr << \"错误: \" << e.what() << std::endl;");
  out.push("        return 1;");
  out.push("    }");
  out.push("    return 0;");
  out.push("}");
  return out.join("\n");
}

export function genWsCppLibwebsockets(r: WsReq): string {
  const u = parseWsUrl(r.url);
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（libwebsockets：C 库，C++ 可直接调用）");
  out.push(" * 官网: https://libwebsockets.org");
  out.push(" * GitHub: https://github.com/warmcat/libwebsockets");
  out.push(" * 安装: sudo apt install libwebsockets-dev    (Ubuntu/Debian)");
  out.push(" *       brew install libwebsockets             (macOS)");
  out.push(" * 编译: g++ -std=c++17 -o ws_client ws_client.cpp -lwebsockets");
  out.push(" *       （连接 wss:// 时需另链接 OpenSSL: -lssl -lcrypto，并启用下方 wss 两行）");
  out.push(" *");
  for (const h of r.headers) out.push(` * 请求头: ${h.key}: ${h.value}（通过 LWS_CALLBACK_CLIENT_APPEND_HANDSHAKE_HEADER 回调追加）`);
  out.push(" */");
  out.push("#include <cstdio>");
  out.push("#include <cstring>");
  out.push("#include <string>");
  out.push("#include <libwebsockets.h>");
  out.push("");
  out.push(`static const std::string MSG = ${JSON.stringify(r.message || "hello, this is a websocket echo message")};`);
  out.push("");
  out.push("static int g_done = 0;");
  out.push("");
  out.push("static int ws_callback(struct lws *wsi, enum lws_callback_reasons reason,");
  out.push("                       void *user, void *in, size_t len) {");
  out.push("    (void)user;");
  out.push("    switch (reason) {");
  out.push("    case LWS_CALLBACK_CLIENT_ESTABLISHED: {");
  out.push("        std::string buf(LWS_PRE + MSG.size(), 0);");
  out.push("        MSG.copy(&buf[LWS_PRE], MSG.size());");
  out.push("        std::printf(\">>> 发送: %s\\n\", MSG.c_str());");
  out.push("        lws_write(wsi, reinterpret_cast<unsigned char *>(&buf[LWS_PRE]), MSG.size(), LWS_WRITE_TEXT);");
  out.push("        break;");
  out.push("    }");
  out.push("    case LWS_CALLBACK_CLIENT_RECEIVE:");
  out.push("        std::printf(\"<<< 接收: %.*s\\n\", static_cast<int>(len), static_cast<const char *>(in));");
  out.push("        g_done = 1;");
  out.push("        break;");
  out.push("    case LWS_CALLBACK_CLIENT_CONNECTION_ERROR:");
  out.push("        std::fprintf(stderr, \"连接失败\\n\");");
  out.push("        g_done = 1;");
  out.push("        break;");
  out.push("    case LWS_CALLBACK_CLIENT_CLOSED:");
  out.push("        g_done = 1;");
  out.push("        break;");
  out.push("    default:");
  out.push("        break;");
  out.push("    }");
  out.push("    return 0;");
  out.push("}");
  out.push("");
  out.push("static const struct lws_protocols protocols[] = {");
  out.push("    { \"api-manager\", ws_callback, 0, 4096, 0, nullptr, 0 },");
  out.push("    LWS_PROTOCOL_LIST_TERM");
  out.push("};");
  out.push("");
  out.push("int main() {");
  out.push("    struct lws_context_creation_info info;");
  out.push("    std::memset(&info, 0, sizeof(info));");
  out.push("    info.port = CONTEXT_PORT_NO_LISTEN;");
  out.push("    info.protocols = protocols;");
  out.push("    struct lws_context *ctx = lws_create_context(&info);");
  out.push("    if (!ctx) {");
  out.push("        std::fprintf(stderr, \"创建上下文失败\\n\");");
  out.push("        return 1;");
  out.push("    }");
  out.push("");
  out.push("    struct lws_client_connect_info cci;");
  out.push("    std::memset(&cci, 0, sizeof(cci));");
  out.push("    cci.context = ctx;");
  out.push(`    cci.address = ${JSON.stringify(u.host)};`);
  out.push(`    cci.port = ${u.port};`);
  out.push(`    cci.path = ${JSON.stringify(u.path)};`);
  out.push("    cci.host = cci.address;   /* Host 请求头 */");
  out.push("    cci.origin = \"api-manager\";");
  out.push("    cci.protocol = protocols[0].name;");
  if (u.scheme === "wss") {
    out.push("    cci.ssl_connection = LCCSCF_USE_SSL | LCCSCF_ALLOW_SELFSIGNED; /* wss */");
  } else {
    out.push("    /* wss: cci.ssl_connection = LCCSCF_USE_SSL | LCCSCF_ALLOW_SELFSIGNED; */");
  }
  out.push("    if (!lws_client_connect_via_info(&cci)) {");
  out.push("        std::fprintf(stderr, \"发起连接失败\\n\");");
  out.push("        lws_context_destroy(ctx);");
  out.push("        return 1;");
  out.push("    }");
  out.push("    while (!g_done) {");
  out.push("        lws_service(ctx, 50);");
  out.push("    }");
  out.push("    lws_context_destroy(ctx);");
  out.push("    return 0;");
  out.push("}");
  return out.join("\n");
}

export function genWsCppUwebsockets(r: WsReq): string {
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（uWebSockets：高性能、事件驱动、人气很高）");
  out.push(" * GitHub/官网: https://github.com/uNetworking/uWebSockets");
  out.push(" * 依赖: uSockets（https://github.com/uNetworking/uSockets）");
  out.push(" * 安装:");
  out.push(" *   git clone https://github.com/uNetworking/uWebSockets");
  out.push(" *   git clone https://github.com/uNetworking/uSockets");
  out.push(" * 编译: g++ -std=c++17 -IuWebSockets/src -IuSockets/src \\");
  out.push(" *       ws_client.cpp uSockets/src/uSockets.c -lpthread -o ws_client");
  out.push(" *");
  for (const h of r.headers) out.push(` * 请求头: ${h.key}: ${h.value}（uWS 客户端暂不支持自定义请求头，请改用 query 参数）`);
  out.push(" */");
  out.push("#include <App.h>");
  out.push("#include <iostream>");
  out.push("#include <string>");
  out.push("");
  out.push(`static const std::string MSG = ${JSON.stringify(r.message || "hello, this is a websocket echo message")};`);
  out.push("");
  out.push("int main() {");
  out.push(`    uWS::App().connect(${JSON.stringify(r.url)}, {`);
  out.push("        .open = [](auto *ws) {");
  out.push("            std::cout << \">>> 发送: \" << MSG << std::endl;");
  out.push("            ws->send(MSG, uWS::OpCode::TEXT);");
  out.push("        },");
  out.push("        .message = [](auto *ws, std::string_view message, uWS::OpCode opCode) {");
  out.push("            (void)opCode;");
  out.push("            std::cout << \"<<< 接收: \" << message << std::endl;");
  out.push("            ws->close();");
  out.push("        },");
  out.push("        .close = [](auto *ws, int code, std::string_view message) {");
  out.push("            (void)ws; (void)code; (void)message;");
  out.push("            std::cout << \"连接已关闭\" << std::endl;");
  out.push("        },");
  out.push("        .error = [](auto *err) {");
  out.push("            std::cerr << \"连接失败: \" << err->what() << std::endl;");
  out.push("        },");
  out.push("    }).run();");
  out.push("    return 0;");
  out.push("}");
  return out.join("\n");
}

export function genWsCppQt(r: WsReq): string {
  const out: string[] = [];
  out.push("/*");
  out.push(" * WebSocket 客户端示例（Qt QWebSocket：Qt 框架，快速开发）");
  out.push(" * 官网: https://doc.qt.io/qt-6/qwebsocket.html");
  out.push(" * 安装: sudo apt install qt6-websockets-dev    (Ubuntu/Debian, Qt6)");
  out.push(" * 编译: g++ -std=c++17 -fPIC ws_client.cpp -o ws_client \\");
  out.push(" *       $(pkg-config --cflags --libs Qt6WebSockets Qt6Core)");
  out.push(" *       （或 CMake: find_package(Qt6 COMPONENTS WebSockets Core)）");
  out.push(" *");
  for (const h of r.headers) out.push(` * 请求头: ${h.key}: ${h.value}（通过 setRequestHeaders 设置）`);
  out.push(" */");
  out.push("#include <QCoreApplication>");
  out.push("#include <QWebSocket>");
  out.push("#include <QDebug>");
  out.push("#include <QUrl>");
  out.push("");
  out.push(`static const QString MSG = QStringLiteral(${JSON.stringify(r.message || "hello, this is a websocket echo message")});`);
  out.push("");
  out.push("int main(int argc, char *argv[]) {");
  out.push("    QCoreApplication app(argc, argv);");
  out.push("    QWebSocket socket;");
  if (r.headers.length) {
    out.push("    socket.setRequestHeaders({");
    for (const h of r.headers) out.push(`        { QStringLiteral(${JSON.stringify(h.key)}), QStringLiteral(${JSON.stringify(h.value)}) },`);
    out.push("    });");
  }
  out.push("");
  out.push("    QObject::connect(&socket, &QWebSocket::connected, [&socket]() {");
  out.push("        qDebug() << \">>> 连接成功\";");
  out.push("        socket.sendTextMessage(MSG);");
  out.push("    });");
  out.push("");
  out.push("    QObject::connect(&socket, &QWebSocket::textMessageReceived,");
  out.push("                    [&socket](const QString &message) {");
  out.push("        qDebug() << \"<<< 接收:\" << message;");
  out.push("        socket.close();");
  out.push("    });");
  out.push("");
  out.push("    QObject::connect(&socket, &QWebSocket::errorOccurred,");
  out.push("                    [&app, &socket](QAbstractSocket::SocketError error) {");
  out.push("        (void)error;");
  out.push("        qDebug() << \"连接失败:\" << socket.errorString();");
  out.push("        app.quit();");
  out.push("    });");
  out.push("");
  out.push("    QObject::connect(&socket, &QWebSocket::disconnected, &app, &QCoreApplication::quit);");
  out.push(`    socket.open(QUrl(${JSON.stringify(r.url)}));`);
  out.push("    return app.exec();");
  out.push("}");
  return out.join("\n");
}
