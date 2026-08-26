//! 由 import.rs 拆分：Apifox / Apipost
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

/// 解析 Apifox 项目文件：apiCollection + webSocketCollection 全部导入
pub(crate) fn import_apifox_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let content = fs::read_to_string(file).map_err(|e| format!("读取文件失败: {e}"))?;
    let json: Value =
        serde_json::from_str(&content).map_err(|e| format!("解析 JSON 失败: {e}"))?;
    if json.get("apiCollection").is_none() && json.get("webSocketCollection").is_none() {
        return Err("不是有效的 Apifox 项目文件（缺少 apiCollection 字段）".into());
    }
    let coll_name = json
        .pointer("/info/name")
        .and_then(|v| v.as_str())
        .unwrap_or("Apifox 导入")
        .to_string();
    let dir_name = sanitize_filename(&coll_name);
    let dir_name = if dir_name.is_empty() {
        "Apifox 导入".to_string()
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
            description: format!("从 Apifox 项目导入（{src_name}）"),
            base_url: None,
            mock_port: None,
            order: None,
            collapsed: None,
            deprecated: None,
        },
    )?;
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    // HTTP 接口：apiCollection 为数组（可能多个集合）或对象 {items:[...]}
    match json.get("apiCollection") {
        Some(Value::Array(arr)) => {
            for c in arr {
                count += import_apifox_items(&folder, c, &mut stats)?;
            }
        }
        Some(obj) => {
            if let Some(items) = obj.get("items").and_then(|v| v.as_array()) {
                count += import_apifox_items_arr(&folder, items, &mut stats)?;
            }
        }
        _ => {}
    }
    // WebSocket 接口：webSocketCollection（api 无 method，path 即 ws url，消息体在 requestBody.message）
    if let Some(arr) = json.get("webSocketCollection").and_then(|v| v.as_array()) {
        for c in arr {
            count += import_apifox_items(&folder, c, &mut stats)?;
        }
    }
    Ok(OpenApiImportResult {
        folder: folder.to_string_lossy().to_string(),
        count,
        http: stats.http,
        ws: stats.ws,
        graphql: stats.graphql,
        socketio: stats.socketio,
        ..Default::default()
    })
}

/// 单个 Apifox 集合：取 items 递归导入
fn import_apifox_items(dir: &Path, collection: &Value, stats: &mut ImportStats) -> Result<usize, String> {
    let items = collection
        .get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    import_apifox_items_arr(dir, &items, stats)
}

/// Apifox items 递归：带 api 的为接口，带 items 的为分组
fn import_apifox_items_arr(dir: &Path, items: &[Value], stats: &mut ImportStats) -> Result<usize, String> {
    let mut count = 0usize;
    for item in items {
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("未命名")
            .to_string();
        if let Some(api_obj) = item.get("api") {
            let api = apifox_api_to_api(&name, api_obj)?;
            stats.add(&api.protocol);
            let file_base = sanitize_filename(&name);
            let file_base = if file_base.is_empty() {
                "未命名接口".to_string()
            } else {
                file_base
            };
            let file_path = unique_path(dir, &file_base, ".json");
            write_pretty(&file_path, &api)?;
            count += 1;
        } else if let Some(sub) = item.get("items").and_then(|v| v.as_array()) {
            if sub.is_empty() {
                // Apifox 导出的目录占位（如 webSocketCollection 中的空分组）不创建
                continue;
            }
            let sub_base = sanitize_filename(&name);
            let sub_base = if sub_base.is_empty() {
                "子分组".to_string()
            } else {
                sub_base
            };
            let sub_dir = dir.join(&sub_base);
            // 同名分组已存在时复用目录，避免生成大量「xx (2)」重复分组
            if !sub_dir.is_dir() {
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
            }
            count += import_apifox_items_arr(&sub_dir, sub, stats)?;
        }
    }
    Ok(count)
}

