//! 由 import.rs 拆分：OpenAPI
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

/// 解析 OpenAPI / Swagger 文件（支持 .json 与 .yml/.yaml），在工作区根新建分组，按 tag 分小组并导入全部接口
pub(crate) fn import_openapi_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let mut stats = ImportStats::default();
    let mut failed = 0usize;
    let mut duplicated = 0usize;
    let content = fs::read_to_string(file).map_err(|e| format!("读取文件失败: {e}"))?;
    let is_yaml = matches!(
        file.extension().and_then(|e| e.to_str()),
        Some("yml") | Some("yaml")
    );
    let json: Value = if is_yaml {
        serde_yaml::from_str(&content).map_err(|e| format!("解析 YAML 失败: {e}"))?
    } else {
        serde_json::from_str(&content).map_err(|e| format!("解析 JSON 失败: {e}"))?
    };

    // 校验 OpenAPI / Swagger 版本字段
    let version = if let Some(v) = json.get("openapi").and_then(|v| v.as_str()) {
        v.to_string()
    } else if let Some(v) = json.get("swagger").and_then(|v| v.as_str()) {
        v.to_string()
    } else {
        return Err("不是有效的 OpenAPI / Swagger 文件（缺少 openapi 或 swagger 字段）".into());
    };

    let title = json
        .pointer("/info/title")
        .and_then(|v| v.as_str())
        .unwrap_or("OpenAPI 导入")
        .to_string();
    let dir_name = sanitize_filename(&title);
    let dir_name = if dir_name.is_empty() {
        "OpenAPI 导入".to_string()
    } else {
        dir_name
    };
    let folder = unique_path(root, &dir_name, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    let src_name = file
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some(title.clone()),
            description: format!("从 OpenAPI {version} 导入（{src_name}）"),
            base_url: None,
            mock_port: None,
            collapsed: None,
            deprecated: None,
            dirs: vec![],
            apis: vec![],
        },
    )?;

    let base_url = openapi_base_url(&json);
    // $ref 以整个文档为根解析（#/components/schemas/xxx 或 #/definitions/xxx）
    let defs = &json;

    let mut count = 0usize;
    let mut tag_dirs: std::collections::HashMap<String, PathBuf> = std::collections::HashMap::new();
    if let Some(paths) = json.get("paths").and_then(|v| v.as_object()) {
        let mut entries: Vec<(&String, &Value)> = paths.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));
        for (path_str, path_item) in entries {
            let shared_params = path_item
                .get("parameters")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            for method in ["get", "post", "put", "delete", "patch", "options", "head"] {
                let Some(op) = path_item.get(method) else {
                    continue;
                };
                if !op.is_object() {
                    continue;
                }
                let api = match openapi_op_to_api(method, path_str, op, &shared_params, &base_url, &defs)
                {
                    Ok(a) => a,
                    Err(_) => {
                        failed += 1;
                        continue;
                    }
                };
                // 按第一个 tag 分组，无 tag 的放在顶层
                let tag = op
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.first())
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let target = if tag.is_empty() {
                    folder.clone()
                } else {
                    let dir = tag_dirs.entry(tag.clone()).or_insert_with(|| {
                        let base = sanitize_filename(&tag);
                        let base = if base.is_empty() { "未分组".to_string() } else { base };
                        let d = unique_path(&folder, &base, "");
                        let _ = fs::create_dir_all(&d);
                        let _ = write_pretty(
                            &d.join(INFO_FILE),
                            &InfoJson {
                                name: Some(tag.clone()),
                                description: String::new(),
                                base_url: None,
                                mock_port: None,
                                collapsed: None,
                                deprecated: None,
                                dirs: vec![],
                                apis: vec![],
                            },
                        );
                        // 追加到父分组 __info.json 的 dirs（导入顺序即显示顺序）
                        let dname = d
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();
                        crate::info_append_child(&folder, &dname, true);
                        d
                    });
                    dir.clone()
                };
                let file_base =
                    sanitize_filename(&format!("{} {}", method.to_uppercase(), path_str));
                let file_base = if file_base.is_empty() {
                    "未命名接口".to_string()
                } else {
                    file_base
                };
                let target_path = unique_path(&target, &file_base, ".json");
                if target_path != target.join(format!("{file_base}.json")) {
                    duplicated += 1;
                }
                write_pretty(&target_path, &api)?;
                count += 1;
                stats.add(&api.protocol);
            }
        }
    }
    if count == 0 {
        let _ = fs::remove_dir_all(&folder);
        return Err("OpenAPI 文件中没有可导入的接口（未找到 paths）".into());
    }
    Ok(OpenApiImportResult {
        folder: folder.to_string_lossy().to_string(),
        count,
        http: stats.http,
        ws: stats.ws,
        graphql: stats.graphql,
        socketio: stats.socketio,
        failed,
        duplicated,
        ..Default::default()
    })
}

