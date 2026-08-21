/** Rust（reqwest / tokio-tungstenite / tungstenite）代码生成 */

import { esc, Req, WsReq } from "./shared";
export function rustRaw(s: string): string {
  let hashes = 1;
  while (s.includes('"#' + "#".repeat(hashes - 1))) hashes++;
  const d = '"#' + "#".repeat(hashes - 1);
  return `r${d}${s}${d}`;
}

export function genRust(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("// 该表单包含文件上传（multipart/form-data），Rust 请使用 reqwest::multipart 构造请求");
  }
  out.push("use reqwest;");
  out.push("");
  out.push("#[tokio::main]");
  out.push("async fn main() -> Result<(), Box<dyn std::error::Error>> {");
  out.push("    let client = reqwest::Client::new();");
  out.push("");
  out.push("    let resp = client");
  const m = r.method.toLowerCase();
  out.push(`        .${m}("${esc(r.url, '"')}")`);
  for (const h of r.headers) {
    out.push(`        .header("${esc(h.key, '"')}", "${esc(h.value, '"')}")`);
  }
  if (r.body) {
    out.push(`        .body(${rustRaw(r.body)})`);
  }
  out.push("        .send()");
  out.push("        .await?;");
  out.push("");
  out.push("    println!(\"{}\", resp.text().await?);");
  out.push("    Ok(())");
  out.push("}");
  return out.join("\n");
}

export function genWsRust(r: WsReq): string {
  const out: string[] = [];
  out.push("// WebSocket 客户端示例（tokio-tungstenite：异步，tokio 运行时，工业级，生产首选）");
  out.push("// 官网: https://github.com/snapview/tokio-tungstenite");
  out.push("// 依赖（Cargo.toml）:");
  out.push("//   [dependencies]");
  out.push("//   tokio = { version = \"1\", features = [\"full\"] }");
  out.push("//   tokio-tungstenite = \"0.24\"");
  out.push("//   futures-util = \"0.3\"");
  out.push("// 编译运行: cargo run");
  out.push("use futures_util::{SinkExt, StreamExt};");
  out.push("use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};");
  out.push("");
  out.push("#[tokio::main]");
  out.push("async fn main() -> Result<(), Box<dyn std::error::Error>> {");
  if (r.headers.length) {
    out.push("    let mut request = " + JSON.stringify(r.url) + ".to_string();");
    for (const h of r.headers) out.push(`    // 请求头：${h.key}: ${h.value}（可通过 tungstenite 的 handshake 设置）`);
    out.push("    let (mut ws, _) = connect_async(request).await?;");
  } else {
    out.push(`    let (mut ws, _) = connect_async(${JSON.stringify(r.url)}).await?;`);
  }
  out.push("");
  if (r.message) {
    out.push(`    let message = ${JSON.stringify(r.message)}.to_string();`);
    out.push("    println!(\">>> 发送: {}\", message);");
    out.push("    ws.send(Message::Text(message.into())).await?;");
    out.push("");
    out.push("    if let Some(Ok(msg)) = ws.next().await {");
    out.push("        println!(\"<<< 接收: {}\", msg);");
    out.push("    }");
  } else {
    out.push("    if let Some(Ok(msg)) = ws.next().await {");
    out.push("        println!(\"<<< 接收: {}\", msg);");
    out.push("    }");
  }
  out.push("    Ok(())");
  out.push("}");
  return out.join("\n");
}

export function genWsRustDispatch(r: WsReq, lib?: string): string {
  switch (lib) {
    case "sync":
      return genWsRustSync(r);
    default:
      return genWsRust(r);
  }
}

export function genWsRustSync(r: WsReq): string {
  const out: string[] = [];
  out.push("// WebSocket 客户端示例（tungstenite：同步阻塞版本，适合简单脚本）");
  out.push("// 官网: https://github.com/snapview/tungstenite-rs");
  out.push("// 依赖（Cargo.toml）:");
  out.push("//   [dependencies]");
  out.push("//   tungstenite = \"0.24\"");
  out.push("//   url = \"2\"");
  out.push("//   http = \"1\"   （自定义请求头时使用）");
  out.push("// 编译运行: cargo run");
  out.push("use tungstenite::{connect, Message};");
  out.push("use url::Url;");
  out.push("");
  out.push("fn main() -> Result<(), Box<dyn std::error::Error>> {");
  if (r.headers.length) {
    out.push("    // 自定义请求头（握手时发送）");
    out.push("    let mut req = http::Request::builder()");
    out.push(`        .uri(${JSON.stringify(r.url)})`);
    for (const h of r.headers) out.push(`        .header(${JSON.stringify(h.key)}, ${JSON.stringify(h.value)})`);
    out.push("        .body(())?;");
    out.push("    let (mut ws, response) = connect(req)?;");
    out.push("    println!(\">>> 连接成功: {}\", response.status());");
  } else {
    out.push(`    let url = Url::parse(${JSON.stringify(r.url)})?;`);
    out.push("    let (mut ws, _) = connect(url)?;");
    out.push("    println!(\">>> 连接成功\");");
  }
  out.push("");
  if (r.message) {
    out.push(`    let message = ${JSON.stringify(r.message)};`);
    out.push("    println!(\">>> 发送: {}\", message);");
    out.push("    ws.send(Message::Text(message.into()))?;");
    out.push("");
    out.push("    // 阻塞接收一条消息");
    out.push("    if let Some(msg) = ws.read()? {");
    out.push("        println!(\"<<< 接收: {}\", msg);");
    out.push("    }");
  } else {
    out.push("    // 阻塞接收一条消息");
    out.push("    if let Some(msg) = ws.read()? {");
    out.push("        println!(\"<<< 接收: {}\", msg);");
    out.push("    }");
  }
  out.push("    Ok(())");
  out.push("}");
  return out.join("\n");
}