/// 将 Apifox api 对象转换为 ApiFile（WebSocket 接口无 method，path 即地址）
fn apifox_api_to_api(name: &str, api_obj: &Value) -> Result<ApiFile, String> {
    let is_ws = api_obj.get("method").is_none() && api_obj.get("path").and_then(|v| v.as_str()).map_or(true, |p| p.contains("ws://") || p.contains("wss://"));
    // 接口协议：优先读 api.protocol（Apifox 导出字段），否则按 is_ws 规则
    let proto = api_obj
        .get("protocol")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    let protocol = if proto.contains("websocket") || proto.contains("ws") {
        "websocket".to_string()
    } else if proto.contains("graphql") {
        "graphql".to_string()
    } else if proto.contains("socket") {
        "socketio".to_string()
    } else if is_ws {
        "websocket".to_string()
    } else {
        "http".to_string()
    };
    let method = if is_ws {
        "WS".to_string()
    } else {
        api_obj
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_uppercase()
    };    let path = api_obj
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("/")
        .to_string();
    let (mut headers, mut query, mut params) = (Vec::new(), Vec::new(), Vec::new());
    if let Some(p) = api_obj.get("parameters").and_then(|v| v.as_object()) {
        let kv_list = |arr: Option<&Vec<Value>>| -> Vec<KeyValue> {
            arr.map(|arr| {
                arr.iter()
                    .map(|v| KeyValue {
                        key: v.get("name").and_then(|x| x.as_str()).unwrap_or("").trim().to_string(),
                        value: v.get("value").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                        enabled: v.get("enable").and_then(|x| x.as_bool()).unwrap_or(true),
                        description: v.get("description").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                        is_file: false,
                    })
                    .collect()
            })
            .unwrap_or_default()
        };
        headers = kv_list(p.get("header").and_then(|v| v.as_array()));
        query = kv_list(p.get("query").and_then(|v| v.as_array()));
        params = kv_list(p.get("path").and_then(|v| v.as_array()));
    }
    let body = if is_ws {
        let msg = api_obj
            .pointer("/requestBody/message")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        BodyData {
            mode: if msg.trim().is_empty() {
                "none".into()
            } else if msg.trim_start().starts_with('{') || msg.trim_start().starts_with('[') {
                "json".into()
            } else {
                "raw".into()
            },
            raw: msg,
            form: vec![],
            binary_path: String::new(),
        }
    } else {
        apifox_body(api_obj.get("requestBody"))
    };
    let description = api_obj
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Ok(ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        method,
        url: path.clone(),
        path,
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

/// 转换 Apifox requestBody（type: json / form-data / x-www-form-urlencoded / raw / none）
fn apifox_body(body: Option<&Value>) -> BodyData {
    let mut out = BodyData::default();
    let Some(body) = body else {
        return out;
    };
    let ty = body.get("type").and_then(|v| v.as_str()).unwrap_or("none");
    match ty {
        "json" | "raw" => {
            // JSON 示例体在 examples[].data；raw 直接用 raw
            let raw = if ty == "raw" {
                body.get("raw").and_then(|v| v.as_str()).unwrap_or("").to_string()
            } else {
                body.get("examples")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|e| e.get("data"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            };
            out.mode = if ty == "raw" { "raw".into() } else { "json".into() };
            out.raw = raw;
        }
        "form-data" | "x-www-form-urlencoded" => {
            out.mode = "form".into();
            if let Some(arr) = body.get("parameters").and_then(|v| v.as_array()) {
                for f in arr {
                    let key = f.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                    if key.is_empty() {
                        continue;
                    }
                    out.form.push(KeyValue {
                        key,
                        value: f.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        enabled: f.get("enable").and_then(|v| v.as_bool()).unwrap_or(true),
                        is_file: f.get("type").and_then(|v| v.as_str()).unwrap_or("") == "file",
                        description: String::new(),
                    });
                }
            }
        }
        _ => {}
    }
    out
}

/// 解析 Apipost 项目文件：apis 平铺数组按 parent_id 组织成树
pub(crate) fn import_apipost_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let content = fs::read_to_string(file).map_err(|e| format!("读取文件失败: {e}"))?;
    let json: Value =
        serde_json::from_str(&content).map_err(|e| format!("解析 JSON 失败: {e}"))?;
    let apis = json.get("apis").and_then(|v| v.as_array()).cloned();
    let Some(apis) = apis else {
        return Err("不是有效的 Apipost 项目文件（缺少 apis 字段）".into());
    };
    let coll_name = json
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Apipost 导入")
        .to_string();
    let dir_name = sanitize_filename(&coll_name);
    let dir_name = if dir_name.is_empty() {
        "Apipost 导入".to_string()
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
            description: format!("从 Apipost 项目导入（{src_name}）"),
            base_url: None,
            mock_port: None,
            order: None,
            collapsed: None,
            deprecated: None,
        },
    )?;
    // target_id → 节点索引
    let mut by_id: HashMap<String, &Value> = HashMap::new();
    for a in &apis {
        if let Some(id) = a.get("target_id").and_then(|v| v.as_str()) {
            by_id.insert(id.to_string(), a);
        }
    }
    // 根节点：parent_id 为 0 或指向不存在的节点
    let mut roots: Vec<&Value> = apis
        .iter()
        .filter(|a| {
            let pid = a.get("parent_id").and_then(|v| v.as_str()).unwrap_or("0");
            pid == "0" || !by_id.contains_key(pid)
        })
        .collect();
    roots.sort_by_key(|a| a.get("sort").and_then(|v| v.as_i64()).unwrap_or(0));
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    for r in roots {
        count += import_apipost_node(&folder, r, &by_id, &mut stats)?;
    }
    Ok(OpenApiImportResult {
        folder: folder.to_string_lossy().to_string(),
        count,
        http: stats.http,
        ws: stats.ws,
        graphql: stats.graphql,
        socketio: stats.socketio,
        ..Default::default()
    })
}

/// Apipost 节点递归：folder 建分组，api/graphql 写接口文件
fn import_apipost_node(
    dir: &Path,
    node: &Value,
    by_id: &HashMap<String, &Value>,
    stats: &mut ImportStats,
) -> Result<usize, String> {
    let name = node
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("未命名")
        .to_string();
    match node.get("target_type").and_then(|v| v.as_str()) {
        Some("folder") => {
            let kids = apipost_node_children(node, by_id);
            if kids.is_empty() {
                // 空文件夹不创建，避免生成空分组
                return Ok(0);
            }
            let sub_dir = dir.join(&sanitize_filename(&name));
            // 同名分组已存在时复用目录（Apipost 树中可能多个同名文件夹）
            if !sub_dir.is_dir() {
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
            }
            let mut count = 0usize;
            for c in kids {
                count += import_apipost_node(&sub_dir, c, by_id, stats)?;
            }
            Ok(count)
        }
        _ => {
            let api = apipost_request_to_api(&name, node)?;
            stats.add(&api.protocol);
            let file_base = sanitize_filename(&name);
            let file_base = if file_base.is_empty() {
                "未命名接口".to_string()
            } else {
                file_base
            };
            let file_path = unique_path(dir, &file_base, ".json");
            write_pretty(&file_path, &api)?;
            Ok(1)
        }
    }
}

/// 取某节点的直接子节点（按 sort 排序）
fn apipost_node_children<'a>(node: &Value, by_id: &HashMap<String, &'a Value>) -> Vec<&'a Value> {
    let id = node.get("target_id").and_then(|v| v.as_str()).unwrap_or("");
    let mut kids: Vec<&'a Value> = by_id
        .values()
        .filter(|v| v.get("parent_id").and_then(|x| x.as_str()).unwrap_or("") == id)
        .copied()
        .collect();
    kids.sort_by_key(|v| v.get("sort").and_then(|x| x.as_i64()).unwrap_or(0));
    kids
}