/// 拼接 OpenAPI 基础地址：OAS3 取 servers[0].url，Swagger 2.0 取 schemes + host + basePath
fn openapi_base_url(json: &Value) -> String {
    if let Some(url) = json.pointer("/servers/0/url").and_then(|v| v.as_str()) {
        let trimmed = url.trim_end_matches('/').to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }
    let scheme = json
        .get("schemes")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .unwrap_or("http");
    let host = json.get("host").and_then(|v| v.as_str()).unwrap_or("");
    let base = json.get("basePath").and_then(|v| v.as_str()).unwrap_or("");
    if host.is_empty() {
        return String::new();
    }
    format!("{scheme}://{host}{}", base.trim_end_matches('/'))
}

/// 将 OpenAPI operation 对象转换为 ApiFile
fn openapi_op_to_api(
    method: &str,
    path: &str,
    op: &Value,
    shared_params: &[Value],
    base_url: &str,
    defs: &Value,
) -> Result<ApiFile, String> {
    let mut headers = Vec::new();
    let mut query = Vec::new();
    let mut params = Vec::new();
    let mut body = BodyData::default();

    // 合并 path item 级与 operation 级参数
    let mut op_params: Vec<Value> = shared_params.to_vec();
    if let Some(arr) = op.get("parameters").and_then(|v| v.as_array()) {
        op_params.extend(arr.iter().cloned());
    }
    for p in &op_params {
        let loc = p.get("in").and_then(|v| v.as_str()).unwrap_or("");
        let key = p
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if key.is_empty() {
            continue;
        }
        let value = param_sample_value(p, defs);
        let desc = p
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        match loc {
            "header" => headers.push(KeyValue {
                key,
                value,
                enabled: true,
                is_file: false,
                description: desc,
            }),
            "query" => query.push(KeyValue {
                key,
                value,
                enabled: true,
                is_file: false,
                description: desc,
            }),
            "path" => params.push(KeyValue {
                key,
                value,
                enabled: true,
                is_file: false,
                description: desc,
            }),
            _ => {}
        }
    }

    // OpenAPI 3.0 请求体：requestBody.content[application/json].schema
    if let Some(rb) = op.get("requestBody") {
        if let Some(schema) = rb.pointer("/content/application~1json/schema") {
            let sample = schema_sample(schema, defs, 0);
            body.mode = "json".into();
            body.raw = serde_json::to_string_pretty(&sample).unwrap_or_else(|_| "{}".into());
        }
    }
    // Swagger 2.0 请求体：in: body 参数
    for p in &op_params {
        if p.get("in").and_then(|v| v.as_str()) == Some("body") {
            if let Some(schema) = p.get("schema") {
                let sample = schema_sample(schema, defs, 0);
                body.mode = "json".into();
                body.raw = serde_json::to_string_pretty(&sample).unwrap_or_else(|_| "{}".into());
            }
        }
    }

    let summary = op.get("summary").and_then(|v| v.as_str()).unwrap_or("").trim();
    let description = op
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let desc = match (summary.is_empty(), description.is_empty()) {
        (false, false) => format!("{summary}\n{description}"),
        (true, false) => description.to_string(),
        (false, true) => summary.to_string(),
        (true, true) => String::new(),
    };

    Ok(ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name: format!("{} {}", method.to_uppercase(), path),
        method: method.to_uppercase(),
        path: path.to_string(),
        url: format!("{}{}", base_url.trim_end_matches('/'), path),
        description: desc,
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
        protocol: "http".into(),
    })
}

