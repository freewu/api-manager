//! 由 export.rs 拆分：OpenAPI 3.0
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

/// 生成 OpenAPI 3.0 规范 JSON
pub fn to_openapi(title: &str, apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let mut paths: Map<String, Value> = Map::new();
    for (segs, api) in apis {
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
        // 同一路径 + 同一方法重复时追加序号（如 /api/users (2)），保证全部接口都导出
        let mut key = p.clone();
        let mut n = 2;
        while paths.get(&key).and_then(|v| v.get(&method)).is_some() {
            key = format!("{p} ({n})");
            n += 1;
        }
        let entry = paths.entry(key).or_insert_with(|| json!({}));
        let obj = entry.as_object_mut().expect("paths 条目为对象");
        obj.insert(method, openapi_operation(segs, api));
    }
    json!({
        "openapi": "3.0.1",
        "info": {
            "title": if title.trim().is_empty() { "API 文档" } else { title },
            "version": "1.0.0",
            "description": "由 API Manager 导出的 OpenAPI 规范"
        },
        "paths": Value::Object(paths)
    })
}

pub(crate) fn is_valid_method(m: &str) -> bool {
    matches!(m, "get" | "put" | "post" | "delete" | "options" | "head" | "patch" | "trace")
}

fn openapi_operation(segs: &[(String, bool)], api: &ApiFile) -> Value {
    let mut params: Vec<Value> = Vec::new();
    for prm in api.params.iter().filter(|p| !p.key.trim().is_empty()) {
        params.push(json!({
            "name": prm.key,
            "in": "path",
            "required": true,
            "description": prm.description,
            "schema": { "type": "string" }
        }));
    }
    for q in api.query.iter().filter(|q| !q.key.trim().is_empty()) {
        params.push(json!({
            "name": q.key,
            "in": "query",
            "description": q.description,
            "schema": { "type": "string" }
        }));
    }
    for h in api.headers.iter().filter(|h| !h.key.trim().is_empty()) {
        params.push(json!({
            "name": h.key,
            "in": "header",
            "description": h.description,
            "schema": { "type": "string" }
        }));
    }
    let mut responses = Map::new();
    // 优先使用「响应」页签条目（名称 + 状态码 + 示例体）；旧数据回退到 Mock 响应
    for r in &api.responses {
        let status = if r.status > 0 {
            r.status.to_string()
        } else {
            "default".to_string()
        };
        let mut desc = r.name.trim().to_string();
        if desc.is_empty() {
            desc = "响应".to_string();
        }
        let mut content = Map::new();
        if !r.body.trim().is_empty() {
            let example = serde_json::from_str::<Value>(&r.body)
                .unwrap_or_else(|_| Value::String(r.body.clone()));
            content.insert(
                r.content_type.trim().to_string(),
                json!({ "example": example }),
            );
        }
        responses.insert(status, json!({ "description": desc, "content": content }));
    }
    if responses.is_empty() {
        responses.insert(
            "200".to_string(),
            json!({ "description": format!("Mock 响应（状态码 {}）", api.mock.status) }),
        );
    }
    let mut op = json!({
        "summary": api.name,
        "description": api.description,
        "parameters": params,
        "responses": responses
    });
    if !segs.is_empty() {
        let tag = segs
            .iter()
            .map(|(n, dep)| if *dep { format!("{n}（已废弃）") } else { n.clone() })
            .collect::<Vec<_>>()
            .join("/");
        op["tags"] = json!([tag]);
    }
    match api.body.mode.as_str() {
        "json" => {
            let example = serde_json::from_str::<Value>(&api.body.raw).unwrap_or_else(|_| Value::String(api.body.raw.clone()));
            op["requestBody"] = json!({ "content": { "application/json": { "example": example } } });
        }
        "raw" => {
            op["requestBody"] = json!({ "content": { "text/plain": { "example": api.body.raw } } });
        }
        "form" => {
            let mut props = Map::new();
            for f in api.body.form.iter().filter(|f| !f.key.trim().is_empty()) {
                props.insert(
                    f.key.clone(),
                    json!({ "type": "string", "description": f.description }),
                );
            }
            op["requestBody"] = json!({
                "content": {
                    "application/x-www-form-urlencoded": {
                        "schema": { "type": "object", "properties": Value::Object(props) }
                    }
                }
            });
        }
        _ => {}
    }
    op
}

// ==================== Docsify 文档目录 ====================
