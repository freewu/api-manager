//! 由 export.rs 拆分：批量 10 格式 + RAP2
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

struct ExtraGroup<'a> {
    name: String,
    apis: Vec<&'a ApiFile>,
}

/// 建立分组树（顶层分组 + 根级接口归入「未分组」）
fn extra_build_tree<'a>(apis: &'a [(Vec<(String, bool)>, ApiFile)]) -> (Vec<ExtraGroup<'a>>, String) {
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
    let root_name = root
        .children
        .values()
        .next()
        .map(|n| n.name.clone())
        .unwrap_or_else(|| "API 文档".to_string());
    let mut groups: Vec<ExtraGroup> = root
        .children
        .values()
        .map(|n| ExtraGroup {
            name: n.name.clone(),
            apis: n.apis.clone(),
        })
        .collect();
    if !root.apis.is_empty() {
        groups.push(ExtraGroup {
            name: "未分组".to_string(),
            apis: root.apis.clone(),
        });
    }
    (groups, root_name)
}

fn extra_kv_enabled(kv: &KeyValue) -> bool {
    kv.enabled && !kv.key.trim().is_empty()
}

// ---------- apiDog ----------

pub fn to_apidog(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let (top_groups, root_name) = extra_build_tree(apis);
    let mut folders: Vec<Value> = Vec::new();
    let mut sort = 1i64;
    for g in &top_groups {
        let mut group_apis: Vec<Value> = Vec::new();
        for api in &g.apis {
            group_apis.push(apidog_api_out(api));
        }
        folders.push(json!({
            "name": g.name,
            "description": "",
            "sort": sort,
            "apis": group_apis,
        }));
        sort += 1;
    }
    json!({
        "version": "1.0",
        "projectMeta": { "name": root_name, "description": "", "maintainer": "", "createdAt": "" },
        "environments": [],
        "globalParams": [],
        "folders": folders,
    })
}

fn apidog_api_out(api: &ApiFile) -> Value {
    let mut headers: Vec<Value> = Vec::new();
    for h in api.headers.iter().filter(|h| extra_kv_enabled(h)) {
        headers.push(json!({ "key": h.key, "value": h.value, "description": h.description }));
    }
    let mut query_out: Vec<Value> = Vec::new();
    for q in api.query.iter().filter(|q| extra_kv_enabled(q)) {
        query_out.push(json!({ "key": q.key, "value": q.value, "description": q.description }));
    }
    let mut request_body = json!({ "mode": "none" });
    match api.body.mode.as_str() {
        "json" => {
            let example = serde_json::from_str::<Value>(&api.body.raw).unwrap_or(Value::Null);
            if !example.is_null() {
                request_body = json!({ "mode": "json", "example": example });
            }
        }
        "form" => {
            let mut formdata: Vec<Value> = Vec::new();
            for f in api.body.form.iter().filter(|f| extra_kv_enabled(f)) {
                formdata.push(json!({
                    "key": f.key, "value": f.value, "description": f.description,
                    "type": if f.is_file { "file" } else { "text" },
                }));
            }
            request_body = json!({ "mode": "formdata", "formdata": formdata });
        }
        _ => {
            if !api.body.raw.is_empty() {
                request_body = json!({ "mode": "raw", "raw": api.body.raw });
            }
        }
    }
    let mut responses: Vec<Value> = Vec::new();
    for r in &api.responses {
        if r.body.trim().is_empty() {
            continue;
        }
        let example = serde_json::from_str::<Value>(&r.body).unwrap_or(Value::Null);
        if !example.is_null() {
            responses.push(json!({
                "statusCode": r.status,
                "description": r.name,
                "example": example,
            }));
        }
    }
    json!({
        "name": api.name,
        "method": api.method.to_uppercase(),
        "path": api.path,
        "description": api.description,
        "status": "released",
        "auth": { "type": "none" },
        "request": {
            "headers": headers,
            "query": query_out,
            "body": request_body,
        },
        "responses": responses,
    })
}

// ---------- Bruno ----------

