//! 由 import.rs 拆分：Postman Collection
use super::*;
#[allow(unused_imports)]
use crate::{ApiFile, BodyData, DocParam, EnvVariable, KeyValue, MockConfig, ResponseItem, sanitize_filename, unique_path, workspace_root, ENV_FILE, INFO_FILE};
#[allow(unused_imports)]
use serde::Serialize;
#[allow(unused_imports)]
use serde_json::Value;
#[allow(unused_imports)]
use std::collections::{BTreeMap, HashMap};
#[allow(unused_imports)]
use std::fs;
#[allow(unused_imports)]
use std::path::{Path, PathBuf};

/// 解析 Postman Collection 文件，在工作区根新建同名分组并导入全部接口；
/// 同时把集合级 `variable` 合并到工作区环境变量（__envs.json）
pub(crate) fn import_postman_file(root: &Path, file: &Path) -> Result<PostmanImportResult, String> {
    let mut stats = ImportStats::default();
    let mut failed = 0usize;
    let mut duplicated = 0usize;
    let content = fs::read_to_string(file).map_err(|e| format!("读取文件失败: {e}"))?;
    let json: Value =
        serde_json::from_str(&content).map_err(|e| format!("解析 JSON 失败: {e}"))?;
    let info = json
        .get("info")
        .ok_or("不是有效的 Postman Collection 文件（缺少 info 字段）")?;
    let coll_name = info
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Postman 导入")
        .to_string();
    let dir_name = sanitize_filename(&coll_name);
    let dir_name = if dir_name.is_empty() {
        "Postman 导入".to_string()
    } else {
        dir_name
    };
    let folder = unique_path(root, &dir_name, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    let src_name = file
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some(coll_name.clone()),
            description: format!("从 Postman Collection 导入（{src_name}）"),
            base_url: None,
            mock_port: None,
            order: None,
            collapsed: None,
            deprecated: None,
        },
    )?;
    let items = json
        .get("item")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    import_postman_items(&folder, &items, &mut stats, &mut failed, &mut duplicated)?;
    // 集合级变量 -> 环境变量集（同名合并，否则新建；无激活环境时自动激活）
    let mut env = String::new();
    let mut vars = 0usize;
    if let Some(arr) = json.get("variable").and_then(|v| v.as_array()) {
        let env_vars = postman_variables_to_env(arr);
        if !env_vars.is_empty() {
            vars = env_vars.len();
            env = coll_name.clone();
            merge_postman_env(root, &env, env_vars)?;
        }
    }
    Ok(PostmanImportResult {
        folder: folder.to_string_lossy().to_string(),
        env,
        vars,
        http: stats.http,
        ws: stats.ws,
        graphql: stats.graphql,
        socketio: stats.socketio,
        objects: 0,
        failed,
        duplicated,
    })
}

