//! 由 export.rs 拆分：Insomnia
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

/// 生成 Insomnia 导出格式（collection.insomnia.rest/5.0 YAML）
pub fn to_insomnia(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let mut root = PNode {
        name: "root".into(),
        apis: Vec::new(),
        children: BTreeMap::new(),
    };
    for (segs, api) in apis {
        let mut node = &mut root;
        for (s, _dep) in segs {
            node = node.children.entry(s.clone()).or_insert_with(|| PNode {
                name: s.clone(),
                apis: Vec::new(),
                children: BTreeMap::new(),
            });
        }
        node.apis.push(api);
    }
    let mut children = Vec::new();
    for api in &root.apis {
        children.push(insomnia_request_value(api));
    }
    for (_, c) in &root.children {
        children.push(insomnia_folder_value(c));
    }
    json!({
        "type": "collection.insomnia.rest/5.0",
        "name": "API Manager 导出",
        "meta": {
            "id": "coll_export",
            "created": "2026-01-01T00:00:00.000Z",
            "modified": "2026-01-01T00:00:00.000Z"
        },
        "children": children,
        "environment": {
            "baseUrl": ""
        }
    })
}

/// 分组 → Insomnia 文件夹节点
fn insomnia_folder_value(n: &PNode) -> Value {
    let mut children = Vec::new();
    for api in &n.apis {
        children.push(insomnia_request_value(api));
    }
    for (_, c) in &n.children {
        children.push(insomnia_folder_value(c));
    }
    json!({
        "name": n.name,
        "meta": { "id": format!("fld_{}", n.name) },
        "children": children
    })
}

/// 接口 → Insomnia 请求节点
fn insomnia_request_value(api: &ApiFile) -> Value {
    let is_ws = api.protocol == "websocket";
    let url = if is_ws {
        api.path.clone()
    } else {
        format!("{{{{baseUrl}}}}{}", api.path)
    };
    // 收集 Bearer token（如存在）
    let mut token = String::new();
    let mut headers: Vec<Value> = Vec::new();
    for h in api.headers.iter().filter(|h| h.enabled && !h.key.trim().is_empty()) {
        if h.key.eq_ignore_ascii_case("authorization") && h.value.starts_with("Bearer ") {
            token = h.value.trim_start_matches("Bearer ").trim().to_string();
            continue; // 交给 authentication 表达
        }
        headers.push(json!({ "name": h.key, "value": h.value }));
    }
    let parameters: Vec<Value> = api
        .query
        .iter()
        .filter(|q| q.enabled && !q.key.trim().is_empty())
        .map(|q| json!({ "name": q.key, "value": q.value }))
        .collect();
    let body = match api.body.mode.as_str() {
        "json" => json!({ "mimeType": "application/json", "text": api.body.raw }),
        "form" => {
            // 拼接 urlencoded 文本
            let pairs: Vec<String> = api
                .body
                .form
                .iter()
                .filter(|f| f.enabled && !f.key.trim().is_empty())
                .map(|f| format!("{}={}", f.key, f.value))
                .collect();
            json!({ "mimeType": "application/x-www-form-urlencoded", "text": pairs.join("&") })
        }
        "raw" => json!({ "mimeType": "text/plain", "text": api.body.raw }),
        _ => Value::Null,
    };
    let mut req = json!({
        "name": api.name,
        "meta": { "id": format!("req_{}", api.uuid) },
        "url": url,
        "method": api.method,
        "body": body,
        "headers": headers,
        "parameters": parameters,
        "authentication": { "type": "none" }
    });
    if !token.is_empty() {
        req["authentication"] = json!({ "type": "bearer", "token": token });
    }
    req
}

// ==================== JMeter 导出 ====================
