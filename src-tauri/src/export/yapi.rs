//! 由 export.rs 拆分：YApi
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

/// 生成 YApi 原生导出格式（分组树 + api 对象，YApi 可导入）
pub fn to_yapi(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let mut tree = PNode {
        name: "root".into(),
        apis: Vec::new(),
        children: BTreeMap::new(),
    };
    for (segs, api) in apis {
        if segs.is_empty() {
            // 根级接口（无分组）直接输出为顶层接口项
            tree.apis.push(api);
            continue;
        }
        let mut node = &mut tree;
        for (seg, _dep) in segs {
            node = node
                .children
                .entry(seg.clone())
                .or_insert_with(|| PNode {
                    name: seg.clone(),
                    apis: Vec::new(),
                    children: BTreeMap::new(),
                });
        }
        node.apis.push(api);
    }
    let mut out = Vec::new();
    for api in &tree.apis {
        out.push(api_to_yapi(api));
    }
    for (_, c) in &tree.children {
        out.push(pnode_to_yapi(c));
    }
    json!(out)
}

/// PNode → YApi 分组节点（含 children 与 api 接口项）
fn pnode_to_yapi(n: &PNode) -> Value {
    let mut children = Vec::new();
    for api in &n.apis {
        children.push(api_to_yapi(api));
    }
    for (_, c) in &n.children {
        children.push(pnode_to_yapi(c));
    }
    json!({
        "name": n.name,
        "desc": "",
        "children": children,
    })
}

/// 单个接口 → YApi 接口节点 { name, api: {...} }
fn api_to_yapi(api: &ApiFile) -> Value {
    let path = path_to_yapi(&api.path);
    let req_query: Vec<Value> = api
        .query
        .iter()
        .filter(|q| q.enabled && !q.key.trim().is_empty())
        .map(|q| {
            json!({
                "name": q.key,
                "value": q.value,
                "desc": q.description,
                "required": false,
                "example": q.value,
                "type": "text"
            })
        })
        .collect();
    let req_headers: Vec<Value> = api
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .map(|h| {
            json!({
                "name": h.key,
                "value": h.value,
                "desc": h.description,
                "required": false
            })
        })
        .collect();
    let (req_body_type, req_body_other, req_body_form) = match api.body.mode.as_str() {
        "json" | "raw" => (
            "json",
            api.body.raw.clone(),
            Vec::<Value>::new(),
        ),
        "form" => {
            let form: Vec<Value> = api
                .body
                .form
                .iter()
                .filter(|f| f.enabled && !f.key.trim().is_empty())
                .map(|f| {
                    json!({
                        "name": f.key,
                        "value": f.value,
                        "type": if f.is_file { "file" } else { "text" },
                        "desc": f.description,
                        "required": false
                    })
                })
                .collect();
            ("form", String::new(), form)
        }
        _ => ("null", String::new(), Vec::new()),
    };
    let res_body = api
        .responses
        .first()
        .map(|r| r.body.clone())
        .unwrap_or_default();
    let res_body_type = if res_body.trim().is_empty() {
        "null"
    } else {
        "json"
    };
    json!({
        "name": api.name,
        "api": {
            "method": api.method,
            "path": path,
            "title": api.name,
            "desc": api.description,
            "req_query": req_query,
            "req_headers": req_headers,
            "req_body_type": req_body_type,
            "req_body_other": req_body_other,
            "req_body_form": req_body_form,
            "res_body_type": res_body_type,
            "res_body": res_body,
            "protocol": if api.protocol == "websocket" { "ws" } else { "http" }
        }
    })
}

/// 路径参数 {id} → :id（YApi 语法）
fn path_to_yapi(p: &str) -> String {
    let mut out = String::with_capacity(p.len());
    let mut chars = p.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            let mut var = String::new();
            for c2 in chars.by_ref() {
                if c2 == '}' {
                    break;
                }
                var.push(c2);
            }
            if !var.is_empty() {
                out.push(':');
                out.push_str(&var);
            }
        } else {
            out.push(c);
        }
    }
    out
}

// ==================== Eolink 导出 ====================