/// 将 Postman variable 数组（{key, value, type, description}）转换为应用环境变量
fn postman_variables_to_env(vars: &[Value]) -> Vec<EnvVariable> {
    vars.iter()
        .filter_map(|v| {
            let key = v
                .get("key")
                .and_then(|k| k.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if key.is_empty() {
                return None;
            }
            let value = match v.get("value") {
                Some(Value::String(s)) => s.clone(),
                Some(Value::Number(n)) => n.to_string(),
                Some(Value::Bool(b)) => b.to_string(),
                Some(Value::Null) | None => String::new(),
                Some(other) => other.to_string(),
            };
            let description = match v.get("description") {
                Some(Value::String(s)) => s.clone(),
                // Postman 结构化描述：{ "content": "...", "type": "text/plain" }
                Some(Value::Object(o)) => o
                    .get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string(),
                _ => String::new(),
            };
            Some(EnvVariable {
                key,
                value,
                default_value: String::new(),
                description,
                enabled: !v.get("disabled").and_then(|d| d.as_bool()).unwrap_or(false),
            })
        })
        .collect()
}

/// 把导入的变量合并进工作区 __envs.json：
/// 同名环境变量集存在则按 key 合并，否则新建；无激活环境时自动激活该集
fn merge_postman_env(root: &Path, env_name: &str, variables: Vec<EnvVariable>) -> Result<(), String> {
    if variables.is_empty() {
        return Ok(());
    }
    let mut store = read_env_file(root);
    if let Some(env) = store.environments.iter_mut().find(|e| e.name == env_name) {
        for v in variables {
            match env.variables.iter_mut().find(|x| x.key == v.key) {
                Some(existing) => {
                    if !v.value.is_empty() {
                        existing.value = v.value;
                    }
                    if !v.description.is_empty() {
                        existing.description = v.description;
                    }
                    existing.enabled = true;
                }
                None => env.variables.push(v),
            }
        }
    } else {
        store.environments.push(Environment {
            name: env_name.to_string(),
            variables,
        });
    }
    if store.active.is_empty() {
        store.active = env_name.to_string();
    }
    write_pretty(&root.join(ENV_FILE), &store)
}

/// 递归导入 item 列表：带 request 的生成接口文件，带 item 的生成子分组
fn import_postman_items(
    dir: &Path,
    items: &[Value],
    stats: &mut ImportStats,
    failed: &mut usize,
    duplicated: &mut usize,
) -> Result<(), String> {
    for item in items {
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("未命名")
            .to_string();
        if item.get("request").is_some() {
            let request = &item["request"];
            let api = match postman_request_to_api(&name, request) {
                Ok(a) => a,
                Err(_) => {
                    *failed += 1;
                    continue;
                }
            };
            stats.add(&api.protocol);
            let file_base = sanitize_filename(&name);
            let file_base = if file_base.is_empty() {
                "未命名接口".to_string()
            } else {
                file_base
            };
            let file_path = unique_path(dir, &file_base, ".json");
            if file_path != dir.join(format!("{file_base}.json")) {
                *duplicated += 1;
            }
            write_pretty(&file_path, &api)?;
        } else if let Some(sub) = item.get("item").and_then(|v| v.as_array()) {
            let sub_base = sanitize_filename(&name);
            let sub_base = if sub_base.is_empty() {
                "子分组".to_string()
            } else {
                sub_base
            };
            let sub_dir = unique_path(dir, &sub_base, "");
            fs::create_dir_all(&sub_dir).map_err(|e| format!("创建分组失败: {e}"))?;
            write_pretty(
                &sub_dir.join(INFO_FILE),
                &InfoJson {
                    name: Some(name.clone()),
                    description: String::new(),
                    base_url: None,
                    mock_port: None,
                    order: None,
                    collapsed: None,
                    deprecated: None,
                },
            )?;
            import_postman_items(&sub_dir, sub, stats, failed, duplicated)?;
        }
    }
    Ok(())
}

/// 将 Postman request 对象转换为 ApiFile
fn postman_request_to_api(name: &str, request: &Value) -> Result<ApiFile, String> {
    let method = request
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET")
        .to_uppercase();
    let url = request.get("url").unwrap_or(&Value::Null);
    let url_raw = url
        .get("raw")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let (path, params) = postman_path_info(url);

    // 接口协议分类：WebSocket（method=WS 或 ws://） / GraphQL（body.mode=graphql） / Socket.IO（method=SOCKET.IO 或 socket.io://） / HTTP
    let body_mode = request
        .pointer("/body/mode")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    let url_lower = url_raw.to_lowercase();
    let protocol = if method == "WS"
        || url_lower.starts_with("ws://")
        || url_lower.starts_with("wss://")
    {
        "websocket".to_string()
    } else if body_mode == "graphql" {
        "graphql".to_string()
    } else if method == "SOCKET.IO"
        || url_lower.starts_with("socket.io://")
        || url_lower.starts_with("socketio://")
    {
        "socketio".to_string()
    } else {
        "http".to_string()
    };

    let mut headers = Vec::new();
    if let Some(arr) = request.pointer("/header").and_then(|v| v.as_array()) {
        for h in arr {
            let key = h
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if key.is_empty() {
                continue;
            }
            headers.push(KeyValue {
                key,
                value: h.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                enabled: !h.get("disabled").and_then(|v| v.as_bool()).unwrap_or(false),
                is_file: false,
                description: String::new(),
            });
        }
    }

    let mut query = Vec::new();
    if let Some(arr) = url.get("query").and_then(|v| v.as_array()) {
        for q in arr {
            let key = q
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if key.is_empty() {
                continue;
            }
            query.push(KeyValue {
                key,
                value: q
                    .get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                enabled: !q.get("disabled").and_then(|v| v.as_bool()).unwrap_or(false),
                is_file: false,
                description: String::new(),
            });
        }
    }

    let body = postman_body(request.get("body"));
    let description = request
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        method,
        path,
        url: url_raw,
        description,
        headers,
        query,
        params,
        body,
        mock: MockConfig::default(),
        prescript: String::new(),
        examples: vec![],
        responses: vec![],
        doc_params: vec![],
        deprecated: false,
        protocol,
    })
}

