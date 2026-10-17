//! 由 import.rs 拆分：OpenAPI
use super::*;
#[allow(unused_imports)]
use crate::{ApiFile, BodyData, DocParam, EnvVariable, KeyValue, MockConfig, ResponseItem, sanitize_filename, unique_path, workspace_root, ENV_FILE, INFO_FILE};
use crate::objects::{capitalize, list_objects_impl, object_hash, save_objects_impl, ObjectDef, ObjectGroup, ObjectProp, ObjectStore};
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
            selected_api: None,
            dirs: vec![],
            apis: vec![],
            favorites: vec![],
        },
    )?;

    let base_url = openapi_base_url(&json);
    // $ref 以整个文档为根解析（#/components/schemas/xxx 或 #/definitions/xxx）
    let defs = &json;

    // 文档级对象定义（OAS3 components/schemas / Swagger 2.0 definitions）→ 对象管理。
    // 分组名与接口分组（目录名）一致，便于对照；写入在接口全部导入成功之后进行。
    let group_name = folder
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let obj_store = if has_object_schemas(&json) {
        Some(list_objects_impl(root)?)
    } else {
        None
    };
    let obj_result = match &obj_store {
        Some(store) => build_openapi_objects(store, &json, &group_name),
        None => OpenApiObjectResult::default(),
    };
    let obj_names = &obj_result.name_by_schema;

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
                let api = match openapi_op_to_api(
                    method,
                    path_str,
                    op,
                    &shared_params,
                    &base_url,
                    &defs,
                    obj_names,
                ) {
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
                                selected_api: None,
                                dirs: vec![],
                                apis: vec![],
                                favorites: vec![],
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
    // 接口导入成功后写入对象（导入失败时不留下多余对象）
    let mut objects = 0usize;
    if let Some(mut store) = obj_store {
        objects = obj_result.count;
        if !obj_result.objects.is_empty() {
            let mut groups = read_object_groups(root);
            if !group_name.is_empty() && !groups.iter().any(|g| g.id == group_name) {
                groups.push(ObjectGroup {
                    id: group_name.clone(),
                    name: group_name.clone(),
                    deprecated: false,
                    order: None,
                });
            }
            store.groups = groups;
            store.objects.extend(obj_result.objects);
            save_objects_impl(root, &store)?;
        }
    }
    Ok(OpenApiImportResult {
        folder: folder.to_string_lossy().to_string(),
        count,
        http: stats.http,
        ws: stats.ws,
        graphql: stats.graphql,
        socketio: stats.socketio,
        objects,
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
/// obj_names：文档 schema 名 → 对象管理中的对象名（请求体引用对象时写入接口文档绑定）
fn openapi_op_to_api(
    method: &str,
    path: &str,
    op: &Value,
    shared_params: &[Value],
    base_url: &str,
    defs: &Value,
    obj_names: &HashMap<String, String>,
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

    // 请求体引用文档对象（$ref）时写入「文档」页签的 Body 对象绑定：
    // source=body 且 key 为空，接口文档按该对象展开请求体字段，同时对象管理可统计接口引用数
    let mut doc_params: Vec<DocParam> = Vec::new();
    if let Some(schema_name) = body_schema_ref_name(op, &op_params) {
        if let Some(obj_name) = obj_names.get(&schema_name) {
            doc_params.push(DocParam {
                source: "body".into(),
                key: String::new(),
                r#type: "Object".into(),
                description: String::new(),
                item_type: String::new(),
                object_name: obj_name.clone(),
                children: vec![],
            });
        }
    }

    Ok(ApiFile {
        pack: vec![],
        unpack: vec![],
        net: None,
        mq: None,
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
        doc_params,
        deprecated: false,
        protocol: "http".into(),
    })
}

/// 请求体引用的文档级 schema 名：
/// OAS3 取 requestBody.content[application/json].schema.$ref，Swagger 2.0 取 in=body 参数的 schema.$ref
fn body_schema_ref_name(op: &Value, op_params: &[Value]) -> Option<String> {
    if let Some(schema) = op.pointer("/requestBody/content/application~1json/schema") {
        if let Some(name) = schema.get("$ref").and_then(|v| v.as_str()).and_then(ref_schema_name) {
            return Some(name);
        }
    }
    for p in op_params {
        if p.get("in").and_then(|v| v.as_str()) != Some("body") {
            continue;
        }
        if let Some(name) = p
            .get("schema")
            .and_then(|s| s.get("$ref"))
            .and_then(|v| v.as_str())
            .and_then(ref_schema_name)
        {
            return Some(name);
        }
    }
    None
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

// ==================== 文档对象（components/schemas / definitions）→ 对象管理 ====================
// 解析文档级 schema 并写入「对象管理」：对象名取 schema 名，分组与接口分组（目录名）一致。
// 说明：
//  - 只生成「对象」形态的 schema（有 properties / allOf / oneOf / anyOf）；纯基础类型、空 object 不生成对象
//  - 嵌套对象（内联 object、数组元素 object）递归生成为独立对象，命名 = 父对象名 + 字段名首字母大写
//  - $ref 指向文档级 schema 时建立 refHash 引用；结构相同的对象直接复用（对象管理不允许结构重复的对象）
//  - 循环引用的回边不建立 refHash（对象 hash 依赖引用 hash，避免 hash 循环依赖）

/// OpenAPI 文档对象导入结果
#[derive(Default)]
struct OpenApiObjectResult {
    /// 待写入对象管理的新对象（嵌套对象在前，父对象在后）
    objects: Vec<ObjectDef>,
    /// 文档 schema 名 → 对象管理中的对象名（用于接口文档的 Body 对象绑定）
    name_by_schema: HashMap<String, String>,
    /// 本次涉及的对象数量（新建 + 复用已有对象）
    count: usize,
}

/// 文档是否包含对象定义（OAS3 components/schemas / Swagger 2.0 definitions）
fn has_object_schemas(json: &Value) -> bool {
    json.pointer("/components/schemas")
        .or_else(|| json.get("definitions"))
        .and_then(|v| v.as_object())
        .map(|m| !m.is_empty())
        .unwrap_or(false)
}

/// 读取对象分组信息：保持 __info_obj.json 原始内容（避免丢失 order 等字段）
fn read_object_groups(root: &Path) -> Vec<ObjectGroup> {
    let p = root.join(crate::OBJECT_DATA_DIR).join("__info_obj.json");
    fs::read_to_string(&p)
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<ObjectGroup>>(&t).ok())
        .unwrap_or_default()
}

/// 解析文档级 schema 并转换为对象定义（结构相同则复用已有对象）
fn build_openapi_objects(store: &ObjectStore, json: &Value, group: &str) -> OpenApiObjectResult {
    let schemas = json
        .pointer("/components/schemas")
        .or_else(|| json.get("definitions"))
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    // 已有对象：结构 hash → (对象名, 实际 hash)。对象管理不允许结构重复的对象，结构相同即视为同一对象
    let mut existing: HashMap<String, (String, String)> = HashMap::new();
    for o in &store.objects {
        existing
            .entry(object_hash(&o.properties))
            .or_insert_with(|| (o.name.clone(), o.hash.clone()));
    }

    let mut ctx = ObjCtx {
        schemas,
        group: group.to_string(),
        existing,
        built: HashMap::new(),
        new_by_hash: HashMap::new(),
        visiting: Vec::new(),
        out: Vec::new(),
        reused: 0,
    };
    // 按名称排序，保证多次导入结果一致
    let mut names: Vec<String> = ctx.schemas.keys().cloned().collect();
    names.sort();
    for n in &names {
        ensure_named_object(&mut ctx, n);
    }

    let mut name_by_schema: HashMap<String, String> = HashMap::new();
    for n in &names {
        if let Some((obj_name, _)) = ctx.built.get(n) {
            name_by_schema.insert(n.clone(), obj_name.clone());
        }
    }
    let count = ctx.out.len() + ctx.reused;
    OpenApiObjectResult {
        objects: ctx.out,
        name_by_schema,
        count,
    }
}

/// 对象生成上下文
struct ObjCtx {
    /// 文档级 schema 定义
    schemas: serde_json::Map<String, Value>,
    /// 目标分组 id（对象管理中的分组路径）
    group: String,
    /// 已有对象：结构 hash → (对象名, 实际 hash)
    existing: HashMap<String, (String, String)>,
    /// 已生成的顶层 schema：schema 名 → (对象名, 实际 hash)
    built: HashMap<String, (String, String)>,
    /// 本次新建对象：结构 hash → (对象名, 实际 hash)
    new_by_hash: HashMap<String, (String, String)>,
    /// 递归栈（循环引用回边不建立 refHash）
    visiting: Vec<String>,
    /// 新建对象
    out: Vec<ObjectDef>,
    /// 复用的已有对象数量
    reused: usize,
}

/// 按文档 schema 名生成/复用对象的结果
enum NamedResult {
    /// 生成/复用的对象 hash
    Built(String),
    /// 该 schema 不是对象（无属性），无法生成对象
    NotAnObject,
    /// 循环引用回边（该 schema 正在生成中）
    Cycle,
}

/// 按文档 schema 名生成/复用对象
fn ensure_named_object(ctx: &mut ObjCtx, name: &str) -> NamedResult {
    if let Some((_, h)) = ctx.built.get(name) {
        return NamedResult::Built(h.clone());
    }
    if ctx.visiting.iter().any(|v| v == name) {
        return NamedResult::Cycle;
    }
    let Some(schema) = ctx.schemas.get(name).cloned() else {
        return NamedResult::NotAnObject;
    };
    ctx.visiting.push(name.to_string());
    let built = build_object(ctx, &schema, name);
    ctx.visiting.pop();
    match built {
        Some((obj_name, h)) => {
            ctx.built.insert(name.to_string(), (obj_name, h.clone()));
            NamedResult::Built(h)
        }
        None => NamedResult::NotAnObject,
    }
}

/// 由 schema 生成对象定义（嵌套对象递归生成），返回 (对象名, hash)；无属性的 schema 不生成对象
fn build_object(ctx: &mut ObjCtx, schema: &Value, name: &str) -> Option<(String, String)> {
    let pairs = collect_schema_props(&ctx.schemas, schema, 0);
    if pairs.is_empty() {
        return None;
    }
    let description = schema_description(&ctx.schemas, schema);
    let mut props: Vec<ObjectProp> = Vec::new();
    for (key, ps) in pairs {
        let key = key.trim().to_string();
        if key.is_empty() {
            continue;
        }
        let hint = format!("{name}{}", capitalize(&key));
        let (kind, item_kind, ref_hash) = schema_prop_type(ctx, &ps, &hint, 0);
        let prop_desc = schema_description(&ctx.schemas, &ps);
        props.push(ObjectProp {
            key,
            kind,
            item_kind,
            ref_hash,
            description: prop_desc,
            mock: String::new(),
        });
    }
    if props.is_empty() {
        return None;
    }
    // 结构相同：复用已有对象（hash 相同即同一结构）
    let hash = object_hash(&props);
    if let Some((obj_name, h)) = ctx.existing.get(&hash).cloned() {
        ctx.reused += 1;
        return Some((obj_name, h));
    }
    if let Some(v) = ctx.new_by_hash.get(&hash).cloned() {
        return Some(v);
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let obj_name = name.to_string();
    ctx.new_by_hash
        .insert(hash.clone(), (obj_name.clone(), hash.clone()));
    ctx.out.push(ObjectDef {
        uuid: uuid::Uuid::new_v4().to_string(),
        hash: hash.clone(),
        name: obj_name.clone(),
        object_name: code_object_name(&obj_name),
        package_name: String::new(),
        group: ctx.group.clone(),
        deprecated: false,
        description,
        properties: props,
        created_at: now,
        updated_at: now,
        order: None,
    });
    Some((obj_name, hash))
}

/// 字段 schema → (kind, itemKind, refHash)
fn schema_prop_type(
    ctx: &mut ObjCtx,
    schema: &Value,
    name_hint: &str,
    depth: usize,
) -> (String, String, String) {
    if depth > 6 {
        return ("Any".into(), String::new(), String::new());
    }
    // $ref 指向文档级 schema：对象按名称生成/复用；基础类型直接映射为字段类型
    if let Some(rn) = schema.get("$ref").and_then(|v| v.as_str()).and_then(ref_schema_name) {
        let Some(target) = ctx.schemas.get(&rn).cloned() else {
            return ("Any".into(), String::new(), String::new());
        };
        return match schema_shape(&target) {
            SchemaShape::Object => match ensure_named_object(ctx, &rn) {
                NamedResult::Built(h) => ("Object".into(), String::new(), h),
                // 循环引用回边：保留 Object 类型信息，但不建立引用（对象 hash 依赖引用 hash）
                NamedResult::Cycle => ("Object".into(), String::new(), String::new()),
                NamedResult::NotAnObject => ("Any".into(), String::new(), String::new()),
            },
            SchemaShape::Array => {
                let (item_kind, ref_hash) = schema_item_type(ctx, &target, name_hint, depth);
                ("List".into(), item_kind, ref_hash)
            }
            SchemaShape::Primitive(k) => (k, String::new(), String::new()),
            SchemaShape::Any => ("Any".into(), String::new(), String::new()),
        };
    }
    match schema_shape(schema) {
        // 内联对象：递归生成为独立对象（无属性时退化为 Any）
        SchemaShape::Object => match build_object(ctx, schema, name_hint) {
            Some((_, h)) => ("Object".into(), String::new(), h),
            None => ("Any".into(), String::new(), String::new()),
        },
        SchemaShape::Array => {
            let (item_kind, ref_hash) = schema_item_type(ctx, schema, name_hint, depth);
            ("List".into(), item_kind, ref_hash)
        }
        SchemaShape::Primitive(k) => (k, String::new(), String::new()),
        SchemaShape::Any => ("Any".into(), String::new(), String::new()),
    }
}

/// 数组元素类型 → (itemKind, refHash)；多维数组退化为 Any（对象属性只支持一层元素类型）
fn schema_item_type(
    ctx: &mut ObjCtx,
    schema: &Value,
    name_hint: &str,
    depth: usize,
) -> (String, String) {
    let Some(items) = schema.get("items") else {
        return ("Any".into(), String::new());
    };
    let (kind, _, ref_hash) = schema_prop_type(ctx, items, name_hint, depth + 1);
    if kind == "List" {
        return ("Any".into(), String::new());
    }
    (kind, ref_hash)
}

/// schema 形态（用于字段类型判定）
enum SchemaShape {
    Object,
    Array,
    Primitive(String),
    Any,
}

/// 判定 schema 形态：properties / allOf / oneOf / anyOf → Object；items 或 type=array → Array
fn schema_shape(s: &Value) -> SchemaShape {
    let ty = s.get("type").and_then(|v| v.as_str());
    if ty == Some("array") || s.get("items").is_some() {
        return SchemaShape::Array;
    }
    if s.get("properties").is_some()
        || s.get("allOf").is_some()
        || s.get("oneOf").is_some()
        || s.get("anyOf").is_some()
        || ty == Some("object")
    {
        return SchemaShape::Object;
    }
    match ty {
        Some(t) => SchemaShape::Primitive(primitive_kind(t, s)),
        // 无 type：按首个枚举值推断，否则 Any
        None => match s.get("enum").and_then(|v| v.as_array()).and_then(|a| a.first()) {
            Some(Value::String(_)) => SchemaShape::Primitive("String".into()),
            Some(Value::Number(n)) => SchemaShape::Primitive(if n.is_i64() || n.is_u64() {
                "Integer".into()
            } else {
                "Float".into()
            }),
            Some(Value::Bool(_)) => SchemaShape::Primitive("Boolean".into()),
            _ => SchemaShape::Any,
        },
    }
}

/// JSON Schema 基础类型 → 对象属性类型（与对象管理页签的类型一致）
fn primitive_kind(ty: &str, s: &Value) -> String {
    match ty {
        "integer" => "Integer".into(),
        "number" => "Float".into(),
        "boolean" => "Boolean".into(),
        "string" => match s.get("format").and_then(|f| f.as_str()) {
            Some("date-time") => "Datetime".into(),
            Some("date") => "Date".into(),
            Some("time") => "Time".into(),
            _ => "String".into(),
        },
        _ => "Any".into(),
    }
}

/// 汇总 schema 的属性（合并 allOf；oneOf / anyOf 取首个分支），按 key 升序返回
fn collect_schema_props(
    schemas: &serde_json::Map<String, Value>,
    schema: &Value,
    depth: usize,
) -> Vec<(String, Value)> {
    if depth > 6 {
        return Vec::new();
    }
    let mut out: BTreeMap<String, Value> = BTreeMap::new();
    if let Some(p) = schema.get("properties").and_then(|v| v.as_object()) {
        for (k, v) in p {
            out.insert(k.clone(), v.clone());
        }
    }
    for comb in ["allOf", "oneOf", "anyOf"] {
        let Some(arr) = schema.get(comb).and_then(|v| v.as_array()) else {
            continue;
        };
        // allOf 全部分支合并；oneOf / anyOf 只取首个（多分支无法合并为单一结构）
        let members: Vec<&Value> = if comb == "allOf" {
            arr.iter().collect()
        } else {
            arr.iter().take(1).collect()
        };
        for m in members {
            let resolved = resolve_schema_value(schemas, m);
            for (k, v) in collect_schema_props(schemas, &resolved, depth + 1) {
                out.entry(k).or_insert(v);
            }
        }
    }
    out.into_iter().collect()
}

/// 解析 $ref（指向文档级 schema 时返回目标，否则原样返回）
fn resolve_schema_value(schemas: &serde_json::Map<String, Value>, s: &Value) -> Value {
    if let Some(name) = s.get("$ref").and_then(|v| v.as_str()).and_then(ref_schema_name) {
        if let Some(t) = schemas.get(&name) {
            return t.clone();
        }
    }
    s.clone()
}

/// 解析 $ref 指向的文档级 schema 名（#/components/schemas/X 或 #/definitions/X）
fn ref_schema_name(r: &str) -> Option<String> {
    let r = r.trim();
    let rest = r
        .strip_prefix("#/components/schemas/")
        .or_else(|| r.strip_prefix("#/definitions/"))?;
    if rest.trim().is_empty() {
        None
    } else {
        Some(rest.to_string())
    }
}

/// 字段说明：优先取字段自身 description，$ref 字段回退到目标 schema 的 description
fn schema_description(schemas: &serde_json::Map<String, Value>, schema: &Value) -> String {
    if let Some(d) = schema.get("description").and_then(|v| v.as_str()) {
        if !d.trim().is_empty() {
            return d.trim().to_string();
        }
    }
    if let Some(name) = schema.get("$ref").and_then(|v| v.as_str()).and_then(ref_schema_name) {
        if let Some(d) = schemas
            .get(&name)
            .and_then(|t| t.get("description"))
            .and_then(|v| v.as_str())
        {
            return d.trim().to_string();
        }
    }
    String::new()
}

/// 代码生成类名：仅保留 ASCII 字母/数字（其余转下划线），首字符必须是字母，否则留空（不生成代码）
fn code_object_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if cleaned
        .chars()
        .next()
        .map(|c| c.is_ascii_alphabetic())
        .unwrap_or(false)
    {
        cleaned
    } else {
        String::new()
    }
}
