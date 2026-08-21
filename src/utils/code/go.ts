/** Go（net/http；gorilla/websocket）代码生成 */

import { esc, Req, WsReq } from "./shared";
export function genGo(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("// 该表单包含文件上传（multipart/form-data），Go 请使用 mime/multipart 构造请求");
  }
  out.push("package main");
  out.push("");
  out.push("import (");
  out.push("\t\"bytes\"");
  out.push("\t\"fmt\"");
  out.push("\t\"io\"");
  out.push("\t\"net/http\"");
  out.push(")");
  out.push("");
  out.push("func main() {");
  out.push(`\turl := "${esc(r.url, '"')}"`);
  if (r.body) {
    out.push("");
    out.push(`\treqBody := []byte("${esc(r.body, '"')}")`);
  }
  out.push("");
  if (r.body) {
    out.push(`\treq, err := http.NewRequest("${r.method}", url, bytes.NewBuffer(reqBody))`);
  } else {
    out.push(`\treq, err := http.NewRequest("${r.method}", url, nil)`);
  }
  out.push("\tif err != nil {");
  out.push("\t\tpanic(err)");
  out.push("\t}");
  for (const h of r.headers) {
    out.push(`\treq.Header.Set("${esc(h.key, '"')}", "${esc(h.value, '"')}")`);
  }
  out.push("");
  out.push("\tresp, err := http.DefaultClient.Do(req)");
  out.push("\tif err != nil {");
  out.push("\t\tpanic(err)");
  out.push("\t}");
  out.push("\tdefer resp.Body.Close()");
  out.push("");
  out.push("\tbody, err := io.ReadAll(resp.Body)");
  out.push("\tif err != nil {");
  out.push("\t\tpanic(err)");
  out.push("\t}");
  out.push("\tfmt.Println(string(body))");
  out.push("}");
  return out.join("\n");
}

export function genWsGo(r: WsReq): string {
  const hdrArg = r.headers.length ? ", httpHeader()" : ", nil";
  const out: string[] = [];
  out.push("package main");
  out.push("");
  out.push("import (");
  out.push("    \"fmt\"");
  out.push("    \"log\"");
  if (r.headers.length) out.push("    \"net/http\"");
  out.push("    \"github.com/gorilla/websocket\"");
  out.push(")");
  out.push("");
  if (r.headers.length) {
    out.push("func httpHeader() http.Header {");
    out.push("    header := http.Header{}");
    for (const h of r.headers) out.push(`    header.Set(${JSON.stringify(h.key)}, ${JSON.stringify(h.value)})`);
    out.push("    return header");
    out.push("}");
    out.push("");
  }
  out.push("func main() {");
  out.push(`    conn, _, err := websocket.DefaultDialer.Dial(${JSON.stringify(r.url)}${hdrArg})`);
  out.push("    if err != nil {");
  out.push("        log.Fatal(\"连接失败:\", err)");
  out.push("    }");
  out.push("    defer conn.Close()");
  out.push("");
  if (r.message) {
    out.push(`    message := ${JSON.stringify(r.message)}`);
    out.push("    fmt.Println(\">>> 发送:\", message)");
    out.push("    if err := conn.WriteMessage(websocket.TextMessage, []byte(message)); err != nil {");
    out.push("        log.Fatal(\"发送失败:\", err)");
    out.push("    }");
    out.push("");
    out.push("    if _, p, err := conn.ReadMessage(); err != nil {");
    out.push("        log.Fatal(\"接收失败:\", err)");
    out.push("    } else {");
    out.push("        fmt.Println(\"<<< 接收:\", string(p))");
    out.push("    }");
  } else {
    out.push("    if _, p, err := conn.ReadMessage(); err != nil {");
    out.push("        log.Fatal(\"接收失败:\", err)");
    out.push("    } else {");
    out.push("        fmt.Println(\"<<< 接收:\", string(p))");
    out.push("    }");
  }
  out.push("}");
  return out.join("\n");
}
