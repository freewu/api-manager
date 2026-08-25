//! 由 export.rs 拆分：Postman Collection v2.1
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

/// 生成 Postman Collection v2.1 JSON
pub fn to_postman(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let mut root = PNode {
        name: "API Manager".to_string(),
        apis: Vec::new(),
        children: BTreeMap::new(),
    };
    for (segs, api) in apis {
        let mut cur = &mut root;
        for (s, _dep) in segs {
            cur = cur.children.entry(s.clone()).or_insert_with(|| PNode {
                name: s.clone(),
                apis: Vec::new(),
                children: BTreeMap::new(),
            });
        }
        cur.apis.push(api);
    }
    let mut items = Vec::new();
    for api in &root.apis {
        items.push(api_to_postman(api));
    }
    for (_, c) in &root.children {
        items.push(pnode_to_postman(c));
    }
    json!({
        "info": {
            "name": "API Manager 导出",
            "description": "由 API Manager 导出的 Postman Collection",
            "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
        },
        "item": items
    })
}

fn pnode_to_postman(n: &PNode) -> Value {
    let mut item = Vec::new();
    for api in &n.apis {
        item.push(api_to_postman(api));
    }
    for (_, c) in &n.children {
        item.push(pnode_to_postman(c));
    }
    json!({ "name": n.name, "item": item })
}

/// 单个接口 → Postman request item
fn api_to_postman(api: &ApiFile) -> Value {
    let headers: Vec<Value> = api
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .map(|h| json!({ "key": h.key, "value": h.value, "type": "text", "description": h.description }))
        .collect();
    let query: Vec<Value> = api
        .query
        .iter()
        .filter(|q| q.enabled && !q.key.trim().is_empty())
        .map(|q| json!({ "key": q.key, "value": q.value, "description": q.description }))
        .collect();
    let url_raw = if !api.url.trim().is_empty() {
        api.url.trim().to_string()
    } else {
        api.path.trim().to_string()
    };
    let (host, path) = parse_url(&url_raw);
    let mut url = json!({
        "raw": url_raw,
        "host": host,
        "path": path,
    });
    if !query.is_empty() {
        url["query"] = Value::Array(query);
    }
    let mut request = json!({
        "method": api.method,
        "header": headers,
        "url": url,
    });
    match api.body.mode.as_str() {
        "json" | "raw" => {
            request["body"] = json!({ "mode": "raw", "raw": api.body.raw });
        }
        "form" => {
            let fields: Vec<Value> = api
                .body
                .form
                .iter()
                .filter(|f| f.enabled && !f.key.trim().is_empty())
                .map(|f| {
                    if f.is_file {
                        json!({ "key": f.key, "type": "file", "src": Value::Null })
                    } else {
                        json!({ "key": f.key, "value": f.value, "type": "text", "description": f.description })
                    }
                })
                .collect();
            request["body"] = json!({ "mode": "urlencoded", "urlencoded": fields });
        }
        _ => {}
    }
    json!({ "name": api.name, "request": request })
}
