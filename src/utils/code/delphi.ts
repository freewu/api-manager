/** Delphi（Indy TIdHTTP）代码生成 */

import { esc, parseWsUrl, Req, WsReq } from "./shared";
export const DELPHI_METHODS: Record<string, string> = {
  GET: "Get",
  POST: "Post",
  PUT: "Put",
  DELETE: "Delete",
  PATCH: "Patch",
  HEAD: "Head",
  OPTIONS: "Options",
};

export function genDelphi(r: Req): string {
  const out: string[] = [];
  if (r.files.length) {
    out.push("// 该表单包含文件上传（multipart/form-data），请使用 TIdMultiPartFormDataStream 构造请求");
  }
  out.push("uses");
  out.push("  System.SysUtils, IdHTTP, IdSSLOpenSSL;");
  out.push("");
  out.push("procedure DoRequest;");
  out.push("var");
  out.push("  HTTP: TIdHTTP;");
  out.push("  SSL: TIdSSLIOHandlerSocketOpenSSL;");
  out.push("  Resp: string;");
  if (r.body) out.push("  Stream: TStringStream;");
  out.push("begin");
  out.push("  HTTP := TIdHTTP.Create(nil);");
  out.push("  SSL := TIdSSLIOHandlerSocketOpenSSL.Create(nil);");
  out.push("  HTTP.IOHandler := SSL;");
  out.push("  try");
  out.push(`    HTTP.Request.Method := '${r.method}';`);
  for (const h of r.headers) {
    out.push(`    HTTP.Request.CustomHeaders.AddValue('${esc(h.key, "'")}', '${esc(h.value, "'")}');`);
  }
  const m = DELPHI_METHODS[r.method] ?? "Get";
  if (r.body) {
    out.push(`    Stream := TStringStream.Create('${esc(r.body, "'")}', TEncoding.UTF8);`);
    out.push("    try");
    out.push(`      Resp := HTTP.${m}('${esc(r.url, "'")}', Stream);`);
    out.push("    finally");
    out.push("      Stream.Free;");
    out.push("    end;");
  } else {
    out.push(`    Resp := HTTP.${m}('${esc(r.url, "'")}');`);
  }
  out.push("    WriteLn(Resp);");
  out.push("  finally");
  out.push("    SSL.Free;");
  out.push("    HTTP.Free;");
  out.push("  end;");
  out.push("end;");
  return out.join("\n");
}

/* ------------------------------------------------------------------ */
/* WebSocket：Delphi 没有原生 WebSocket，常见三种方式：                */
/*  1. Indy 10（Delphi XE8 起内置 TIdWebSocket，最常用）              */
/*  2. Delphi-WebSocket（基于 Synapse 的免费开源库）                  */
/*  3. Websocket4Delphi（封装 Windows WinHTTP WebSocket API）        */
/* ------------------------------------------------------------------ */

function wsHeader(out: string[], u: { scheme: string; host: string; port: number; path: string }, lib: string, note: string[]): void {
  out.push("// WebSocket 客户端示例（" + lib + "）");
  for (const n of note) out.push("// " + n);
  out.push("// 目标: " + u.scheme + "://" + u.host + ":" + u.port + u.path);
  out.push("");
}

/** 1) Indy 10 内置 TIdWebSocket（Delphi XE8+，最常用） */
export function genWsDelphiIndy(r: WsReq): string {
  const u = parseWsUrl(r.url);
  const out: string[] = [];
  wsHeader(out, u, "Indy 10 TIdWebSocket", [
    "Indy 10 从 Delphi XE8 起内置 WebSocket 支持（IdWebSocketClient 单元）",
    "无需安装第三方包；wss:// 时把 Scheme 改为 'wss' 并引入 IdSSLOpenSSL",
  ]);
  out.push("program ws_client;");
  out.push("");
  out.push("{$APPTYPE CONSOLE}");
  out.push("");
  out.push("uses");
  out.push("  System.SysUtils,");
  out.push("  IdWebSocketClient;");
  out.push("");
  out.push("var");
  out.push("  WS: TIdWebSocketClient;");
  out.push("  Msg: string;");
  out.push("begin");
  out.push("  WS := TIdWebSocketClient.Create(nil);");
  out.push("  try");
  out.push("    WS.Host := '" + esc(u.host, "'") + "';");
  out.push("    WS.Port := " + u.port + ";");
  out.push("    WS.Path := '" + esc(u.path, "'") + "';");
  out.push("    WS.Scheme := '" + u.scheme + "';");
  out.push("    WS.Origin := '';");
  for (const h of r.headers) {
    out.push("    WS.CustomHeaders.AddValue('" + esc(h.key, "'") + "', '" + esc(h.value, "'") + "');");
  }
  out.push("");
  out.push("    WS.Connect;");
  out.push("    WriteLn('>>> 连接成功');");
  out.push("    WS.SendText('" + esc(r.message || "hello, this is a websocket echo message", "'") + "');");
  out.push("    WriteLn('>>> 发送完成');");
  out.push("");
  out.push("    Msg := WS.Receive;");
  out.push("    WriteLn('<<< 接收: ' + Msg);");
  out.push("  finally");
  out.push("    WS.Disconnect;");
  out.push("    WS.Free;");
  out.push("  end;");
  out.push("end.");
  return out.join("\n");
}