/// 将 Apipost 接口节点转换为 ApiFile
fn apipost_request_to_api(name: &str, node: &Value) -> Result<ApiFile, String> {
    let method = node
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET")
        .to_uppercase();
    let url = node.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let protocol = node.get("protocol").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let proto_lower = protocol.to_lowercase();
    // 接口协议分类：优先读 protocol 字段（http/websocket/graphql/socket.io 等）
    let protocol = if proto_lower.contains("websocket") || proto_lower == "ws" {
        "websocket".to_string()
    } else if proto_lower.contains("graphql") {
        "graphql".to_string()
    } else if proto_lower.contains("socket") {
        "socketio".to_string()
    } else {
        "http".to_string()
    };
    let (path, mut params) = extract_path(&url);
    let request = node.get("request");
    let mut headers = Vec::new();
    let mut query = Vec::new();
    if let Some(req) = request {
        headers = apipost_param_list(
            req.get("header")
                .and_then(|h| h.get("parameter"))
                .and_then(|v| v.as_array()),
        );
        query = apipost_param_list(
            req.get("query")
                .and_then(|h| h.get("parameter"))
                .and_then(|v| v.as_array()),
        );
        let restful = apipost_param_list(
            req.get("restful")
                .and_then(|h| h.get("parameter"))
                .and_then(|v| v.as_array()),
        );
        if !restful.is_empty() {
            params = restful;
        }
    }
    let body = request
        .map(|r| apipost_body(r.get("body")))
        .unwrap_or_default();
    let description = node
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Ok(ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        method,
        path,
        url,
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

/// 读取 Apipost 参数数组（{key, value, description, is_checked}）
fn apipost_param_list(arr: Option<&Vec<Value>>) -> Vec<KeyValue> {
    arr.map(|arr| {
        arr.iter()
            .map(|v| KeyValue {
                key: v.get("key").and_then(|x| x.as_str()).unwrap_or("").trim().to_string(),
                value: v.get("value").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                enabled: v.get("is_checked").and_then(|x| x.as_i64()).unwrap_or(1) != 0,
                description: v.get("description").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                is_file: false,
            })
            .collect()
    })
    .unwrap_or_default()
}

/// 转换 Apipost body（mode: json / raw / form-data / urlencoded / none）
fn apipost_body(body: Option<&Value>) -> BodyData {
    let mut out = BodyData::default();
    let Some(body) = body else {
        return out;
    };
    match body.get("mode").and_then(|v| v.as_str()) {
        Some("json") | Some("raw") | Some("xml") => {
            out.mode = if body.get("mode").and_then(|v| v.as_str()) == Some("xml") {
                "raw".to_string()
            } else {
                "json".to_string()
            };
            out.raw = body.get("raw").and_then(|v| v.as_str()).unwrap_or("").to_string();
        }
        Some("form-data") | Some("urlencoded") | Some("form") => {
            out.mode = "form".into();
            if let Some(arr) = body.get("parameter").and_then(|v| v.as_array()) {
                for f in arr {
                    let key = f.get("key").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                    if key.is_empty() {
                        continue;
                    }
                    out.form.push(KeyValue {
                        key,
                        value: f.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        enabled: f.get("is_checked").and_then(|v| v.as_i64()).unwrap_or(1) != 0,
                        is_file: f.get("type").and_then(|v| v.as_str()).unwrap_or("") == "file",
                        description: String::new(),
                    });
                }
            }
        }
        _ => {}
    }
    out
}