/// 参数示例值：example / default / enum 优先，否则取空
fn param_sample_value(p: &Value, defs: &Value) -> String {
    if let Some(v) = p.get("example") {
        return scalar_to_string(v);
    }
    if let Some(schema) = p.get("schema") {
        let schema = resolve_schema_ref(schema, defs);
        if let Some(v) = schema.get("example") {
            return scalar_to_string(v);
        }
        if let Some(v) = schema.get("default") {
            return scalar_to_string(v);
        }
        if let Some(enumv) = schema.get("enum").and_then(|e| e.as_array()) {
            if let Some(first) = enumv.first() {
                return scalar_to_string(first);
            }
        }
    }
    String::new()
}

fn scalar_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

/// 解析 $ref（#/components/schemas/xxx 或 #/definitions/xxx），解析失败时原样返回
fn resolve_schema_ref<'a>(schema: &'a Value, defs: &'a Value) -> &'a Value {
    let Some(r) = schema.get("$ref").and_then(|v| v.as_str()) else {
        return schema;
    };
    let mut cur = defs;
    for seg in r.trim_start_matches("#/").split('/') {
        cur = match cur.get(seg) {
            Some(v) => v,
            None => return schema,
        };
    }
    cur
}

/// 根据 JSON Schema 生成示例值（支持 $ref / enum / 嵌套对象 / 数组，最多递归 3 层防循环引用）
fn schema_sample(schema: &Value, defs: &Value, depth: usize) -> Value {
    if depth > 3 {
        return Value::Null;
    }
    let schema = resolve_schema_ref(schema, defs);
    if let Some(v) = schema.get("example") {
        return v.clone();
    }
    if let Some(v) = schema.get("default") {
        return v.clone();
    }
    if let Some(enumv) = schema.get("enum").and_then(|e| e.as_array()) {
        if let Some(first) = enumv.first() {
            return first.clone();
        }
    }
    if let Some(all_of) = schema.get("allOf").and_then(|v| v.as_array()) {
        if let Some(first) = all_of.first() {
            return schema_sample(first, defs, depth + 1);
        }
    }
    let ty = schema.get("type").and_then(|v| v.as_str()).unwrap_or("object");
    match ty {
        "object" => {
            let mut map = serde_json::Map::new();
            if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
                for (k, v) in props {
                    map.insert(k.clone(), schema_sample(v, defs, depth + 1));
                }
            }
            Value::Object(map)
        }
        "array" => {
            let item = schema
                .get("items")
                .map(|i| schema_sample(i, defs, depth + 1))
                .unwrap_or(Value::Null);
            Value::Array(vec![item])
        }
        "integer" | "number" => Value::from(0),
        "boolean" => Value::Bool(false),
        "string" => match schema.get("format").and_then(|f| f.as_str()) {
            Some("date-time") | Some("date") => Value::String("2024-01-01T00:00:00Z".into()),
            Some("email") => Value::String("user@example.com".into()),
            Some("uuid") => Value::String("00000000-0000-0000-0000-000000000000".into()),
            Some("uri") | Some("url") => Value::String("https://example.com".into()),
            Some("password") => Value::String("********".into()),
            _ => Value::String(String::new()),
        },
        "null" => Value::Null,
        _ => Value::Null,
    }
}