pub fn to_bruno(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let (top_groups, root_name) = extra_build_tree(apis);
    let mut folders: Vec<Value> = Vec::new();
    let mut seq = 1i64;
    for g in &top_groups {
        let mut requests: Vec<Value> = Vec::new();
        for api in &g.apis {
            requests.push(bruno_req_out(api, seq));
            seq += 1;
        }
        folders.push(json!({
            "info": { "name": g.name, "seq": seq },
            "scripts": {},
            "auth": { "mode": "none" },
            "requests": requests,
        }));
        seq += 1;
    }
    json!({
        "version": "1.0.0",
        "info": { "name": root_name, "description": "", "schema": "bruno-schema/1" },
        "settings": { "encodeUrl": true, "followRedirects": false, "maxRedirects": 5, "timeout": 0 },
        "scripts": { "flow": [], "filesystemAccess": "read", "preRequest": [], "postResponse": [] },
        "auth": { "mode": "none" },
        "environments": { "local": {} },
        "folders": folders,
    })
}

fn bruno_req_out(api: &ApiFile, seq: i64) -> Value {
    let mut headers = serde_json::Map::new();
    for h in api.headers.iter().filter(|h| extra_kv_enabled(h)) {
        headers.insert(h.key.clone(), Value::String(h.value.clone()));
    }
    let mut query_out: Vec<Value> = Vec::new();
    for q in api.query.iter().filter(|q| extra_kv_enabled(q)) {
        query_out.push(json!({ "key": q.key, "value": q.value }));
    }
    let body = match api.body.mode.as_str() {
        "form" => {
            let mut parts: Vec<String> = Vec::new();
            for f in api.body.form.iter().filter(|f| extra_kv_enabled(f)) {
                parts.push(format!("{}={}", f.key, f.value));
            }
            json!({ "type": "form-urlencoded", "data": parts.join("&") })
        }
        _ => {
            if api.body.raw.is_empty() {
                json!({ "type": "raw", "data": "" })
            } else {
                json!({ "type": "json", "data": api.body.raw })
            }
        }
    };
    json!({
        "info": { "name": api.name, "type": "http", "seq": seq },
        "http": {
            "method": api.method.to_uppercase(),
            "url": api.path,
            "headers": headers,
            "query": query_out,
            "body": body,
            "auth": { "mode": "none" },
        },
        "runtime": { "scripts": [] },
    })
}

// ---------- Apizza ----------

pub fn to_apizza(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let (top_groups, root_name) = extra_build_tree(apis);
    let mut folders: Vec<Value> = Vec::new();
    for g in &top_groups {
        let mut group_apis: Vec<Value> = Vec::new();
        for api in &g.apis {
            group_apis.push(apizza_api_out(api));
        }
        folders.push(json!({
            "folderName": g.name,
            "folderDesc": "",
            "children": [],
            "apis": group_apis,
        }));
    }
    json!({
        "version": "1.0.0",
        "projectName": root_name,
        "projectDesc": "",
        "createTime": "",
        "updateTime": "",
        "envs": [],
        "folders": folders,
    })
}

fn apizza_api_out(api: &ApiFile) -> Value {
    let mut headers: Vec<Value> = Vec::new();
    for h in api.headers.iter().filter(|h| extra_kv_enabled(h)) {
        headers.push(json!({ "key": h.key, "value": h.value }));
    }
    let mut query_params: Vec<Value> = Vec::new();
    for q in api.query.iter().filter(|q| extra_kv_enabled(q)) {
        query_params.push(json!({ "key": q.key, "value": q.value, "desc": q.description }));
    }
    let mut path_params: Vec<Value> = Vec::new();
    for p in api.params.iter().filter(|p| extra_kv_enabled(p)) {
        path_params.push(json!({ "key": p.key, "value": p.value, "desc": p.description }));
    }
    let (body_mode, body_raw, body_form, body_formdata) = match api.body.mode.as_str() {
        "form" => {
            let mut fd: Vec<Value> = Vec::new();
            for f in api.body.form.iter().filter(|f| extra_kv_enabled(f)) {
                fd.push(json!({
                    "key": f.key, "value": f.value, "type": if f.is_file { "file" } else { "text" }, "desc": f.description,
                }));
            }
            ("formdata".to_string(), String::new(), Vec::<Value>::new(), fd)
        }
        _ => {
            ("raw".to_string(), api.body.raw.clone(), Vec::<Value>::new(), Vec::<Value>::new())
        }
    };
    let mut responses: Vec<Value> = Vec::new();
    for r in &api.responses {
        if r.body.trim().is_empty() {
            continue;
        }
        responses.push(json!({
            "status": r.status,
            "name": r.name,
            "contentType": "application/json",
            "body": r.body,
        }));
    }
    json!({
        "apiName": api.name,
        "apiDesc": api.description,
        "method": api.method.to_uppercase(),
        "url": api.path,
        "headers": headers,
        "cookies": [],
        "queryParams": query_params,
        "pathParams": path_params,
        "bodyMode": body_mode,
        "bodyRaw": body_raw,
        "bodyForm": body_form,
        "bodyFormData": body_formdata,
        "responses": responses,
    })
}

