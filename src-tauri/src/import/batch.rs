//! 由 import.rs 拆分：批量 10 格式 + RAP2 + apiDoc
use super::*;
#[allow(unused_imports)]
use crate::{ApiFile, BodyData, DocParam, EnvVariable, KeyValue, MockConfig, ResponseItem, sanitize_filename, unique_path, workspace_root, ENV_FILE, INFO_FILE};
#[allow(unused_imports)]
use serde::Serialize;
#[allow(unused_imports)]
use serde_json::Value;
#[allow(unused_imports)]
use std::collections::{BTreeMap, HashMap};
#[allow(unused_imports)]
use std::fs;
#[allow(unused_imports)]
use std::path::{Path, PathBuf};

/// 通用变量替换：{{key}} 与 ${key}
fn var_replace(s: &str, vars: &HashMap<String, String>) -> String {
    let mut out = s.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("{{{{{k}}}}}"), v).replace(&format!("${{{k}}}"), v);
    }
    out
}

fn str_field(v: &Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").trim().to_string()
}

fn map_str(m: &serde_json::Map<String, Value>, key: &str) -> String {
    m.get(key).and_then(|x| x.as_str()).unwrap_or("").trim().to_string()
}

/// jsonSchema → 示例值
fn schema_to_value(schema: &Value) -> Value {
    let ty = str_field(schema, "type");
    match ty.as_str() {
        "object" => {
            let mut m = serde_json::Map::new();
            if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
                for (k, pv) in props {
                    m.insert(k.clone(), schema_to_value(pv));
                }
            }
            Value::Object(m)
        }
        "array" => {
            let items = schema.get("items").unwrap_or(&Value::Null);
            let item_v = if items.is_null() { Value::String(String::new()) } else { schema_to_value(items) };
            Value::Array(vec![item_v])
        }
        "integer" | "number" => Value::from(0),
        "boolean" => Value::Bool(false),
        _ => Value::String(String::new()),
    }
}

/// 建分组目录 + INFO
fn mk_group_dir(parent: &Path, name: &str, desc: &str) -> Result<PathBuf, String> {
    let sub = parent.join(sanitize_filename(name));
    if !sub.is_dir() {
        fs::create_dir_all(&sub).map_err(|e| format!("创建分组失败: {e}"))?;
    }
    write_pretty(
        &sub.join(INFO_FILE),
        &InfoJson {
            name: Some(name.to_string()),
            description: desc.to_string(),
            base_url: None,
            mock_port: None,
            collapsed: None,
            deprecated: None,
            dirs: vec![],
            apis: vec![],
        },
    )?;
    // 追加到父分组 __info.json 的 dirs（导入顺序即显示顺序）
    let dir_name = sub
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    crate::info_append_child(parent, &dir_name, true);
    Ok(sub)
}

fn kv_of(v: &Value, key_k: &str, value_k: &str, desc_k: Option<&str>, enabled_k: Option<&str>) -> KeyValue {
    KeyValue {
        key: str_field(v, key_k),
        value: str_field(v, value_k),
        enabled: enabled_k.map(|ek| v.get(ek).and_then(|x| x.as_bool()).unwrap_or(true)).unwrap_or(true),
        is_file: false,
        description: desc_k.map(|dk| str_field(v, dk)).unwrap_or_default(),
    }
}

fn resp_item(status: i64, name: &str, body: &str) -> ResponseItem {
    ResponseItem {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        status: status.try_into().unwrap_or(200),
        content_type: "application/json".to_string(),
        body: body.to_string(),
    }
}

// ---------- apiDog ----------

fn import_apidog_files(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let v: Value = serde_json::from_str(&fs::read_to_string(file).map_err(|e| e.to_string())?)
        .map_err(|e| format!("解析 apiDog 文件失败: {e}"))?;
    let pm = v.get("projectMeta").cloned().unwrap_or(Value::Null);
    let name = str_field(&pm, "name");
    let folder = unique_path(root, &if name.is_empty() { "apiDog 导入".to_string() } else { name.clone() }, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    // base_url：environments[].variables 里 key == baseUrl
    let mut base_url: Option<String> = None;
    if let Some(envs) = v.get("environments").and_then(|x| x.as_array()) {
        'outer: for env in envs {
            if let Some(vars) = env.get("variables").and_then(|x| x.as_array()) {
                for var in vars {
                    if str_field(var, "key") == "baseUrl" {
                        let val = str_field(var, "value");
                        if !val.is_empty() && !val.starts_with("{{") {
                            base_url = Some(val);
                            break 'outer;
                        }
                    }
                }
            }
        }
    }
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some(name.clone()),
            description: str_field(&pm, "description"),
            base_url,
            mock_port: None,
            collapsed: None,
            deprecated: None,
            dirs: vec![],
            apis: vec![],
        },
    )?;
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    if let Some(folders) = v.get("folders").and_then(|x| x.as_array()) {
        for f in folders {
            let fname = str_field(f, "name");
            if fname.is_empty() {
                continue;
            }
            let sub = mk_group_dir(&folder, &fname, &str_field(f, "description"))?;
            if let Some(apis) = f.get("apis").and_then(|x| x.as_array()) {
                for a in apis {
                    count += apidog_api_to_api(&sub, a, &mut stats)?;
                }
            }
        }
    }
    Ok(OpenApiImportResult { folder: folder.to_string_lossy().to_string(), count, http: stats.http, ws: stats.ws, graphql: stats.graphql, socketio: stats.socketio, ..Default::default() })
}

fn apidog_api_to_api(dir: &Path, a: &Value,
    stats: &mut ImportStats) -> Result<usize, String> {
    let mut method = str_field(a, "method").to_uppercase();
    if method.is_empty() {
        method = "GET".to_string();
    }
    let raw_url = str_field(a, "path");
    if raw_url.is_empty() {
        return Ok(0);
    }
    let is_ws = raw_url.starts_with("ws://") || raw_url.starts_with("wss://");
    let protocol = if is_ws { "websocket".to_string() } else { "http".to_string() };
    let (path, params) = if is_ws { (raw_url.clone(), Vec::new()) } else { extract_path(&raw_url) };
    let name = str_field(a, "name");
    let name = if name.is_empty() { format!("{method} {path}") } else { name };
    let description = str_field(a, "description");
    let mut headers = Vec::new();
    let mut query = Vec::new();
    let mut body = BodyData::default();
    if let Some(req) = a.get("request").and_then(|x| x.as_object()) {
        if let Some(hs) = req.get("headers").and_then(|x| x.as_array()) {
            for h in hs {
                let kv = kv_of(h, "key", "value", Some("description"), None);
                if !kv.key.is_empty() {
                    headers.push(kv);
                }
            }
        }
        if let Some(qs) = req.get("query").and_then(|x| x.as_array()) {
            for q in qs {
                let kv = kv_of(q, "key", "value", Some("description"), None);
                if !kv.key.is_empty() {
                    query.push(kv);
                }
            }
        }
        if let Some(bd) = req.get("body").and_then(|x| x.as_object()) {
            let mode = map_str(bd, "mode");
            if mode == "json" {
                let example = bd.get("example").cloned().unwrap_or(Value::Null);
                let schema = bd.get("jsonSchema").cloned().unwrap_or(Value::Null);
                let val = if !example.is_null() {
                    example
                } else if !schema.is_null() {
                    schema_to_value(&schema)
                } else {
                    Value::Null
                };
                if !val.is_null() {
                    body.mode = "json".to_string();
                    body.raw = serde_json::to_string_pretty(&val).unwrap_or_default();
                }
            } else if mode == "raw" {
                let raw = map_str(bd, "raw");
                if !raw.is_empty() {
                    body.mode = "json".to_string();
                    body.raw = raw;
                }
            } else if mode == "form" || mode == "formdata" {
                let key = if mode == "form" { "form" } else { "formdata" };
                if let Some(fs) = bd.get(key).and_then(|x| x.as_array()) {
                    body.mode = "form".to_string();
                    for f in fs {
                        let mut kv = kv_of(f, "key", "value", Some("description"), None);
                        kv.is_file = str_field(f, "type") == "file";
                        if !kv.key.is_empty() {
                            body.form.push(kv);
                        }
                    }
                }
            }
        }
    }
    let mut responses = Vec::new();
    if let Some(rs) = a.get("responses").and_then(|x| x.as_array()) {
        for r in rs {
            let status = r.get("statusCode").and_then(|x| x.as_i64()).unwrap_or(200);
            let example = r.get("example").cloned().unwrap_or(Value::Null);
            let body_txt = if example.is_null() {
                String::new()
            } else {
                serde_json::to_string_pretty(&example).unwrap_or_default()
            };
            if !body_txt.is_empty() {
                responses.push(resp_item(status, &str_field(r, "description"), &body_txt));
            }
        }
    }
    let api = ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name,
        method,
        path,
        url: String::new(),
        description,
        headers,
        query,
        params,
        body,
        mock: MockConfig::default(),
        prescript: String::new(),
        examples: vec![],
        responses,
        doc_params: vec![],
        deprecated: false,
        protocol,
    };
    write_pretty(&unique_path(dir, &api.name, ".json"), &api)?;
        stats.add(&api.protocol);
    Ok(1)
}

// ---------- Bruno ----------

