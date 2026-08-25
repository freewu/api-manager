//! 由 export.rs 拆分：Apipost
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

/// 生成 Apipost 项目 JSON（apis 平铺数组 + target_id/parent_id 组织树）
pub fn to_apipost(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
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
    let mut apis_out: Vec<Value> = Vec::new();
    let mut counter = 0usize;
    pnode_to_apipost(&root, "0", &mut counter, &mut apis_out);
    json!({
        "project_id": "apipost-export",
        "name": "API Manager 导出",
        "intro": "",
        "global": {},
        "models": [],
        "apis": apis_out,
        "samples": [],
        "automated_testings": []
    })
}

fn pnode_to_apipost(n: &PNode, parent_id: &str, counter: &mut usize, out: &mut Vec<Value>) {
    for api in &n.apis {
        *counter += 1;
        let id = format!("a{counter}");
        out.push(api_to_apipost(api, &id, parent_id));
    }
    for (_, c) in &n.children {
        *counter += 1;
        let id = format!("f{counter}");
        out.push(json!({
            "target_id": id,
            "project_id": "apipost-export",
            "parent_id": parent_id,
            "target_type": "folder",
            "name": c.name,
            "sort": 0,
            "request": {},
            "description": ""
        }));
        pnode_to_apipost(c, &id, counter, out);
    }
}

fn api_to_apipost(api: &ApiFile, id: &str, parent_id: &str) -> Value {
    let to_param = |kv: &KeyValue| {
        json!({
            "key": kv.key,
            "value": kv.value,
            "description": kv.description,
            "is_checked": if kv.enabled { 1 } else { 0 },
            "not_null": 0,
            "field_type": "string"
        })
    };
    let headers: Vec<Value> = api
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .map(&to_param)
        .collect();
    let query: Vec<Value> = api
        .query
        .iter()
        .filter(|q| q.enabled && !q.key.trim().is_empty())
        .map(&to_param)
        .collect();
    let restful: Vec<Value> = api
        .params
        .iter()
        .filter(|p| p.enabled && !p.key.trim().is_empty())
        .map(&to_param)
        .collect();
    let (mode, raw, form_params) = match api.body.mode.as_str() {
        "json" | "raw" => ("json", api.body.raw.clone(), Vec::<Value>::new()),
        "form" => {
            let params: Vec<Value> = api
                .body
                .form
                .iter()
                .filter(|f| f.enabled && !f.key.trim().is_empty())
                .map(|f| {
                    json!({
                        "key": f.key,
                        "value": f.value,
                        "type": if f.is_file { "file" } else { "text" },
                        "description": f.description
                    })
                })
                .collect();
            ("form-data", String::new(), params)
        }
        _ => ("none", String::new(), Vec::new()),
    };
    let url = if !api.url.trim().is_empty() {
        api.url.trim().to_string()
    } else {
        api.path.clone()
    };
    json!({
        "target_id": id,
        "project_id": "apipost-export",
        "parent_id": parent_id,
        "target_type": "api",
        "name": api.name,
        "method": api.method,
        "url": url,
        "description": api.description,
        "protocol": if api.protocol == "websocket" { "websocket" } else { "http/1.1" },
        "sort": 0,
        "request": {
            "header": { "parameter": headers },
            "query": { "query_add_equal": 1, "parameter": query },
            "restful": { "parameter": restful },
            "cookie": { "parameter": [] },
            "body": { "mode": mode, "parameter": form_params, "raw": raw }
        },
        "response": []
    })
}

// ==================== RAML 导出 ====================