// ---------- NEI ----------

/// json → 字段列表（点分 key，返回 [{name, type, required, description, example}]）
fn nei_json_to_params(prefix: &str, v: &Value, out: &mut Vec<Value>) {
    match v {
        Value::Object(m) => {
            for (k, val) in m {
                let key = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
                if val.is_object() {
                    out.push(json!({
                        "name": key, "type": "object", "required": true, "description": "", "example": serde_json::json!({}),
                    }));
                    nei_json_to_params(&key, val, out);
                } else if val.is_array() {
                    let arr = val.as_array().unwrap();
                    let elem = arr.first().cloned().unwrap_or(Value::Null);
                    let mut item = json!({ "type": "string" });
                    if elem.is_object() {
                        item = json!({ "type": "object" });
                    }
                    out.push(json!({
                        "name": key, "type": "array", "required": true, "description": "", "items": item, "example": val,
                    }));
                } else {
                    let ty = match val {
                        Value::Number(n) if n.as_i64().is_some() => "long",
                        Value::Number(_) => "double",
                        Value::Bool(_) => "boolean",
                        _ => "string",
                    };
                    out.push(json!({
                        "name": key, "type": ty, "required": true, "description": "", "example": val,
                    }));
                }
            }
        }
        _ => {}
    }
}

pub fn to_nei(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let (top_groups, root_name) = extra_build_tree(apis);
    let mut groups: Vec<Value> = Vec::new();
    let mut interfaces: Vec<Value> = Vec::new();
    let mut gid = 101i64;
    for g in &top_groups {
        let my_gid = gid;
        gid += 1;
        groups.push(json!({
            "id": my_gid,
            "name": g.name,
            "description": "",
            "parentId": 0,
        }));
        for api in &g.apis {
            interfaces.push(nei_api_out(api, my_gid));
        }
    }
    json!({
        "id": 1,
        "name": root_name,
        "description": "",
        "properties": { "baseUrl": "", "createTime": "" },
        "groups": groups,
        "datatypes": [],
        "interfaces": interfaces,
    })
}

fn nei_api_out(api: &ApiFile, group: i64) -> Value {
    let mut headers: Vec<Value> = Vec::new();
    for h in api.headers.iter().filter(|h| extra_kv_enabled(h)) {
        headers.push(json!({
            "name": h.key, "type": "string", "required": true,
            "description": h.description, "example": h.value,
        }));
    }
    let mut path_params: Vec<Value> = Vec::new();
    for p in api.params.iter().filter(|p| extra_kv_enabled(p)) {
        path_params.push(json!({
            "name": p.key, "type": "string", "required": true,
            "description": p.description, "example": p.value,
        }));
    }
    let mut query_params: Vec<Value> = Vec::new();
    for q in api.query.iter().filter(|q| extra_kv_enabled(q)) {
        query_params.push(json!({
            "name": q.key, "type": "string", "required": false,
            "description": q.description, "example": q.value,
        }));
    }
    let mut body_params: Vec<Value> = Vec::new();
    let body_type = if api.body.mode == "json" {
        if let Ok(v) = serde_json::from_str::<Value>(&api.body.raw) {
            nei_json_to_params("", &v, &mut body_params);
        }
        "json"
    } else if api.body.mode == "form" {
        for f in api.body.form.iter().filter(|f| extra_kv_enabled(f)) {
            body_params.push(json!({
                "name": f.key, "type": "string", "required": true,
                "description": f.description, "example": f.value,
            }));
        }
        "form"
    } else {
        "none"
    };
    let mut responses: Vec<Value> = Vec::new();
    for r in &api.responses {
        if r.body.trim().is_empty() {
            continue;
        }
        let example = serde_json::from_str::<Value>(&r.body).unwrap_or(Value::Null);
        responses.push(json!({
            "status": r.status,
            "description": r.name,
            "body": { "type": "json", "example": example },
        }));
    }
    json!({
        "id": 0,
        "name": api.name,
        "description": api.description,
        "group": group,
        "url": api.path,
        "method": api.method.to_uppercase(),
        "status": 1,
        "request": {
            "headers": headers,
            "pathParams": path_params,
            "queryParams": query_params,
            "body": { "type": body_type, "params": body_params },
        },
        "responses": responses,
    })
}