/// 从 Postman URL 中提取路径（:id 转为 {id}）与路径参数
fn postman_path_info(url: &Value) -> (String, Vec<KeyValue>) {
    let raw = url.get("raw").and_then(|v| v.as_str()).unwrap_or("");
    let no_query = raw.split('?').next().unwrap_or("");
    let no_hash = no_query.split('#').next().unwrap_or("");
    let after_scheme = match no_hash.splitn(2, "://").nth(1) {
        Some(rest) => rest,
        None => no_hash,
    };
    let path_only = match after_scheme.find('/') {
        Some(i) => &after_scheme[i..],
        None => "/",
    };
    let mut params = Vec::new();
    let segs: Vec<String> = path_only
        .split('/')
        .map(|seg| {
            if let Some(var) = seg.strip_prefix(':') {
                if !var.is_empty() {
                    params.push(KeyValue {
                        key: var.to_string(),
                        value: String::new(),
                        enabled: true,
                        is_file: false,
                        description: String::new(),
                    });
                    return format!("{{{var}}}");
                }
            }
            seg.to_string()
        })
        .collect();
    let mut path = segs.join("/");
    if !path.starts_with('/') {
        path.insert(0, '/');
    }
    if path.is_empty() {
        path = "/".to_string();
    }
    (path, params)
}

/// 转换 Postman body（raw / urlencoded / formdata）
fn postman_body(body: Option<&Value>) -> BodyData {
    let mut out = BodyData::default();
    let Some(body) = body else {
        return out;
    };
    match body.get("mode").and_then(|v| v.as_str()) {
        Some("raw") => {
            let raw = body.get("raw").and_then(|v| v.as_str()).unwrap_or("");
            let trimmed = raw.trim_start();
            out.mode = if trimmed.starts_with('{') || trimmed.starts_with('[') {
                "json".to_string()
            } else {
                "raw".to_string()
            };
            out.raw = raw.to_string();
        }
        Some("urlencoded") | Some("formdata") => {
            out.mode = "form".into();
            let arr = body
                .get("urlencoded")
                .or_else(|| body.get("formdata"))
                .and_then(|v| v.as_array());
            if let Some(arr) = arr {
                for f in arr {
                    let key = f
                        .get("key")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    if key.is_empty() {
                        continue;
                    }
                    // Postman formdata 可带 type: file 表示文件字段
                    let is_file = f.get("type").and_then(|v| v.as_str()).unwrap_or("") == "file";
                    out.form.push(KeyValue {
                        key,
                        value: f
                            .get("value")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        enabled: !f.get("disabled").and_then(|v| v.as_bool()).unwrap_or(false),
                        is_file,
                        description: String::new(),
                    });
                }
            }
        }
        _ => {}
    }
    out
}
