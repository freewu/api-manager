/** PowerShell（Invoke-RestMethod / ClientWebSocket）代码生成 */

import { esc, Req, WsReq } from "./shared";
export function genPowershell(r: Req): string {
  const out: string[] = [];
  if (r.headers.length) {
    out.push("$headers = @{");
    for (const h of r.headers) out.push(`    "${esc(h.key, '"')}" = "${esc(h.value, '"')}"`);
    out.push("}");
  }
  if (r.body) {
    out.push("");
    out.push(`$body = '${esc(r.body, "'")}'`);
  }
  if (r.files.length) {
    out.push("");
    out.push("# 文件上传（PowerShell 7+）：使用 -Form 参数");
    out.push("$form = @{");
    for (const t of r.formText) out.push(`    "${esc(t.key, '"')}" = "${esc(t.value, '"')}"`);
    for (const f of r.files) out.push(`    "${esc(f.key, '"')}" = Get-Item "${esc(f.path, '"')}"`);
    out.push("}");
  }
  out.push("");
  const args: string[] = [`-Uri "${esc(r.url, '"')}"`, `-Method ${r.method}`];
  if (r.headers.length) args.push("-Headers $headers");
  if (r.files.length) args.push("-Form $form");
  else if (r.body) args.push("-Body $body");
  out.push(`$response = Invoke-RestMethod ${args.join(" ")}`);
  out.push("");
  out.push("$response | ConvertTo-Json -Depth 10");
  return out.join("\n");
}

export function genWsPowershell(r: WsReq): string {
  const ps = (s: string) => "'" + s.replace(/'/g, "''") + "'";
  const out: string[] = [];
  out.push("# WebSocket 客户端示例（System.Net.WebSockets.ClientWebSocket：.NET 原生，PowerShell 5.1 / PowerShell 7+，推荐）");
  out.push("# 官网: https://learn.microsoft.com/dotnet/api/system.net.websockets.clientwebsocket");
  out.push("# 无需额外安装，直接复用 .NET 底层 API");
  out.push("# 运行: powershell -File ws_client.ps1");
  out.push("");
  out.push("# 1. 创建 WebSocket 客户端");
  out.push("$ws = [System.Net.WebSockets.ClientWebSocket]::new()");
  out.push("");
  if (r.headers.length) {
    out.push("# 2. 设置自定义请求头（握手时发送）");
    for (const h of r.headers) out.push(`$ws.Options.SetRequestHeader(${ps(h.key)}, ${ps(h.value)})`);
    out.push("");
    out.push("# 3. 建立连接（wss:// 自动使用 TLS）");
  } else {
    out.push("# 2. 建立连接（wss:// 自动使用 TLS）");
  }
  out.push(`$uri = [uri]${ps(r.url)}`);
  out.push("$ws.ConnectAsync($uri, [Threading.CancellationToken]::None).GetAwaiter().GetResult()");
  out.push("Write-Host '>>> 连接成功'");
  out.push("");
  out.push("# 发送一条文本消息");
  out.push(`$msg = ${ps(r.message || "hello, this is a websocket echo message")}`);
  out.push("$bytes = [Text.Encoding]::UTF8.GetBytes($msg)");
  out.push("# PS 7（.NET 5+）用 ReadOnlyMemory 重载，PS 5.1（.NET Framework）用 ArraySegment 重载");
  out.push("if ($PSVersionTable.PSVersion.Major -ge 7) {");
  out.push("    $ws.SendAsync($bytes, [Net.WebSockets.WebSocketMessageType]::Text, $true, [Threading.CancellationToken]::None).GetAwaiter().GetResult()");
  out.push("} else {");
  out.push("    $ws.SendAsync([ArraySegment[byte]]::new($bytes), [Net.WebSockets.WebSocketMessageType]::Text, $true, [Threading.CancellationToken]::None).GetAwaiter().GetResult()");
  out.push("}");
  out.push(`Write-Host ">>> 发送: $msg"`);
  out.push("");
  out.push("# 接收回显");
  out.push("$buffer = New-Object byte[] 4096");
  out.push("do {");
  out.push("    if ($PSVersionTable.PSVersion.Major -ge 7) {");
  out.push("        $result = $ws.ReceiveAsync($buffer, [Threading.CancellationToken]::None).GetAwaiter().GetResult()");
  out.push("    } else {");
  out.push("        $result = $ws.ReceiveAsync([ArraySegment[byte]]::new($buffer), [Threading.CancellationToken]::None).GetAwaiter().GetResult()");
  out.push("    }");
  out.push("    if ($result.MessageType -eq [Net.WebSockets.WebSocketMessageType]::Close) {");
  out.push("        Write-Host \"连接已关闭: $($result.CloseStatusDescription)\"");
  out.push("        break");
  out.push("    }");
  out.push("    $text = [Text.Encoding]::UTF8.GetString($buffer, 0, $result.Count)");
  out.push("    Write-Host \"<<< 接收: $text\"");
  out.push("} while (-not $result.EndOfMessage)");
  out.push("");
  out.push("# 关闭连接");
  out.push("$ws.CloseAsync([Net.WebSockets.WebSocketCloseStatus]::NormalClosure, 'bye', [Threading.CancellationToken]::None).GetAwaiter().GetResult()");
  out.push("$ws.Dispose()");
  return out.join("\n");
}
