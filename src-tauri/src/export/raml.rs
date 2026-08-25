//! 由 export.rs 拆分：RAML
use super::openapi::is_valid_method;
#[allow(unused_imports)]
use crate::{read_api, read_info_file, sanitize_filename, ApiFile, BodyData, KeyValue, MockConfig, ENV_FILE, INFO_FILE};
#[allow(unused_imports)]
use serde_json::{json, Map, Value};
#[allow(unused_imports)]
use std::collections::BTreeMap;
#[allow(unused_imports)]
use std::fs;
#[allow(unused_imports)]
use std::path::{Path, PathBuf};

/// 生成 RAML 1.0 文档（YAML Value，调用方用 serde_yaml 序列化并拼接 #%RAML 1.0 头）
pub fn to_raml(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let mut paths: Map<String, Value> = Map::new();
    let mut base_uri = String::new();
    for (_, api) in apis {
        if api.protocol == "websocket" {
            continue;
        }
        let p = if !api.path.trim().is_empty() {
            api.path.trim().to_string()
        } else {
            api.url.trim().to_string()
        };
        if p.is_empty() {
            continue;
        }
        let method = api.method.trim().to_lowercase();
        if !is_valid_method(&method) {
            continue;
        }
        if base_uri.is_empty() {
            base_uri = extract_base_url(&api.url);
        }
        let entry = paths.entry(p.clone()).or_insert_with(|| json!({}));
        let obj = entry.as_object_mut().expect("paths 条目为对象");
        obj.insert(method.clone(), api_to_raml_method(api));
    }
    let mut doc = Map::new();
    doc.insert("title".into(), json!("API Manager 导出"));
    if !base_uri.is_empty() {
        doc.insert("baseUri".into(), json!(base_uri));
    }
    doc.insert("mediaType".into(), json!("application/json"));
    for (k, v) in paths {
        doc.insert(k, v);
    }
    json!(doc)
}

/// 单个接口 → RAML method 对象
fn api_to_raml_method(api: &ApiFile) -> Value {
    let mut op = Map::new();
    if !api.description.trim().is_empty() {
        op.insert("description".into(), json!(api.description));
    }
    let query: Map<String, Value> = api
        .query
        .iter()
        .filter(|q| q.enabled && !q.key.trim().is_empty())
        .map(|q| {
            let mut p = Map::new();
            p.insert("type".into(), json!("string"));
            p.insert("required".into(), json!(false));
            if !q.value.is_empty() {
                p.insert("default".into(), json!(q.value));
            }
            if !q.description.is_empty() {
                p.insert("description".into(), json!(q.description));
            }
            (q.key.clone(), json!(p))
        })
        .collect();
    if !query.is_empty() {
        op.insert("queryParameters".into(), json!(query));
    }
    let headers: Map<String, Value> = api
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .map(|h| {
            let mut p = Map::new();
            p.insert("type".into(), json!("string"));
            p.insert("required".into(), json!(false));
            if !h.description.is_empty() {
                p.insert("description".into(), json!(h.description));
            }
            (h.key.clone(), json!(p))
        })
        .collect();
    if !headers.is_empty() {
        op.insert("headers".into(), json!(headers));
    }
    if matches!(api.body.mode.as_str(), "json" | "raw") && !api.body.raw.trim().is_empty() {
        op.insert(
            "body".into(),
            json!({ "application/json": { "example": api.body.raw } }),
        );
    }
    json!(op)
}

/// 从 URL 提取 base（scheme://host[:port]，去掉路径）
pub(crate) fn extract_base_url(raw: &str) -> String {
    let no_q = raw.split(['?', '#']).next().unwrap_or("");
    let Some((scheme, rest)) = no_q.split_once("://") else {
        return String::new();
    };
    let host = rest.split('/').next().unwrap_or("");
    if host.is_empty() {
        return String::new();
    }
    format!("{scheme}://{host}")
}

// ==================== WADL 导出 ====================
