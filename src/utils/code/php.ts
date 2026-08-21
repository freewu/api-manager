/** PHP（cURL / PECL / Snoopy / Guzzle；Swoole / Ratchet）代码生成 */

import { esc, parseWsUrl, Req, WsReq } from "./shared";
export function genPhp(r: Req): string {
  const out: string[] = [];
  out.push("<?php");
  out.push("");
  out.push(`$url = '${esc(r.url, "'")}';`);
  if (r.headers.length) {
    out.push("");
    out.push("$headers = [");
    for (const h of r.headers) out.push(`    '${esc(`${h.key}: ${h.value}`, "'")}',`);
    out.push("];");
  }
  if (r.files.length) {
    out.push("");
    out.push("// 文件上传（multipart/form-data）：文本字段与 CURLFile 文件混用");
    out.push("$postData = [");
    for (const t of r.formText) out.push(`    '${esc(t.key, "'")}' => '${esc(t.value, "'")}',`);
    for (const f of r.files) out.push(`    '${esc(f.key, "'")}' => new CURLFile('${esc(f.path, "'")}'),`);
    out.push("];");
  } else if (r.body) {
    out.push("");
    out.push(`$body = '${esc(r.body, "'")}';`);
  }
  out.push("");
  out.push("$ch = curl_init($url);");
  out.push("curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);");
  out.push(`curl_setopt($ch, CURLOPT_CUSTOMREQUEST, '${r.method}');`);
  if (r.headers.length) out.push("curl_setopt($ch, CURLOPT_HTTPHEADER, $headers);");
  if (r.files.length) {
    out.push("curl_setopt($ch, CURLOPT_POSTFIELDS, $postData);");
  } else if (r.body) {
    out.push("curl_setopt($ch, CURLOPT_POSTFIELDS, $body);");
  }
  out.push("$response = curl_exec($ch);");
  out.push("$status = curl_getinfo($ch, CURLINFO_HTTP_CODE);");
  out.push("curl_close($ch);");
  out.push("");
  out.push('echo $status . "\\n";');
  out.push("echo $response;");
  return out.join("\n");
}

export function genPhpPecl(r: Req): string {
  const out: string[] = [];
  out.push("<?php");
  out.push("");
  out.push("$client = new http\\Client;");
  out.push(`$request = new http\\Client\\Request("${r.method}", '${esc(r.url, "'")}');`);
  if (r.headers.length || r.bodyKind === "json") {
    out.push("$request->setOptions([");
    out.push("    'headers' => [");
    for (const h of r.headers) out.push(`        '${esc(h.key, "'")}' => '${esc(h.value, "'")}',`);
    if (r.bodyKind === "json" && !r.headers.some((h) => h.key.toLowerCase() === "content-type")) {
      out.push("        'Content-Type' => 'application/json',");
    }
    out.push("    ],");
    out.push("]);");
  }
  if (r.body) out.push(`$request->setBody('${esc(r.body, "'")}');`);
  out.push("$client->enqueue($request)->send();");
  out.push("$response = $request->getResponse();");
  out.push("");
  out.push('echo $response->getStatusCode() . "\\n";');
  out.push("echo $response->getBody();");
  return out.join("\n");
}

export function genPhpSnoopy(r: Req): string {
  const out: string[] = [];
  out.push("<?php");
  out.push("");
  out.push("require_once 'Snoopy.class.php';");
  out.push("");
  out.push("$snoopy = new Snoopy;");
  if (r.headers.length || r.bodyKind === "json") {
    out.push("$snoopy->rawheaders = [");
    for (const h of r.headers) out.push(`    '${esc(h.key, "'")}' => '${esc(h.value, "'")}',`);
    if (r.bodyKind === "json" && !r.headers.some((h) => h.key.toLowerCase() === "content-type")) {
      out.push("    'Content-Type' => 'application/json',");
    }
    out.push("];");
  }
  if (r.files.length) {
    out.push("// 注意：Snoopy 不支持 multipart 文件上传，请改用 cURL / Guzzle");
  } else if (r.body) {
    out.push(`$snoopy->submit('${esc(r.url, "'")}', ['payload' => '${esc(r.body, "'")}']);`);
  } else {
    out.push(`$snoopy->fetch('${esc(r.url, "'")}');`);
  }
  out.push("");
  out.push('echo $snoopy->status . "\\n";');
  out.push("echo $snoopy->results;");
  return out.join("\n");
}

export function genPhpGuzzle(r: Req): string {
  const out: string[] = [];
  out.push("<?php");
  out.push("");
  out.push("require 'vendor/autoload.php';");
  out.push("");
  out.push("use GuzzleHttp\\Client;");
  out.push("");
  out.push("$client = new Client();");
  out.push("$options = [");
  if (r.headers.length || (r.bodyKind === "json" && !r.headers.some((h) => h.key.toLowerCase() === "content-type"))) {
    out.push("    'headers' => [");
    for (const h of r.headers) out.push(`        '${esc(h.key, "'")}' => '${esc(h.value, "'")}',`);
    if (r.bodyKind === "json" && !r.headers.some((h) => h.key.toLowerCase() === "content-type")) {
      out.push("        'Content-Type' => 'application/json',");
    }
    out.push("    ],");
  }
  if (r.files.length) {
    out.push("    // 文件上传（multipart）");
    out.push("    'multipart' => [");
    for (const t of r.formText) out.push(`        ['name' => '${esc(t.key, "'")}', 'contents' => '${esc(t.value, "'")}'],`);
    for (const f of r.files) out.push(`        ['name' => '${esc(f.key, "'")}', 'contents' => fopen('${esc(f.path, "'")}', 'r')],`);
    out.push("    ],");
  } else if (r.body) {
    out.push(`    'body' => '${esc(r.body, "'")}',`);
  }
  out.push("];");
  out.push(`$response = $client->request('${r.method}', '${esc(r.url, "'")}', $options);`);
  out.push("");
  out.push('echo $response->getStatusCode() . "\\n";');
  out.push("echo $response->getBody();");
  return out.join("\n");
}