fn import_bruno_files(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let v: Value = serde_json::from_str(&fs::read_to_string(file).map_err(|e| e.to_string())?)
        .map_err(|e| format!("解析 Bruno 文件失败: {e}"))?;
    let info = v.get("info").cloned().unwrap_or(Value::Null);
    let name = str_field(&info, "name");
    let folder = unique_path(root, &if name.is_empty() { "Bruno 导入".to_string() } else { name.clone() }, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    // 环境变量（local 优先）
    let mut vars: HashMap<String, String> = HashMap::new();
    let mut base_url: Option<String> = None;
    if let Some(envs) = v.get("environments").and_then(|x| x.as_object()) {
        for env_name in ["local", "staging", "production"] {
            if let Some(env) = envs.get(env_name).and_then(|x| x.as_object()) {
                for (k, val) in env {
                    let sv = val.as_str().unwrap_or("").to_string();
                    if k == "base_url" && base_url.is_none() && !sv.is_empty() {
                        base_url = Some(sv.clone());
                    }
                    vars.insert(k.clone(), sv);
                }
                if !vars.is_empty() {
                    break;
                }
            }
        }
    }
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some(name.clone()),
            description: str_field(&info, "description"),
            base_url,
            mock_port: None,
            collapsed: None,
            deprecated: None,
            dirs: vec![],
            apis: vec![],
        },
    )?;
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    if let Some(folders) = v.get("folders").and_then(|x| x.as_array()) {
        for f in folders {
            let fname = str_field(&f.get("info").cloned().unwrap_or(Value::Null), "name");
            if fname.is_empty() {
                continue;
            }
            let sub = mk_group_dir(&folder, &fname, "")?;
            count += bruno_walk(&sub, f, &vars, &mut stats)?;
        }
    }
    Ok(OpenApiImportResult { folder: folder.to_string_lossy().to_string(), count, http: stats.http, ws: stats.ws, graphql: stats.graphql, socketio: stats.socketio, ..Default::default() })
}

/// 递归处理 bruno 分组（requests + 嵌套 folders）
fn bruno_walk(dir: &Path, f: &Value, vars: &HashMap<String, String>,
    stats: &mut ImportStats) -> Result<usize, String> {
    let mut count = 0usize;
    if let Some(reqs) = f.get("requests").and_then(|x| x.as_array()) {
        for r in reqs {
            count += bruno_req_to_api(dir, r, vars, stats)?;
        }
    }
    if let Some(subs) = f.get("folders").and_then(|x| x.as_array()) {
        for sf in subs {
            let fname = str_field(&sf.get("info").cloned().unwrap_or(Value::Null), "name");
            if fname.is_empty() {
                continue;
            }
            let sub = mk_group_dir(dir, &fname, "")?;
            count += bruno_walk(&sub, sf, vars, stats)?;
        }
    }
    Ok(count)
}

fn bruno_req_to_api(dir: &Path, r: &Value, vars: &HashMap<String, String>,
    stats: &mut ImportStats) -> Result<usize, String> {
    let info = r.get("info").cloned().unwrap_or(Value::Null);
    if str_field(&info, "type") == "graphql" {
        return Ok(0);
    }
    let http = r.get("http").cloned().unwrap_or(Value::Null);
    if http.is_null() {
        return Ok(0);
    }
    let mut method = str_field(&http, "method").to_uppercase();
    if method.is_empty() {
        method = "GET".to_string();
    }
    let raw_url = var_replace(&str_field(&http, "url"), vars);
    if raw_url.is_empty() {
        return Ok(0);
    }
    let is_ws = raw_url.starts_with("ws://") || raw_url.starts_with("wss://");
    let protocol = if is_ws { "websocket".to_string() } else { "http".to_string() };
    let (path, params) = if is_ws { (raw_url.clone(), Vec::new()) } else { extract_path(&raw_url) };
    let name = str_field(&info, "name");
    let name = if name.is_empty() { format!("{method} {path}") } else { name };
    // headers：object {k: v}
    let mut headers = Vec::new();
    if let Some(hs) = http.get("headers").and_then(|x| x.as_object()) {
        for (k, val) in hs {
            let sv = var_replace(val.as_str().unwrap_or(""), vars);
            headers.push(KeyValue {
                key: k.clone(),
                value: sv,
                enabled: true,
                is_file: false,
                description: String::new(),
            });
        }
    }
    let mut query = Vec::new();
    if let Some(qs) = http.get("query").and_then(|x| x.as_array()) {
        for q in qs {
            let kv = kv_of(q, "key", "value", None, None);
            if !kv.key.is_empty() {
                query.push(KeyValue { value: var_replace(&kv.value, vars), ..kv });
            }
        }
    }
    let mut body = BodyData::default();
    if let Some(bd) = http.get("body").and_then(|x| x.as_object()) {
        let ty = map_str(bd, "type");
        let data = var_replace(&map_str(bd, "data"), vars);
        if !data.is_empty() {
            if ty == "form-urlencoded" || ty == "multipart-form" {
                body.mode = "form".to_string();
                for kv in data.split('&') {
                    let mut it = kv.splitn(2, '=');
                    let k = it.next().unwrap_or("");
                    let val = it.next().unwrap_or("");
                    if !k.is_empty() {
                        body.form.push(KeyValue {
                            key: k.to_string(),
                            value: val.to_string(),
                            enabled: true,
                            is_file: false,
                            description: String::new(),
                        });
                    }
                }
            } else {
                body.mode = "json".to_string();
                body.raw = data;
            }
        }
    }
    let api = ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name,
        method,
        path,
        url: String::new(),
        description: str_field(&info, "description"),
        headers,
        query,
        params,
        body,
        mock: MockConfig::default(),
        prescript: String::new(),
        examples: vec![],
        responses: vec![],
        doc_params: vec![],
        deprecated: false,
        protocol,
    };
    write_pretty(&unique_path(dir, &api.name, ".json"), &api)?;
        stats.add(&api.protocol);
    Ok(1)
}

// ---------- Apizza ----------

fn import_apizza_files(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let v: Value = serde_json::from_str(&fs::read_to_string(file).map_err(|e| e.to_string())?)
        .map_err(|e| format!("解析 Apizza 文件失败: {e}"))?;
    let name = str_field(&v, "projectName");
    let folder = unique_path(root, &if name.is_empty() { "Apizza 导入".to_string() } else { name.clone() }, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    // envs[0].variables → 变量；host → base_url
    let mut vars: HashMap<String, String> = HashMap::new();
    let mut base_url: Option<String> = None;
    if let Some(envs) = v.get("envs").and_then(|x| x.as_array()) {
        if let Some(env) = envs.first() {
            if let Some(vars_arr) = env.get("variables").and_then(|x| x.as_array()) {
                for var in vars_arr {
                    let key = str_field(var, "key");
                    let val = str_field(var, "value");
                    if key == "host" && base_url.is_none() && !val.is_empty() {
                        base_url = Some(val.clone());
                    }
                    vars.insert(key, val);
                }
            }
        }
    }
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some(name.clone()),
            description: str_field(&v, "projectDesc"),
            base_url,
            mock_port: None,
            collapsed: None,
            deprecated: None,
            dirs: vec![],
            apis: vec![],
        },
    )?;
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    if let Some(folders) = v.get("folders").and_then(|x| x.as_array()) {
        count += apizza_walk(&folder, folders, &vars, &mut stats)?;
    }
    Ok(OpenApiImportResult { folder: folder.to_string_lossy().to_string(), count, http: stats.http, ws: stats.ws, graphql: stats.graphql, socketio: stats.socketio, ..Default::default() })
}

fn apizza_walk(dir: &Path, folders: &[Value], vars: &HashMap<String, String>,
    stats: &mut ImportStats) -> Result<usize, String> {
    let mut count = 0usize;
    for f in folders {
        let fname = str_field(f, "folderName");
        if fname.is_empty() {
            continue;
        }
        let sub = mk_group_dir(dir, &fname, &str_field(f, "folderDesc"))?;
        if let Some(apis) = f.get("apis").and_then(|x| x.as_array()) {
            for a in apis {
                count += apizza_api_to_api(&sub, a, vars, stats)?;
            }
        }
        if let Some(ch) = f.get("children").and_then(|x| x.as_array()) {
            count += apizza_walk(&sub, ch, vars, stats)?;
        }
    }
    Ok(count)
}

fn apizza_api_to_api(dir: &Path, a: &Value, vars: &HashMap<String, String>,
    stats: &mut ImportStats) -> Result<usize, String> {
    let mut method = str_field(a, "method").to_uppercase();
    if method.is_empty() {
        method = "GET".to_string();
    }
    let raw_url = var_replace(&str_field(a, "url"), vars);
    if raw_url.is_empty() {
        return Ok(0);
    }
    let is_ws = raw_url.starts_with("ws://") || raw_url.starts_with("wss://");
    let protocol = if is_ws { "websocket".to_string() } else { "http".to_string() };
    let (path, params) = if is_ws { (raw_url.clone(), Vec::new()) } else { extract_path(&raw_url) };
    let name = str_field(a, "apiName");
    let name = if name.is_empty() { format!("{method} {path}") } else { name };
    let mut headers = Vec::new();
    if let Some(hs) = a.get("headers").and_then(|x| x.as_array()) {
        for h in hs {
            let kv = kv_of(h, "key", "value", None, None);
            if !kv.key.is_empty() {
                headers.push(KeyValue { value: var_replace(&kv.value, vars), ..kv });
            }
        }
    }
    let mut query = Vec::new();
    if let Some(qs) = a.get("queryParams").and_then(|x| x.as_array()) {
        for q in qs {
            let kv = kv_of(q, "key", "value", Some("desc"), None);
            if !kv.key.is_empty() {
                query.push(KeyValue { value: var_replace(&kv.value, vars), ..kv });
            }
        }
    }
    let mut body = BodyData::default();
    let mode = str_field(a, "bodyMode");
    match mode.as_str() {
        "raw" | "json" => {
            let raw = str_field(a, "bodyRaw");
            if !raw.is_empty() {
                body.mode = "json".to_string();
                body.raw = raw;
            }
        }
        "form" => {
            if let Some(fs) = a.get("bodyForm").and_then(|x| x.as_array()) {
                body.mode = "form".to_string();
                for f in fs {
                    let kv = kv_of(f, "key", "value", Some("desc"), None);
                    if !kv.key.is_empty() {
                        body.form.push(kv);
                    }
                }
            }
        }
        "formdata" => {
            if let Some(fs) = a.get("bodyFormData").and_then(|x| x.as_array()) {
                body.mode = "form".to_string();
                for f in fs {
                    let mut kv = kv_of(f, "key", "value", Some("desc"), None);
                    kv.is_file = str_field(f, "type") == "file";
                    if !kv.key.is_empty() {
                        body.form.push(kv);
                    }
                }
            }
        }
        _ => {}
    }
    let mut responses = Vec::new();
    if let Some(rs) = a.get("responses").and_then(|x| x.as_array()) {
        for r in rs {
            let status = r.get("status").and_then(|x| x.as_i64()).unwrap_or(200);
            let body_txt = str_field(r, "body");
            if !body_txt.is_empty() {
                responses.push(resp_item(status, &str_field(r, "name"), &body_txt));
            }
        }
    }
    let api = ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name,
        method,
        path,
        url: String::new(),
        description: str_field(a, "apiDesc"),
        headers,
        query,
        params,
        body,
        mock: MockConfig::default(),
        prescript: String::new(),
        examples: vec![],
        responses,
        doc_params: vec![],
        deprecated: false,
        protocol,
    };
    write_pretty(&unique_path(dir, &api.name, ".json"), &api)?;
        stats.add(&api.protocol);
    Ok(1)
}

