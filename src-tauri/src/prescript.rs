//! HTTP 接口前置脚本：发送请求前执行的 JS 脚本。
//!
//! 脚本运行在 boa_engine（纯 Rust JS 引擎）中，提供：
//! - `ctx.query` / `ctx.path` / `ctx.headers`：当前请求参数（对象）
//! - `ctx.body`：请求体（JSON 可解析时为对象，否则为原始字符串）
//! - `ctx.global.get(key)` / `ctx.global.set(key, value)`：读写全局变量
//!   （即「环境」当前激活环境里的变量，脚本内 set 的值会写回环境，
//!   之后可用 `{{变量名}}` 绑定到 query / body / path / headers，发送时自动替换）
//! - `CryptoJS`（crypto-js 全量）与 `console.log` 调试日志

use std::collections::HashMap;

use boa_engine::{Context, Source};
use serde::{Deserialize, Serialize};

use crate::{EnvVariable, Environment, KeyValue, WorkspaceState};
use tauri::State;

/// 前置脚本运行结果
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrescriptResult {
    /// console.log / console.error 等打印的日志（按行）
    pub logs: Vec<String>,
    /// 脚本返回值（String / 序列化后的对象；无返回为空串）
    pub result: String,
    /// 脚本执行后更新的全局变量（脚本内 global.set 会写入）
    pub globals: HashMap<String, String>,
}

/// 注入 crypto-js（UMD 单文件，纯 JS），供脚本计算 MD5 / SHA / HMAC / AES 等
const CRYPTO_JS: &str = include_str!("../assets/crypto-js.js");

/// 注入的 polyfill：crypto-js 的 Utf8 编解码依赖浏览器全局 escape / unescape；
/// 新版 crypto-js 的 secure random 依赖原生 crypto.getRandomValues（无 fallback）
const ESCAPE_POLYFILL: &str = r#"
if (typeof globalThis.crypto === 'undefined' || typeof globalThis.crypto.getRandomValues !== 'function') {
  globalThis.crypto = {
    getRandomValues: function (arr) {
      for (var i = 0; i < arr.length; i++) { arr[i] = (Math.random() * 0x100000000) | 0; }
      return arr;
    }
  };
}
function escape(s) {
  return String(s).replace(/[^\x20-\x7E]/g, function (c) {
    var code = c.charCodeAt(0);
    if (code < 256) return '%' + code.toString(16).toUpperCase().padStart(2, '0');
    return '%u' + ('0000' + code.toString(16).toUpperCase()).slice(-4);
  });
}
function unescape(s) {
  return String(s)
    .replace(/%u([0-9a-fA-F]{4})/g, function (m, h) { return String.fromCharCode(parseInt(h, 16)); })
    .replace(/%([0-9a-fA-F]{2})/g, function (m, h) { return String.fromCharCode(parseInt(h, 16)); });
}
"#;

/// KV 列表 → JS 对象字面量（`{"k":"v",...}`，值已 JSON 转义）
fn kv_to_js_obj(pairs: &[KeyValue]) -> String {
    let map: HashMap<&str, &str> = pairs
        .iter()
        .filter(|p| p.enabled && !p.key.trim().is_empty())
        .map(|p| (p.key.trim(), p.value.as_str()))
        .collect();
    serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
}

/// 执行前置脚本。返回日志 / 返回值 / 更新后的全局变量。
pub fn run_prescript_impl(
    code: &str,
    query: &[KeyValue],
    path: &[KeyValue],
    headers: &[KeyValue],
    body: &str,
    globals: &HashMap<String, String>,
) -> Result<PrescriptResult, String> {
    if code.trim().is_empty() {
        return Ok(PrescriptResult {
            logs: vec![],
            result: String::new(),
            globals: globals.clone(),
        });
    }
    // body：尝试按 JSON 对象注入，失败给原始字符串（JS 里再兜底）
    let body_json = serde_json::to_string(body).unwrap_or_else(|_| "\"\"".to_string());
    let globals_json = serde_json::to_string(globals).unwrap_or_else(|_| "{}".to_string());
    let skeleton = format!(
        r#"
var __logs = [];
var console = {{
  log: function () {{ __logs.push(Array.prototype.map.call(arguments, function (x) {{ return (typeof x === 'object' && x !== null) ? JSON.stringify(x) : String(x); }}).join(' ')); }},
  info: function () {{ console.log.apply(null, arguments); }},
  warn: function () {{ __logs.push('[warn] ' + Array.prototype.map.call(arguments, function (x) {{ return String(x); }}).join(' ')); }},
  error: function () {{ __logs.push('[error] ' + Array.prototype.map.call(arguments, function (x) {{ return String(x); }}).join(' ')); }}
}};
var __g = {globals_json};
var ctx = {{
  query: {q},
  path: {p},
  headers: {h},
  body: (function () {{ try {{ return JSON.parse({body_json}); }} catch (e) {{ return {body_json}; }} }})(),
  global: {{
    get: function (k) {{ return __g[k]; }},
    set: function (k, v) {{ __g[k] = String(v); }}
  }}
}};
var __ret;
try {{
  __ret = (function () {{
    {user_code}
  }})();
}} catch (e) {{
  __logs.push('[error] ' + e.message);
}}
JSON.stringify({{ logs: __logs, result: (typeof __ret === 'string') ? __ret : ((__ret === undefined || __ret === null) ? '' : JSON.stringify(__ret)), globals: __g }});
"#,
        globals_json = globals_json,
        q = kv_to_js_obj(query),
        p = kv_to_js_obj(path),
        h = kv_to_js_obj(headers),
        body_json = body_json,
        user_code = code,
    );
    let src = format!("{ESCAPE_POLYFILL}\n{CRYPTO_JS}\n{skeleton}");

    let mut ctx = Context::default();
    let v = ctx
        .eval(Source::from_bytes(src.as_bytes()))
        .map_err(|e| format!("脚本执行失败: {e}"))?;
    let s = v
        .to_string(&mut ctx)
        .map_err(|e| format!("脚本结果解析失败: {e}"))?;
    let json = s.to_std_string_escaped();
    serde_json::from_str::<PrescriptResult>(&json)
        .map_err(|e| format!("脚本结果解析失败: {e}"))
}