export function genWsPhp(r: WsReq, lib?: string): string {
  switch (lib) {
    case "ratchet":
      return genWsPhpRatchet(r);
    default:
      return genWsPhpSwoole(r);
  }
}

export function genWsPhpSwoole(r: WsReq): string {
  const u = parseWsUrl(r.url);
  const out: string[] = [];
  out.push("<?php");
  out.push("/**");
  out.push(" * WebSocket 客户端示例（Swoole / OpenSwoole，协程，生产环境首选）");
  out.push(" * Swoole 官网: https://www.swoole.com/");
  out.push(" * Swoole 文档: https://wiki.swoole.com/#/websocket_client");
  out.push(" * OpenSwoole 官网: https://openswoole.com/");
  out.push(" * 安装: pecl install swoole      （Swoole）");
  out.push(" *       pecl install openswoole  （OpenSwoole）");
  out.push(" * 运行: php ws_client.php");
  out.push(" */");
  out.push("Co\\run(function () {");
  out.push(`    $client = new Swoole\\WebSocket\\Client(${JSON.stringify(u.host)}, ${u.port}, ${JSON.stringify(u.path)});`);
  out.push("    // 使用 OpenSwoole 时改为：");
  out.push(`    // $client = new OpenSwoole\\WebSocket\\Client(${JSON.stringify(u.host)}, ${u.port}, ${JSON.stringify(u.path)});`);
  if (r.headers.length) {
    out.push("    $client->setHeaders([");
    for (const h of r.headers) out.push(`        ${JSON.stringify(h.key)} => ${JSON.stringify(h.value)},`);
    out.push("    ]);");
  }
  out.push("");
  out.push("    $client->on('open', function (Swoole\\WebSocket\\Client $client) {");
  out.push(`        $msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")};`);
  out.push("        echo '>>> 发送: ' . $msg . PHP_EOL;");
  out.push("        $client->push($msg);");
  out.push("    });");
  out.push("");
  out.push("    $client->on('message', function (Swoole\\WebSocket\\Client $client, Swoole\\WebSocket\\Frame $frame) {");
  out.push("        echo '<<< 接收: ' . $frame->data . PHP_EOL;");
  out.push("        $client->close();");
  out.push("    });");
  out.push("");
  out.push("    $client->on('error', function (Swoole\\WebSocket\\Client $client, $error) {");
  out.push("        echo '连接失败: ' . $error . PHP_EOL;");
  out.push("        $client->close();");
  out.push("    });");
  out.push("");
  out.push("    $client->on('close', function (Swoole\\WebSocket\\Client $client) {");
  out.push("        echo '连接已关闭' . PHP_EOL;");
  out.push("    });");
  out.push("");
  out.push("    if (!$client->connect()) {");
  out.push("        echo '连接失败' . PHP_EOL;");
  out.push("    }");
  out.push("});");
  return out.join("\n");
}

export function genWsPhpRatchet(r: WsReq): string {
  const out: string[] = [];
  out.push("<?php");
  out.push("/**");
  out.push(" * WebSocket 客户端示例（Ratchet：PHP 纯用户态库，基于 ReactPHP，传统 PHP）");
  out.push(" * 官网: http://socketo.me/");
  out.push(" * GitHub: https://github.com/ratchetphp/Ratchet");
  out.push(" * 客户端库 Pawl: https://github.com/ratchetphp/Pawl");
  out.push(" * 安装: composer require ratchet/pawl");
  out.push(" * 运行: php ws_client.php");
  out.push(" */");
  out.push("require __DIR__ . '/vendor/autoload.php';");
  out.push("");
  out.push("use Ratchet\\Client\\Connector;");
  out.push("use Ratchet\\Client\\WebSocket;");
  out.push("use Ratchet\\RFC6455\\Messaging\\MessageInterface;");
  out.push("use React\\EventLoop\\Loop;");
  out.push("");
  out.push("$loop = Loop::get();");
  out.push("$connector = new Connector($loop);");
  out.push("");
  out.push(`$connector(${JSON.stringify(r.url)}, [], [`);
  for (const h of r.headers) out.push(`    ${JSON.stringify(h.key)} => ${JSON.stringify(h.value)},`);
  out.push("])->then(function (WebSocket $conn) {");
  out.push(`    $msg = ${JSON.stringify(r.message || "hello, this is a websocket echo message")};`);
  out.push("    echo '>>> 发送: ' . $msg . PHP_EOL;");
  out.push("    $conn->send($msg);");
  out.push("");
  out.push("    $conn->on('message', function (MessageInterface $message) use ($conn) {");
  out.push("        echo '<<< 接收: ' . $message . PHP_EOL;");
  out.push("        $conn->close();");
  out.push("    });");
  out.push("");
  out.push("    $conn->on('close', function () {");
  out.push("        echo '连接已关闭' . PHP_EOL;");
  out.push("    });");
  out.push("}, function (\\Exception $e) {");
  out.push("    echo '连接失败: ' . $e->getMessage() . PHP_EOL;");
  out.push("});");
  out.push("");
  out.push("$loop->run();");
  return out.join("\n");
}

export function genPhpDispatch(lib: string | undefined, r: Req): string {
  switch (lib) {
    case "pecl":
      return genPhpPecl(r);
    case "snoopy":
      return genPhpSnoopy(r);
    case "guzzle":
      return genPhpGuzzle(r);
    default:
      return genPhp(r);
  }
}
