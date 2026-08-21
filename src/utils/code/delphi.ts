/** Delphi（Indy TIdHTTP）代码生成 */

import { esc, Req } from "./shared";
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
