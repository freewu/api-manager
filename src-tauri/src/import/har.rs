//! 由 import.rs 拆分：HAR
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

/// 解析 HAR 文件：log.entries 逐条导入接口，按 host 分小组
pub(crate) fn import_har_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let content = fs::read_to_string(file).map_err(|e| format!("读取文件失败: {e}"))?;
    let json: Value =
        serde_json::from_str(&content).map_err(|e| format!("解析 HAR 失败: {e}"))?;
    let entries = json
        .pointer("/log/entries")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "不是有效的 HAR 文件（缺少 log.entries）".to_string())?;
    let title = json
        .pointer("/log/pages/0/title")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            file.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "HAR 导入".to_string())
        });
    let dir_name = sanitize_filename(&title);
    let dir_name = if dir_name.is_empty() {
        "HAR 导入".to_string()
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
            name: Some(title.clone()),
            description: format!("从 HAR 抓包文件导入（{src_name}）"),
            base_url: None,
            mock_port: None,
            collapsed: None,
            deprecated: None,
            dirs: vec![],
            apis: vec![],
        },
    )?;
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    // 按 host 分组，避免重复建同名 host 目录
    let mut by_host: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    for e in entries {
        let url = e
            .pointer("/request/url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let host = url
            .split("://")
            .nth(1)
            .and_then(|r| r.split('/').next())
            .unwrap_or("")
            .to_string();
        let host = if host.is_empty() {
            "未分类".to_string()
        } else {
            host
        };
        by_host.entry(host).or_default().push(e);
    }
    for (host, list) in &by_host {
        let sub = folder.join(&sanitize_filename(host));
        // 同名 host 分组已存在时复用目录
        if !sub.is_dir() {
            fs::create_dir_all(&sub).map_err(|e| format!("创建分组失败: {e}"))?;
            write_pretty(
                &sub.join(INFO_FILE),
                &InfoJson {
                    name: Some(host.clone()),
                    description: String::new(),
                    base_url: None,
                    mock_port: None,
                    collapsed: None,
                    deprecated: None,
                    dirs: vec![],
                    apis: vec![],
                },
            )?;
        }
        for e in list {
            count += har_entry_to_api(&sub, e, &mut stats)?;
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

/// HAR entry → ApiFile：method/url/headers/queryString/postData，响应存为返回示例
fn har_entry_to_api(dir: &Path, entry: &Value,
    stats: &mut ImportStats) -> Result<usize, String> {
    let req = entry.get("request").and_then(|v| v.as_object()).cloned().unwrap_or_default();
    let method = req
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET")
        .to_uppercase();
    let url = req
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if url.is_empty() || method.is_empty() {
        return Ok(0);
    }
    let (path, params) = extract_path(&url);
    // 请求头：过滤浏览器自动头
    let mut headers = Vec::new();
    if let Some(hs) = req.get("headers").and_then(|v| v.as_array()) {
        for h in hs {
            let key = h
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_lowercase();
            if key.is_empty() || HAR_SKIP_HEADERS.contains(&key.as_str()) {
                continue;
            }
            let value = h.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
            headers.push(KeyValue {
                key,
                value,
                enabled: true,
                is_file: false,
                description: String::new(),
            });
        }
    }
    // 查询参数
    let mut query = Vec::new();
    if let Some(qs) = req.get("queryString").and_then(|v| v.as_array()) {
        for q in qs {
            let key = q.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if key.is_empty() {
                continue;
            }
            let value = q.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
            query.push(KeyValue {
                key,
                value,
                enabled: true,
                is_file: false,
                description: String::new(),
            });
        }
    }
    // 请求体
    let mut body = BodyData::default();
    if let Some(pd) = req.get("postData") {
        let text = pd.get("text").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let mime = pd.get("mimeType").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
        if !text.is_empty() {
            if mime.contains("json") {
                body.mode = "json".into();
            } else if mime.contains("form-urlencoded") || mime.contains("multipart") {
                body.mode = "form".into();
                // urlencoded 表单：a=1&b=2 解析为 form 列表
                for pair in text.split('&') {
                    if let Some((k, v)) = pair.split_once('=') {
                        body.form.push(KeyValue {
                            key: k.trim().to_string(),
                            value: percent_encoding::percent_decode_str(v.trim())
                                .decode_utf8_lossy()
                                .to_string(),
                            enabled: true,
                            is_file: false,
                            description: String::new(),
                        });
                    }
                }
                body.raw = String::new();
            } else {
                body.mode = "raw".into();
            }
            if body.mode != "form" {
                body.raw = text;
            }
        }
    }
    // 响应存为返回示例
    let mut responses = Vec::new();
    if let Some(resp) = entry.get("response") {
        let status = resp.get("status").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
        let content_type = resp
            .pointer("/content/mimeType")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let resp_text = resp
            .pointer("/content/text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if !resp_text.is_empty() {
            let truncated = if resp_text.chars().count() > 100_000 {
                resp_text.chars().take(100_000).collect()
            } else {
                resp_text
            };
            responses.push(ResponseItem {
                id: uuid::Uuid::new_v4().to_string(),
                name: format!("HTTP {status}"),
                status,
                content_type,
                body: truncated,
            });
        }
    }
    let name = format!("{} {}", method, path);
    let file_base = sanitize_filename(&name);
    let file_path = unique_path(dir, &file_base, ".json");
    let api = ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name: name.clone(),
        method: method.clone(),
        path: path.clone(),
        url,
        description: String::new(),
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
        protocol: "http".into(),
    };
    write_pretty(&file_path, &api)?;
        stats.add(&api.protocol);
    Ok(1)
}

// ==================== YApi 导入 ====================