/// 把变量表写回「环境」当前激活环境（存在则更新值，不存在则新增变量行）。
/// 无激活环境时自动创建并激活一个默认环境。
fn write_env_vars(root: &std::path::Path, vars: &HashMap<String, String>) -> Result<(), String> {
    let mut store = crate::read_env_file(root);
    if store.active.trim().is_empty() {
        store.active = "Default".to_string();
    }
    let active = store.active.clone();
    let mut found = false;
    for env in store.environments.iter_mut() {
        if env.name == active {
            found = true;
            for (k, v) in vars {
                if let Some(vv) = env.variables.iter_mut().find(|x| x.key == *k) {
                    vv.value = v.clone();
                    vv.enabled = true;
                } else {
                    env.variables.push(EnvVariable {
                        key: k.clone(),
                        value: v.clone(),
                        default_value: String::new(),
                        description: String::new(),
                        enabled: true,
                    });
                }
            }
        }
    }
    if !found {
        let mut env = Environment {
            name: active.clone(),
            variables: Vec::new(),
        };
        for (k, v) in vars {
            env.variables.push(EnvVariable {
                key: k.clone(),
                value: v.clone(),
                default_value: String::new(),
                description: String::new(),
                enabled: true,
            });
        }
        store.environments.push(env);
    }
    crate::write_pretty(&root.join(crate::ENV_FILE), &store)
}

/// 前置脚本测试命令：读取接口参数 + 全局变量，执行脚本并返回日志与结果；
/// 脚本内 global.set 的变量自动写回「环境」当前激活环境
#[tauri::command]
pub fn run_prescript(
    state: State<'_, WorkspaceState>,
    code: String,
    query: Vec<KeyValue>,
    path: Vec<KeyValue>,
    headers: Vec<KeyValue>,
    body: String,
    globals: HashMap<String, String>,
) -> Result<PrescriptResult, String> {
    let result = run_prescript_impl(&code, &query, &path, &headers, &body, &globals)?;
    // global.set 的变量写回环境（前端保存时也会调用 set_global_vars，幂等）
    let root = crate::workspace_root(&state)?;
    write_env_vars(&root, &result.globals)?;
    Ok(result)
}

/// 读取全局变量：即「环境」当前激活环境的所有启用变量
#[tauri::command]
pub fn get_global_vars(state: State<'_, WorkspaceState>) -> Result<HashMap<String, String>, String> {
    let root = crate::workspace_root(&state)?;
    Ok(crate::read_env_map(&root))
}

