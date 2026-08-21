/** C#（HttpClient / ClientWebSocket）代码生成 */

import { esc, Req, WsReq } from "./shared";
export const CS_METHODS: Record<string, string> = {
  GET: "Get",
  POST: "Post",
  PUT: "Put",
  DELETE: "Delete",
  PATCH: "Patch",
  HEAD: "Head",
  OPTIONS: "Options",
};

export function genCsharp(r: Req): string {
  const out: string[] = [];
  const contentType = r.bodyKind === "json" ? "application/json" : "text/plain";
  out.push("using System;");
  out.push("using System.Net.Http;");
  out.push("using System.Net.Http.Headers;");
  out.push("using System.Threading.Tasks;");
  out.push("");
  out.push("class Program");
  out.push("{");
  out.push("    static async Task Main()");
  out.push("    {");
  out.push("        using var client = new HttpClient();");
  out.push(`        var request = new HttpRequestMessage(HttpMethod.${CS_METHODS[r.method] ?? "Get"}, "${esc(r.url, '"')}");`);
  for (const h of r.headers) {
    out.push(`        request.Headers.TryAddWithoutValidation("${esc(h.key, '"')}", "${esc(h.value, '"')}");`);
  }
  if (r.body) {
    out.push("");
    out.push(`        var body = "${esc(r.body, '"')}";`);
    out.push("        request.Content = new StringContent(body);");
    out.push(`        request.Content.Headers.ContentType = new MediaTypeHeaderValue("${contentType}");`);
  } else if (r.files.length) {
    out.push("");
    out.push("        // 该表单包含文件上传（multipart/form-data），请使用 MultipartFormDataContent 构造请求");
  }
  out.push("");
  out.push("        var response = await client.SendAsync(request);");
  out.push("        Console.WriteLine((int)response.StatusCode);");
  out.push("        Console.WriteLine(await response.Content.ReadAsStringAsync());");
  out.push("    }");
  out.push("}");
  return out.join("\n");
}

export function genWsCsharp(r: WsReq): string {
  const out: string[] = [];
  out.push("using System;");
  out.push("using System.Net.WebSockets;");
  out.push("using System.Text;");
  out.push("using System.Threading;");
  out.push("using System.Threading.Tasks;");
  out.push("");
  out.push("class Program");
  out.push("{");
  out.push("    static async Task Main()");
  out.push("    {");
  out.push("        using var ws = new ClientWebSocket();");
  if (r.headers.length) {
    for (const h of r.headers) out.push(`        ws.Options.SetRequestHeader(${JSON.stringify(h.key)}, ${JSON.stringify(h.value)});`);
  }
  out.push(`        await ws.ConnectAsync(new Uri(${JSON.stringify(r.url)}), CancellationToken.None);`);
  if (r.message) {
    out.push("");
    out.push(`        var message = ${JSON.stringify(r.message)};`);
    out.push("        var bytes = Encoding.UTF8.GetBytes(message);");
    out.push("        await ws.SendAsync(new ArraySegment<byte>(bytes), WebSocketMessageType.Text, true, CancellationToken.None);");
    out.push("");
    out.push("        var buffer = new byte[4096];");
    out.push("        var result = await ws.ReceiveAsync(new ArraySegment<byte>(buffer), CancellationToken.None);");
    out.push("        Console.WriteLine(\"<<< 接收:\" + Encoding.UTF8.GetString(buffer, 0, result.Count));");
  } else {
    out.push("");
    out.push("        var buffer = new byte[4096];");
    out.push("        var result = await ws.ReceiveAsync(new ArraySegment<byte>(buffer), CancellationToken.None);");
    out.push("        Console.WriteLine(\"<<< 接收:\" + Encoding.UTF8.GetString(buffer, 0, result.Count));");
  }
  out.push("    }");
  out.push("}");
  return out.join("\n");
}