// ---------- DOClever ----------

pub fn to_doclever(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let (top_groups, _) = extra_build_tree(apis);
    let mut arr: Vec<Value> = Vec::new();
    let mut sort = 1i64;
    for g in &top_groups {
        arr.push(json!({
            "id": format!("folder_{sort}"),
            "name": g.name,
            "desc": "",
            "folder": true,
            "sort": sort,
            "children": [],
        }));
        for api in &g.apis {
            arr.push(doclever_api_out(api, sort));
            sort += 1;
        }
        sort += 1;
    }
    Value::Array(arr)
}

fn doclever_api_out(api: &ApiFile, sort: i64) -> Value {
    let mut headers: Vec<Value> = Vec::new();
    for h in api.headers.iter().filter(|h| extra_kv_enabled(h)) {
        headers.push(json!({
            "name": h.key, "value": h.value, "required": true, "desc": h.description,
        }));
    }
    let mut query_params: Vec<Value> = Vec::new();
    for q in api.query.iter().filter(|q| extra_kv_enabled(q)) {
        query_params.push(json!({ "name": q.key, "value": q.value, "desc": q.description }));
    }
    let body_info = match api.body.mode.as_str() {
        "form" => {
            let mut params: Vec<Value> = Vec::new();
            for f in api.body.form.iter().filter(|f| extra_kv_enabled(f)) {
                params.push(json!({
                    "name": f.key, "type": "String", "required": true,
                    "desc": f.description, "example": f.value, "range": [],
                }));
            }
            json!({ "bodyType": "form", "raw": "", "params": params })
        }
        _ => json!({
            "bodyType": "raw",
            "raw": api.body.raw,
            "params": [],
        }),
    };
    let mut responses: Vec<Value> = Vec::new();
    for r in &api.responses {
        if r.body.trim().is_empty() {
            continue;
        }
        responses.push(json!({
            "code": r.status,
            "name": r.name,
            "body": r.body,
        }));
    }
    json!({
        "id": format!("api_{}", uuid::Uuid::new_v4().simple()),
        "name": api.name,
        "desc": api.description,
        "path": api.path,
        "method": api.method.to_uppercase(),
        "status": 1,
        "sort": sort,
        "folder": false,
        "baseUrl": "",
        "inject": "",
        "headers": headers,
        "params": [],
        "queryParams": query_params,
        "bodyInfo": body_info,
        "responseParams": [],
        "mock": {},
        "children": [],
    })
}

// ---------- IO-Docs ----------

pub fn to_io_docs(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let (top_groups, root_name) = extra_build_tree(apis);
    let mut resources = serde_json::Map::new();
    for g in &top_groups {
        let mut methods = serde_json::Map::new();
        for api in &g.apis {
            let mkey = format!("{}_{}", api.method.to_lowercase(), api.path.replace('/', "_").replace('{', "").replace('}', ""));
            methods.insert(mkey, io_docs_api_out(api));
        }
        resources.insert(
            g.name.clone(),
            json!({ "description": "", "methods": methods }),
        );
    }
    json!({
        "name": root_name,
        "protocol": "https",
        "basePath": "/",
        "publicPath": [],
        "privatePath": [],
        "auth": { "oauth2": { "flows": [] } },
        "resources": resources,
    })
}

fn io_docs_api_out(api: &ApiFile) -> Value {
    let mut headers_out: Vec<Value> = Vec::new();
    for h in api.headers.iter().filter(|h| extra_kv_enabled(h)) {
        headers_out.push(json!({ "key": h.key, "value": h.value, "description": h.description }));
    }
    let mut parameters = serde_json::Map::new();
    let is_body = matches!(api.method.to_uppercase().as_str(), "POST" | "PUT" | "PATCH");
    if is_body && api.body.mode == "json" {
        if let Ok(v) = serde_json::from_str::<Value>(&api.body.raw) {
            if let Value::Object(m) = v {
                for (k, val) in m {
                    let (ty, default) = match &val {
                        Value::Number(n) if n.as_i64().is_some() => ("integer", Value::from(0)),
                        Value::Number(_) => ("number", Value::from(0)),
                        Value::Bool(_) => ("boolean", Value::Bool(false)),
                        Value::Array(_) => ("array", Value::Array(vec![])),
                        _ => ("string", Value::String(String::new())),
                    };
                    parameters.insert(k, json!({ "type": ty, "required": true, "default": default, "description": "" }));
                }
            }
        }
    } else {
        for q in api.query.iter().filter(|q| extra_kv_enabled(q)) {
            parameters.insert(
                q.key.clone(),
                json!({ "type": "string", "required": false, "default": q.value, "description": q.description }),
            );
        }
    }
    json!({
        "name": api.name,
        "description": api.description,
        "httpMethod": api.method.to_uppercase(),
        "path": api.path,
        "requiresOAuth": false,
        "headers": headers_out,
        "parameters": parameters,
    })
}