/// 保存全局变量：写回「环境」当前激活环境（存在则更新值，不存在则新增变量行）
#[tauri::command]
pub fn set_global_vars(
    state: State<'_, WorkspaceState>,
    vars: HashMap<String, String>,
) -> Result<(), String> {
    let root = crate::workspace_root(&state)?;
    write_env_vars(&root, &vars)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn run(code: &str) -> PrescriptResult {
        let mut globals = HashMap::new();
        globals.insert("secret".to_string(), "my-secret".to_string());
        run_prescript_impl(
            code,
            &[
                KeyValue { key: "page".into(), value: "2".into(), enabled: true, description: String::new(), is_file: false },
                KeyValue { key: "off".into(), value: "x".into(), enabled: false, description: String::new(), is_file: false },
            ],
            &[KeyValue { key: "id".into(), value: "42".into(), enabled: true, description: String::new(), is_file: false }],
            &[KeyValue { key: "X-Token".into(), value: "abc".into(), enabled: true, description: String::new(), is_file: false }],
            r#"{"name":"蓝","amount":9.5}"#,
            &globals,
        )
        .unwrap()
    }

    #[test]
    fn test_prescript_reads_params_and_console() {
        let r = run(
            "console.log('page=' + ctx.query.page, 'id=' + ctx.path.id);\nconsole.log('token=' + ctx.headers['X-Token']);\nconsole.log('body name=' + ctx.body.name, 'amount=' + ctx.body.amount);",
        );
        assert_eq!(r.logs.len(), 3, "日志应为 3 行");
        assert!(r.logs[0].contains("page=2"), "query 参数应可读: {:?}", r.logs);
        assert!(r.logs[0].contains("id=42"), "path 参数应可读");
        assert!(r.logs[1].contains("token=abc"));
        assert!(r.logs[2].contains("body name=蓝"));
        assert!(r.logs[2].contains("amount=9.5"));
        assert!(r.result.is_empty(), "无 return 时 result 为空");
    }

    #[test]
    fn test_prescript_global_get_set() {
        let r = run(
            "console.log('secret=' + ctx.global.get('secret'));\nctx.global.set('token', 'new-token');\nreturn ctx.global.get('token');",
        );
        assert!(r.logs[0].contains("secret=my-secret"), "应能读取全局变量");
        assert_eq!(r.result, "new-token", "应能 return 脚本结果");
        assert_eq!(r.globals.get("token").map(|s| s.as_str()), Some("new-token"), "global.set 应写回");
        assert_eq!(r.globals.get("secret").map(|s| s.as_str()), Some("my-secret"), "未修改的变量应保留");
    }

    #[test]
    fn test_prescript_crypto_js() {
        let r = run(
            "console.log('md5=' + CryptoJS.MD5('abc').toString());\nconsole.log('sha=' + CryptoJS.SHA256('abc').toString().substring(0, 8));\nconsole.log('hmac=' + CryptoJS.HmacSHA256('data', ctx.global.get('secret')).toString().substring(0, 8));\nconsole.log('aes=' + CryptoJS.AES.encrypt('hello', 'key').ciphertext.toString().substring(0, 8));",
        );
        assert_eq!(r.logs.len(), 4);
        assert!(r.logs[0].contains("md5=900150983cd24fb0d6963f7d28e17f72"), "MD5 应与标准值一致: {:?}", r.logs);
        assert!(r.logs[1].contains("sha=ba7816bf"), "SHA256 前缀");
        assert!(r.logs[2].contains("hmac="), "HMAC 应可计算");
        assert!(r.logs[3].contains("aes="), "AES 应可加密: {:?}", r.logs);
    }

    #[test]
    fn test_prescript_error_keeps_logs() {
        let r = run("console.log('before');\nthrow new Error('boom');\nconsole.log('after');");
        assert!(r.logs[0].contains("before"));
        assert!(r.logs.iter().any(|l| l.contains("boom")), "异常应记录到日志: {:?}", r.logs);
        assert!(!r.logs.iter().any(|l| l.contains("after")), "异常后的代码不应执行");
    }

    #[test]
    fn test_prescript_disabled_params_excluded() {
        let r = run("console.log('off=' + ctx.query.off);");
        assert!(r.logs[0].contains("off=undefined"), "未启用参数不应注入: {:?}", r.logs);
    }

    #[test]
    fn test_write_env_vars_roundtrip() {
        // 临时目录（Windows cargo 无法访问 WSL /tmp，用系统 temp）
        let tag = format!(
            "apimgr-prescript-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let dir = std::env::temp_dir().join(tag);
        std::fs::create_dir_all(&dir).unwrap();

        // 无激活环境时自动创建 Default 环境
        let mut vars = HashMap::new();
        vars.insert("token".to_string(), "abc".to_string());
        write_env_vars(&dir, &vars).unwrap();
        let store = crate::read_env_file(&dir);
        assert_eq!(store.active, "Default", "无激活环境时应自动创建");
        assert_eq!(store.environments.len(), 1);
        assert_eq!(store.environments[0].variables[0].key, "token");
        assert_eq!(crate::read_env_map(&dir).get("token").unwrap(), "abc");

        // 已有变量更新值、新变量追加行
        vars.insert("token".to_string(), "xyz".to_string());
        vars.insert("new".to_string(), "1".to_string());
        write_env_vars(&dir, &vars).unwrap();
        let map = crate::read_env_map(&dir);
        assert_eq!(map.get("token").unwrap(), "xyz", "已有变量应更新值");
        assert_eq!(map.get("new").unwrap(), "1", "新变量应追加");

        std::fs::remove_dir_all(&dir).ok();
    }
}
