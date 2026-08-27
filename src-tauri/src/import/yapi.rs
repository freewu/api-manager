//! 由 import.rs 拆分：YApi
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

/// 解析 YApi 导出文件：Swagger 结构走 openapi 导入；否则按原生树（name/children/api）导入
pub(crate) fn import_yapi_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let content = fs::read_to_string(file).map_err(|e| format!("读取文件失败: {e}"))?;
    let json: Value =
        serde_json::from_str(&content).map_err(|e| format!("解析 YApi 文件失败: {e}"))?;
    // Swagger / OpenAPI 结构：YApi 的 swagger 数据导出
    if json.get("swagger").is_some() || json.get("openapi").is_some() {
        return import_openapi_file(root, file);
    }
    // YApi 原生导出：顶层为分组数组（name/children/api）
    let arr = json.as_array().ok_or_else(|| {
        "不是有效的 YApi 文件（既非 Swagger 也非分组树结构）".to_string()
    })?;
    let dir_name = "YApi 导入".to_string();
    let folder = unique_path(root, &dir_name, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    let src_name = file
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some("YApi 导入".into()),
            description: format!("从 YApi 导出文件导入（{src_name}）"),
            base_url: None,
            mock_port: None,
            order: None,
            collapsed: None,
            deprecated: None,
        },
    )?;
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    for g in arr {
        count += yapi_node_to_apis(&folder, g, &mut stats)?;
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

/// YApi 节点递归：有 children 建分组，有 api 写接口文件
fn yapi_node_to_apis(dir: &Path, node: &Value, stats: &mut ImportStats) -> Result<usize, String> {
    let name = node
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("未命名")
        .to_string();
    if let Some(api) = node.get("api") {
        return yapi_api_to_api(dir, &name, api, stats);
    }
    let kids = node.get("children").and_then(|v| v.as_array());
    let Some(kids) = kids else {
        // 既无 api 也无 children 的占位节点，跳过
        return Ok(0);
    };
    if kids.is_empty() {
        return Ok(0); // 空分组不创建
    }
    let sub_base = sanitize_filename(&name);
    let sub_base = if sub_base.is_empty() {
        "子分组".to_string()
    } else {
        sub_base
    };
    let sub_dir = dir.join(&sub_base);
    // 同名分组已存在时复用目录
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
        count += yapi_node_to_apis(&sub_dir, c, stats)?;
    }
    Ok(count)
}

/// YApi api 对象 → ApiFile
fn yapi_api_to_api(dir: &Path, title: &str, api: &Value,
    stats: &mut ImportStats) -> Result<usize, String> {
    let method = api
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET")
        .to_uppercase();
    let raw_path = api.get("path").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    if raw_path.is_empty() {
        return Ok(0);
    }
    let protocol = match api.get("protocol").and_then(|v| v.as_str()) {
        Some(p) if p.eq_ignore_ascii_case("ws") || p.eq_ignore_ascii_case("websocket") => {
            "websocket".to_string()
        }
        _ => "http".to_string(),
    };
    let (path, params) = if protocol == "websocket" {
        // WS 地址保留完整 ws:// 前缀
        (raw_path.clone(), Vec::new())
    } else {
        extract_path(&raw_path)
    };
    // 请求头
    let mut headers = Vec::new();
    if let Some(hs) = api.get("req_headers").and_then(|v| v.as_array()) {
        for h in hs {
            let key = h.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if key.is_empty() {
                continue;
            }
            let value = h.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let desc = h.get("desc").and_then(|v| v.as_str()).unwrap_or("").to_string();
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
    if let Some(qs) = api.get("req_query").and_then(|v| v.as_array()) {
        for q in qs {
            let key = q.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if key.is_empty() {
                continue;
            }
            let value = q
                .get("example")
                .and_then(|v| v.as_str())
                .or_else(|| q.get("value").and_then(|v| v.as_str()))
                .unwrap_or("")
                .to_string();
            let desc = q
                .get("desc")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
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
    let body_type = api.get("req_body_type").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
    match body_type.as_str() {
        "json" => {
            if let Some(other) = api.get("req_body_other").and_then(|v| v.as_str()) {
                let trimmed = other.trim();
                if !trimmed.is_empty() {
                    // 尝试格式化 JSON
                    if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
                        body.raw = serde_json::to_string_pretty(&v).unwrap_or_else(|_| trimmed.to_string());
                    } else {
                        body.raw = trimmed.to_string();
                    }
                    body.mode = "json".into();
                }
            }
        }
        "form" => {
            let mut form = Vec::new();
            if let Some(fs) = api.get("req_body_form").and_then(|v| v.as_array()) {
                for f in fs {
                    let key = f.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                    if key.is_empty() {
                        continue;
                    }
                    let value = f.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let desc = f.get("desc").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let is_file = f.get("type").and_then(|v| v.as_str()).unwrap_or("") == "file";
                    form.push(KeyValue {
                        key,
                        value,
                        enabled: true,
                        is_file,
                        description: desc,
                    });
                }
            }
            if !form.is_empty() {
                body.mode = "form".into();
                body.form = form;
            }
        }
        "raw" | "text" | "file" => {
            if let Some(other) = api.get("req_body_other").and_then(|v| v.as_str()) {
                let trimmed = other.trim();
                if !trimmed.is_empty() {
                    body.mode = "raw".into();
                    body.raw = trimmed.to_string();
                }
            }
        }
        _ => {}
    }
    // 响应示例
    let mut responses = Vec::new();
    if let Some(res) = api.get("res_body").and_then(|v| v.as_str()) {
        let trimmed = res.trim();
        if !trimmed.is_empty() {
            let content_type = match api.get("res_body_type").and_then(|v| v.as_str()) {
                Some(t) if t.eq_ignore_ascii_case("json") => "application/json".to_string(),
                _ => "text/plain".to_string(),
            };
            responses.push(ResponseItem {
                id: uuid::Uuid::new_v4().to_string(),
                name: "HTTP 200".into(),
                status: 200,
                content_type,
                body: trimmed.to_string(),
            });
        }
    }
    let desc = api.get("desc").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let name = if title.trim().is_empty() {
        format!("{} {}", method, path)
    } else {
        title.trim().to_string()
    };
    let file_base = sanitize_filename(&name);
    let file_path = unique_path(dir, &file_base, ".json");
    let api_file = ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name: name.clone(),
        method: method.clone(),
        path: path.clone(),
        url: path.clone(),
        description: desc,
        headers,
        query,
        params,
        body,
        mock: MockConfig::default(),
        prescript: String::new(),
        examples: vec![],
        responses,
        doc_params: vec![],
        deprecated: false,
        protocol,
        order: None,
    };
    write_pretty(&file_path, &api_file)?;
        stats.add(&api_file.protocol);
    Ok(1)
}

