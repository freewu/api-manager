/** 代码生成入口：generateRequestCode / generateWebSocketCode */
import { ApiFile } from "../../types";
import { CodeLang, buildReq, buildWsReq } from "./shared";
import { genCurl, genBashWget, genBashHttpie, genWsBash } from "./bash";
import { genPython, genPythonHttpClient, genWsPythonDispatch } from "./python";
import { genC, genWsC } from "./c";
import { genCpp, genWsCpp } from "./cpp";
import { genJavaDispatch, genWsJavaDispatch } from "./java";
import { genCsharp, genWsCsharp } from "./csharp";
import { genJsDispatch, genWsJavaScript } from "./javascript";
import { genR, genRRCurl } from "./r";
import { genRust, genWsRustDispatch } from "./rust";
import { genDelphi } from "./delphi";
import { genPhpDispatch, genWsPhp } from "./php";
import { genGo, genWsGo } from "./go";
import { genRuby, genWsRuby } from "./ruby";
import { genSwift, genWsSwiftDispatch } from "./swift";
import { genPerl, genWsPerlDispatch } from "./perl";
import { genObjectiveC } from "./objectivec";
import { genJulia, genWsJulia } from "./julia";
import { genKotlin, genWsKotlinDispatch } from "./kotlin";
import { genTsDispatch, genWsTypeScript } from "./typescript";
import { genErlang, genWsErlang } from "./erlang";
import { genLuaDispatch, genWsLuaDispatch } from "./lua";
import { genPowershell, genWsPowershell } from "./powershell";

export function generateWebSocketCode(lang: CodeLang, api: ApiFile, baseUrl: string, lib?: string): string {
  const r = buildWsReq(api, baseUrl);
  switch (lang) {
    case "bash":
    case "curl":
      return genWsBash(r);
    case "python":
      return genWsPythonDispatch(r, lib);
    case "javascript":
      return genWsJavaScript(r);
    case "typescript":
      return genWsTypeScript(r);
    case "go":
      return genWsGo(r);
    case "java":
      return genWsJavaDispatch(r, lib);
    case "csharp":
      return genWsCsharp(r);
    case "rust":
      return genWsRustDispatch(r, lib);
    case "c":
      return genWsC(r, lib);
    case "cpp":
      return genWsCpp(r, lib);
    case "php":
      return genWsPhp(r, lib);
    case "ruby":
      return genWsRuby(r, lib);
    case "swift":
      return genWsSwiftDispatch(r, lib);
    case "perl":
      return genWsPerlDispatch(r, lib);
    case "julia":
      return genWsJulia(r);
    case "kotlin":
      return genWsKotlinDispatch(r, lib);
    case "erlang":
      return genWsErlang(r);
    case "powershell":
      return genWsPowershell(r);
    case "lua":
      return genWsLuaDispatch(r, lib);
    default:
      return genWsUnsupported(lang);
  }
}

export function generateRequestCode(lang: CodeLang, api: ApiFile, baseUrl: string, lib?: string): string {
  const r = buildReq(api, baseUrl);
  switch (lang) {
    case "bash":
    case "curl":
      if (lib === "wget") return genBashWget(r);
      if (lib === "httpie") return genBashHttpie(r);
      return genCurl(r);
    case "python":
      return lib === "httpclient" ? genPythonHttpClient(r) : genPython(r);
    case "c":
      return genC(r);
    case "cpp":
      return genCpp(r);
    case "java":
      return genJavaDispatch(lib, r);
    case "csharp":
      return genCsharp(r);
    case "javascript":
      return genJsDispatch(lib, r);
    case "r":
      return lib === "rcurl" ? genRRCurl(r) : genR(r);
    case "rust":
      return genRust(r);
    case "delphi":
      return genDelphi(r);
    case "php":
      return genPhpDispatch(lib, r);
    case "go":
      return genGo(r);
    case "ruby":
      return genRuby(r);
    case "swift":
      return genSwift(r);
    case "perl":
      return genPerl(r);
    case "objectivec":
      return genObjectiveC(r);
    case "julia":
      return genJulia(r);
    case "kotlin":
      return genKotlin(r);
    case "typescript":
      return genTsDispatch(lib, r);
    case "erlang":
      return genErlang(r);
    case "lua":
      return genLuaDispatch(lib, r);
    case "powershell":
      return genPowershell(r);
  }
}

function genWsUnsupported(lang: string): string {
  return `// ${lang}：暂未内置 WebSocket 客户端代码生成`;
}

export * from "./shared";
