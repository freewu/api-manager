//! 由 export.rs 拆分：Apifox
use super::*;
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

/// 生成 Apifox 项目 JSON（apifox-project.json 结构）
pub fn to_apifox(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let mut root = PNode {
        name: "根目录".to_string(),
        apis: Vec::new(),
        children: BTreeMap::new(),
    };
    for (segs, api) in apis {
        let mut cur = &mut root;
        for (s, _dep) in segs {
            cur = cur
                .children
                .entry(s.clone())
                .or_insert_with(|| PNode {
                    name: s.clone(),
                    apis: Vec::new(),
                    children: BTreeMap::new(),
                });
        }
        cur.apis.push(api);
    }
    let mut items = Vec::new();
    for api in &root.apis {
        items.push(api_to_apifox(api));
    }
    for (_, c) in &root.children {
        items.push(pnode_to_apifox(c));
    }
    json!({
        "$schema": "https://apifox.com/schemas/apifox-project.json",
        "info": {
            "name": "API Manager 导出",
            "description": "由 API Manager 导出的 Apifox 项目",
            "version": "1.0.0"
        },
        "apiCollection": [{ "name": "根目录", "items": items }]
    })
}

fn pnode_to_apifox(n: &PNode) -> Value {
    let mut item = Vec::new();
    for api in &n.apis {
        item.push(api_to_apifox(api));
    }
    for (_, c) in &n.children {
        item.push(pnode_to_apifox(c));
    }
    json!({ "name": n.name, "items": item })
}

/// 单个接口 → Apifox api item
fn api_to_apifox(api: &ApiFile) -> Value {
    let to_param = |kv: &KeyValue| {
        json!({
            "name": kv.key,
            "type": "string",
            "required": kv.enabled,
            "enable": kv.enabled,
            "description": kv.description,
            "value": kv.value
        })
    };
    let path_params: Vec<Value> = api
        .params
        .iter()
        .filter(|p| p.enabled && !p.key.trim().is_empty())
        .map(|p| {
            json!({
                "name": p.key,
                "type": "string",
                "required": true,
                "enable": true,
                "description": p.description
            })
        })
        .collect();
    let query: Vec<Value> = api
        .query
        .iter()
        .filter(|q| q.enabled && !q.key.trim().is_empty())
        .map(&to_param)
        .collect();
    let headers: Vec<Value> = api
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .map(&to_param)
        .collect();
    let (body_type, body_params, body_examples, media_type) = match api.body.mode.as_str() {
        "json" | "raw" => {
            let examples = if api.body.raw.trim().is_empty() {
                Vec::new()
            } else {
                vec![json!({
                    "name": "默认示例",
                    "data": api.body.raw,
                    "mediaType": "application/json"
                })]
            };
            ("json", Vec::<Value>::new(), examples, "application/json")
        }
        "form" => {
            let params: Vec<Value> = api
                .body
                .form
                .iter()
                .filter(|f| f.enabled && !f.key.trim().is_empty())
                .map(|f| {
                    json!({
                        "name": f.key,
                        "type": if f.is_file { "file" } else { "text" },
                        "enable": true,
                        "required": true,
                        "description": f.description
                    })
                })
                .collect();
            ("form-data", params, Vec::new(), "multipart/form-data")
        }
        _ => ("none", Vec::new(), Vec::new(), ""),
    };
    json!({
        "name": api.name,
        "api": {
            "method": api.method.to_lowercase(),
            "path": api.path,
            "parameters": {
                "path": path_params,
                "query": query,
                "header": headers,
                "cookie": []
            },
            "requestBody": {
                "type": body_type,
                "parameters": body_params,
                "examples": body_examples,
                "mediaType": media_type
            },
            "description": api.description
        }
    })
}

// ==================== Apipost 导出 ====================