// ---------- NEI ----------

/// nei params 列表 → JSON 树（支持 items/ref，datatypes 用于解析引用，depth 防循环）
fn nei_params_to_value(params: &[Value], datatypes: &HashMap<i64, &Value>, depth: usize) -> Value {
    if depth > 4 {
        return Value::Null;
    }
    let mut m = serde_json::Map::new();
    for p in params {
        let pname = str_field(p, "name");
        if pname.is_empty() {
            continue;
        }
        // example 优先
        if let Some(ex) = p.get("example") {
            if !ex.is_null() {
                m.insert(pname.clone(), ex.clone());
                continue;
            }
        }
        let ty = str_field(p, "type");
        let val = match ty.as_str() {
            "array" => {
                let items = p.get("items").cloned().unwrap_or(Value::Null);
                let elem = nei_item_to_value(&items, datatypes, depth + 1);
                if elem.is_null() { Value::Array(vec![]) } else { Value::Array(vec![elem]) }
            }
            "ref" => {
                let ref_id = p.get("refId").and_then(|x| x.as_i64()).unwrap_or(0);
                if let Some(dt) = datatypes.get(&ref_id) {
                    if let Some(dparams) = dt.get("params").and_then(|x| x.as_array()) {
                        if !dparams.is_empty() {
                            nei_params_to_value(dparams, datatypes, depth + 1)
                        } else {
                            Value::Null
                        }
                    } else {
                        Value::Null
                    }
                } else {
                    Value::Null
                }
            }
            "object" => {
                let items = p.get("items").cloned().unwrap_or(Value::Null);
                nei_item_to_value(&items, datatypes, depth + 1)
            }
            "int" | "integer" | "long" | "number" | "float" | "double" => Value::from(0),
            "boolean" | "bool" => Value::Bool(false),
            _ => Value::String(String::new()),
        };
        m.insert(pname.clone(), val);
    }
    Value::Object(m)
}

fn nei_item_to_value(items: &Value, datatypes: &HashMap<i64, &Value>, depth: usize) -> Value {
    if items.is_null() {
        return Value::Null;
    }
    let ity = str_field(items, "type");
    match ity.as_str() {
        "ref" => {
            let ref_id = items.get("refId").and_then(|x| x.as_i64()).unwrap_or(0);
            if let Some(dt) = datatypes.get(&ref_id) {
                if let Some(dparams) = dt.get("params").and_then(|x| x.as_array()) {
                    nei_params_to_value(dparams, datatypes, depth)
                } else {
                    Value::Null
                }
            } else {
                Value::Null
            }
        }
        "object" => {
            if let Some(dparams) = items.get("params").and_then(|x| x.as_array()) {
                nei_params_to_value(dparams, datatypes, depth)
            } else {
                Value::Null
            }
        }
        "array" => {
            let sub = items.get("items").cloned().unwrap_or(Value::Null);
            let elem = nei_item_to_value(&sub, datatypes, depth + 1);
            if elem.is_null() { Value::Array(vec![]) } else { Value::Array(vec![elem]) }
        }
        "int" | "integer" | "long" | "number" | "float" | "double" => Value::from(0),
        "boolean" | "bool" => Value::Bool(false),
        _ => Value::String(String::new()),
    }
}

fn import_nei_files(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let v: Value = serde_json::from_str(&fs::read_to_string(file).map_err(|e| e.to_string())?)
        .map_err(|e| format!("解析 NEI 文件失败: {e}"))?;
    let name = str_field(&v, "name");
    let folder = unique_path(root, &if name.is_empty() { "NEI 导入".to_string() } else { name.clone() }, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    let base_url = v.get("properties").and_then(|p| p.get("baseUrl")).and_then(|x| x.as_str()).map(|s| s.to_string());
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some(name.clone()),
            description: str_field(&v, "description"),
            base_url,
            mock_port: None,
            collapsed: None,
            deprecated: None,
            dirs: vec![],
            apis: vec![],
        },
    )?;
    // datatypes 索引
    let mut datatypes: HashMap<i64, &Value> = HashMap::new();
    if let Some(dts) = v.get("datatypes").and_then(|x| x.as_array()) {
        for dt in dts {
            if let Some(id) = dt.get("id").and_then(|x| x.as_i64()) {
                datatypes.insert(id, dt);
            }
        }
    }
    // groups：parentId → 目录
    let mut group_dirs: HashMap<i64, PathBuf> = HashMap::new();
    let groups: Vec<&Value> = v
        .get("groups")
        .and_then(|x| x.as_array())
        .map(|gs| gs.iter().collect())
        .unwrap_or_default();
    for g in &groups {
        let parent = g.get("parentId").and_then(|x| x.as_i64()).unwrap_or(0);
        if parent == 0 {
            let gname = str_field(g, "name");
            if !gname.is_empty() {
                let sub = mk_group_dir(&folder, &gname, &str_field(g, "description"))?;
                if let Some(id) = g.get("id").and_then(|x| x.as_i64()) {
                    group_dirs.insert(id, sub);
                }
            }
        }
    }
    loop {
        let mut added = false;
        for g in &groups {
            let id = g.get("id").and_then(|x| x.as_i64());
            let parent = g.get("parentId").and_then(|x| x.as_i64()).unwrap_or(0);
            let Some(id) = id else { continue };
            if group_dirs.contains_key(&id) {
                continue;
            }
            if let Some(pdir) = group_dirs.get(&parent) {
                let gname = str_field(g, "name");
                if !gname.is_empty() {
                    let sub = mk_group_dir(pdir, &gname, &str_field(g, "description"))?;
                    group_dirs.insert(id, sub);
                    added = true;
                }
            }
        }
        if !added {
            break;
        }
    }
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    if let Some(ifs) = v.get("interfaces").and_then(|x| x.as_array()) {
        for it in ifs {
            let gid = it.get("group").and_then(|x| x.as_i64()).unwrap_or(0);
            let dir = group_dirs.get(&gid).cloned().unwrap_or_else(|| folder.clone());
            count += nei_api_to_api(&dir, it, &datatypes, &mut stats)?;
        }
    }
    Ok(OpenApiImportResult { folder: folder.to_string_lossy().to_string(), count, http: stats.http, ws: stats.ws, graphql: stats.graphql, socketio: stats.socketio, ..Default::default() })
}

fn nei_api_to_api(dir: &Path, it: &Value, datatypes: &HashMap<i64, &Value>,
    stats: &mut ImportStats) -> Result<usize, String> {
    let mut method = str_field(it, "method").to_uppercase();
    if method.is_empty() {
        method = "GET".to_string();
    }
    let raw_url = str_field(it, "url");
    if raw_url.is_empty() {
        return Ok(0);
    }
    let is_ws = raw_url.starts_with("ws://") || raw_url.starts_with("wss://");
    let protocol = if is_ws { "websocket".to_string() } else { "http".to_string() };
    let (path, mut params) = if is_ws { (raw_url.clone(), Vec::new()) } else { extract_path(&raw_url) };
    let name = str_field(it, "name");
    let name = if name.is_empty() { format!("{method} {path}") } else { name };
    let req = it.get("request").cloned().unwrap_or(Value::Null);
    let mut headers = Vec::new();
    if let Some(hs) = req.get("headers").and_then(|x| x.as_array()) {
        for h in hs {
            let key = str_field(h, "name");
            if key.is_empty() {
                continue;
            }
            let ex = h.get("example").map(|x| match x {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            }).unwrap_or_default();
            headers.push(KeyValue {
                key,
                value: ex,
                enabled: true,
                is_file: false,
                description: str_field(h, "description"),
            });
        }
    }
    let mut query = Vec::new();
    if let Some(qs) = req.get("queryParams").and_then(|x| x.as_array()) {
        for q in qs {
            let key = str_field(q, "name");
            if key.is_empty() {
                continue;
            }
            query.push(KeyValue {
                key,
                value: q.get("example").map(|x| x.to_string()).unwrap_or_default(),
                enabled: true,
                is_file: false,
                description: str_field(q, "description"),
            });
        }
    }
    if let Some(ps) = req.get("pathParams").and_then(|x| x.as_array()) {
        for p in ps {
            let key = str_field(p, "name");
            if let Some(kv) = params.iter_mut().find(|kv| kv.key == key) {
                kv.description = str_field(p, "description");
            } else if !key.is_empty() {
                params.push(KeyValue {
                    key,
                    value: String::new(),
                    enabled: true,
                    is_file: false,
                    description: str_field(p, "description"),
                });
            }
        }
    }
    let mut body = BodyData::default();
    if let Some(bd) = req.get("body").and_then(|x| x.as_object()) {
        let ty = map_str(bd, "type");
        if let Some(ps) = bd.get("params").and_then(|x| x.as_array()) {
            if !ps.is_empty() && ty != "none" {
                if ty == "form" {
                    body.mode = "form".to_string();
                    for p in ps {
                        let key = str_field(p, "name");
                        if key.is_empty() {
                            continue;
                        }
                        body.form.push(KeyValue {
                            key,
                            value: p.get("example").map(|x| x.to_string()).unwrap_or_default(),
                            enabled: true,
                            is_file: false,
                            description: str_field(p, "description"),
                        });
                    }
                } else {
                    let val = nei_params_to_value(ps, datatypes, 0);
                    if !val.is_null() {
                        body.mode = "json".to_string();
                        body.raw = serde_json::to_string_pretty(&val).unwrap_or_default();
                    }
                }
            }
        }
    }
    let mut responses = Vec::new();
    if let Some(rs) = it.get("responses").and_then(|x| x.as_array()) {
        for r in rs {
            let status = r.get("status").and_then(|x| x.as_i64()).unwrap_or(200);
            let mut body_txt = String::new();
            if let Some(ex) = r.get("body").and_then(|b| b.get("example")) {
                if !ex.is_null() {
                    body_txt = serde_json::to_string_pretty(ex).unwrap_or_default();
                }
            }
            if body_txt.is_empty() {
                if let Some(ps) = r.get("body").and_then(|b| b.get("params")).and_then(|x| x.as_array()) {
                    let val = nei_params_to_value(ps, datatypes, 0);
                    if !val.is_null() {
                        body_txt = serde_json::to_string_pretty(&val).unwrap_or_default();
                    }
                }
            }
            if !body_txt.is_empty() {
                responses.push(resp_item(status, &str_field(r, "description"), &body_txt));
            }
        }
    }
    let api = ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name,
        method,
        path,
        url: String::new(),
        description: str_field(it, "description"),
        headers,
        query,
        params,
        body,
        mock: MockConfig::default(),
        prescript: String::new(),
        examples: vec![],
        responses,
        doc_params: vec![],
        deprecated: false,
        protocol,
    };
    write_pretty(&unique_path(dir, &api.name, ".json"), &api)?;
        stats.add(&api.protocol);
    Ok(1)
}