/** 2) Delphi-WebSocket：Synapse 库（httpsend.pas）+ websocket 单元，免费开源 */
export function genWsDelphiSynapse(r: WsReq): string {
  const u = parseWsUrl(r.url);
  const out: string[] = [];
  wsHeader(out, u, "Delphi-WebSocket（Synapse）", [
    "Synapse 库 + websocket 单元，免费开源（基于 Synapse 的 WebSocket 实现）",
    "将 httpsend.pas / websocket.pas 加入工程即可，无需注册组件",
  ]);
  out.push("program ws_client;");
  out.push("");
  out.push("{$APPTYPE CONSOLE}");
  out.push("");
  out.push("uses");
  out.push("  System.SysUtils,");
  out.push("  httpsend,"); // Synapse
  out.push("  websocket;"); // Delphi-WebSocket
  out.push("");
  out.push("var");
  out.push("  WS: TWebsocketClient;");
  out.push("  Msg: string;");
  out.push("begin");
  out.push("  WS := TWebsocketClient.Create;");
  out.push("  try");
  out.push("    WS.URL := '" + esc(r.url, "'") + "';");
  out.push("    WS.Protocol := 'echo';");
  for (const h of r.headers) {
    out.push("    WS.Headers.Add('" + esc(h.key, "'") + "', '" + esc(h.value, "'") + "');");
  }
  out.push("");
  out.push("    WS.Connect;");
  out.push("    WriteLn('>>> 连接成功');");
  out.push("    WS.Send('" + esc(r.message || "hello, this is a websocket echo message", "'") + "');");
  out.push("    WriteLn('>>> 发送完成');");
  out.push("");
  out.push("    Msg := WS.Receive;");
  out.push("    WriteLn('<<< 接收: ' + Msg);");
  out.push("  finally");
  out.push("    WS.Free;");
  out.push("  end;");
  out.push("end.");
  return out.join("\n");
}

/** 3) Websocket4Delphi：第三方封装库，封装 Windows WinHTTP WebSocket API */
export function genWsDelphiWebsocket4Delphi(r: WsReq): string {
  const u = parseWsUrl(r.url);
  const out: string[] = [];
  wsHeader(out, u, "Websocket4Delphi（WinHTTP）", [
    "第三方封装库：封装 Windows WinHTTP WebSocket API（winhttp.dll，Win8+ 自带）",
    "免费开源，支持 ws/wss，需在工程中加入库单元",
  ]);
  out.push("program ws_client;");
  out.push("");
  out.push("{$APPTYPE CONSOLE}");
  out.push("");
  out.push("uses");
  out.push("  System.SysUtils,");
  out.push("  Winapi.Windows,");
  out.push("  Winapi.Winhttp,");
  out.push("  WSWebsocket;"); // Websocket4Delphi 单元（按库实际名称调整）
  out.push("");
  out.push("var");
  out.push("  WS: TWSWebSocketClient;");
  out.push("  Msg: string;");
  out.push("begin");
  out.push("  WS := TWSWebSocketClient.Create(nil);");
  out.push("  try");
  out.push("    WS.URL := '" + esc(r.url, "'") + "';");
  for (const h of r.headers) {
    out.push("    WS.Headers.AddValue('" + esc(h.key, "'") + "', '" + esc(h.value, "'") + "');");
  }
  out.push("");
  out.push("    WS.Connect;");
  out.push("    WriteLn('>>> 连接成功');");
  out.push("    WS.SendText('" + esc(r.message || "hello, this is a websocket echo message", "'") + "');");
  out.push("    WriteLn('>>> 发送完成');");
  out.push("");
  out.push("    Msg := WS.ReceiveText;");
  out.push("    WriteLn('<<< 接收: ' + Msg);");
  out.push("  finally");
  out.push("    WS.Free;");
  out.push("  end;");
  out.push("end.");
  return out.join("\n");
}

export function genWsDelphiDispatch(r: WsReq, lib?: string): string {
  if (lib === "synapse") return genWsDelphiSynapse(r);
  if (lib === "websocket4delphi") return genWsDelphiWebsocket4Delphi(r);
  return genWsDelphiIndy(r); // 默认 Indy（Delphi 自带，最常用）
}
