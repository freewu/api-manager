//! 由 import.rs 拆分：Eolink
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

pub(crate) fn import_eolink_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let content = fs::read_to_string(file).map_err(|e| format!("读取文件失败: {e}"))?;
    let json: Value =
        serde_json::from_str(&content).map_err(|e| format!("解析 Eolink 文件失败: {e}"))?;
    let project = json.get("projectInfo").cloned().unwrap_or(Value::Null);
    let project_name = project
        .get("projectName")
        .and_then(|v| v.as_str())
        .unwrap_or("Eolink 导入")
        .to_string();
    let folder = unique_path(root, &project_name, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    let src_name = file
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    // 环境 host 作为 base_url
    let env_host = json
        .get("environmentList")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|e| e.get("envHost"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let base_url = if env_host.is_empty() {
        None
    } else {
        Some(env_host)
    };
    let desc = project
        .get("projectDesc")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some(project_name.clone()),
            description: if desc.is_empty() {
                format!("从 Eolink 导出文件导入（{src_name}）")
            } else {
                desc
            },
            base_url,
            mock_port: None,
            collapsed: None,
            deprecated: None,
            dirs: vec![],
            apis: vec![],
        },
    )?;
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    if let Some(groups) = json.get("apiGroupList").and_then(|v| v.as_array()) {
        for g in groups {
            count += eolink_group_to_apis(&folder, g, &mut stats)?;
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

/// Eolink 分组递归：本组建目录，apiList 写入本组，childGroupList 递归子目录
fn eolink_group_to_apis(dir: &Path, group: &Value,
    stats: &mut ImportStats) -> Result<usize, String> {
    let name = group
        .get("groupName")
        .and_then(|v| v.as_str())
        .unwrap_or("未命名分组")
        .to_string();
    let sub_base = sanitize_filename(&name);
    let sub_base = if sub_base.is_empty() {
        "子分组".to_string()
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
                collapsed: None,
                deprecated: None,
                dirs: vec![],
                apis: vec![],
            },
        )?;
    }
    let mut count = 0usize;
    if let Some(apis) = group.get("apiList").and_then(|v| v.as_array()) {
        for a in apis {
            count += eolink_api_to_api(&sub_dir, a, stats)?;
        }
    }
    if let Some(children) = group.get("childGroupList").and_then(|v| v.as_array()) {
        for c in children {
            count += eolink_group_to_apis(&sub_dir, c, stats)?;
        }
    }
    Ok(count)
}

