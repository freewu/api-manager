//! 由 import.rs 拆分：Insomnia
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

pub(crate) fn import_insomnia_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let content = fs::read_to_string(file).map_err(|e| format!("读取文件失败: {e}"))?;
    let doc: Value = serde_yaml::from_str(&content).map_err(|e| format!("解析 Insomnia 文件失败: {e}"))?;
    let coll_name = doc
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Insomnia 导入")
        .to_string();
    let folder = unique_path(root, &coll_name, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    // 集合级环境变量 baseUrl
    let base_url = doc
        .get("environment")
        .and_then(|e| e.get("baseUrl"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let src_name = file
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some(coll_name.clone()),
            description: format!("从 Insomnia 导出文件导入（{src_name}）"),
            base_url,
            mock_port: None,
            order: None,
            collapsed: None,
            deprecated: None,
        },
    )?;
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    let coll_env = doc
        .get("environment")
        .cloned()
        .unwrap_or(Value::Null);
    if let Some(children) = doc.get("children").and_then(|v| v.as_array()) {
        for c in children {
            count += insomnia_node_to_apis(&folder, c, &coll_env, &mut stats)?;
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

/// Insomnia 节点递归：有 url/method 的是请求，否则是文件夹
fn insomnia_node_to_apis(dir: &Path, node: &Value, coll_env: &Value,
    stats: &mut ImportStats) -> Result<usize, String> {
    let name = node
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("未命名")
        .to_string();
    let url = node
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let method = node
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if !url.is_empty() && !method.is_empty() {
        return insomnia_request_to_api(dir, &name, node, &url, &method, coll_env, stats);
    }
    let kids = node.get("children").and_then(|v| v.as_array());
    let Some(kids) = kids else {
        return Ok(0);
    };
    if kids.is_empty() {
        return Ok(0); // 空文件夹不创建
    }
    let sub_base = sanitize_filename(&name);
    let sub_base = if sub_base.is_empty() {
        "子文件夹".to_string()
    } else {
        sub_base
    };
    let sub_dir = dir.join(&sub_base);
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
        count += insomnia_node_to_apis(&sub_dir, c, coll_env, stats)?;
    }
    Ok(count)
}

/// Insomnia 请求对象 → ApiFile
fn insomnia_request_to_api(
    dir: &Path,
    name: &str,
    node: &Value,
    url: &str,
    method: &str,
    coll_env: &Value,

    stats: &mut ImportStats) -> Result<usize, String> {
    let protocol = if url.starts_with("ws://") || url.starts_with("wss://") {
        "websocket".to_string()
    } else {
        "http".to_string()
    };
    // {{baseUrl}} 等模板变量替换（节点级环境优先，集合级兜底）
    let mut env_map: Vec<(String, String)> = Vec::new();
    let mut collect_env = |env: &Value| {
        if let Some(m) = env.as_object() {
            for (k, v) in m {
                if let Some(s) = v.as_str() {
                    env_map.push((k.clone(), s.to_string()));
                }
            }
        }
    };
    if let Some(env) = node.get("environment") {
        collect_env(env);
    }
    collect_env(coll_env);
    let replace_vars = |s: &str| -> String {
        let mut out = s.to_string();
        for (k, v) in &env_map {
            out = out.replace(&format!("{{{{{k}}}}}"), v);
        }
        out
    };
    let url2 = replace_vars(url);
    let (path, params) = if protocol == "websocket" {
        (url2.clone(), Vec::new())
    } else {
        extract_path(&url2)
    };
    // 请求头
    let mut headers = Vec::new();
    if let Some(hs) = node.get("headers").and_then(|v| v.as_array()) {
        for h in hs {
            let key = h.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if key.is_empty() {
                continue;
            }
            let value = h.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let desc = h.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
            headers.push(KeyValue {
                key,
                value,
                enabled: true,
                is_file: false,
                description: desc,
            });
        }
    }
    // 查询参数
    let mut query = Vec::new();
    if let Some(ps) = node.get("parameters").and_then(|v| v.as_array()) {
        for p in ps {
            let key = p.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if key.is_empty() {
                continue;
            }
            let value = p.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let desc = p.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
            query.push(KeyValue {
                key,
                value,
                enabled: true,
                is_file: false,
                description: desc,
            });
        }
    }
    // 请求体
    let mut body = BodyData::default();
    if let Some(b) = node.get("body") {
        let mime = b.get("mimeType").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
        let text = b.get("text").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        if mime.contains("json") && !text.is_empty() {
            body.mode = "json".into();
            body.raw = text;
        } else if (mime.contains("form") || mime.contains("urlencoded")) && !text.is_empty() {
            // x-www-form-urlencoded → 表单字段
            let mut form = Vec::new();
            for pair in text.split('&') {
                let mut it = pair.splitn(2, '=');
                let key = it.next().unwrap_or("").trim().to_string();
                if key.is_empty() {
                    continue;
                }
                let value = it.next().unwrap_or("").to_string();
                form.push(KeyValue {
                    key,
                    value,
                    enabled: true,
                    is_file: false,
                    description: String::new(),
                });
            }
            if !form.is_empty() {
                body.mode = "form".into();
                body.form = form;
            }
        } else if !text.is_empty() {
            body.mode = "raw".into();
            body.raw = text;
        }
    }
    // 鉴权：bearer → Authorization 头
    if let Some(auth) = node.get("authentication") {
        let atype = auth.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if atype == "bearer" {
            if let Some(token) = auth.get("token").and_then(|v| v.as_str()) {
                let token = replace_vars(token);
                if !headers.iter().any(|h| h.key.eq_ignore_ascii_case("authorization")) {
                    headers.push(KeyValue {
                        key: "Authorization".into(),
                        value: format!("Bearer {token}"),
                        enabled: true,
                        is_file: false,
                        description: String::new(),
                    });
                }
            }
        }
    }
    let api_name = if name.trim().is_empty() {
        format!("{} {}", method.to_uppercase(), path)
    } else {
        name.trim().to_string()
    };
    let file_base = sanitize_filename(&api_name);
    let file_path = unique_path(dir, &file_base, ".json");
    let api_file = ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name: api_name.clone(),
        method: method.to_uppercase(),
        path: path.clone(),
        url: path.clone(),
        description: node
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        headers,
        query,
        params,
        body,
        mock: MockConfig::default(),
        examples: vec![],
        responses: vec![],
        doc_params: vec![],
        deprecated: false,
        protocol,
    };
    write_pretty(&file_path, &api_file)?;
        stats.add(&api_file.protocol);
    Ok(1)
}
