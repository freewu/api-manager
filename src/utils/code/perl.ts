/** Perl（LWP::UserAgent；Mojo::UserAgent / AnyEvent::WebSocket::Client）代码生成 */

import { esc, Req, WsReq } from "./shared";
export function genPerl(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("# 该表单包含文件上传（multipart/form-data），请使用 HTTP::Request::Common 构造请求");
  }
  out.push("#!/usr/bin/perl");
  out.push("use strict;");
  out.push("use warnings;");
  out.push("use LWP::UserAgent;");
  out.push("use HTTP::Request;");
  out.push("");
  out.push(`my $url = '${esc(r.url, "'")}';`);
  out.push("my $ua = LWP::UserAgent->new;");
  out.push(`my $req = HTTP::Request->new('${r.method}', $url);`);
  for (const h of r.headers) {
    out.push(`$req->header('${esc(h.key, "'")}' => '${esc(h.value, "'")}');`);
  }
  if (r.body) {
    out.push(`$req->content('${esc(r.body, "'")}');`);
    out.push(`$req->content_type('${r.bodyKind === "json" ? "application/json" : "text/plain"}');`);
  }
  out.push("");
  out.push("my $resp = $ua->request($req);");
  out.push('print $resp->code, "\\n";');
  out.push('print $resp->decoded_content, "\\n";');
  return out.join("\n");
}

export function genWsPerlDispatch(r: WsReq, lib?: string): string {
  switch (lib) {
    case "anyevent":
      return genWsPerlAnyEvent(r);
    default:
      return genWsPerlMojo(r);
  }
}

export function genWsPerlMojo(r: WsReq): string {
  const out: string[] = [];
  out.push("#!/usr/bin/env perl");
  out.push("# WebSocket 客户端示例（Mojo::UserAgent：Mojolicious 全家桶，工业级，推荐）");
  out.push("# 官网: https://mojolicious.org/");
  out.push("# 文档: https://docs.mojolicious.org/Mojo/UserAgent");
  out.push("# 特性: 支持 ws/wss、文本/二进制帧、ping/pong（自动处理）");
  out.push("# 安装: cpanm Mojolicious   （或 apt install libmojolicious-perl）");
  out.push("# 运行: perl ws_client.pl");
  out.push("use strict;");
  out.push("use warnings;");
  out.push("use v5.10;");
  out.push("use Mojo::UserAgent;");
  out.push("");
  out.push("my $ua = Mojo::UserAgent->new;");
  if (r.headers.length) {
    out.push("");
    out.push("# 自定义请求头（握手时发送）");
    out.push("$ua->on(start => sub {");
    out.push("    my ($ua, $tx) = @_;");
    for (const h of r.headers) out.push(`    $tx->req->headers->header(${JSON.stringify(h.key)} => ${JSON.stringify(h.value)});`);
    out.push("});");
  }
  out.push("");
  out.push(`$ua->websocket(${JSON.stringify(r.url)} => sub {`);
  out.push("    my ($ua, $tx) = @_;");
  out.push("");
  out.push("    unless ($tx->is_websocket) {");
  out.push("        say '连接失败: ' . ($tx->res->message || 'unknown');");
  out.push("        return;");
  out.push("    }");
  out.push("");
  out.push("    say '>>> 连接成功';");
  out.push(`    my $msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")};`);
  out.push("    say \">>> 发送: $msg\";");
  out.push("    $tx->send($msg);                  # 文本帧");
  out.push("    # $tx->send({binary => $msg});   # 二进制帧");
  out.push("");
  out.push("    $tx->on(message => sub {");
  out.push("        my ($tx, $msg) = @_;");
  out.push("        say \"<<< 接收: $msg\";");
  out.push("        $tx->close;");
  out.push("    });");
  out.push("");
  out.push("    $tx->on(finish => sub {");
  out.push("        my ($tx, $code, $reason) = @_;");
  out.push("        say \"连接已关闭: $code $reason\";");
  out.push("        Mojo::IOLoop->stop;");
  out.push("    });");
  out.push("");
  out.push("    $tx->on(error => sub {");
  out.push("        my ($tx, $err) = @_;");
  out.push("        say \"错误: $err\";");
  out.push("    });");
  out.push("});");
  out.push("");
  out.push("# 启动事件循环（保持运行）");
  out.push("Mojo::IOLoop->start unless Mojo::IOLoop->is_running;");
  return out.join("\n");
}

export function genWsPerlAnyEvent(r: WsReq): string {
  const out: string[] = [];
  out.push("#!/usr/bin/env perl");
  out.push("# WebSocket 客户端示例（AnyEvent::WebSocket::Client：AnyEvent 事件驱动，非阻塞）");
  out.push("# 官网: https://metacpan.org/pod/AnyEvent::WebSocket::Client");
  out.push("# AnyEvent 官网: https://metacpan.org/pod/AnyEvent");
  out.push("# 安装: cpanm AnyEvent AnyEvent::WebSocket::Client");
  out.push("# 运行: perl ws_client.pl");
  out.push("use strict;");
  out.push("use warnings;");
  out.push("use v5.10;");
  out.push("use AnyEvent;");
  out.push("use AnyEvent::WebSocket::Client 0.22;");
  out.push("");
  out.push("my $client = AnyEvent::WebSocket::Client->new;");
  out.push("");
  if (r.headers.length) {
    out.push("# 自定义请求头（握手时发送）");
    out.push("my %headers = (");
    for (const h of r.headers) out.push(`    ${JSON.stringify(h.key)} => ${JSON.stringify(h.value)},`);
    out.push(");");
    out.push("");
    out.push(`$client->connect(${JSON.stringify(r.url)}, headers => \\%headers)->cb(sub {`);
  } else {
    out.push(`$client->connect(${JSON.stringify(r.url)})->cb(sub {`);
  }
  out.push("    my $cv = shift;");
  out.push("    my $conn = eval { $cv->recv };");
  out.push("    unless ($conn) {");
  out.push("        say \"连接失败: $@\";");
  out.push("        return;");
  out.push("    }");
  out.push("");
  out.push("    say '>>> 连接成功';");
  out.push(`    my $msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")};`);
  out.push("    say \">>> 发送: $msg\";");
  out.push("    $conn->send($msg);   # 文本消息");
  out.push("");
  out.push("    $conn->on(each_message => sub {");
  out.push("        my ($conn, $message) = @_;");
  out.push("        say \"<<< 接收: \" . $message->body;");
  out.push("    });");
  out.push("");
  out.push("    $conn->on(finish => sub {");
  out.push("        my ($conn, $code, $reason) = @_;");
  out.push("        say \"连接已关闭: $code $reason\";");
  out.push("        exit 0;");
  out.push("    });");
  out.push("");
  out.push("    $conn->on(error => sub {");
  out.push("        my ($conn, $err) = @_;");
  out.push("        say \"错误: $err\";");
  out.push("    });");
  out.push("});");
  out.push("");
  out.push("# 保持运行（事件循环）");
  out.push("AnyEvent->condvar->recv;");
  return out.join("\n");
}