/// Eolink API 对象 → ApiFile
fn eolink_api_to_api(dir: &Path, api: &Value,
    stats: &mut ImportStats) -> Result<usize, String> {
    let method = api
        .get("apiMethod")
        .and_then(|v| v.as_str())
        .unwrap_or("GET")
        .to_uppercase();
    let raw_uri = api
        .get("apiUri")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if raw_uri.is_empty() {
        return Ok(0);
    }
    let protocol = match api.get("apiProtocol").and_then(|v| v.as_str()) {
        Some(p) if p.eq_ignore_ascii_case("ws") || p.eq_ignore_ascii_case("websocket") => {
            "websocket".to_string()
        }
        _ => "http".to_string(),
    };
    let (path, mut params) = if protocol == "websocket" {
        (raw_uri.clone(), Vec::new())
    } else {
        extract_path(&raw_uri)
    };
    let info = api.get("requestInfo").cloned().unwrap_or(Value::Null);
    // 请求头
    let mut headers = Vec::new();
    if let Some(hs) = info.get("requestHeaderList").and_then(|v| v.as_array()) {
        for h in hs {
            let key = h.get("key").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if key.is_empty() {
                continue;
            }
            let value = h
                .get("example")
                .and_then(|v| v.as_str())
                .or_else(|| h.get("value").and_then(|v| v.as_str()))
                .unwrap_or("")
                .to_string();
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
    if let Some(qs) = info.get("requestQueryList").and_then(|v| v.as_array()) {
        for q in qs {
            let key = q.get("key").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if key.is_empty() {
                continue;
            }
            let value = q
                .get("example")
                .and_then(|v| v.as_str())
                .or_else(|| q.get("value").and_then(|v| v.as_str()))
                .unwrap_or("")
                .to_string();
            let desc = q.get("desc").and_then(|v| v.as_str()).unwrap_or("").to_string();
            query.push(KeyValue {
                key,
                value,
                enabled: true,
                is_file: false,
                description: desc,
            });
        }
    }
    // 路径参数（requestRestList）补充 value/desc
    if let Some(rs) = info.get("requestRestList").and_then(|v| v.as_array()) {
        for r in rs {
            let key = r.get("key").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if key.is_empty() {
                continue;
            }
            let value = r
                .get("example")
                .and_then(|v| v.as_str())
                .or_else(|| r.get("value").and_then(|v| v.as_str()))
                .unwrap_or("")
                .to_string();
            let desc = r.get("desc").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if let Some(p) = params.iter_mut().find(|p| p.key == key) {
                p.value = value;
                p.description = desc;
            } else {
                params.push(KeyValue {
                    key,
                    value,
                    enabled: true,
                    is_file: false,
                    description: desc,
                });
            }
        }
    }
    // 请求体
    let mut body = BodyData::default();
    let body_type = info
        .get("requestBodyType")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    match body_type.as_str() {
        "json" => {
            if let Some(list) = info.get("requestBodyJsonList").and_then(|v| v.as_array()) {
                let v = eolink_json_list_to_value(list);
                if v.as_object().map(|m| !m.is_empty()).unwrap_or(false) {
                    body.mode = "json".into();
                    body.raw = serde_json::to_string_pretty(&v).unwrap_or_default();
                }
            }
        }
        "x-www-form-urlencoded" | "form" => {
            if let Some(fl) = info.get("requestBodyFormList").and_then(|v| v.as_array()) {
                let mut form = Vec::new();
                for f in fl {
                    let key = f.get("key").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                    if key.is_empty() {
                        continue;
                    }
                    let value = f
                        .get("example")
                        .and_then(|v| v.as_str())
                        .or_else(|| f.get("value").and_then(|v| v.as_str()))
                        .unwrap_or("")
                        .to_string();
                    let desc = f.get("desc").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let is_file = f
                        .get("type")
                        .and_then(|v| v.as_str())
                        .map(|t| t.eq_ignore_ascii_case("file"))
                        .unwrap_or(false);
                    form.push(KeyValue {
                        key,
                        value,
                        enabled: true,
                        is_file,
                        description: desc,
                    });
                }
                if !form.is_empty() {
                    body.mode = "form".into();
                    body.form = form;
                }
            }
        }
        "raw" | "text" => {
            if let Some(text) = info.get("requestBodyRaw").and_then(|v| v.as_str()) {
                let trimmed = text.trim();
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
    if let Some(list) = api.get("responseInfoList").and_then(|v| v.as_array()) {
        for r in list {
            let status = r
                .get("responseCode")
                .and_then(|v| v.as_u64())
                .unwrap_or(200)
                .min(u64::from(u16::MAX)) as u16;
            let content_type = match r
                .get("responseContentType")
                .and_then(|v| v.as_str())
                .unwrap_or("")
            {
                s if s.eq_ignore_ascii_case("json") => "application/json".to_string(),
                s if s.eq_ignore_ascii_case("xml") => "application/xml".to_string(),
                s if s.eq_ignore_ascii_case("html") => "text/html".to_string(),
                s if s.eq_ignore_ascii_case("text") => "text/plain".to_string(),
                _ => "application/json".to_string(),
            };
            let mut rbody = String::new();
            if let Some(rl) = r.get("responseBodyJsonList").and_then(|v| v.as_array()) {
                let v = eolink_json_list_to_value(rl);
                if v.as_object().map(|m| !m.is_empty()).unwrap_or(false) {
                    rbody = serde_json::to_string_pretty(&v).unwrap_or_default();
                }
            } else if let Some(text) = r.get("responseBodyRaw").and_then(|v| v.as_str()) {
                rbody = text.trim().to_string();
            }
            let rname = r
                .get("responseName")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if !rbody.is_empty() {
                responses.push(ResponseItem {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: if rname.is_empty() {
                        format!("HTTP {status}")
                    } else {
                        rname
                    },
                    status,
                    content_type,
                    body: rbody,
                });
            }
        }
    }
    let name = api
        .get("apiName")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let name = if name.is_empty() {
        format!("{} {}", method, path)
    } else {
        name
    };
    let mut desc = api
        .get("apiDesc")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if let Some(note) = api.get("apiNote").and_then(|v| v.as_str()) {
        if !note.trim().is_empty() {
            if !desc.is_empty() {
                desc.push('\n');
            }
            desc.push_str(note);
        }
    }
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
    };
    write_pretty(&file_path, &api_file)?;
        stats.add(&api_file.protocol);
    Ok(1)
}

/// Eolink JSON 字段列表（含嵌套 children）→ serde_json Value
fn eolink_json_list_to_value(list: &[Value]) -> Value {
    let mut map = serde_json::Map::new();
    for item in list {
        let key = item.get("key").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        if key.is_empty() {
            continue;
        }
        let kids = item.get("children").and_then(|v| v.as_array());
        let value = if let Some(kids) = kids {
            if kids.is_empty() {
                Value::Null
            } else {
                eolink_json_list_to_value(kids)
            }
        } else {
            match item.get("example") {
                Some(v) => v.clone(),
                None => match item.get("value") {
                    Some(v) => v.clone(),
                    None => Value::Null,
                },
            }
        };
        map.insert(key, value);
    }
    Value::Object(map)
}

// ==================== Insomnia 导入 ====================