// ---------- EasyDoc ----------

pub fn to_easydoc(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let (top_groups, root_name) = extra_build_tree(apis);
    let mut catalog: Vec<Value> = Vec::new();
    let mut api_list: Vec<Value> = Vec::new();
    let mut cat_id = 501i64;
    for g in &top_groups {
        let my_id = cat_id;
        cat_id += 1;
        catalog.push(json!({
            "id": my_id,
            "parent_id": 0,
            "title": g.name,
            "sort": my_id - 500,
            "children": [],
        }));
        for api in &g.apis {
            api_list.push(easydoc_api_out(api, my_id));
        }
    }
    json!({
        "code": 0,
        "msg": "ok",
        "data": {
            "project_id": 1,
            "name": root_name,
            "description": "",
            "create_time": "",
            "update_time": "",
            "base_url": "",
            "catalog": catalog,
            "api_list": api_list,
        },
    })
}

fn easydoc_api_out(api: &ApiFile, cat_id: i64) -> Value {
    let mut headers: Vec<Value> = Vec::new();
    for h in api.headers.iter().filter(|h| extra_kv_enabled(h)) {
        headers.push(json!({
            "name": h.key, "value": h.value, "required": 1, "desc": h.description,
        }));
    }
    let mut req_params: Vec<Value> = Vec::new();
    for q in api.query.iter().filter(|q| extra_kv_enabled(q)) {
        req_params.push(json!({
            "name": q.key, "type": "string", "required": 1, "desc": q.description, "default": q.value,
        }));
    }
    let mut response_params: Vec<Value> = Vec::new();
    let (mut response_demo, mut error_demo) = (String::new(), String::new());
    for r in &api.responses {
        if r.body.trim().is_empty() {
            continue;
        }
        if r.status == 0 || r.status >= 400 {
            if error_demo.is_empty() {
                error_demo = r.body.clone();
            }
        } else if response_demo.is_empty() {
            response_demo = r.body.clone();
        }
    }
    // response_params 从第一个成功响应解析
    if !response_demo.is_empty() {
        if let Ok(v) = serde_json::from_str::<Value>(&response_demo) {
            let mut tmp: Vec<Value> = Vec::new();
            nei_json_to_params("", &v, &mut tmp);
            for t in tmp {
                response_params.push(json!({
                    "name": t["name"], "type": t["type"], "required": 1,
                    "desc": "", "example": t["example"],
                }));
            }
        }
    }
    let (req_body, req_form, body_type) = match api.body.mode.as_str() {
        "form" => {
            let mut form: Vec<Value> = Vec::new();
            for f in api.body.form.iter().filter(|f| extra_kv_enabled(f)) {
                form.push(json!({ "name": f.key, "value": f.value, "desc": f.description }));
            }
            (String::new(), form, "form".to_string())
        }
        _ => (api.body.raw.clone(), Vec::<Value>::new(), "raw".to_string()),
    };
    json!({
        "id": 0,
        "catalog_id": cat_id,
        "title": api.name,
        "desc": api.description,
        "method": api.method.to_uppercase(),
        "url": api.path,
        "request_type": "application/json",
        "response_type": "application/json",
        "mock_open": 0,
        "mock_url": "",
        "request_headers": headers,
        "request_params": req_params,
        "request_body": req_body,
        "request_body_type": body_type,
        "request_form": req_form,
        "response_params": response_params,
        "response_demo": response_demo,
        "error_demo": error_demo,
        "create_time": "",
        "update_time": "",
        "sort": 0,
    })
}

// ---------- DocWay ----------

