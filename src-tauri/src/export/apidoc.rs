//! 由 export.rs 拆分：apiDoc
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

/// 导出 apiDoc 格式（api_project.json + api_data.json 两个文件内容）
pub fn to_apidoc(apis: &[(Vec<(String, bool)>, ApiFile)]) -> (Value, Value) {
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
    // 顶层分组列表（保持顺序）
    let top_groups: Vec<&PNode> = root.children.values().collect();
    let root_name = top_groups.first().map(|n| n.name.as_str()).unwrap_or("API 文档");
    let mut groups: Vec<Value> = Vec::new();
    let mut group_order: Vec<String> = Vec::new();
    if !top_groups.is_empty() {
        for g in &top_groups {
            groups.push(json!({
                "name": g.name,
                "title": g.name,
                "description": "",
            }));
            group_order.push(g.name.clone());
        }
    }
    if !root.apis.is_empty() {
        groups.push(json!({
            "name": "未分组",
            "title": "未分组",
            "description": "",
        }));
        group_order.push("未分组".to_string());
    }
    // apis
    let mut apis_out: Vec<Value> = Vec::new();
    for g in &top_groups {
        for api in &g.apis {
            apis_out.push(api_to_apidoc(api, &g.name));
        }
    }
    for api in &root.apis {
        apis_out.push(api_to_apidoc(api, "未分组"));
    }
    let project = json!({
        "name": root_name,
        "version": "1.0.0",
        "description": "",
        "title": "API接口文档",
        "url": "/api",
        "sampleUrl": "",
        "header": { "title": "", "content": "" },
        "footer": { "title": "", "content": "" },
        "template": { "withCompare": true, "withGenerator": true, "withEditor": false },
        "order": group_order,
        "exclude": [],
        "language": "zh-cn"
    });
    let data = json!({
        "groups": groups,
        "defines": [],
        "apis": apis_out
    });
    (project, data)
}

/// 从 docParams 查找字段类型
fn apidoc_doc_type(api: &ApiFile, source: &str, key: &str) -> String {
    api.doc_params
        .iter()
        .find(|d| d.source == source && d.key == key)
        .map(|d| d.r#type.clone())
        .unwrap_or_else(|| "String".to_string())
}

/// 构造 {field, type, required, description} 字段对象
fn apidoc_field_obj(field: &str, ty: &str, required: bool, description: &str) -> Value {
    json!({
        "field": field,
        "type": ty,
        "required": required,
        "description": description
    })
}

fn apidoc_kv_fields(api: &ApiFile, source: &str, kv: &[KeyValue]) -> Vec<Value> {
    kv.iter()
        .filter(|k| k.enabled && !k.key.trim().is_empty())
        .map(|k| {
            apidoc_field_obj(
                k.key.trim(),
                &apidoc_doc_type(api, source, k.key.trim()),
                true,
                &k.description,
            )
        })
        .collect()
}

/// JSON 值 → 点分字段列表
fn apidoc_json_to_fields(prefix: &str, v: &Value, out: &mut Vec<Value>) {
    match v {
        Value::Object(m) => {
            for (k, val) in m {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                if val.is_object() {
                    apidoc_json_to_fields(&key, val, out);
                } else if val.is_array() {
                    let arr = val.as_array().unwrap();
                    let elem_ty = if arr.is_empty() {
                        "Object".to_string()
                    } else {
                        apidoc_type_str(&arr[0])
                    };
                    out.push(apidoc_field_obj(&key, &format!("{elem_ty}[]"), true, ""));
                    if let Some(first) = arr.first() {
                        if first.is_object() {
                            for (sk, sv) in first.as_object().unwrap() {
                                apidoc_json_to_fields(&format!("{key}[].{sk}"), sv, out);
                            }
                        }
                    }
                } else {
                    out.push(apidoc_field_obj(&key, &apidoc_type_str(val), true, ""));
                }
            }
        }
        _ => {
            let key = if prefix.is_empty() { "body" } else { prefix };
            out.push(apidoc_field_obj(key, &apidoc_type_str(v), true, ""));
        }
    }
}

fn apidoc_type_str(v: &Value) -> String {
    match v {
        Value::Number(n) => {
            if n.as_i64().is_some() {
                "Number".to_string()
            } else {
                "Float".to_string()
            }
        }
        Value::Bool(_) => "Boolean".to_string(),
        Value::Array(_) => "List".to_string(),
        Value::Object(_) => "Object".to_string(),
        _ => "String".to_string(),
    }
}

/// ApiFile → apiDoc api 对象
fn api_to_apidoc(api: &ApiFile, group_name: &str) -> Value {
    let mut parameter_fields = serde_json::Map::new();
    // body 字段
    let mut body_fields: Vec<Value> = Vec::new();
    match api.body.mode.as_str() {
        "json" => {
            if !api.body.raw.trim().is_empty() {
                if let Ok(v) = serde_json::from_str::<Value>(&api.body.raw) {
                    apidoc_json_to_fields("", &v, &mut body_fields);
                }
            }
        }
        "form" => {
            body_fields = apidoc_kv_fields(api, "body", &api.body.form);
        }
        _ => {
            if !api.body.raw.trim().is_empty() {
                body_fields.push(apidoc_field_obj("body", "String", true, ""));
            }
        }
    }
    if !body_fields.is_empty() {
        parameter_fields.insert("Parameter".to_string(), Value::Array(body_fields));
    }
    // query 字段
    let query_fields = apidoc_kv_fields(api, "query", &api.query);
    if !query_fields.is_empty() {
        parameter_fields.insert("Query".to_string(), Value::Array(query_fields));
    }
    // header 字段
    let header_fields = apidoc_kv_fields(api, "header", &api.headers);
    let header = if header_fields.is_empty() {
        Value::Null
    } else {
        json!({ "fields": { "Header": header_fields } })
    };
    // 响应
    let mut success_examples: Vec<Value> = Vec::new();
    let mut error_examples: Vec<Value> = Vec::new();
    for r in &api.responses {
        if r.body.trim().is_empty() {
            continue;
        }
        if r.status == 0 || r.status >= 400 {
            error_examples.push(json!({ "title": r.name, "content": r.body }));
        } else {
            success_examples.push(json!({ "title": r.name, "content": r.body }));
        }
    }
    let success = if success_examples.is_empty() {
        Value::Null
    } else {
        json!({ "examples": success_examples })
    };
    let error = if error_examples.is_empty() {
        Value::Null
    } else {
        json!({ "examples": error_examples })
    };
    // path {id} → :id
    let url = api.path.replace('{', ":").replace('}', "");
    json!({
        "group": group_name,
        "name": api.name,
        "title": api.name,
        "description": api.description,
        "method": api.method.to_uppercase(),
        "url": url,
        "parameter": { "fields": parameter_fields },
        "header": header,
        "success": success,
        "error": error,
        "successExamples": success_examples
    })
}
// ==================== 批量格式导出（10 种） ====================