// ---------- DOClever ----------

fn import_doclever_files(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let v: Value = serde_json::from_str(&fs::read_to_string(file).map_err(|e| e.to_string())?)
        .map_err(|e| format!("解析 DOClever 文件失败: {e}"))?;
    let arr = v.as_array().ok_or("DOClever 文件应为数组".to_string())?;
    let folder = unique_path(root, "DOClever 导入", "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some("DOClever 导入".to_string()),
            description: String::new(),
            base_url: None,
            mock_port: None,
            collapsed: None,
            deprecated: None,
            dirs: vec![],
            apis: vec![],
        },
    )?;
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    count += doclever_walk(&folder, arr, &mut stats)?;
    Ok(OpenApiImportResult { folder: folder.to_string_lossy().to_string(), count, http: stats.http, ws: stats.ws, graphql: stats.graphql, socketio: stats.socketio, ..Default::default() })
}

fn doclever_walk(dir: &Path, items: &[Value],
    stats: &mut ImportStats) -> Result<usize, String> {
    let mut count = 0usize;
    for it in items {
        let is_folder = it.get("folder").and_then(|x| x.as_bool()).unwrap_or(false);
        if is_folder {
            let fname = str_field(it, "name");
            if fname.is_empty() {
                continue;
            }
            let sub = mk_group_dir(dir, &fname, &str_field(it, "desc"))?;
            if let Some(ch) = it.get("children").and_then(|x| x.as_array()) {
                count += doclever_walk(&sub, ch, stats)?;
            }
        } else {
            count += doclever_api_to_api(dir, it, stats)?;
        }
    }
    Ok(count)
}