/// json → docway 字段列表（点分 key + example）
fn docway_json_to_params(prefix: &str, v: &Value, out: &mut Vec<Value>) {
    match v {
        Value::Object(m) => {
            for (k, val) in m {
                let key = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
                if val.is_object() {
                    out.push(json!({
                        "name": key, "type": "object", "required": true, "description": "", "example": json!({}),
                    }));
                    docway_json_to_params(&key, val, out);
                } else if val.is_array() {
                    out.push(json!({
                        "name": key, "type": "array", "required": true, "description": "", "example": val,
                    }));
                } else {
                    let ty = match val {
                        Value::Number(n) if n.as_i64().is_some() => "int",
                        Value::Number(_) => "float",
                        Value::Bool(_) => "boolean",
                        _ => "string",
                    };
                    out.push(json!({
                        "name": key, "type": ty, "required": true, "description": "", "example": val,
                    }));
                }
            }
        }
        _ => {}
    }
}

pub fn to_docway(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let (top_groups, root_name) = extra_build_tree(apis);
    let mut docs: Vec<Value> = Vec::new();
    for g in &top_groups {
        let mut children: Vec<Value> = Vec::new();
        for api in &g.apis {
            children.push(docway_api_out(api));
        }
        docs.push(json!({ "name": g.name, "children": children }));
    }
    json!({ "name": root_name, "description": "", "docs": docs })
}

