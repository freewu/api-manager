//! 由 export.rs 拆分：Eolink
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

/// 生成 Eolink 导出格式（apiGroupList 分组树 + 接口对象）
pub fn to_eolink(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
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
    let mut next_id = 0i64;
    let mut groups = Vec::new();
    // 根级接口 → 「未分组」组
    if !root.apis.is_empty() {
        next_id += 1;
        let api_list: Vec<Value> = root.apis.iter().map(|a| api_to_eolink_api(a)).collect();
        groups.push(json!({
            "groupID": next_id,
            "groupName": "未分组",
            "parentGroupID": 0,
            "sort": 0,
            "apiList": api_list,
            "childGroupList": []
        }));
    }
    for (_, c) in &root.children {
        groups.push(eolink_pnode(&mut next_id, 0, c));
    }
    json!({
        "exportVersion": "1.0",
        "projectInfo": {
            "projectName": "API Manager 导出",
            "projectDesc": "",
            "projectVersion": "1.0.0"
        },
        "apiGroupList": groups,
        "environmentList": [],
        "dataStructureList": [],
        "statusCodeList": [],
        "projectDocList": []
    })
}

/// PNode → Eolink 分组（分配 groupID/parentGroupID）
fn eolink_pnode(next: &mut i64, parent: i64, n: &PNode) -> Value {
    *next += 1;
    let gid = *next;
    let api_list: Vec<Value> = n.apis.iter().map(|a| api_to_eolink_api(a)).collect();
    let child_groups: Vec<Value> = n
        .children
        .iter()
        .map(|(_, c)| eolink_pnode(next, gid, c))
        .collect();
    json!({
        "groupID": gid,
        "groupName": n.name,
        "parentGroupID": parent,
        "sort": 0,
        "apiList": api_list,
        "childGroupList": child_groups
    })
}

/// 单个接口 → Eolink API 对象
fn api_to_eolink_api(api: &ApiFile) -> Value {
    let (api_uri, protocol) = if api.protocol == "websocket" {
        (api.path.clone(), "WS".to_string())
    } else {
        (api.path.clone(), "HTTP".to_string())
    };
    let req_headers: Vec<Value> = api
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .map(|h| {
            json!({
                "key": h.key,
                "type": "string",
                "isRequired": 1,
                "example": h.value,
                "mock": "",
                "desc": h.description
            })
        })
        .collect();
    let req_query: Vec<Value> = api
        .query
        .iter()
        .filter(|q| q.enabled && !q.key.trim().is_empty())
        .map(|q| {
            json!({
                "key": q.key,
                "type": "string",
                "isRequired": 1,
                "example": q.value,
                "mock": "",
                "desc": q.description
            })
        })
        .collect();
    let req_rest: Vec<Value> = api
        .params
        .iter()
        .filter(|p| p.enabled && !p.key.trim().is_empty())
        .map(|p| {
            json!({
                "key": p.key,
                "type": "string",
                "isRequired": 1,
                "example": p.value,
                "mock": "",
                "desc": p.description
            })
        })
        .collect();
    let (req_body_type, req_body_json, req_body_form) = match api.body.mode.as_str() {
        "json" => {
            let list = parse_json_to_eolink_list(&api.body.raw);
            ("json", list, Vec::<Value>::new())
        }
        "form" => {
            let form: Vec<Value> = api
                .body
                .form
                .iter()
                .filter(|f| f.enabled && !f.key.trim().is_empty())
                .map(|f| {
                    json!({
                        "key": f.key,
                        "type": if f.is_file { "file" } else { "text" },
                        "isRequired": 1,
                        "example": f.value,
                        "mock": "",
                        "desc": f.description
                    })
                })
                .collect();
            ("x-www-form-urlencoded", Vec::<Value>::new(), form)
        }
        "raw" => ("raw", Vec::<Value>::new(), Vec::<Value>::new()),
        _ => ("", Vec::<Value>::new(), Vec::<Value>::new()),
    };
    let req_body_raw = if api.body.mode == "raw" {
        api.body.raw.clone()
    } else {
        String::new()
    };
    let response_info: Vec<Value> = api
        .responses
        .iter()
        .map(|r| {
            let list = parse_json_to_eolink_list(&r.body);
            json!({
                "responseName": r.name,
                "responseCode": r.status,
                "responseContentType": if r.content_type.contains("json") { "json" } else { "raw" },
                "responseBodyJsonList": list
            })
        })
        .collect();
    json!({
        "apiID": format!("api_{}", api.uuid),
        "apiName": api.name,
        "apiMethod": api.method,
        "apiUri": api_uri,
        "apiProtocol": protocol,
        "apiStatus": "已完成",
        "apiTagList": [],
        "apiDesc": api.description,
        "apiNote": "",
        "requestInfo": {
            "requestHeaderList": req_headers,
            "requestQueryList": req_query,
            "requestRestList": req_rest,
            "requestBodyType": req_body_type,
            "requestBodyJsonList": req_body_json,
            "requestBodyFormList": req_body_form,
            "requestBodyRaw": req_body_raw
        },
        "responseInfoList": response_info,
        "testCaseList": []
    })
}

/// 解析 JSON 字符串为 Eolink 字段列表（嵌套 children）
fn parse_json_to_eolink_list(raw: &str) -> Vec<Value> {
    match serde_json::from_str::<Value>(raw.trim()) {
        Ok(Value::Object(m)) => m
            .iter()
            .map(|(k, v)| json_value_to_eolink_item(k, v))
            .collect(),
        _ => Vec::new(),
    }
}

/// 单个 JSON 值 → Eolink 字段项
fn json_value_to_eolink_item(key: &str, v: &Value) -> Value {
    let (ty, example, children) = match v {
        Value::Object(m) => {
            let kids: Vec<Value> = m.iter().map(|(k, x)| json_value_to_eolink_item(k, x)).collect();
            ("object", json!({}), kids)
        }
        Value::Array(a) => {
            let kids: Vec<Value> = a
                .iter()
                .enumerate()
                .map(|(i, x)| json_value_to_eolink_item(&format!("[{i}]"), x))
                .collect();
            ("array", json!([]), kids)
        }
        Value::String(s) => ("string", json!(s), Vec::new()),
        Value::Number(n) => ("number", json!(n), Vec::new()),
        Value::Bool(b) => ("boolean", json!(b), Vec::new()),
        Value::Null => ("string", json!(""), Vec::new()),
    };
    json!({
        "key": key,
        "type": ty,
        "isRequired": 1,
        "example": example,
        "mock": "",
        "desc": "",
        "children": children
    })
}

// ==================== Insomnia 导出 ====================