fn doclever_api_to_api(dir: &Path, a: &Value,
    stats: &mut ImportStats) -> Result<usize, String> {
    let mut method = str_field(a, "method").to_uppercase();
    if method.is_empty() {
        method = "GET".to_string();
    }
    let raw_url = str_field(a, "path");
    if raw_url.is_empty() {
        return Ok(0);
    }
    let is_ws = raw_url.starts_with("ws://") || raw_url.starts_with("wss://");
    let protocol = if is_ws { "websocket".to_string() } else { "http".to_string() };
    let (path, params) = if is_ws { (raw_url.clone(), Vec::new()) } else { extract_path(&raw_url) };
    let name = str_field(a, "name");
    let name = if name.is_empty() { format!("{method} {path}") } else { name };
    let mut headers = Vec::new();
    if let Some(hs) = a.get("headers").and_then(|x| x.as_array()) {
        for h in hs {
            let kv = kv_of(h, "name", "value", Some("desc"), None);
            if !kv.key.is_empty() {
                headers.push(kv);
            }
        }
    }
    let mut query = Vec::new();
    if let Some(qs) = a.get("queryParams").and_then(|x| x.as_array()) {
        for q in qs {
            let kv = kv_of(q, "name", "value", Some("desc"), None);
            if !kv.key.is_empty() {
                query.push(kv);
            }
        }
    }
    let mut body = BodyData::default();
    if let Some(bi) = a.get("bodyInfo").and_then(|x| x.as_object()) {
        let bt = map_str(bi, "bodyType");
        match bt.as_str() {
            "raw" | "json" => {
                let raw = map_str(bi, "raw");
                if !raw.is_empty() {
                    body.mode = "json".to_string();
                    body.raw = raw;
                }
            }
            "form" | "urlencoded" => {
                if let Some(ps) = bi.get("params").and_then(|x| x.as_array()) {
                    body.mode = "form".to_string();
                    for p in ps {
                        let kv = kv_of(p, "name", "value", Some("desc"), None);
                        if !kv.key.is_empty() {
                            body.form.push(kv);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    let mut responses = Vec::new();
    if let Some(rs) = a.get("responseDemo").and_then(|x| x.as_array()) {
        for r in rs {
            let status = r.get("code").and_then(|x| x.as_i64()).unwrap_or(200);
            let body_txt = str_field(r, "body");
            if !body_txt.is_empty() {
                responses.push(resp_item(status, &str_field(r, "name"), &body_txt));
            }
        }
    }
    let api = ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name,
        method,
        path,
        url: String::new(),
        description: str_field(a, "desc"),
        headers,
        query,
        params,
        body,
        mock: MockConfig::default(),
        prescript: String::new(),
        examples: vec![],
        responses,
        doc_params: vec![],
        deprecated: false,
        protocol,
    };
    write_pretty(&unique_path(dir, &api.name, ".json"), &api)?;
        stats.add(&api.protocol);
    Ok(1)
}

// ---------- IO-Docs ----------

fn import_io_docs_files(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let v: Value = serde_json::from_str(&fs::read_to_string(file).map_err(|e| e.to_string())?)
        .map_err(|e| format!("解析 IO-Docs 文件失败: {e}"))?;
    let name = str_field(&v, "name");
    let folder = unique_path(root, &if name.is_empty() { "IO-Docs 导入".to_string() } else { name.clone() }, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    let base_url = v.get("basePath").and_then(|x| x.as_str()).map(|s| s.to_string());
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some(name.clone()),
            description: String::new(),
            base_url,
            mock_port: None,
            collapsed: None,
            deprecated: None,
            dirs: vec![],
            apis: vec![],
        },
    )?;
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    if let Some(res) = v.get("resources").and_then(|x| x.as_object()) {
        for (rname, rv) in res {
            if rname.is_empty() {
                continue;
            }
            let sub = mk_group_dir(&folder, rname, &str_field(rv, "description"))?;
            if let Some(ms) = rv.get("methods").and_then(|x| x.as_object()) {
                for (_, mv) in ms {
                    count += io_docs_api_to_api(&sub, mv, &mut stats)?;
                }
            }
        }
    }
    Ok(OpenApiImportResult { folder: folder.to_string_lossy().to_string(), count, http: stats.http, ws: stats.ws, graphql: stats.graphql, socketio: stats.socketio, ..Default::default() })
}

fn io_docs_api_to_api(dir: &Path, a: &Value,
    stats: &mut ImportStats) -> Result<usize, String> {
    let mut method = str_field(a, "httpMethod").to_uppercase();
    if method.is_empty() {
        method = "GET".to_string();
    }
    let raw_url = str_field(a, "path");
    if raw_url.is_empty() {
        return Ok(0);
    }
    let (path, params) = extract_path(&raw_url);
    let name = str_field(a, "name");
    let name = if name.is_empty() { format!("{method} {path}") } else { name };
    let mut headers = Vec::new();
    if let Some(hs) = a.get("headers").and_then(|x| x.as_array()) {
        for h in hs {
            let kv = kv_of(h, "key", "value", Some("description"), None);
            if !kv.key.is_empty() {
                headers.push(kv);
            }
        }
    }
    let mut query = Vec::new();
    let mut body = BodyData::default();
    let is_body = matches!(method.as_str(), "POST" | "PUT" | "PATCH") && !str_field(a, "contentType").is_empty();
    if let Some(ps) = a.get("parameters").and_then(|x| x.as_object()) {
        if is_body {
            let mut m = serde_json::Map::new();
            for (k, pv) in ps {
                let val = match str_field(pv, "type").as_str() {
                    "integer" | "number" => Value::from(0),
                    "boolean" => Value::Bool(false),
                    "array" => Value::Array(vec![Value::String(String::new())]),
                    _ => Value::String(String::new()),
                };
                m.insert(k.clone(), val);
            }
            if !m.is_empty() {
                body.mode = "json".to_string();
                body.raw = serde_json::to_string_pretty(&Value::Object(m)).unwrap_or_default();
            }
        } else {
            for (k, pv) in ps {
                query.push(KeyValue {
                    key: k.clone(),
                    value: String::new(),
                    enabled: true,
                    is_file: false,
                    description: str_field(pv, "description"),
                });
            }
        }
    }
    let api = ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name,
        method,
        path,
        url: String::new(),
        description: str_field(a, "description"),
        headers,
        query,
        params,
        body,
        mock: MockConfig::default(),
        prescript: String::new(),
        examples: vec![],
        responses: vec![],
        doc_params: vec![],
        deprecated: false,
        protocol: "http".to_string(),
    };
    write_pretty(&unique_path(dir, &api.name, ".json"), &api)?;
        stats.add(&api.protocol);
    Ok(1)
}

// ---------- EasyDoc ----------

fn import_easydoc_files(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let v: Value = serde_json::from_str(&fs::read_to_string(file).map_err(|e| e.to_string())?)
        .map_err(|e| format!("解析 EasyDoc 文件失败: {e}"))?;
    let data = v.get("data").cloned().unwrap_or(Value::Null);
    let name = str_field(&data, "name");
    let folder = unique_path(root, &if name.is_empty() { "EasyDoc 导入".to_string() } else { name.clone() }, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    let base_url = data.get("base_url").and_then(|x| x.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string());
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some(name.clone()),
            description: str_field(&data, "description"),
            base_url,
            mock_port: None,
            collapsed: None,
            deprecated: None,
            dirs: vec![],
            apis: vec![],
        },
    )?;
    let mut cat_dirs: HashMap<i64, PathBuf> = HashMap::new();
    if let Some(cats) = data.get("catalog").and_then(|x| x.as_array()) {
        for c in cats {
            let cid = c.get("id").and_then(|x| x.as_i64()).unwrap_or(0);
            let cname = str_field(c, "title");
            if cid != 0 && !cname.is_empty() {
                let sub = mk_group_dir(&folder, &cname, "")?;
                cat_dirs.insert(cid, sub);
            }
        }
        loop {
            let mut added = false;
            for c in cats {
                let cid = c.get("id").and_then(|x| x.as_i64());
                let parent = c.get("parent_id").and_then(|x| x.as_i64()).unwrap_or(0);
                let Some(cid) = cid else { continue };
                if cat_dirs.contains_key(&cid) {
                    continue;
                }
                if let Some(pdir) = cat_dirs.get(&parent) {
                    let cname = str_field(c, "title");
                    if !cname.is_empty() {
                        let sub = mk_group_dir(pdir, &cname, "")?;
                        cat_dirs.insert(cid, sub);
                        added = true;
                    }
                }
            }
            if !added {
                break;
            }
        }
    }
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    if let Some(apis) = data.get("api_list").and_then(|x| x.as_array()) {
        for a in apis {
            let cid = a.get("catalog_id").and_then(|x| x.as_i64()).unwrap_or(0);
            let dir = cat_dirs.get(&cid).cloned().unwrap_or_else(|| folder.clone());
            count += easydoc_api_to_api(&dir, a, &mut stats)?;
        }
    }
    Ok(OpenApiImportResult { folder: folder.to_string_lossy().to_string(), count, http: stats.http, ws: stats.ws, graphql: stats.graphql, socketio: stats.socketio, ..Default::default() })
}

fn easydoc_api_to_api(dir: &Path, a: &Value,
    stats: &mut ImportStats) -> Result<usize, String> {
    let mut method = str_field(a, "method").to_uppercase();
    if method.is_empty() {
        method = "GET".to_string();
    }
    let raw_url = str_field(a, "url");
    if raw_url.is_empty() {
        return Ok(0);
    }
    let is_ws = raw_url.starts_with("ws://") || raw_url.starts_with("wss://");
    let protocol = if is_ws { "websocket".to_string() } else { "http".to_string() };
    let (path, params) = if is_ws { (raw_url.clone(), Vec::new()) } else { extract_path(&raw_url) };
    let name = str_field(a, "title");
    let name = if name.is_empty() { format!("{method} {path}") } else { name };
    let mut headers = Vec::new();
    if let Some(hs) = a.get("request_headers").and_then(|x| x.as_array()) {
        for h in hs {
            let kv = kv_of(h, "name", "value", Some("desc"), None);
            if !kv.key.is_empty() {
                headers.push(kv);
            }
        }
    }
    let mut query = Vec::new();
    if let Some(qs) = a.get("request_params").and_then(|x| x.as_array()) {
        for q in qs {
            let key = str_field(q, "name");
            if key.is_empty() {
                continue;
            }
            query.push(KeyValue {
                key,
                value: str_field(q, "default"),
                enabled: true,
                is_file: false,
                description: str_field(q, "desc"),
            });
        }
    }
    let mut body = BodyData::default();
    let rbt = str_field(a, "request_body_type");
    if let Some(rb) = a.get("request_body").and_then(|x| x.as_str()) {
        if !rb.is_empty() && rbt != "form" {
            body.mode = "json".to_string();
            body.raw = rb.to_string();
        }
    }
    if let Some(fs) = a.get("request_form").and_then(|x| x.as_array()) {
        if !fs.is_empty() {
            body.mode = "form".to_string();
            for f in fs {
                let kv = kv_of(f, "name", "value", Some("desc"), None);
                if !kv.key.is_empty() {
                    body.form.push(kv);
                }
            }
        }
    }
    let mut responses = Vec::new();
    for (key, status) in [("response_demo", 200i64), ("error_demo", 0i64)] {
        if let Some(demo) = a.get(key).and_then(|x| x.as_str()) {
            if !demo.is_empty() {
                let name = if status == 200 { "返回成功" } else { "返回失败" };
                responses.push(resp_item(status, name, demo));
            }
        }
    }
    let api = ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name,
        method,
        path,
        url: String::new(),
        description: str_field(a, "desc"),
        headers,
        query,
        params,
        body,
        mock: MockConfig::default(),
        prescript: String::new(),
        examples: vec![],
        responses,
        doc_params: vec![],
        deprecated: false,
        protocol,
    };
    write_pretty(&unique_path(dir, &api.name, ".json"), &api)?;
        stats.add(&api.protocol);
    Ok(1)
}

// ---------- DocWay ----------

fn import_docway_files(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let v: Value = serde_json::from_str(&fs::read_to_string(file).map_err(|e| e.to_string())?)
        .map_err(|e| format!("解析 DocWay 文件失败: {e}"))?;
    let name = str_field(&v, "name");
    let folder = unique_path(root, &if name.is_empty() { "DocWay 导入".to_string() } else { name.clone() }, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some(name.clone()),
            description: str_field(&v, "description"),
            base_url: None,
            mock_port: None,
            collapsed: None,
            deprecated: None,
            dirs: vec![],
            apis: vec![],
        },
    )?;
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    if let Some(docs) = v.get("docs").and_then(|x| x.as_array()) {
        for d in docs {
            let dname = str_field(d, "name");
            if dname.is_empty() {
                continue;
            }
            let sub = mk_group_dir(&folder, &dname, "")?;
            if let Some(ch) = d.get("children").and_then(|x| x.as_array()) {
                for c in ch {
                    if c.get("method").is_some() {
                        count += docway_api_to_api(&sub, c, &mut stats)?;
                    } else {
                        let cname = str_field(c, "name");
                        if !cname.is_empty() {
                            let sub2 = mk_group_dir(&sub, &cname, "")?;
                            if let Some(ch2) = c.get("children").and_then(|x| x.as_array()) {
                                for c2 in ch2 {
                                    count += docway_api_to_api(&sub2, c2, &mut stats)?;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(OpenApiImportResult { folder: folder.to_string_lossy().to_string(), count, http: stats.http, ws: stats.ws, graphql: stats.graphql, socketio: stats.socketio, ..Default::default() })
}

fn docway_api_to_api(dir: &Path, a: &Value,
    stats: &mut ImportStats) -> Result<usize, String> {
    let mut method = str_field(a, "method").to_uppercase();
    if method.is_empty() {
        method = "GET".to_string();
    }
    let raw_url = str_field(a, "url");
    if raw_url.is_empty() {
        return Ok(0);
    }
    let is_ws = raw_url.starts_with("ws://") || raw_url.starts_with("wss://");
    let protocol = if is_ws { "websocket".to_string() } else { "http".to_string() };
    let (path, params) = if is_ws { (raw_url.clone(), Vec::new()) } else { extract_path(&raw_url) };
    let name = str_field(a, "name");
    let name = if name.is_empty() { format!("{method} {path}") } else { name };
    let mut headers = Vec::new();
    if let Some(hs) = a.get("requestHeaders").and_then(|x| x.as_array()) {
        for h in hs {
            let kv = kv_of(h, "name", "value", Some("description"), None);
            if !kv.key.is_empty() {
                headers.push(kv);
            }
        }
    }
    let mut body = BodyData::default();
    // requestParams → body 字段（点分 → JSON 树）
    let mut root_v = serde_json::Map::new();
    if let Some(ps) = a.get("requestParams").and_then(|x| x.as_array()) {
        for p in ps {
            let key = str_field(p, "name");
            if key.is_empty() {
                continue;
            }
            let val = p.get("example").cloned().unwrap_or_else(|| Value::String(String::new()));
            let parts: Vec<&str> = key.split('.').collect();
            let mut cur = &mut root_v;
            for (i, part) in parts.iter().enumerate() {
                if i == parts.len() - 1 {
                    cur.insert((*part).to_string(), val.clone());
                } else {
                    cur = cur
                        .entry((*part).to_string())
                        .or_insert_with(|| Value::Object(serde_json::Map::new()))
                        .as_object_mut()
                        .unwrap();
                }
            }
        }
    }
    if !root_v.is_empty() {
        body.mode = "json".to_string();
        body.raw = serde_json::to_string_pretty(&Value::Object(root_v)).unwrap_or_default();
    }
    let api = ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name,
        method,
        path,
        url: String::new(),
        description: str_field(a, "description"),
        headers,
        query: vec![],
        params,
        body,
        mock: MockConfig::default(),
        prescript: String::new(),
        examples: vec![],
        responses: vec![],
        doc_params: vec![],
        deprecated: false,
        protocol,
    };
    write_pretty(&unique_path(dir, &api.name, ".json"), &api)?;
        stats.add(&api.protocol);
    Ok(1)
}

// ---------- Hoppscotch ----------

fn import_hoppscotch_files(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let v: Value = serde_json::from_str(&fs::read_to_string(file).map_err(|e| e.to_string())?)
        .map_err(|e| format!("解析 Hoppscotch 文件失败: {e}"))?;
    let name = str_field(&v, "name");
    let folder = unique_path(root, &if name.is_empty() { "Hoppscotch 导入".to_string() } else { name.clone() }, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some(name.clone()),
            description: str_field(&v, "description"),
            base_url: None,
            mock_port: None,
            collapsed: None,
            deprecated: None,
            dirs: vec![],
            apis: vec![],
        },
    )?;
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    if let Some(folders) = v.get("folders").and_then(|x| x.as_array()) {
        count += hoppscotch_walk(&folder, folders, &mut stats)?;
    }
    if let Some(reqs) = v.get("requests").and_then(|x| x.as_array()) {
        for r in reqs {
            count += hoppscotch_req_to_api(&folder, r, &mut stats)?;
        }
    }
    Ok(OpenApiImportResult { folder: folder.to_string_lossy().to_string(), count, http: stats.http, ws: stats.ws, graphql: stats.graphql, socketio: stats.socketio, ..Default::default() })
}

fn hoppscotch_walk(dir: &Path, folders: &[Value],
    stats: &mut ImportStats) -> Result<usize, String> {
    let mut count = 0usize;
    for f in folders {
        let fname = str_field(f, "name");
        if fname.is_empty() {
            continue;
        }
        let sub = mk_group_dir(dir, &fname, &str_field(f, "description"))?;
        if let Some(reqs) = f.get("requests").and_then(|x| x.as_array()) {
            for r in reqs {
                count += hoppscotch_req_to_api(&sub, r, stats)?;
            }
        }
        if let Some(subs) = f.get("folders").and_then(|x| x.as_array()) {
            count += hoppscotch_walk(&sub, subs, stats)?;
        }
    }
    Ok(count)
}

fn hoppscotch_req_to_api(dir: &Path, r: &Value,
    stats: &mut ImportStats) -> Result<usize, String> {
    let mut method = str_field(r, "method").to_uppercase();
    if method.is_empty() {
        method = "GET".to_string();
    }
    // endpoint 含 <<base_url>> → 移除
    let raw_url = str_field(r, "endpoint").replace("<<base_url>>", "");
    if raw_url.is_empty() {
        return Ok(0);
    }
    let is_ws = raw_url.starts_with("ws://") || raw_url.starts_with("wss://");
    let protocol = if is_ws { "websocket".to_string() } else { "http".to_string() };
    let (path, params) = if is_ws { (raw_url.clone(), Vec::new()) } else { extract_path(&raw_url) };
    let name = str_field(r, "name");
    let name = if name.is_empty() { format!("{method} {path}") } else { name };
    let mut headers = Vec::new();
    if let Some(hs) = r.get("headers").and_then(|x| x.as_array()) {
        for h in hs {
            let kv = kv_of(h, "key", "value", None, Some("active"));
            if !kv.key.is_empty() {
                headers.push(kv);
            }
        }
    }
    let mut query = Vec::new();
    if let Some(qs) = r.get("params").and_then(|x| x.as_array()) {
        for q in qs {
            let kv = kv_of(q, "key", "value", None, Some("active"));
            if !kv.key.is_empty() {
                query.push(kv);
            }
        }
    }
    let mut body = BodyData::default();
    if let Some(bd) = r.get("body").and_then(|x| x.as_object()) {
        let mode = map_str(bd, "mode");
        match mode.as_str() {
            "raw" => {
                let raw = map_str(bd, "raw");
                if !raw.is_empty() {
                    body.mode = "json".to_string();
                    body.raw = raw;
                }
            }
            "urlencoded" | "formdata" => {
                let key = if mode == "urlencoded" { "urlencoded" } else { "formdata" };
                if let Some(fs) = bd.get(key).and_then(|x| x.as_array()) {
                    body.mode = "form".to_string();
                    for f in fs {
                        let mut kv = kv_of(f, "key", "value", None, Some("active"));
                        kv.is_file = str_field(f, "type") == "file";
                        if !kv.key.is_empty() {
                            body.form.push(kv);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    let api = ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name,
        method,
        path,
        url: String::new(),
        description: String::new(),
        headers,
        query,
        params,
        body,
        mock: MockConfig::default(),
        prescript: String::new(),
        examples: vec![],
        responses: vec![],
        doc_params: vec![],
        deprecated: false,
        protocol,
    };
    write_pretty(&unique_path(dir, &api.name, ".json"), &api)?;
        stats.add(&api.protocol);
    Ok(1)
}

// ---------- MeterSphere ----------

fn import_metersphere_files(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let v: Value = serde_json::from_str(&fs::read_to_string(file).map_err(|e| e.to_string())?)
        .map_err(|e| format!("解析 MeterSphere 文件失败: {e}"))?;
    let name = str_field(&v, "projectName");
    let folder = unique_path(root, &if name.is_empty() { "MeterSphere 导入".to_string() } else { name.clone() }, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some(name.clone()),
            description: String::new(),
            base_url: None,
            mock_port: None,
            collapsed: None,
            deprecated: None,
            dirs: vec![],
            apis: vec![],
        },
    )?;
    // nodeTree → id → dir
    let mut node_dirs: HashMap<String, PathBuf> = HashMap::new();
    fn walk_nodes(
        folder: &Path,
        nodes: &[Value],
        node_dirs: &mut HashMap<String, PathBuf>,
    ) -> Result<(), String> {
        for n in nodes {
            let nid = str_field(n, "id");
            let nname = str_field(n, "name");
            if nid.is_empty() || nname.is_empty() {
                continue;
            }
            let sub = mk_group_dir(folder, &nname, "")?;
            if let Some(ch) = n.get("children").and_then(|x| x.as_array()) {
                walk_nodes(&sub, ch, node_dirs)?;
            }
            node_dirs.insert(nid, sub);
        }
        Ok(())
    }
    if let Some(nt) = v.get("nodeTree").and_then(|x| x.as_array()) {
        walk_nodes(&folder, nt, &mut node_dirs)?;
    }
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    if let Some(apis) = v.get("data").and_then(|x| x.as_array()) {
        for a in apis {
            let mid = str_field(a, "moduleId");
            let dir = node_dirs.get(&mid).cloned().unwrap_or_else(|| folder.clone());
            count += metersphere_api_to_api(&dir, a, &mut stats)?;
        }
    }
    Ok(OpenApiImportResult { folder: folder.to_string_lossy().to_string(), count, http: stats.http, ws: stats.ws, graphql: stats.graphql, socketio: stats.socketio, ..Default::default() })
}

fn metersphere_api_to_api(dir: &Path, a: &Value,
    stats: &mut ImportStats) -> Result<usize, String> {
    let mut method = str_field(a, "method").to_uppercase();
    if method.is_empty() {
        method = "GET".to_string();
    }
    let raw_url = str_field(a, "path");
    if raw_url.is_empty() {
        return Ok(0);
    }
    let is_ws = raw_url.starts_with("ws://") || raw_url.starts_with("wss://");
    let protocol = if is_ws { "websocket".to_string() } else { "http".to_string() };
    let (path, params) = if is_ws { (raw_url.clone(), Vec::new()) } else { extract_path(&raw_url) };
    let name = str_field(a, "name");
    let name = if name.is_empty() { format!("{method} {path}") } else { name };
    let mut headers = Vec::new();
    let mut query = Vec::new();
    let mut body = BodyData::default();
    if let Some(req) = a.get("request").and_then(|x| x.as_object()) {
        if let Some(hs) = req.get("headers").and_then(|x| x.as_array()) {
            for h in hs {
                let kv = kv_of(h, "key", "value", None, Some("enable"));
                if !kv.key.is_empty() {
                    headers.push(kv);
                }
            }
        }
        if let Some(qs) = req.get("query").and_then(|x| x.as_array()) {
            for q in qs {
                let kv = kv_of(q, "key", "value", None, Some("enable"));
                if !kv.key.is_empty() {
                    query.push(kv);
                }
            }
        }
        if let Some(bd) = req.get("body").and_then(|x| x.as_object()) {
            let raw = map_str(bd, "raw");
            if !raw.is_empty() {
                body.mode = "json".to_string();
                body.raw = raw;
            }
        }
    }
    let mut responses = Vec::new();
    if let Some(resp) = a.get("response").and_then(|x| x.as_object()) {
        let raw = map_str(resp, "raw");
        if !raw.is_empty() {
            responses.push(resp_item(200, "返回成功", &raw));
        }
    }
    let api = ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name,
        method,
        path,
        url: String::new(),
        description: str_field(a, "description"),
        headers,
        query,
        params,
        body,
        mock: MockConfig::default(),
        prescript: String::new(),
        examples: vec![],
        responses,
        doc_params: vec![],
        deprecated: false,
        protocol,
    };
    write_pretty(&unique_path(dir, &api.name, ".json"), &api)?;
        stats.add(&api.protocol);
    Ok(1)
}

// ---------- 统一分发 ----------


// ---------- RAP2 ----------

/// 常见请求头名（rap2 无显式 header 分类，靠名字识别）
fn is_common_header(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    matches!(
        n.as_str(),
        "authorization" | "content-type" | "accept" | "accept-language" | "accept-encoding"
            | "user-agent" | "cookie" | "origin" | "referer" | "host" | "x-requested-with"
            | "x-token" | "token" | "cache-control" | "pragma" | "if-none-match" | "if-modified-since"
    )
}

/// rap2 属性类型 → 示例值（递归解析 parentId 树）
fn rap2_build_value(
    props: &[Value],
    parent_id: i64,
    is_root: bool,
    depth: usize,
) -> Value {
    let mut m = serde_json::Map::new();
    let mut arr_items: Vec<Value> = Vec::new();
    for p in props {
        let pid = p.get("parentId").and_then(|x| x.as_i64()).unwrap_or(-1);
        if pid != parent_id {
            continue;
        }
        let name = str_field(p, "name");
        if name.is_empty() {
            continue;
        }
        let ty = str_field(p, "type");
        let val = rap2_prop_value(p);
        let lower = ty.to_ascii_lowercase();
        if lower.contains("object") {
            let child = rap2_build_value(props, p.get("id").and_then(|x| x.as_i64()).unwrap_or(-1), false, depth + 1);
            m.insert(name.clone(), if child.is_object() && child.as_object().map(|c| c.is_empty()).unwrap_or(true) && !val.is_null() { val } else { child });
        } else if lower.contains("array") {
            // 数组元素：寻找该数组下的 Object 子属性
            let elem_id = p.get("id").and_then(|x| x.as_i64()).unwrap_or(-1);
            let elem_obj = rap2_build_value(props, elem_id, false, depth + 1);
            if elem_obj.as_object().map(|c| c.is_empty()).unwrap_or(true) {
                arr_items.push(Value::String(String::new()));
            } else {
                // 数组本身还有更深的子元素（Array 的 Object 元素）
                arr_items.push(elem_obj.clone());
            }
            m.insert(name.clone(), Value::Array(arr_items.clone()));
            arr_items.clear();
        } else {
            m.insert(name.clone(), val);
        }
    }
    if is_root {
        // 根级：多个顶层属性合并为一个对象
        if !m.is_empty() {
            return Value::Object(m);
        }
        Value::Object(serde_json::Map::new())
    } else {
        Value::Object(m)
    }
}

/// 单个 rap2 属性 → 示例值（mock 表达式原样保留）
fn rap2_prop_value(p: &Value) -> Value {
    let ty = str_field(p, "type");
    let raw = p.get("value").cloned().unwrap_or(Value::Null);
    let v: Value = if raw.is_null() { Value::String(String::new()) } else { raw };
    match ty.as_str() {
        "Number" | "Integer" | "Float" | "Double" => {
            if let Some(n) = v.as_i64() {
                Value::from(n)
            } else if let Some(f) = v.as_f64() {
                Value::from(f)
            } else {
                let s = v.as_str().unwrap_or("");
                if let Ok(n) = s.parse::<i64>() {
                    Value::from(n)
                } else if let Ok(f) = s.parse::<f64>() {
                    Value::from(f)
                } else {
                    Value::String(String::new())
                }
            }
        }
        "Boolean" => Value::Bool(v.as_bool().unwrap_or(false)),
        "Null" => Value::Null,
        _ => {
            // 字符串/文件/其他：保留原始 value（含 @mock 表达式）
            if v.is_string() {
                v
            } else {
                Value::String(v.to_string())
            }
        }
    }
}

/// rap2 接口 → ApiFile
fn rap2_interface_to_api(it: &Value) -> ApiFile {
    let raw_url = str_field(it, "url");
    let method = str_field(it, "method").to_uppercase();
    let mut path = raw_url.clone();
    let mut url_query: Vec<KeyValue> = Vec::new();
    if let Some(qi) = path.find('?') {
        for pair in path[qi + 1..].split('&') {
            if pair.is_empty() {
                continue;
            }
            let (k, v) = match pair.split_once('=') {
                Some((a, b)) => (a, b),
                None => (pair, ""),
            };
            if !k.is_empty() {
                url_query.push(KeyValue {
                    key: k.to_string(),
                    value: v.to_string(),
                    enabled: true,
                    is_file: false,
                    description: String::new(),
                });
            }
        }
        path = path[..qi].to_string();
    }
    let props: Vec<Value> = it
        .get("properties")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    let mut headers: Vec<KeyValue> = Vec::new();
    let mut query: Vec<KeyValue> = url_query;
    let mut params: Vec<KeyValue> = Vec::new();
    let mut body_parts: serde_json::Map<String, Value> = serde_json::Map::new();
    let mut response_objs: serde_json::Map<String, Value> = serde_json::Map::new();
    for p in &props {
        let scope = str_field(p, "scope");
        let pid = p.get("parentId").and_then(|x| x.as_i64()).unwrap_or(-1);
        if pid != -1 {
            continue; // 嵌套属性由父级递归处理
        }
        let name = str_field(p, "name");
        if name.is_empty() {
            continue;
        }
        let ty = str_field(p, "type");
        let lower = ty.to_ascii_lowercase();
        let is_body_like = lower.contains("object") || lower.contains("array");
        if scope == "request" {
            if path.contains(&format!("{{{name}}}")) {
                params.push(KeyValue {
                    key: name.clone(),
                    value: String::new(),
                    enabled: true,
                    is_file: false,
                    description: str_field(p, "description"),
                });
            } else if is_body_like {
                let pid_v = p.get("id").and_then(|x| x.as_i64()).unwrap_or(-1);
                let child = rap2_build_value(&props, pid_v, false, 0);
                body_parts.insert(name.clone(), child);
            } else if is_common_header(&name) {
                headers.push(KeyValue {
                    key: name,
                    value: str_field(p, "value"),
                    enabled: true,
                    is_file: false,
                    description: str_field(p, "description"),
                });
            } else {
                query.push(KeyValue {
                    key: name,
                    value: str_field(p, "value"),
                    enabled: true,
                    is_file: false,
                    description: str_field(p, "description"),
                });
            }
        } else if scope == "response" {
            let pid_v = p.get("id").and_then(|x| x.as_i64()).unwrap_or(-1);
            let child = rap2_build_value(&props, pid_v, false, 0);
            if !(child.is_object() && child.as_object().map(|c| c.is_empty()).unwrap_or(true)) || !rap2_prop_value(p).is_null() {
                if is_body_like || !(child.is_object() && child.as_object().map(|c| c.is_empty()).unwrap_or(true)) {
                    response_objs.insert(name.clone(), child);
                } else {
                    response_objs.insert(name.clone(), rap2_prop_value(p));
                }
            } else {
                response_objs.insert(name.clone(), rap2_prop_value(p));
            }
        }
    }
    let mut body = BodyData::default();
    if !body_parts.is_empty() {
        body.mode = "json".to_string();
        body.raw = serde_json::to_string_pretty(&Value::Object(body_parts)).unwrap_or_default();
    }
    let mut responses: Vec<ResponseItem> = Vec::new();
    if !response_objs.is_empty() {
        let resp_body = serde_json::to_string_pretty(&Value::Object(response_objs)).unwrap_or_default();
        responses.push(resp_item(200, "成功", &resp_body));
    }
    ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name: str_field(it, "name"),
        method: if method.is_empty() { "GET".to_string() } else { method },
        path,
        url: raw_url,
        description: str_field(it, "description"),
        headers,
        query,
        params,
        body,
        mock: MockConfig::default(),
        prescript: String::new(),
        examples: vec![],
        responses,
        doc_params: vec![],
        deprecated: false,
        protocol: "http".to_string(),
    }
}

/// 项目格式：data.modules[] → 分组
fn import_rap2_project(root: &Path, data: &Value) -> Result<OpenApiImportResult, String> {
    let name = str_field(data, "name");
    let folder = unique_path(root, &if name.is_empty() { "RAP2 导入".to_string() } else { name.clone() }, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    let mut dirs: Vec<String> = Vec::new();
    let modules = data.get("modules").and_then(|x| x.as_array()).cloned().unwrap_or_default();
    for m in modules {
        let mname = str_field(&m, "name");
        let dir = mk_group_dir(&folder, &if mname.is_empty() { "未分组".to_string() } else { mname.clone() }, &str_field(&m, "description"))?;
        dirs.push(
            dir.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
        );
        let interfaces = m.get("interfaces").and_then(|x| x.as_array()).cloned().unwrap_or_default();
        for it in interfaces {
            let api = rap2_interface_to_api(&it);
            stats.add(&api.protocol);
            write_pretty(
                &dir.join(format!("{}.json", sanitize_filename(&api.name))),
                &api,
            )?;
            count += 1;
        }
    }
    // 分组顺序：按导入顺序写入父分组 __info.json 的 dirs
    let info_path = folder.join(INFO_FILE);
    if let Ok(mut info) = serde_json::from_str::<InfoJson>(&fs::read_to_string(&info_path).unwrap_or_default()) {
        info.dirs = dirs;
        write_pretty(&info_path, &info)?;
    }
    Ok(OpenApiImportResult {
        folder: folder.to_string_lossy().to_string(),
        count,
        http: stats.http,
        ws: stats.ws,
        graphql: stats.graphql,
        socketio: stats.socketio,
        ..Default::default()
    })
}

/// 单接口格式：data 直接是接口
fn import_rap2_single(root: &Path, data: &Value) -> Result<OpenApiImportResult, String> {
    let folder = unique_path(root, "RAP2 导入", "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    let api = rap2_interface_to_api(data);
    let fname = sanitize_filename(&api.name);
    let mut stats = ImportStats::default();
    stats.add(&api.protocol);
    write_pretty(&folder.join(format!("{fname}.json")), &api)?;
    Ok(OpenApiImportResult {
        folder: folder.to_string_lossy().to_string(),
        count: 1,
        http: stats.http,
        ws: stats.ws,
        graphql: stats.graphql,
        socketio: stats.socketio,
        ..Default::default()
    })
}

/// 自动识别：data.modules 存在 → 项目格式，否则单接口
pub(crate) fn import_rap2_files(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let v: Value = serde_json::from_str(&fs::read_to_string(file).map_err(|e| e.to_string())?)
        .map_err(|e| format!("解析 RAP2 文件失败: {e}"))?;
    let data = v.get("data").cloned().unwrap_or(v);
    if data.get("modules").and_then(|x| x.as_array()).map(|a| !a.is_empty()).unwrap_or(false) {
        import_rap2_project(root, &data)
    } else if data.get("url").is_some() {
        import_rap2_single(root, &data)
    } else {
        Err("无法识别 RAP2 文件（缺少 modules 或 url 字段）".to_string())
    }
}

pub(crate) fn import_extra_files(root: &Path, file: &Path, format: &str) -> Result<OpenApiImportResult, String> {
    match format {
        "apidog" => import_apidog_files(root, file),
        "bruno" => import_bruno_files(root, file),
        "apizza" => import_apizza_files(root, file),
        "nei" => import_nei_files(root, file),
        "doclever" => import_doclever_files(root, file),
        "io-docs" => import_io_docs_files(root, file),
        "easydoc" => import_easydoc_files(root, file),
        "docway" => import_docway_files(root, file),
        "hoppscotch" => import_hoppscotch_files(root, file),
        "metersphere" => import_metersphere_files(root, file),
        "rap2" => import_rap2_files(root, file),
        _ => Err(format!("不支持的格式: {format}")),
    }
}

pub(crate) fn import_apidoc_files(
    root: &Path,
    project_path: &Path,
    data_path: &Path,
) -> Result<OpenApiImportResult, String> {
    let proj: Value =
        serde_json::from_str(&fs::read_to_string(project_path).map_err(|e| e.to_string())?)
            .map_err(|e| format!("解析 api_project.json 失败: {e}"))?;
    let data: Value =
        serde_json::from_str(&fs::read_to_string(data_path).map_err(|e| e.to_string())?)
            .map_err(|e| format!("解析 api_data.json 失败: {e}"))?;
    let name = proj
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("apiDoc 导入")
        .trim()
        .to_string();
    let folder = unique_path(root, &name, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    // 描述：description + header.content（去 HTML 标签）
    let mut desc = proj
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if let Some(hc) = proj.get("header").and_then(|h| h.get("content")).and_then(|v| v.as_str()) {
        let stripped = strip_html(hc);
        if !stripped.is_empty() {
            if !desc.is_empty() {
                desc.push_str("\n\n");
            }
            desc.push_str(&stripped);
        }
    }
    // base_url：sampleUrl 优先
    let base_url = proj
        .get("sampleUrl")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .or_else(|| proj.get("url").and_then(|v| v.as_str()).filter(|s| !s.is_empty()))
        .map(|s| s.to_string());
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some(name.clone()),
            description: desc,
            base_url,
            mock_port: None,
            collapsed: None,
            deprecated: None,
            dirs: vec![],
            apis: vec![],
        },
    )?;
    // 分组
    let mut group_dirs: HashMap<String, PathBuf> = HashMap::new();
    if let Some(gs) = data.get("groups").and_then(|v| v.as_array()) {
        for g in gs {
            let gname = g.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if gname.is_empty() {
                continue;
            }
            let gtitle = g
                .get("title")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(&gname)
                .to_string();
            let sub_base = sanitize_filename(&gtitle);
            let sub = folder.join(&sub_base);
            if !sub.is_dir() {
                fs::create_dir_all(&sub).map_err(|e| format!("创建分组失败: {e}"))?;
                write_pretty(
                    &sub.join(INFO_FILE),
                    &InfoJson {
                        name: Some(gtitle),
                        description: g
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        base_url: None,
                        mock_port: None,
                        collapsed: None,
                        deprecated: None,
                        dirs: vec![],
                        apis: vec![],
                    },
                )?;
            }
            group_dirs.insert(gname, sub);
        }
    }
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    if let Some(apis) = data.get("apis").and_then(|v| v.as_array()) {
        for a in apis {
            let gname = a.get("group").and_then(|v| v.as_str()).unwrap_or("");
            let dir = group_dirs.get(gname).unwrap_or(&folder);
            count += apidoc_api_to_api(dir, a, &mut stats)?;
        }
    }
    Ok(OpenApiImportResult {
        folder: folder.to_string_lossy().to_string(),
        count,
        http: stats.http,
        ws: stats.ws,
        graphql: stats.graphql,
        socketio: stats.socketio,
        ..Default::default()
    })
}

/// 去掉 HTML 标签与常见实体
fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .trim()
        .to_string()
}

/// apiDoc api 对象 → ApiFile
fn apidoc_api_to_api(dir: &Path, a: &Value,
    stats: &mut ImportStats) -> Result<usize, String> {
    let mut method = a.get("method").and_then(|v| v.as_str()).unwrap_or("GET").trim().to_uppercase();
    if method.is_empty() {
        method = "GET".to_string();
    }
    let raw_url = a.get("url").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    if raw_url.is_empty() {
        return Ok(0);
    }
    let is_ws = raw_url.starts_with("ws://") || raw_url.starts_with("wss://");
    let protocol = if is_ws { "websocket".to_string() } else { "http".to_string() };
    let (path, params) = if is_ws {
        (raw_url.clone(), Vec::new())
    } else {
        extract_path(&raw_url)
    };
    let name = a
        .get("title")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{method} {path}"));
    let description = a.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
    // 请求头
    let mut headers = Vec::new();
    if let Some(fs) = a
        .get("header")
        .and_then(|h| h.get("fields"))
        .and_then(|f| f.get("Header"))
        .and_then(|v| v.as_array())
    {
        for f in fs {
            let key = f.get("field").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if key.is_empty() {
                continue;
            }
            headers.push(KeyValue {
                key: key.clone(),
                value: String::new(),
                enabled: true,
                is_file: false,
                description: f.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            });
        }
    }
    // query + body 字段
    let mut query = Vec::new();
    let mut body_fields: Vec<Value> = Vec::new();
    if let Some(pf) = a.get("parameter").and_then(|p| p.get("fields")).and_then(|v| v.as_object()) {
        for (k, arr) in pf {
            let Some(arr) = arr.as_array() else { continue };
            if k == "Query" {
                for f in arr {
                    let key = f.get("field").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                    if key.is_empty() {
                        continue;
                    }
                    query.push(KeyValue {
                        key: key.clone(),
                        value: String::new(),
                        enabled: true,
                        is_file: false,
                        description: f
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    });
                }
            } else {
                body_fields.extend(arr.iter().cloned());
            }
        }
    }
    // body：字段列表 → 嵌套 JSON 示例
    let mut body = BodyData::default();
    let mut doc_params: Vec<DocParam> = Vec::new();
    if !body_fields.is_empty() {
        let json_val = apidoc_fields_to_value(&body_fields);
        if json_val != Value::Null {
            body.mode = "json".to_string();
            body.raw = serde_json::to_string_pretty(&json_val).unwrap_or_default();
        }
        for f in &body_fields {
            let key = f.get("field").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if key.is_empty() {
                continue;
            }
            doc_params.push(DocParam {
                source: "body".into(),
                key: key.clone(),
                r#type: f.get("type").and_then(|v| v.as_str()).unwrap_or("String").to_string(),
                description: f.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                item_type: String::new(),
                object_name: String::new(),
                children: vec![],
            });
        }
    }
    for h in &headers {
        doc_params.push(DocParam {
            source: "header".into(),
            key: h.key.clone(),
            r#type: "String".into(),
            description: h.description.clone(),
            item_type: String::new(),
            object_name: String::new(),
            children: vec![],
        });
    }
    for q in &query {
        doc_params.push(DocParam {
            source: "query".into(),
            key: q.key.clone(),
            r#type: "String".into(),
            description: q.description.clone(),
            item_type: String::new(),
            object_name: String::new(),
            children: vec![],
        });
    }
    // 响应
    let mut responses: Vec<ResponseItem> = Vec::new();
    if let Some(exs) = a.get("successExamples").and_then(|v| v.as_array()) {
        for ex in exs {
            responses.push(ResponseItem {
                id: uuid::Uuid::new_v4().to_string(),
                name: ex.get("title").and_then(|v| v.as_str()).unwrap_or("返回成功").to_string(),
                status: 200,
                content_type: "application/json".to_string(),
                body: ex.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            });
        }
    }
    if let Some(exs) = a.get("error").and_then(|e| e.get("examples")).and_then(|v| v.as_array()) {
        for ex in exs {
            responses.push(ResponseItem {
                id: uuid::Uuid::new_v4().to_string(),
                name: ex.get("title").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()).unwrap_or("返回失败").to_string(),
                status: 0,
                content_type: "application/json".to_string(),
                body: ex.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            });
        }
    }
    // success.fields → resp_success 文档字段
    if let Some(fs) = a.get("success").and_then(|s| s.get("fields")).and_then(|v| v.as_object()) {
        for (_k, arr) in fs {
            let Some(arr) = arr.as_array() else { continue };
            for f in arr {
                let key = f.get("field").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                if key.is_empty() {
                    continue;
                }
                doc_params.push(DocParam {
                    source: "resp_success".into(),
                    key: key.clone(),
                    r#type: f.get("type").and_then(|v| v.as_str()).unwrap_or("String").to_string(),
                    description: f.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    item_type: String::new(),
                    object_name: String::new(),
                    children: vec![],
                });
            }
        }
    }
    let file_base = sanitize_filename(&name);
    let file_path = unique_path(dir, &file_base, ".json");
    let api_file = ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name: name.clone(),
        method: method.clone(),
        path: path.clone(),
        url: raw_url,
        description,
        headers,
        query,
        params,
        body,
        mock: MockConfig::default(),
        prescript: String::new(),
        examples: vec![],
        responses,
        doc_params,
        deprecated: false,
        protocol,
    };
    write_pretty(&file_path, &api_file)?;
        stats.add(&api_file.protocol);
    Ok(1)
}

/// 字段列表（点分 key）→ 嵌套 JSON 示例值
fn apidoc_fields_to_value(fields: &[Value]) -> Value {
    let entries: Vec<(Vec<String>, &Value)> = fields
        .iter()
        .filter_map(|f| {
            let key = f.get("field").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if key.is_empty() {
                return None;
            }
            let parts: Vec<String> = key.split('.').map(|s| s.to_string()).collect();
            Some((parts, f))
        })
        .collect();
    if entries.is_empty() {
        return Value::Null;
    }
    apidoc_build_node(&entries)
}

fn apidoc_build_node(entries: &[(Vec<String>, &Value)]) -> Value {
    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, Vec<(Vec<String>, &Value)>> = HashMap::new();
    for (parts, f) in entries {
        let root = &parts[0];
        if !groups.contains_key(root) {
            order.push(root.clone());
        }
        let rest = parts[1..].to_vec();
        groups.entry(root.clone()).or_default().push((rest, f));
    }
    let mut obj = serde_json::Map::new();
    for root in order {
        let group = groups.remove(&root).unwrap_or_default();
        let self_field = group.iter().find(|(p, _)| p.is_empty()).map(|(_, f)| *f);
        let children: Vec<(Vec<String>, &Value)> = group
            .iter()
            .filter(|(p, _)| !p.is_empty())
            .map(|(p, f)| (p.clone(), *f))
            .collect();
        let ty = self_field
            .and_then(|f| f.get("type"))
            .and_then(|v| v.as_str())
            .unwrap_or("Object")
            .to_string();
        let val = if ty.contains("[]") || ty.eq_ignore_ascii_case("Array") || ty.eq_ignore_ascii_case("List") {
            if children.is_empty() {
                Value::Array(vec![])
            } else {
                Value::Array(vec![apidoc_build_node(&children)])
            }
        } else if !children.is_empty() {
            apidoc_build_node(&children)
        } else {
            apidoc_default_value(&ty)
        };
        obj.insert(root, val);
    }
    Value::Object(obj)
}

fn apidoc_default_value(ty: &str) -> Value {
    let t = ty.to_lowercase();
    if t.contains("number") || t.contains("integer") || t.contains("float") {
        Value::Number(0.into())
    } else if t.contains("bool") {
        Value::Bool(false)
    } else if t.contains("object") || t.contains("list") || t.contains("array") {
        Value::Object(serde_json::Map::new())
    } else {
        Value::String(String::new())
    }
}

// ==================== JMeter 导入 ====================