fn docway_api_out(api: &ApiFile) -> Value {
    let mut headers: Vec<Value> = Vec::new();
    for h in api.headers.iter().filter(|h| extra_kv_enabled(h)) {
        headers.push(json!({
            "name": h.key, "value": h.value, "required": true, "description": h.description,
        }));
    }
    let mut req_params: Vec<Value> = Vec::new();
    if api.body.mode == "json" {
        if let Ok(v) = serde_json::from_str::<Value>(&api.body.raw) {
            docway_json_to_params("", &v, &mut req_params);
        }
    }
    let mut resp_params: Vec<Value> = Vec::new();
    for r in &api.responses {
        if r.body.trim().is_empty() {
            continue;
        }
        if r.status == 0 || r.status >= 400 {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(&r.body) {
            let mut tmp: Vec<Value> = Vec::new();
            docway_json_to_params("", &v, &mut tmp);
            for t in tmp {
                resp_params.push(json!({
                    "name": t["name"], "type": t["type"], "required": true,
                    "description": "", "example": t["example"],
                }));
            }
        }
        break;
    }
    json!({
        "name": api.name,
        "method": api.method.to_uppercase(),
        "url": api.path,
        "description": api.description,
        "requestHeaders": headers,
        "requestParams": req_params,
        "responseParams": resp_params,
    })
}

// ---------- Hoppscotch ----------

pub fn to_hoppscotch(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let (top_groups, root_name) = extra_build_tree(apis);
    let mut folders: Vec<Value> = Vec::new();
    for g in &top_groups {
        let mut requests: Vec<Value> = Vec::new();
        for api in &g.apis {
            requests.push(hoppscotch_req_out(api));
        }
        folders.push(json!({
            "name": g.name,
            "description": "",
            "folders": [],
            "requests": requests,
        }));
    }
    json!({
        "v": "1.0",
        "name": root_name,
        "description": "",
        "auth": { "authType": "none", "authActive": false },
        "headers": [],
        "folders": folders,
        "requests": [],
    })
}

fn hoppscotch_req_out(api: &ApiFile) -> Value {
    let mut headers: Vec<Value> = Vec::new();
    for h in api.headers.iter().filter(|h| extra_kv_enabled(h)) {
        headers.push(json!({ "key": h.key, "value": h.value, "active": true }));
    }
    let mut params: Vec<Value> = Vec::new();
    for q in api.query.iter().filter(|q| extra_kv_enabled(q)) {
        params.push(json!({ "key": q.key, "value": q.value, "active": true }));
    }
    let body = match api.body.mode.as_str() {
        "form" => {
            let mut fd: Vec<Value> = Vec::new();
            for f in api.body.form.iter().filter(|f| extra_kv_enabled(f)) {
                fd.push(json!({
                    "key": f.key, "value": f.value, "active": true,
                    "type": if f.is_file { "file" } else { "text" },
                }));
            }
            json!({ "mode": "formdata", "formdata": fd })
        }
        _ => {
            if api.body.raw.is_empty() {
                json!({ "mode": "none" })
            } else {
                json!({ "mode": "raw", "raw": api.body.raw })
            }
        }
    };
    json!({
        "name": api.name,
        "method": api.method.to_uppercase(),
        "endpoint": format!("<<base_url>>{}", api.path),
        "params": params,
        "headers": headers,
        "body": body,
        "preRequestScript": "",
        "testScript": "",
        "auth": { "authActive": false, "authType": "none" },
    })
}

// ---------- MeterSphere ----------

pub fn to_metersphere(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let (top_groups, root_name) = extra_build_tree(apis);
    let mut node_tree: Vec<Value> = Vec::new();
    let mut data: Vec<Value> = Vec::new();
    let mut mod_id = 1i64;
    for g in &top_groups {
        let my_id = format!("mod-{mod_id}");
        mod_id += 1;
        node_tree.push(json!({
            "id": my_id,
            "name": g.name,
            "sort": mod_id - 1,
            "children": [],
        }));
        for api in &g.apis {
            data.push(metersphere_api_out(api, &my_id));
        }
    }
    json!({
        "projectName": root_name,
        "projectId": "project_1",
        "protocol": "http",
        "version": "1.0",
        "nodeTree": node_tree,
        "data": data,
        "cases": [],
        "mocks": [],
    })
}

fn metersphere_api_out(api: &ApiFile, module_id: &str) -> Value {
    let mut headers: Vec<Value> = Vec::new();
    for h in api.headers.iter().filter(|h| extra_kv_enabled(h)) {
        headers.push(json!({ "key": h.key, "value": h.value, "enable": true }));
    }
    let mut query: Vec<Value> = Vec::new();
    for q in api.query.iter().filter(|q| extra_kv_enabled(q)) {
        query.push(json!({ "key": q.key, "value": q.value, "enable": true }));
    }
    let body_type = if api.body.mode == "form" { "FORM_DATA" } else { "JSON" };
    let response = api
        .responses
        .iter()
        .find(|r| !r.body.trim().is_empty())
        .map(|r| json!({ "bodyType": "JSON", "raw": r.body }))
        .unwrap_or_else(|| json!({ "bodyType": "JSON", "raw": "" }));
    json!({
        "id": format!("api-{}", uuid::Uuid::new_v4().simple()),
        "name": api.name,
        "method": api.method.to_uppercase(),
        "path": api.path,
        "moduleId": module_id,
        "description": api.description,
        "status": "UNDONE",
        "request": {
            "headers": headers,
            "query": query,
            "body": { "bodyType": body_type, "raw": api.body.raw },
        },
        "response": response,
    })
}

// ---------- 统一导出入口 ----------

/// 返回 (文件内容, 默认文件名, 扩展名)

// ---------- RAP2 ----------

/// ApiFile → rap2 属性列表（scope request/response，parentId 嵌套）
fn rap2_props_from_value(
    prefix: &str,
    v: &Value,
    scope: &str,
    out: &mut Vec<Value>,
    parent_id: i64,
    counter: &mut i64,
) -> i64 {
    match v {
        Value::Object(m) => {
            let my_id = *counter;
            *counter += 1;
            out.push(json!({
                "id": my_id, "scope": scope, "pos": 1, "name": prefix,
                "type": "Object", "required": true, "value": "",
                "description": "", "parentId": parent_id, "priority": 1,
            }));
            for (k, val) in m {
                rap2_props_from_value(k, val, scope, out, my_id, counter);
            }
            my_id
        }
        Value::Array(arr) => {
            let my_id = *counter;
            *counter += 1;
            out.push(json!({
                "id": my_id, "scope": scope, "pos": 1, "name": prefix,
                "type": "Array", "required": true, "value": "",
                "description": "", "parentId": parent_id, "priority": 1,
            }));
            if let Some(first) = arr.first() {
                if first.is_object() {
                    for (k, val) in first.as_object().unwrap() {
                        rap2_props_from_value(k, val, scope, out, my_id, counter);
                    }
                }
            }
            my_id
        }
        Value::Number(n) => {
            let id = *counter;
            *counter += 1;
            out.push(json!({
                "id": id, "scope": scope, "pos": 1, "name": prefix,
                "type": if n.as_i64().is_some() { "Number" } else { "Float" },
                "required": true, "value": n, "description": "", "parentId": parent_id, "priority": 1,
            }));
            id
        }
        Value::Bool(b) => {
            let id = *counter;
            *counter += 1;
            out.push(json!({
                "id": id, "scope": scope, "pos": 1, "name": prefix,
                "type": "Boolean", "required": true, "value": b,
                "description": "", "parentId": parent_id, "priority": 1,
            }));
            id
        }
        _ => {
            let id = *counter;
            *counter += 1;
            out.push(json!({
                "id": id, "scope": scope, "pos": 1, "name": prefix,
                "type": "String", "required": true,
                "value": if v.is_null() { "" } else { v.as_str().unwrap_or("") },
                "description": "", "parentId": parent_id, "priority": 1,
            }));
            id
        }
    }
}

/// 生成接口的 properties（request：headers/query/params/body；response：第一个成功响应）
fn rap2_api_properties(api: &ApiFile) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    let mut counter: i64 = 1;
    for h in api.headers.iter().filter(|h| extra_kv_enabled(h)) {
        out.push(json!({
            "id": counter, "scope": "request", "pos": 1, "name": h.key,
            "type": "String", "required": true, "value": h.value,
            "description": h.description, "parentId": -1, "priority": out.len() as i64 + 1,
        }));
        counter += 1;
    }
    for q in api.query.iter().filter(|q| extra_kv_enabled(q)) {
        out.push(json!({
            "id": counter, "scope": "request", "pos": 2, "name": q.key,
            "type": "String", "required": true, "value": q.value,
            "description": q.description, "parentId": -1, "priority": out.len() as i64 + 1,
        }));
        counter += 1;
    }
    for p in api.params.iter().filter(|p| extra_kv_enabled(p)) {
        out.push(json!({
            "id": counter, "scope": "request", "pos": 3, "name": p.key,
            "type": "String", "required": true, "value": p.value,
            "description": p.description, "parentId": -1, "priority": out.len() as i64 + 1,
        }));
        counter += 1;
    }
    if api.body.mode == "json" {
        if let Ok(v) = serde_json::from_str::<Value>(&api.body.raw) {
            let ty = if v.is_object() { "Object" } else { "Array" };
            out.push(json!({
                "id": counter, "scope": "request", "pos": 4, "name": "body",
                "type": ty, "required": true, "value": "",
                "description": "", "parentId": -1, "priority": out.len() as i64 + 1,
            }));
            counter += 1;
            match &v {
                Value::Object(m) => {
                    for (k, val) in m {
                        rap2_props_from_value(k, val, "request", &mut out, counter - 1, &mut counter);
                    }
                }
                Value::Array(arr) => {
                    if let Some(first) = arr.first() {
                        if first.is_object() {
                            for (k, val) in first.as_object().unwrap() {
                                rap2_props_from_value(k, val, "request", &mut out, counter - 1, &mut counter);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    if let Some(r) = api.responses.iter().find(|r| !r.body.trim().is_empty()) {
        if let Ok(v) = serde_json::from_str::<Value>(&r.body) {
            match &v {
                Value::Object(m) => {
                    for (k, val) in m {
                        rap2_props_from_value(k, val, "response", &mut out, -1, &mut counter);
                    }
                }
                Value::Array(arr) => {
                    if let Some(first) = arr.first() {
                        if first.is_object() {
                            for (k, val) in first.as_object().unwrap() {
                                rap2_props_from_value(k, val, "response", &mut out, -1, &mut counter);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// 单接口 → rap2 interface 对象
fn rap2_interface_out(api: &ApiFile) -> Value {
    json!({
        "id": 0,
        "name": api.name,
        "url": api.path,
        "method": api.method.to_uppercase(),
        "status": "draft",
        "description": api.description,
        "priority": 0,
        "moduleId": -1,
        "repositoryId": -1,
        "creatorId": -1,
        "lockerId": -1,
        "createdAt": "",
        "updatedAt": "",
        "properties": rap2_api_properties(api),
    })
}

/// 项目格式：分组 → modules[]
pub fn to_rap2_project(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let (top_groups, root_name) = extra_build_tree(apis);
    let mut modules: Vec<Value> = Vec::new();
    let mut repo_id = 0i64;
    for g in &top_groups {
        repo_id += 1;
        let mut interfaces: Vec<Value> = Vec::new();
        for api in &g.apis {
            interfaces.push(rap2_interface_out(api));
        }
        modules.push(json!({
            "id": 9000 + repo_id,
            "name": g.name,
            "description": "",
            "priority": repo_id,
            "repositoryId": repo_id,
            "interfaces": interfaces,
        }));
    }
    json!({
        "data": {
            "id": 1,
            "name": root_name,
            "description": "",
            "logo": "",
            "token": "",
            "visibility": "public",
            "createdAt": "",
            "updatedAt": "",
            "modules": modules,
        }
    })
}
