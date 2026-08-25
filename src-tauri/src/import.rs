//! 全部导入格式：OpenAPI / Postman / Apifox / Apipost / RAML / WADL / HAR / YApi /
//! apiDoc / 批量 10 格式 / JMeter / Eolink / Insomnia / Markdown。

use crate::{
    read_env_file, sanitize_filename, unique_path, workspace_root, write_pretty, ApiFile, BodyData,
    DocParam, EnvVariable, Environment, InfoJson, KeyValue, MockConfig, ResponseItem, WorkspaceState,
    ENV_FILE, INFO_FILE,
};
use crate::markdown;
use crate::markdown::MarkdownImportResult;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, State};

// ==================== Postman Collection 导入 ====================

/// Postman 导入结果：folder 为新建分组路径，env/vars 为导入的环境变量信息
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostmanImportResult {
    pub folder: String,
    /// 变量合并到的环境变量集名称（未导入变量时为空串）
    pub env: String,
    /// 导入的变量数量
    pub vars: usize,
    /// 导入统计：http 接口数 / WebSocket 接口数 / GraphQL 接口数 / Socket.IO 接口数 / 对象数 / 失败数 / 重复数
    #[serde(default)]
    pub http: usize,
    #[serde(default)]
    pub ws: usize,
    #[serde(default)]
    pub graphql: usize,
    #[serde(default)]
    pub socketio: usize,
    #[serde(default)]
    pub objects: usize,
    #[serde(default)]
    pub failed: usize,
    #[serde(default)]
    pub duplicated: usize,
}

#[tauri::command]
pub(crate) fn import_postman(
    app: AppHandle,
    state: State<'_, WorkspaceState>,
) -> Result<Option<PostmanImportResult>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app
        .dialog()
        .file()
        .add_filter("Postman Collection", &["json"])
        .blocking_pick_file();
    let Some(path) = picked else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    let root = workspace_root(&state)?;
    let result = import_postman_file(&root, &path)?;
    Ok(Some(result))
}

/// 解析 Postman Collection 文件，在工作区根新建同名分组并导入全部接口；
/// 同时把集合级 `variable` 合并到工作区环境变量（__envs.json）
pub(crate) fn import_postman_file(root: &Path, file: &Path) -> Result<PostmanImportResult, String> {
    let mut stats = ImportStats::default();
    let mut failed = 0usize;
    let mut duplicated = 0usize;
    let content = fs::read_to_string(file).map_err(|e| format!("读取文件失败: {e}"))?;
    let json: Value =
        serde_json::from_str(&content).map_err(|e| format!("解析 JSON 失败: {e}"))?;
    let info = json
        .get("info")
        .ok_or("不是有效的 Postman Collection 文件（缺少 info 字段）")?;
    let coll_name = info
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Postman 导入")
        .to_string();
    let dir_name = sanitize_filename(&coll_name);
    let dir_name = if dir_name.is_empty() {
        "Postman 导入".to_string()
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
            name: Some(coll_name.clone()),
            description: format!("从 Postman Collection 导入（{src_name}）"),
            base_url: None,
            mock_port: None,
            order: None,
            collapsed: None,
            deprecated: None,
        },
    )?;
    let items = json
        .get("item")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    import_postman_items(&folder, &items, &mut stats, &mut failed, &mut duplicated)?;
    // 集合级变量 -> 环境变量集（同名合并，否则新建；无激活环境时自动激活）
    let mut env = String::new();
    let mut vars = 0usize;
    if let Some(arr) = json.get("variable").and_then(|v| v.as_array()) {
        let env_vars = postman_variables_to_env(arr);
        if !env_vars.is_empty() {
            vars = env_vars.len();
            env = coll_name.clone();
            merge_postman_env(root, &env, env_vars)?;
        }
    }
    Ok(PostmanImportResult {
        folder: folder.to_string_lossy().to_string(),
        env,
        vars,
        http: stats.http,
        ws: stats.ws,
        graphql: stats.graphql,
        socketio: stats.socketio,
        objects: 0,
        failed,
        duplicated,
    })
}

/// 将 Postman variable 数组（{key, value, type, description}）转换为应用环境变量
fn postman_variables_to_env(vars: &[Value]) -> Vec<EnvVariable> {
    vars.iter()
        .filter_map(|v| {
            let key = v
                .get("key")
                .and_then(|k| k.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if key.is_empty() {
                return None;
            }
            let value = match v.get("value") {
                Some(Value::String(s)) => s.clone(),
                Some(Value::Number(n)) => n.to_string(),
                Some(Value::Bool(b)) => b.to_string(),
                Some(Value::Null) | None => String::new(),
                Some(other) => other.to_string(),
            };
            let description = match v.get("description") {
                Some(Value::String(s)) => s.clone(),
                // Postman 结构化描述：{ "content": "...", "type": "text/plain" }
                Some(Value::Object(o)) => o
                    .get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string(),
                _ => String::new(),
            };
            Some(EnvVariable {
                key,
                value,
                default_value: String::new(),
                description,
                enabled: !v.get("disabled").and_then(|d| d.as_bool()).unwrap_or(false),
            })
        })
        .collect()
}

/// 把导入的变量合并进工作区 __envs.json：
/// 同名环境变量集存在则按 key 合并，否则新建；无激活环境时自动激活该集
fn merge_postman_env(root: &Path, env_name: &str, variables: Vec<EnvVariable>) -> Result<(), String> {
    if variables.is_empty() {
        return Ok(());
    }
    let mut store = read_env_file(root);
    if let Some(env) = store.environments.iter_mut().find(|e| e.name == env_name) {
        for v in variables {
            match env.variables.iter_mut().find(|x| x.key == v.key) {
                Some(existing) => {
                    if !v.value.is_empty() {
                        existing.value = v.value;
                    }
                    if !v.description.is_empty() {
                        existing.description = v.description;
                    }
                    existing.enabled = true;
                }
                None => env.variables.push(v),
            }
        }
    } else {
        store.environments.push(Environment {
            name: env_name.to_string(),
            variables,
        });
    }
    if store.active.is_empty() {
        store.active = env_name.to_string();
    }
    write_pretty(&root.join(ENV_FILE), &store)
}

/// 递归导入 item 列表：带 request 的生成接口文件，带 item 的生成子分组
fn import_postman_items(
    dir: &Path,
    items: &[Value],
    stats: &mut ImportStats,
    failed: &mut usize,
    duplicated: &mut usize,
) -> Result<(), String> {
    for item in items {
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("未命名")
            .to_string();
        if item.get("request").is_some() {
            let request = &item["request"];
            let api = match postman_request_to_api(&name, request) {
                Ok(a) => a,
                Err(_) => {
                    *failed += 1;
                    continue;
                }
            };
            stats.add(&api.protocol);
            let file_base = sanitize_filename(&name);
            let file_base = if file_base.is_empty() {
                "未命名接口".to_string()
            } else {
                file_base
            };
            let file_path = unique_path(dir, &file_base, ".json");
            if file_path != dir.join(format!("{file_base}.json")) {
                *duplicated += 1;
            }
            write_pretty(&file_path, &api)?;
        } else if let Some(sub) = item.get("item").and_then(|v| v.as_array()) {
            let sub_base = sanitize_filename(&name);
            let sub_base = if sub_base.is_empty() {
                "子分组".to_string()
            } else {
                sub_base
            };
            let sub_dir = unique_path(dir, &sub_base, "");
            fs::create_dir_all(&sub_dir).map_err(|e| format!("创建分组失败: {e}"))?;
            write_pretty(
                &sub_dir.join(INFO_FILE),
                &InfoJson {
                    name: Some(name.clone()),
                    description: String::new(),
                    base_url: None,
                    mock_port: None,
                    order: None,
                    collapsed: None,
                    deprecated: None,
                },
            )?;
            import_postman_items(&sub_dir, sub, stats, failed, duplicated)?;
        }
    }
    Ok(())
}

/// 将 Postman request 对象转换为 ApiFile
fn postman_request_to_api(name: &str, request: &Value) -> Result<ApiFile, String> {
    let method = request
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET")
        .to_uppercase();
    let url = request.get("url").unwrap_or(&Value::Null);
    let url_raw = url
        .get("raw")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let (path, params) = postman_path_info(url);

    // 接口协议分类：WebSocket（method=WS 或 ws://） / GraphQL（body.mode=graphql） / Socket.IO（method=SOCKET.IO 或 socket.io://） / HTTP
    let body_mode = request
        .pointer("/body/mode")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    let url_lower = url_raw.to_lowercase();
    let protocol = if method == "WS"
        || url_lower.starts_with("ws://")
        || url_lower.starts_with("wss://")
    {
        "websocket".to_string()
    } else if body_mode == "graphql" {
        "graphql".to_string()
    } else if method == "SOCKET.IO"
        || url_lower.starts_with("socket.io://")
        || url_lower.starts_with("socketio://")
    {
        "socketio".to_string()
    } else {
        "http".to_string()
    };

    let mut headers = Vec::new();
    if let Some(arr) = request.pointer("/header").and_then(|v| v.as_array()) {
        for h in arr {
            let key = h
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if key.is_empty() {
                continue;
            }
            headers.push(KeyValue {
                key,
                value: h.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                enabled: !h.get("disabled").and_then(|v| v.as_bool()).unwrap_or(false),
                is_file: false,
                description: String::new(),
            });
        }
    }

    let mut query = Vec::new();
    if let Some(arr) = url.get("query").and_then(|v| v.as_array()) {
        for q in arr {
            let key = q
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if key.is_empty() {
                continue;
            }
            query.push(KeyValue {
                key,
                value: q
                    .get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                enabled: !q.get("disabled").and_then(|v| v.as_bool()).unwrap_or(false),
                is_file: false,
                description: String::new(),
            });
        }
    }

    let body = postman_body(request.get("body"));
    let description = request
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        method,
        path,
        url: url_raw,
        description,
        headers,
        query,
        params,
        body,
        mock: MockConfig::default(),
        examples: vec![],
        responses: vec![],
        doc_params: vec![],
        deprecated: false,
        protocol,
    })
}

/// 从 Postman URL 中提取路径（:id 转为 {id}）与路径参数
fn postman_path_info(url: &Value) -> (String, Vec<KeyValue>) {
    let raw = url.get("raw").and_then(|v| v.as_str()).unwrap_or("");
    let no_query = raw.split('?').next().unwrap_or("");
    let no_hash = no_query.split('#').next().unwrap_or("");
    let after_scheme = match no_hash.splitn(2, "://").nth(1) {
        Some(rest) => rest,
        None => no_hash,
    };
    let path_only = match after_scheme.find('/') {
        Some(i) => &after_scheme[i..],
        None => "/",
    };
    let mut params = Vec::new();
    let segs: Vec<String> = path_only
        .split('/')
        .map(|seg| {
            if let Some(var) = seg.strip_prefix(':') {
                if !var.is_empty() {
                    params.push(KeyValue {
                        key: var.to_string(),
                        value: String::new(),
                        enabled: true,
                        is_file: false,
                        description: String::new(),
                    });
                    return format!("{{{var}}}");
                }
            }
            seg.to_string()
        })
        .collect();
    let mut path = segs.join("/");
    if !path.starts_with('/') {
        path.insert(0, '/');
    }
    if path.is_empty() {
        path = "/".to_string();
    }
    (path, params)
}

/// 转换 Postman body（raw / urlencoded / formdata）
fn postman_body(body: Option<&Value>) -> BodyData {
    let mut out = BodyData::default();
    let Some(body) = body else {
        return out;
    };
    match body.get("mode").and_then(|v| v.as_str()) {
        Some("raw") => {
            let raw = body.get("raw").and_then(|v| v.as_str()).unwrap_or("");
            let trimmed = raw.trim_start();
            out.mode = if trimmed.starts_with('{') || trimmed.starts_with('[') {
                "json".to_string()
            } else {
                "raw".to_string()
            };
            out.raw = raw.to_string();
        }
        Some("urlencoded") | Some("formdata") => {
            out.mode = "form".into();
            let arr = body
                .get("urlencoded")
                .or_else(|| body.get("formdata"))
                .and_then(|v| v.as_array());
            if let Some(arr) = arr {
                for f in arr {
                    let key = f
                        .get("key")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    if key.is_empty() {
                        continue;
                    }
                    // Postman formdata 可带 type: file 表示文件字段
                    let is_file = f.get("type").and_then(|v| v.as_str()).unwrap_or("") == "file";
                    out.form.push(KeyValue {
                        key,
                        value: f
                            .get("value")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        enabled: !f.get("disabled").and_then(|v| v.as_bool()).unwrap_or(false),
                        is_file,
                        description: String::new(),
                    });
                }
            }
        }
        _ => {}
    }
    out
}

/// OpenAPI / Swagger 导入结果
/// 导入统计：按接口协议分类（http / websocket / graphql / socketio）
#[derive(Default, Clone, Copy)]
pub struct ImportStats {
    pub http: usize,
    pub ws: usize,
    pub graphql: usize,
    pub socketio: usize,
}

impl ImportStats {
    /// 按协议归入对应分类（未知协议按 http 计）
    pub fn add(&mut self, protocol: &str) {
        match protocol {
            "websocket" => self.ws += 1,
            "graphql" => self.graphql += 1,
            "socketio" => self.socketio += 1,
            _ => self.http += 1,
        }
    }

    pub fn total(&self) -> usize {
        self.http + self.ws + self.graphql + self.socketio
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiImportResult {
    pub folder: String,
    pub count: usize,
    /// 导入统计：http 接口数 / WebSocket 接口数 / GraphQL 接口数 / Socket.IO 接口数 / 对象数 / 失败数 / 重复数
    #[serde(default)]
    pub http: usize,
    #[serde(default)]
    pub ws: usize,
    #[serde(default)]
    pub graphql: usize,
    #[serde(default)]
    pub socketio: usize,
    #[serde(default)]
    pub objects: usize,
    #[serde(default)]
    pub failed: usize,
    #[serde(default)]
    pub duplicated: usize,
}

impl Default for OpenApiImportResult {
    fn default() -> Self {
        Self {
            folder: String::new(),
            count: 0,
            http: 0,
            ws: 0,
            graphql: 0,
            socketio: 0,
            objects: 0,
            failed: 0,
            duplicated: 0,
        }
    }
}

/// 导入 Markdown 接口文档：弹窗选 .md 文件，在工作区根新建分组并逐个保存接口
#[tauri::command]
pub(crate) fn import_markdown(
    app: AppHandle,
    state: State<'_, WorkspaceState>,
) -> Result<Option<MarkdownImportResult>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app
        .dialog()
        .file()
        .add_filter("Markdown", &["md", "markdown"])
        .blocking_pick_file();
    let Some(p) = picked else {
        return Ok(None);
    };
    let file = p.into_path().map_err(|e| e.to_string())?;
    let root = workspace_root(&state)?;
    let content = fs::read_to_string(&file).map_err(|e| format!("读取文件失败: {e}"))?;
    let parsed = markdown::parse(&content)?;
    if parsed.apis.is_empty() {
        return Err("文档中没有解析到接口".into());
    }
    let group = parsed.group.trim().to_string();
    let src_name = file
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    // 无分组（# 标题留空）：接口直接写到工作区根目录
    if group.is_empty() {
        let mut count = 0usize;
        let mut stats = ImportStats::default();
        for mut api in parsed.apis {
            api.uuid = uuid::Uuid::new_v4().to_string();
            stats.add(&api.protocol);
            let fname = sanitize_filename(&api.name);
            let fname = if fname.is_empty() {
                "未命名接口".to_string()
            } else {
                fname
            };
            let target = unique_path(&root, &fname, ".json");
            write_pretty(&target, &api)?;
            count += 1;
        }
        return Ok(Some(MarkdownImportResult {
            folder: root.to_string_lossy().to_string(),
            count,
            http: stats.http,
            ws: stats.ws,
            graphql: stats.graphql,
            socketio: stats.socketio,
        }));
    }

    let dir_name = sanitize_filename(&group);
    let dir_name = if dir_name.is_empty() {
        "Markdown 导入".to_string()
    } else {
        dir_name
    };
    let folder = unique_path(&root, &dir_name, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some(group),
            description: format!("从 Markdown 文档导入（{src_name}）"),
            base_url: None,
            mock_port: None,
            order: None,
            collapsed: None,
            deprecated: None,
        },
    )?;
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    for mut api in parsed.apis {
        api.uuid = uuid::Uuid::new_v4().to_string();
        stats.add(&api.protocol);
        let fname = sanitize_filename(&api.name);
        let fname = if fname.is_empty() {
            "未命名接口".to_string()
        } else {
            fname
        };
        let target = unique_path(&folder, &fname, ".json");
        write_pretty(&target, &api)?;
        count += 1;
    }
    Ok(Some(MarkdownImportResult {
        folder: folder.to_string_lossy().to_string(),
        count,
        http: stats.http,
        ws: stats.ws,
        graphql: stats.graphql,
        socketio: stats.socketio,
    }))
}

// ==================== Apifox / Apipost 导入 ====================

/// 导入 Apifox 项目（apifox-project.json）：弹窗选文件，工作区根新建同名分组
#[tauri::command]
pub(crate) fn import_apifox(
    app: AppHandle,
    state: State<'_, WorkspaceState>,
) -> Result<Option<OpenApiImportResult>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app
        .dialog()
        .file()
        .add_filter("Apifox 项目", &["json"])
        .blocking_pick_file();
    let Some(path) = picked else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    let root = workspace_root(&state)?;
    let result = import_apifox_file(&root, &path)?;
    Ok(Some(result))
}

/// 解析 Apifox 项目文件：apiCollection + webSocketCollection 全部导入
pub(crate) fn import_apifox_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let content = fs::read_to_string(file).map_err(|e| format!("读取文件失败: {e}"))?;
    let json: Value =
        serde_json::from_str(&content).map_err(|e| format!("解析 JSON 失败: {e}"))?;
    if json.get("apiCollection").is_none() && json.get("webSocketCollection").is_none() {
        return Err("不是有效的 Apifox 项目文件（缺少 apiCollection 字段）".into());
    }
    let coll_name = json
        .pointer("/info/name")
        .and_then(|v| v.as_str())
        .unwrap_or("Apifox 导入")
        .to_string();
    let dir_name = sanitize_filename(&coll_name);
    let dir_name = if dir_name.is_empty() {
        "Apifox 导入".to_string()
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
            name: Some(coll_name.clone()),
            description: format!("从 Apifox 项目导入（{src_name}）"),
            base_url: None,
            mock_port: None,
            order: None,
            collapsed: None,
            deprecated: None,
        },
    )?;
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    // HTTP 接口：apiCollection 为数组（可能多个集合）或对象 {items:[...]}
    match json.get("apiCollection") {
        Some(Value::Array(arr)) => {
            for c in arr {
                count += import_apifox_items(&folder, c, &mut stats)?;
            }
        }
        Some(obj) => {
            if let Some(items) = obj.get("items").and_then(|v| v.as_array()) {
                count += import_apifox_items_arr(&folder, items, &mut stats)?;
            }
        }
        _ => {}
    }
    // WebSocket 接口：webSocketCollection（api 无 method，path 即 ws url，消息体在 requestBody.message）
    if let Some(arr) = json.get("webSocketCollection").and_then(|v| v.as_array()) {
        for c in arr {
            count += import_apifox_items(&folder, c, &mut stats)?;
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

/// 单个 Apifox 集合：取 items 递归导入
fn import_apifox_items(dir: &Path, collection: &Value, stats: &mut ImportStats) -> Result<usize, String> {
    let items = collection
        .get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    import_apifox_items_arr(dir, &items, stats)
}

/// Apifox items 递归：带 api 的为接口，带 items 的为分组
fn import_apifox_items_arr(dir: &Path, items: &[Value], stats: &mut ImportStats) -> Result<usize, String> {
    let mut count = 0usize;
    for item in items {
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("未命名")
            .to_string();
        if let Some(api_obj) = item.get("api") {
            let api = apifox_api_to_api(&name, api_obj)?;
            stats.add(&api.protocol);
            let file_base = sanitize_filename(&name);
            let file_base = if file_base.is_empty() {
                "未命名接口".to_string()
            } else {
                file_base
            };
            let file_path = unique_path(dir, &file_base, ".json");
            write_pretty(&file_path, &api)?;
            count += 1;
        } else if let Some(sub) = item.get("items").and_then(|v| v.as_array()) {
            if sub.is_empty() {
                // Apifox 导出的目录占位（如 webSocketCollection 中的空分组）不创建
                continue;
            }
            let sub_base = sanitize_filename(&name);
            let sub_base = if sub_base.is_empty() {
                "子分组".to_string()
            } else {
                sub_base
            };
            let sub_dir = dir.join(&sub_base);
            // 同名分组已存在时复用目录，避免生成大量「xx (2)」重复分组
            if !sub_dir.is_dir() {
                fs::create_dir_all(&sub_dir).map_err(|e| format!("创建分组失败: {e}"))?;
                write_pretty(
                    &sub_dir.join(INFO_FILE),
                    &InfoJson {
                        name: Some(name.clone()),
                        description: String::new(),
                        base_url: None,
                        mock_port: None,
                        order: None,
                        collapsed: None,
                        deprecated: None,
                    },
                )?;
            }
            count += import_apifox_items_arr(&sub_dir, sub, stats)?;
        }
    }
    Ok(count)
}

/// 将 Apifox api 对象转换为 ApiFile（WebSocket 接口无 method，path 即地址）
fn apifox_api_to_api(name: &str, api_obj: &Value) -> Result<ApiFile, String> {
    let is_ws = api_obj.get("method").is_none() && api_obj.get("path").and_then(|v| v.as_str()).map_or(true, |p| p.contains("ws://") || p.contains("wss://"));
    // 接口协议：优先读 api.protocol（Apifox 导出字段），否则按 is_ws 规则
    let proto = api_obj
        .get("protocol")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    let protocol = if proto.contains("websocket") || proto.contains("ws") {
        "websocket".to_string()
    } else if proto.contains("graphql") {
        "graphql".to_string()
    } else if proto.contains("socket") {
        "socketio".to_string()
    } else if is_ws {
        "websocket".to_string()
    } else {
        "http".to_string()
    };
    let method = if is_ws {
        "WS".to_string()
    } else {
        api_obj
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_uppercase()
    };    let path = api_obj
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("/")
        .to_string();
    let (mut headers, mut query, mut params) = (Vec::new(), Vec::new(), Vec::new());
    if let Some(p) = api_obj.get("parameters").and_then(|v| v.as_object()) {
        let kv_list = |arr: Option<&Vec<Value>>| -> Vec<KeyValue> {
            arr.map(|arr| {
                arr.iter()
                    .map(|v| KeyValue {
                        key: v.get("name").and_then(|x| x.as_str()).unwrap_or("").trim().to_string(),
                        value: v.get("value").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                        enabled: v.get("enable").and_then(|x| x.as_bool()).unwrap_or(true),
                        description: v.get("description").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                        is_file: false,
                    })
                    .collect()
            })
            .unwrap_or_default()
        };
        headers = kv_list(p.get("header").and_then(|v| v.as_array()));
        query = kv_list(p.get("query").and_then(|v| v.as_array()));
        params = kv_list(p.get("path").and_then(|v| v.as_array()));
    }
    let body = if is_ws {
        let msg = api_obj
            .pointer("/requestBody/message")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        BodyData {
            mode: if msg.trim().is_empty() {
                "none".into()
            } else if msg.trim_start().starts_with('{') || msg.trim_start().starts_with('[') {
                "json".into()
            } else {
                "raw".into()
            },
            raw: msg,
            form: vec![],
            binary_path: String::new(),
        }
    } else {
        apifox_body(api_obj.get("requestBody"))
    };
    let description = api_obj
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Ok(ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        method,
        url: path.clone(),
        path,
        description,
        headers,
        query,
        params,
        body,
        mock: MockConfig::default(),
        examples: vec![],
        responses: vec![],
        doc_params: vec![],
        deprecated: false,
        protocol,
    })
}

/// 转换 Apifox requestBody（type: json / form-data / x-www-form-urlencoded / raw / none）
fn apifox_body(body: Option<&Value>) -> BodyData {
    let mut out = BodyData::default();
    let Some(body) = body else {
        return out;
    };
    let ty = body.get("type").and_then(|v| v.as_str()).unwrap_or("none");
    match ty {
        "json" | "raw" => {
            // JSON 示例体在 examples[].data；raw 直接用 raw
            let raw = if ty == "raw" {
                body.get("raw").and_then(|v| v.as_str()).unwrap_or("").to_string()
            } else {
                body.get("examples")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|e| e.get("data"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            };
            out.mode = if ty == "raw" { "raw".into() } else { "json".into() };
            out.raw = raw;
        }
        "form-data" | "x-www-form-urlencoded" => {
            out.mode = "form".into();
            if let Some(arr) = body.get("parameters").and_then(|v| v.as_array()) {
                for f in arr {
                    let key = f.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                    if key.is_empty() {
                        continue;
                    }
                    out.form.push(KeyValue {
                        key,
                        value: f.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        enabled: f.get("enable").and_then(|v| v.as_bool()).unwrap_or(true),
                        is_file: f.get("type").and_then(|v| v.as_str()).unwrap_or("") == "file",
                        description: String::new(),
                    });
                }
            }
        }
        _ => {}
    }
    out
}

/// 导入 Apipost 项目（apipost 导出 json）：弹窗选文件，工作区根新建同名分组
#[tauri::command]
pub(crate) fn import_apipost(
    app: AppHandle,
    state: State<'_, WorkspaceState>,
) -> Result<Option<OpenApiImportResult>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app
        .dialog()
        .file()
        .add_filter("Apipost 项目", &["json"])
        .blocking_pick_file();
    let Some(path) = picked else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    let root = workspace_root(&state)?;
    let result = import_apipost_file(&root, &path)?;
    Ok(Some(result))
}

/// 解析 Apipost 项目文件：apis 平铺数组按 parent_id 组织成树
pub(crate) fn import_apipost_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let content = fs::read_to_string(file).map_err(|e| format!("读取文件失败: {e}"))?;
    let json: Value =
        serde_json::from_str(&content).map_err(|e| format!("解析 JSON 失败: {e}"))?;
    let apis = json.get("apis").and_then(|v| v.as_array()).cloned();
    let Some(apis) = apis else {
        return Err("不是有效的 Apipost 项目文件（缺少 apis 字段）".into());
    };
    let coll_name = json
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Apipost 导入")
        .to_string();
    let dir_name = sanitize_filename(&coll_name);
    let dir_name = if dir_name.is_empty() {
        "Apipost 导入".to_string()
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
            name: Some(coll_name.clone()),
            description: format!("从 Apipost 项目导入（{src_name}）"),
            base_url: None,
            mock_port: None,
            order: None,
            collapsed: None,
            deprecated: None,
        },
    )?;
    // target_id → 节点索引
    let mut by_id: HashMap<String, &Value> = HashMap::new();
    for a in &apis {
        if let Some(id) = a.get("target_id").and_then(|v| v.as_str()) {
            by_id.insert(id.to_string(), a);
        }
    }
    // 根节点：parent_id 为 0 或指向不存在的节点
    let mut roots: Vec<&Value> = apis
        .iter()
        .filter(|a| {
            let pid = a.get("parent_id").and_then(|v| v.as_str()).unwrap_or("0");
            pid == "0" || !by_id.contains_key(pid)
        })
        .collect();
    roots.sort_by_key(|a| a.get("sort").and_then(|v| v.as_i64()).unwrap_or(0));
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    for r in roots {
        count += import_apipost_node(&folder, r, &by_id, &mut stats)?;
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

/// Apipost 节点递归：folder 建分组，api/graphql 写接口文件
fn import_apipost_node(
    dir: &Path,
    node: &Value,
    by_id: &HashMap<String, &Value>,
    stats: &mut ImportStats,
) -> Result<usize, String> {
    let name = node
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("未命名")
        .to_string();
    match node.get("target_type").and_then(|v| v.as_str()) {
        Some("folder") => {
            let kids = apipost_node_children(node, by_id);
            if kids.is_empty() {
                // 空文件夹不创建，避免生成空分组
                return Ok(0);
            }
            let sub_dir = dir.join(&sanitize_filename(&name));
            // 同名分组已存在时复用目录（Apipost 树中可能多个同名文件夹）
            if !sub_dir.is_dir() {
                fs::create_dir_all(&sub_dir).map_err(|e| format!("创建分组失败: {e}"))?;
                write_pretty(
                    &sub_dir.join(INFO_FILE),
                    &InfoJson {
                        name: Some(name.clone()),
                        description: String::new(),
                        base_url: None,
                        mock_port: None,
                        order: None,
                        collapsed: None,
                        deprecated: None,
                    },
                )?;
            }
            let mut count = 0usize;
            for c in kids {
                count += import_apipost_node(&sub_dir, c, by_id, stats)?;
            }
            Ok(count)
        }
        _ => {
            let api = apipost_request_to_api(&name, node)?;
            stats.add(&api.protocol);
            let file_base = sanitize_filename(&name);
            let file_base = if file_base.is_empty() {
                "未命名接口".to_string()
            } else {
                file_base
            };
            let file_path = unique_path(dir, &file_base, ".json");
            write_pretty(&file_path, &api)?;
            Ok(1)
        }
    }
}

/// 取某节点的直接子节点（按 sort 排序）
fn apipost_node_children<'a>(node: &Value, by_id: &HashMap<String, &'a Value>) -> Vec<&'a Value> {
    let id = node.get("target_id").and_then(|v| v.as_str()).unwrap_or("");
    let mut kids: Vec<&'a Value> = by_id
        .values()
        .filter(|v| v.get("parent_id").and_then(|x| x.as_str()).unwrap_or("") == id)
        .copied()
        .collect();
    kids.sort_by_key(|v| v.get("sort").and_then(|x| x.as_i64()).unwrap_or(0));
    kids
}

/// 将 Apipost 接口节点转换为 ApiFile
fn apipost_request_to_api(name: &str, node: &Value) -> Result<ApiFile, String> {
    let method = node
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET")
        .to_uppercase();
    let url = node.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let protocol = node.get("protocol").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let proto_lower = protocol.to_lowercase();
    // 接口协议分类：优先读 protocol 字段（http/websocket/graphql/socket.io 等）
    let protocol = if proto_lower.contains("websocket") || proto_lower == "ws" {
        "websocket".to_string()
    } else if proto_lower.contains("graphql") {
        "graphql".to_string()
    } else if proto_lower.contains("socket") {
        "socketio".to_string()
    } else {
        "http".to_string()
    };
    let (path, mut params) = extract_path(&url);
    let request = node.get("request");
    let mut headers = Vec::new();
    let mut query = Vec::new();
    if let Some(req) = request {
        headers = apipost_param_list(
            req.get("header")
                .and_then(|h| h.get("parameter"))
                .and_then(|v| v.as_array()),
        );
        query = apipost_param_list(
            req.get("query")
                .and_then(|h| h.get("parameter"))
                .and_then(|v| v.as_array()),
        );
        let restful = apipost_param_list(
            req.get("restful")
                .and_then(|h| h.get("parameter"))
                .and_then(|v| v.as_array()),
        );
        if !restful.is_empty() {
            params = restful;
        }
    }
    let body = request
        .map(|r| apipost_body(r.get("body")))
        .unwrap_or_default();
    let description = node
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Ok(ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        method,
        path,
        url,
        description,
        headers,
        query,
        params,
        body,
        mock: MockConfig::default(),
        examples: vec![],
        responses: vec![],
        doc_params: vec![],
        deprecated: false,
        protocol,
    })
}

/// 读取 Apipost 参数数组（{key, value, description, is_checked}）
fn apipost_param_list(arr: Option<&Vec<Value>>) -> Vec<KeyValue> {
    arr.map(|arr| {
        arr.iter()
            .map(|v| KeyValue {
                key: v.get("key").and_then(|x| x.as_str()).unwrap_or("").trim().to_string(),
                value: v.get("value").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                enabled: v.get("is_checked").and_then(|x| x.as_i64()).unwrap_or(1) != 0,
                description: v.get("description").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                is_file: false,
            })
            .collect()
    })
    .unwrap_or_default()
}

/// 转换 Apipost body（mode: json / raw / form-data / urlencoded / none）
fn apipost_body(body: Option<&Value>) -> BodyData {
    let mut out = BodyData::default();
    let Some(body) = body else {
        return out;
    };
    match body.get("mode").and_then(|v| v.as_str()) {
        Some("json") | Some("raw") | Some("xml") => {
            out.mode = if body.get("mode").and_then(|v| v.as_str()) == Some("xml") {
                "raw".to_string()
            } else {
                "json".to_string()
            };
            out.raw = body.get("raw").and_then(|v| v.as_str()).unwrap_or("").to_string();
        }
        Some("form-data") | Some("urlencoded") | Some("form") => {
            out.mode = "form".into();
            if let Some(arr) = body.get("parameter").and_then(|v| v.as_array()) {
                for f in arr {
                    let key = f.get("key").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                    if key.is_empty() {
                        continue;
                    }
                    out.form.push(KeyValue {
                        key,
                        value: f.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        enabled: f.get("is_checked").and_then(|v| v.as_i64()).unwrap_or(1) != 0,
                        is_file: f.get("type").and_then(|v| v.as_str()).unwrap_or("") == "file",
                        description: String::new(),
                    });
                }
            }
        }
        _ => {}
    }
    out
}

/// 从 URL 提取路径（:id → {id}）与路径参数
fn extract_path(raw: &str) -> (String, Vec<KeyValue>) {
    let no_query = raw.split('?').next().unwrap_or("");
    let no_hash = no_query.split('#').next().unwrap_or("");
    let after_scheme = match no_hash.splitn(2, "://").nth(1) {
        Some(rest) => rest,
        None => no_hash,
    };
    let path_only = match after_scheme.find('/') {
        Some(i) => &after_scheme[i..],
        None => "/",
    };
    let mut params = Vec::new();
    let segs: Vec<String> = path_only
        .split('/')
        .map(|seg| {
            if let Some(var) = seg.strip_prefix(':') {
                if !var.is_empty() {
                    params.push(KeyValue {
                        key: var.to_string(),
                        value: String::new(),
                        enabled: true,
                        is_file: false,
                        description: String::new(),
                    });
                    return format!("{{{var}}}");
                }
            }
            seg.to_string()
        })
        .collect();
    let mut path = segs.join("/");
    if !path.starts_with('/') {
        path.insert(0, '/');
    }
    if path.is_empty() {
        path = "/".to_string();
    }
    (path, params)
}

// ==================== RAML / WADL 导入 ====================

/// 导入 RAML 文件（.raml）：弹窗选文件，工作区根新建同名分组
#[tauri::command]
pub(crate) fn import_raml(
    app: AppHandle,
    state: State<'_, WorkspaceState>,
) -> Result<Option<OpenApiImportResult>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app
        .dialog()
        .file()
        .add_filter("RAML", &["raml"])
        .blocking_pick_file();
    let Some(path) = picked else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    let root = workspace_root(&state)?;
    let result = import_raml_file(&root, &path)?;
    Ok(Some(result))
}

/// 解析 RAML 1.0 文件（YAML）：title 作分组名，顶层路径 key 为资源，方法对象为接口
pub(crate) fn import_raml_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let content = fs::read_to_string(file).map_err(|e| format!("读取文件失败: {e}"))?;
    let json: Value =
        serde_yaml::from_str(&content).map_err(|e| format!("解析 RAML(YAML) 失败: {e}"))?;
    let title = json
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("RAML 导入")
        .to_string();
    let dir_name = sanitize_filename(&title);
    let dir_name = if dir_name.is_empty() {
        "RAML 导入".to_string()
    } else {
        dir_name
    };
    let folder = unique_path(root, &dir_name, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    let base_url = json
        .get("baseUri")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim_end_matches('/')
        .to_string();
    let src_name = file
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some(title.clone()),
            description: format!("从 RAML 文档导入（{src_name}）"),
            base_url: if base_url.is_empty() {
                None
            } else {
                Some(base_url.clone())
            },
            mock_port: None,
            order: None,
            collapsed: None,
            deprecated: None,
        },
    )?;
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    // 顶层 key：以 / 开头的为资源路径，其余为元数据（title/version/baseUri/mediaType/types/...）
    if let Some(obj) = json.as_object() {
        for (key, val) in obj {
            if !key.starts_with('/') {
                continue;
            }
            count += raml_resource_to_apis(&folder, key, val, &base_url, &mut stats)?;
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

/// RAML 资源节点 → 接口文件（路径 key 为资源，值为方法对象或嵌套资源）
fn raml_resource_to_apis(
    dir: &Path,
    path: &str,
    node: &Value,
    base_url: &str,
    stats: &mut ImportStats,
) -> Result<usize, String> {
    let mut count = 0usize;
    let Some(obj) = node.as_object() else {
        return Ok(0);
    };
    // 子资源：key 不以 HTTP 方法开头且值为对象（含 / 前缀的路径）
    for (key, val) in obj {
        if key.starts_with('/') {
            let joined = format!("{}{}", path.trim_end_matches('/'), key);
            count += raml_resource_to_apis(dir, &joined, val, base_url, stats)?;
            continue;
        }
        let method = key.to_uppercase();
        if !matches!(
            method.as_str(),
            "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "HEAD" | "OPTIONS"
        ) {
            continue;
        }
        count += raml_method_to_api(dir, &method, path, val, base_url, stats)?;
    }
    Ok(count)
}

/// RAML method 对象 → ApiFile
fn raml_method_to_api(
    dir: &Path,
    method: &str,
    path: &str,
    op: &Value,
    base_url: &str,

    stats: &mut ImportStats) -> Result<usize, String> {
    let mut headers = Vec::new();
    let mut query = Vec::new();
    // 查询参数：queryParameters[key] = { type, required, default, description }
    if let Some(qp) = op.get("queryParameters").and_then(|v| v.as_object()) {
        for (k, v) in qp {
            let desc = v
                .get("description")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let value = v
                .get("default")
                .and_then(|x| x.as_str().map(|s| s.to_string()))
                .or_else(|| {
                    v.get("default")
                        .and_then(|x| x.as_i64())
                        .map(|n| n.to_string())
                })
                .unwrap_or_default();
            query.push(KeyValue {
                key: k.clone(),
                value,
                enabled: true,
                is_file: false,
                description: desc,
            });
        }
    }
    // 请求头：headers[key] = { required, description }
    if let Some(hp) = op.get("headers").and_then(|v| v.as_object()) {
        for (k, v) in hp {
            let desc = v
                .get("description")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            headers.push(KeyValue {
                key: k.clone(),
                value: String::new(),
                enabled: true,
                is_file: false,
                description: desc,
            });
        }
    }
    // 路径参数：uriParameters / path 中的 {xxx}
    let mut params = Vec::new();
    if let Some(up) = op.get("uriParameters").and_then(|v| v.as_object()) {
        for (k, v) in up {
            let desc = v
                .get("description")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            params.push(KeyValue {
                key: k.clone(),
                value: String::new(),
                enabled: true,
                is_file: false,
                description: desc,
            });
        }
    }
    // 请求体：body[mediaType] = { type, example }
    let mut body = BodyData::default();
    if let Some(b) = op.get("body").and_then(|v| v.as_object()) {
        let example = b
            .values()
            .find_map(|mt| {
                mt.get("example")
                    .and_then(|e| e.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        // 非字符串示例（对象/数组）序列化
                        mt.get("example").map(|e| {
                            serde_json::to_string_pretty(e).unwrap_or_else(|_| "{}".into())
                        })
                    })
            })
            .unwrap_or_default();
        if !example.is_empty() {
            body.mode = "json".into();
            body.raw = example;
        }
    }
    let description = op
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    // 从 path 提取 {xxx} 参数
    for seg in path.split('/') {
        if let Some(var) = seg.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
            if !params.iter().any(|p| p.key == var) {
                params.push(KeyValue {
                    key: var.to_string(),
                    value: String::new(),
                    enabled: true,
                    is_file: false,
                    description: String::new(),
                });
            }
        }
    }
    let name = format!("{} {}", method, path);
    let file_base = sanitize_filename(&name);
    let file_path = unique_path(dir, &file_base, ".json");
    let api = ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name: name.clone(),
        method: method.to_string(),
        path: path.to_string(),
        url: format!("{}{}", base_url.trim_end_matches('/'), path),
        description,
        headers,
        query,
        params,
        body,
        mock: MockConfig::default(),
        examples: vec![],
        responses: vec![],
        doc_params: vec![],
        deprecated: false,
        protocol: "http".into(),
    };
    write_pretty(&file_path, &api)?;
        stats.add(&api.protocol);
    Ok(1)
}

/// 导入 WADL 文件（.wadl）：弹窗选文件，工作区根新建同名分组
#[tauri::command]
pub(crate) fn import_wadl(
    app: AppHandle,
    state: State<'_, WorkspaceState>,
) -> Result<Option<OpenApiImportResult>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app
        .dialog()
        .file()
        .add_filter("WADL", &["wadl"])
        .blocking_pick_file();
    let Some(path) = picked else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    let root = workspace_root(&state)?;
    let result = import_wadl_file(&root, &path)?;
    Ok(Some(result))
}

/// 解析 WADL 文件（XML）：resources base 为基地址，递归 resource/method 导入接口
pub(crate) fn import_wadl_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let content = fs::read_to_string(file).map_err(|e| format!("读取文件失败: {e}"))?;
    let doc = roxmltree::Document::parse(&content).map_err(|e| format!("解析 WADL(XML) 失败: {e}"))?;
    let root_el = doc.root_element();
    if !root_el.has_tag_name("application") {
        return Err("不是有效的 WADL 文件（缺少 application 根节点）".into());
    }
    let title = root_el.attribute("doc").unwrap_or("").to_string();
    let title = if title.is_empty() || title == "WADL 导入" {
        file.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or("WADL 导入".to_string())
    } else {
        title
    };
    let dir_name = sanitize_filename(&title);
    let dir_name = if dir_name.is_empty() {
        "WADL 导入".to_string()
    } else {
        dir_name
    };
    let folder = unique_path(root, &dir_name, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    // 基地址：resources base 属性
    let base_url = root_el
        .children()
        .find(|n| n.is_element() && n.has_tag_name("resources"))
        .and_then(|r| r.attribute("base"))
        .unwrap_or("")
        .trim_end_matches('/')
        .to_string();
    let src_name = file
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some(title.clone()),
            description: format!("从 WADL 文档导入（{src_name}）"),
            base_url: if base_url.is_empty() {
                None
            } else {
                Some(base_url.clone())
            },
            mock_port: None,
            order: None,
            collapsed: None,
            deprecated: None,
        },
    )?;
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    for res in root_el.descendants().filter(|n| n.is_element() && n.has_tag_name("resources")) {
        for child in res.children().filter(|n| n.is_element() && n.has_tag_name("resource")) {
            count += wadl_resource_to_apis(&folder, "", child, &mut stats)?;
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

/// WADL resource 递归：子 resource 拼接 path，method 写接口文件
fn wadl_resource_to_apis(dir: &Path, parent_path: &str, res: roxmltree::Node, stats: &mut ImportStats) -> Result<usize, String> {
    let rel = res.attribute("path").unwrap_or("");
    let path = if parent_path.is_empty() {
        format!("/{}", rel.trim_start_matches('/'))
    } else if rel.is_empty() {
        parent_path.to_string()
    } else {
        format!("{}/{}", parent_path.trim_end_matches('/'), rel.trim_start_matches('/'))
    };
    let mut count = 0usize;
    for child in res.children().filter(|n| n.is_element()) {
        if child.has_tag_name("resource") {
            count += wadl_resource_to_apis(dir, &path, child, stats)?;
        } else if child.has_tag_name("method") {
            count += wadl_method_to_api(dir, &path, child, stats)?;
        }
    }
    Ok(count)
}

/// WADL method 元素 → ApiFile
fn wadl_method_to_api(dir: &Path, path: &str, method_el: roxmltree::Node,
    stats: &mut ImportStats) -> Result<usize, String> {
    let method = method_el.attribute("name").unwrap_or("GET").to_uppercase();
    let mut headers = Vec::new();
    let mut query = Vec::new();
    let mut params = Vec::new();
    let mut raw = String::new();
    // request 子元素：param（query/header/path/template 样式）与 representation
    if let Some(req) = method_el
        .children()
        .find(|n| n.is_element() && n.has_tag_name("request"))
    {
        for p in req.children().filter(|n| n.is_element() && n.has_tag_name("param")) {
            let key = p.attribute("name").unwrap_or("").trim().to_string();
            if key.is_empty() {
                continue;
            }
            let style = p.attribute("style").unwrap_or("query");
            let value = p.attribute("default").unwrap_or("").to_string();
            let kv = KeyValue {
                key,
                value,
                enabled: true,
                is_file: false,
                description: String::new(),
            };
            match style {
                "query" => query.push(kv),
                "header" => headers.push(kv),
                "path" | "template" => params.push(kv),
                _ => query.push(kv),
            }
        }
        // 请求体：representation mediaType 为 json 时取 example 或留空
        for rep in req.children().filter(|n| n.is_element() && n.has_tag_name("representation")) {
            let mt = rep.attribute("mediaType").unwrap_or("").to_lowercase();
            if mt.contains("json") {
                raw = rep.attribute("example").unwrap_or("").to_string();
            }
        }
    }
    let mut body = BodyData::default();
    if !raw.is_empty() {
        body.mode = "json".into();
        body.raw = raw;
    }
    let name = format!("{} {}", method, path);
    let file_base = sanitize_filename(&name);
    let file_path = unique_path(dir, &file_base, ".json");
    let api = ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name: name.clone(),
        method: method.clone(),
        path: path.to_string(),
        url: path.to_string(),
        description: String::new(),
        headers,
        query,
        params,
        body,
        mock: MockConfig::default(),
        examples: vec![],
        responses: vec![],
        doc_params: vec![],
        deprecated: false,
        protocol: "http".into(),
    };
    write_pretty(&file_path, &api)?;
        stats.add(&api.protocol);
    Ok(1)
}

// ==================== HAR 导入 ====================

/// 浏览器自动附带的请求头（导入时过滤，避免冗余）
const HAR_SKIP_HEADERS: [&str; 24] = [
    "accept",
    "accept-encoding",
    "accept-language",
    "cache-control",
    "connection",
    "cookie",
    "host",
    "origin",
    "pragma",
    "referer",
    "user-agent",
    "upgrade-insecure-requests",
    "sec-ch-ua",
    "sec-ch-ua-mobile",
    "sec-ch-ua-platform",
    "sec-fetch-dest",
    "sec-fetch-mode",
    "sec-fetch-site",
    "sec-fetch-user",
    "priority",
    "te",
    "dnt",
    "if-none-match",
    "if-modified-since",
];

/// 导入 HAR 文件（.har）：弹窗选文件，工作区根新建同名分组，按 host 分小组
#[tauri::command]
pub(crate) fn import_har(
    app: AppHandle,
    state: State<'_, WorkspaceState>,
) -> Result<Option<OpenApiImportResult>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app
        .dialog()
        .file()
        .add_filter("HAR", &["har", "json"])
        .blocking_pick_file();
    let Some(path) = picked else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    let root = workspace_root(&state)?;
    let result = import_har_file(&root, &path)?;
    Ok(Some(result))
}

/// 解析 HAR 文件：log.entries 逐条导入接口，按 host 分小组
pub(crate) fn import_har_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let content = fs::read_to_string(file).map_err(|e| format!("读取文件失败: {e}"))?;
    let json: Value =
        serde_json::from_str(&content).map_err(|e| format!("解析 HAR 失败: {e}"))?;
    let entries = json
        .pointer("/log/entries")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "不是有效的 HAR 文件（缺少 log.entries）".to_string())?;
    let title = json
        .pointer("/log/pages/0/title")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            file.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "HAR 导入".to_string())
        });
    let dir_name = sanitize_filename(&title);
    let dir_name = if dir_name.is_empty() {
        "HAR 导入".to_string()
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
            description: format!("从 HAR 抓包文件导入（{src_name}）"),
            base_url: None,
            mock_port: None,
            order: None,
            collapsed: None,
            deprecated: None,
        },
    )?;
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    // 按 host 分组，避免重复建同名 host 目录
    let mut by_host: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    for e in entries {
        let url = e
            .pointer("/request/url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let host = url
            .split("://")
            .nth(1)
            .and_then(|r| r.split('/').next())
            .unwrap_or("")
            .to_string();
        let host = if host.is_empty() {
            "未分类".to_string()
        } else {
            host
        };
        by_host.entry(host).or_default().push(e);
    }
    for (host, list) in &by_host {
        let sub = folder.join(&sanitize_filename(host));
        // 同名 host 分组已存在时复用目录
        if !sub.is_dir() {
            fs::create_dir_all(&sub).map_err(|e| format!("创建分组失败: {e}"))?;
            write_pretty(
                &sub.join(INFO_FILE),
                &InfoJson {
                    name: Some(host.clone()),
                    description: String::new(),
                    base_url: None,
                    mock_port: None,
                    order: None,
                    collapsed: None,
                    deprecated: None,
                },
            )?;
        }
        for e in list {
            count += har_entry_to_api(&sub, e, &mut stats)?;
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

/// HAR entry → ApiFile：method/url/headers/queryString/postData，响应存为返回示例
fn har_entry_to_api(dir: &Path, entry: &Value,
    stats: &mut ImportStats) -> Result<usize, String> {
    let req = entry.get("request").and_then(|v| v.as_object()).cloned().unwrap_or_default();
    let method = req
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET")
        .to_uppercase();
    let url = req
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if url.is_empty() || method.is_empty() {
        return Ok(0);
    }
    let (path, params) = extract_path(&url);
    // 请求头：过滤浏览器自动头
    let mut headers = Vec::new();
    if let Some(hs) = req.get("headers").and_then(|v| v.as_array()) {
        for h in hs {
            let key = h
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_lowercase();
            if key.is_empty() || HAR_SKIP_HEADERS.contains(&key.as_str()) {
                continue;
            }
            let value = h.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
            headers.push(KeyValue {
                key,
                value,
                enabled: true,
                is_file: false,
                description: String::new(),
            });
        }
    }
    // 查询参数
    let mut query = Vec::new();
    if let Some(qs) = req.get("queryString").and_then(|v| v.as_array()) {
        for q in qs {
            let key = q.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if key.is_empty() {
                continue;
            }
            let value = q.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
            query.push(KeyValue {
                key,
                value,
                enabled: true,
                is_file: false,
                description: String::new(),
            });
        }
    }
    // 请求体
    let mut body = BodyData::default();
    if let Some(pd) = req.get("postData") {
        let text = pd.get("text").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let mime = pd.get("mimeType").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
        if !text.is_empty() {
            if mime.contains("json") {
                body.mode = "json".into();
            } else if mime.contains("form-urlencoded") || mime.contains("multipart") {
                body.mode = "form".into();
                // urlencoded 表单：a=1&b=2 解析为 form 列表
                for pair in text.split('&') {
                    if let Some((k, v)) = pair.split_once('=') {
                        body.form.push(KeyValue {
                            key: k.trim().to_string(),
                            value: percent_encoding::percent_decode_str(v.trim())
                                .decode_utf8_lossy()
                                .to_string(),
                            enabled: true,
                            is_file: false,
                            description: String::new(),
                        });
                    }
                }
                body.raw = String::new();
            } else {
                body.mode = "raw".into();
            }
            if body.mode != "form" {
                body.raw = text;
            }
        }
    }
    // 响应存为返回示例
    let mut responses = Vec::new();
    if let Some(resp) = entry.get("response") {
        let status = resp.get("status").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
        let content_type = resp
            .pointer("/content/mimeType")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let resp_text = resp
            .pointer("/content/text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if !resp_text.is_empty() {
            let truncated = if resp_text.chars().count() > 100_000 {
                resp_text.chars().take(100_000).collect()
            } else {
                resp_text
            };
            responses.push(ResponseItem {
                id: uuid::Uuid::new_v4().to_string(),
                name: format!("HTTP {status}"),
                status,
                content_type,
                body: truncated,
            });
        }
    }
    let name = format!("{} {}", method, path);
    let file_base = sanitize_filename(&name);
    let file_path = unique_path(dir, &file_base, ".json");
    let api = ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name: name.clone(),
        method: method.clone(),
        path: path.clone(),
        url,
        description: String::new(),
        headers,
        query,
        params,
        body,
        mock: MockConfig::default(),
        examples: vec![],
        responses,
        doc_params: vec![],
        deprecated: false,
        protocol: "http".into(),
    };
    write_pretty(&file_path, &api)?;
        stats.add(&api.protocol);
    Ok(1)
}

// ==================== YApi 导入 ====================

/// 导入 YApi 导出文件（.json）：自动识别 Swagger（复用 openapi 导入）与 YApi 原生树格式
#[tauri::command]
pub(crate) fn import_yapi(
    app: AppHandle,
    state: State<'_, WorkspaceState>,
) -> Result<Option<OpenApiImportResult>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app
        .dialog()
        .file()
        .add_filter("YApi", &["json"])
        .blocking_pick_file();
    let Some(path) = picked else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    let root = workspace_root(&state)?;
    let result = import_yapi_file(&root, &path)?;
    Ok(Some(result))
}

/// 解析 YApi 导出文件：Swagger 结构走 openapi 导入；否则按原生树（name/children/api）导入
pub(crate) fn import_yapi_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let content = fs::read_to_string(file).map_err(|e| format!("读取文件失败: {e}"))?;
    let json: Value =
        serde_json::from_str(&content).map_err(|e| format!("解析 YApi 文件失败: {e}"))?;
    // Swagger / OpenAPI 结构：YApi 的 swagger 数据导出
    if json.get("swagger").is_some() || json.get("openapi").is_some() {
        return import_openapi_file(root, file);
    }
    // YApi 原生导出：顶层为分组数组（name/children/api）
    let arr = json.as_array().ok_or_else(|| {
        "不是有效的 YApi 文件（既非 Swagger 也非分组树结构）".to_string()
    })?;
    let dir_name = "YApi 导入".to_string();
    let folder = unique_path(root, &dir_name, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    let src_name = file
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some("YApi 导入".into()),
            description: format!("从 YApi 导出文件导入（{src_name}）"),
            base_url: None,
            mock_port: None,
            order: None,
            collapsed: None,
            deprecated: None,
        },
    )?;
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    for g in arr {
        count += yapi_node_to_apis(&folder, g, &mut stats)?;
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

/// YApi 节点递归：有 children 建分组，有 api 写接口文件
fn yapi_node_to_apis(dir: &Path, node: &Value, stats: &mut ImportStats) -> Result<usize, String> {
    let name = node
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("未命名")
        .to_string();
    if let Some(api) = node.get("api") {
        return yapi_api_to_api(dir, &name, api, stats);
    }
    let kids = node.get("children").and_then(|v| v.as_array());
    let Some(kids) = kids else {
        // 既无 api 也无 children 的占位节点，跳过
        return Ok(0);
    };
    if kids.is_empty() {
        return Ok(0); // 空分组不创建
    }
    let sub_base = sanitize_filename(&name);
    let sub_base = if sub_base.is_empty() {
        "子分组".to_string()
    } else {
        sub_base
    };
    let sub_dir = dir.join(&sub_base);
    // 同名分组已存在时复用目录
    if !sub_dir.is_dir() {
        fs::create_dir_all(&sub_dir).map_err(|e| format!("创建分组失败: {e}"))?;
        write_pretty(
            &sub_dir.join(INFO_FILE),
            &InfoJson {
                name: Some(name.clone()),
                description: String::new(),
                base_url: None,
                mock_port: None,
                order: None,
                collapsed: None,
                deprecated: None,
            },
        )?;
    }
    let mut count = 0usize;
    for c in kids {
        count += yapi_node_to_apis(&sub_dir, c, stats)?;
    }
    Ok(count)
}

/// YApi api 对象 → ApiFile
fn yapi_api_to_api(dir: &Path, title: &str, api: &Value,
    stats: &mut ImportStats) -> Result<usize, String> {
    let method = api
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET")
        .to_uppercase();
    let raw_path = api.get("path").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    if raw_path.is_empty() {
        return Ok(0);
    }
    let protocol = match api.get("protocol").and_then(|v| v.as_str()) {
        Some(p) if p.eq_ignore_ascii_case("ws") || p.eq_ignore_ascii_case("websocket") => {
            "websocket".to_string()
        }
        _ => "http".to_string(),
    };
    let (path, params) = if protocol == "websocket" {
        // WS 地址保留完整 ws:// 前缀
        (raw_path.clone(), Vec::new())
    } else {
        extract_path(&raw_path)
    };
    // 请求头
    let mut headers = Vec::new();
    if let Some(hs) = api.get("req_headers").and_then(|v| v.as_array()) {
        for h in hs {
            let key = h.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if key.is_empty() {
                continue;
            }
            let value = h.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let desc = h.get("desc").and_then(|v| v.as_str()).unwrap_or("").to_string();
            headers.push(KeyValue {
                key,
                value,
                enabled: true,
                is_file: false,
                description: desc,
            });
        }
    }
    // 查询参数
    let mut query = Vec::new();
    if let Some(qs) = api.get("req_query").and_then(|v| v.as_array()) {
        for q in qs {
            let key = q.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if key.is_empty() {
                continue;
            }
            let value = q
                .get("example")
                .and_then(|v| v.as_str())
                .or_else(|| q.get("value").and_then(|v| v.as_str()))
                .unwrap_or("")
                .to_string();
            let desc = q
                .get("desc")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            query.push(KeyValue {
                key,
                value,
                enabled: true,
                is_file: false,
                description: desc,
            });
        }
    }
    // 请求体
    let mut body = BodyData::default();
    let body_type = api.get("req_body_type").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
    match body_type.as_str() {
        "json" => {
            if let Some(other) = api.get("req_body_other").and_then(|v| v.as_str()) {
                let trimmed = other.trim();
                if !trimmed.is_empty() {
                    // 尝试格式化 JSON
                    if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
                        body.raw = serde_json::to_string_pretty(&v).unwrap_or_else(|_| trimmed.to_string());
                    } else {
                        body.raw = trimmed.to_string();
                    }
                    body.mode = "json".into();
                }
            }
        }
        "form" => {
            let mut form = Vec::new();
            if let Some(fs) = api.get("req_body_form").and_then(|v| v.as_array()) {
                for f in fs {
                    let key = f.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                    if key.is_empty() {
                        continue;
                    }
                    let value = f.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let desc = f.get("desc").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let is_file = f.get("type").and_then(|v| v.as_str()).unwrap_or("") == "file";
                    form.push(KeyValue {
                        key,
                        value,
                        enabled: true,
                        is_file,
                        description: desc,
                    });
                }
            }
            if !form.is_empty() {
                body.mode = "form".into();
                body.form = form;
            }
        }
        "raw" | "text" | "file" => {
            if let Some(other) = api.get("req_body_other").and_then(|v| v.as_str()) {
                let trimmed = other.trim();
                if !trimmed.is_empty() {
                    body.mode = "raw".into();
                    body.raw = trimmed.to_string();
                }
            }
        }
        _ => {}
    }
    // 响应示例
    let mut responses = Vec::new();
    if let Some(res) = api.get("res_body").and_then(|v| v.as_str()) {
        let trimmed = res.trim();
        if !trimmed.is_empty() {
            let content_type = match api.get("res_body_type").and_then(|v| v.as_str()) {
                Some(t) if t.eq_ignore_ascii_case("json") => "application/json".to_string(),
                _ => "text/plain".to_string(),
            };
            responses.push(ResponseItem {
                id: uuid::Uuid::new_v4().to_string(),
                name: "HTTP 200".into(),
                status: 200,
                content_type,
                body: trimmed.to_string(),
            });
        }
    }
    let desc = api.get("desc").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let name = if title.trim().is_empty() {
        format!("{} {}", method, path)
    } else {
        title.trim().to_string()
    };
    let file_base = sanitize_filename(&name);
    let file_path = unique_path(dir, &file_base, ".json");
    let api_file = ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name: name.clone(),
        method: method.clone(),
        path: path.clone(),
        url: path.clone(),
        description: desc,
        headers,
        query,
        params,
        body,
        mock: MockConfig::default(),
        examples: vec![],
        responses,
        doc_params: vec![],
        deprecated: false,
        protocol,
    };
    write_pretty(&file_path, &api_file)?;
        stats.add(&api_file.protocol);
    Ok(1)
}


// ==================== apiDoc 导入 ====================

/// 导入 apiDoc 导出文件（api_project.json + api_data.json 两个文件）
// ==================== 批量格式导入（10 种） ====================

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
            order: None,
            collapsed: None,
            deprecated: None,
        },
    )?;
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
            order: None,
            collapsed: None,
            deprecated: None,
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
            order: None,
            collapsed: None,
            deprecated: None,
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
            order: None,
            collapsed: None,
            deprecated: None,
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
            order: None,
            collapsed: None,
            deprecated: None,
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
            order: None,
            collapsed: None,
            deprecated: None,
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
            order: None,
            collapsed: None,
            deprecated: None,
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
            order: None,
            collapsed: None,
            deprecated: None,
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
            order: None,
            collapsed: None,
            deprecated: None,
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
            order: None,
            collapsed: None,
            deprecated: None,
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
            order: None,
            collapsed: None,
            deprecated: None,
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
    let mut order = 0i32;
    let modules = data.get("modules").and_then(|x| x.as_array()).cloned().unwrap_or_default();
    for m in modules {
        let mname = str_field(&m, "name");
        let dir = mk_group_dir(&folder, &if mname.is_empty() { "未分组".to_string() } else { mname.clone() }, &str_field(&m, "description"))?;
        let interfaces = m.get("interfaces").and_then(|x| x.as_array()).cloned().unwrap_or_default();
        for it in interfaces {
            order += 1;
            let api = rap2_interface_to_api(&it);
            stats.add(&api.protocol);
            write_pretty(
                &dir.join(format!("{}.json", sanitize_filename(&api.name))),
                &api,
            )?;
            count += 1;
        }
    }
    // 分组顺序
    let info_path = folder.join(INFO_FILE);
    if let Ok(mut info) = serde_json::from_str::<InfoJson>(&fs::read_to_string(&info_path).unwrap_or_default()) {
        info.order = Some(order);
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

#[tauri::command]
pub(crate) fn import_extra(
    app: AppHandle,
    state: State<'_, WorkspaceState>,
    format: String,
) -> Result<Option<OpenApiImportResult>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (filter_name, exts): (&str, &[&str]) = match format.as_str() {
        "apidog" => ("apiDog", &["json"]),
        "bruno" => ("Bruno", &["json"]),
        "apizza" => ("Apizza", &["json"]),
        "nei" => ("NEI", &["json"]),
        "doclever" => ("DOClever", &["json"]),
        "io-docs" => ("IO-Docs", &["json"]),
        "easydoc" => ("EasyDoc", &["json"]),
        "docway" => ("DocWay", &["json", "mjson"]),
        "hoppscotch" => ("Hoppscotch", &["json"]),
        "metersphere" => ("MeterSphere", &["json"]),
        "rap2" => ("RAP2", &["json"]),
        _ => return Err(format!("不支持的格式: {format}")),
    };
    let picked = app
        .dialog()
        .file()
        .add_filter(filter_name, exts)
        .blocking_pick_file();
    let Some(path) = picked else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    let root = workspace_root(&state)?;
    let result = import_extra_files(&root, &path, &format)?;
    Ok(Some(result))
}

#[tauri::command]
pub(crate) fn import_apidoc(
    app: AppHandle,
    state: State<'_, WorkspaceState>,
) -> Result<Option<OpenApiImportResult>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app
        .dialog()
        .file()
        .add_filter("apiDoc", &["json"])
        .blocking_pick_file();
    let Some(path) = picked else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    let root = workspace_root(&state)?;
    let dir = path.parent().unwrap_or(Path::new("."));
    let project_path = dir.join("api_project.json");
    let data_path = dir.join("api_data.json");
    if !project_path.exists() || !data_path.exists() {
        return Err("apiDoc 导出需要同目录下的 api_project.json 与 api_data.json 两个文件".into());
    }
    let result = import_apidoc_files(&root, &project_path, &data_path)?;
    Ok(Some(result))
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
            order: None,
            collapsed: None,
            deprecated: None,
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
                        order: None,
                        collapsed: None,
                        deprecated: None,
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

/// 导入 JMeter 测试计划（.jmx）
#[tauri::command]
pub(crate) fn import_jmeter(
    app: AppHandle,
    state: State<'_, WorkspaceState>,
) -> Result<Option<OpenApiImportResult>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app
        .dialog()
        .file()
        .add_filter("JMeter", &["jmx"])
        .blocking_pick_file();
    let Some(path) = picked else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    let root = workspace_root(&state)?;
    let result = import_jmeter_file(&root, &path)?;
    Ok(Some(result))
}

pub(crate) fn import_jmeter_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let content = fs::read_to_string(file).map_err(|e| format!("读取文件失败: {e}"))?;
    // JMX 中 URL 参数常带未转义 &，解析前清洗
    let content = sanitize_jmx_entities(&content);
    let doc = roxmltree::Document::parse(&content).map_err(|e| format!("解析 JMX 失败: {e}"))?;
    let root_el = doc.root_element();
    if root_el.tag_name().name() != "jmeterTestPlan" {
        return Err("不是有效的 JMeter 测试计划（缺少 jmeterTestPlan 根节点）".into());
    }
    // TestPlan 用户定义变量
    let mut vars: HashMap<String, String> = HashMap::new();
    for tp in root_el
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "TestPlan")
    {
        for arg in tp
            .descendants()
            .filter(|n| n.is_element() && n.attribute("elementType") == Some("Argument"))
        {
            let name = jmeter_child_string(arg, "Argument.name");
            let value = jmeter_child_string(arg, "Argument.value");
            if !name.is_empty() {
                vars.insert(name, value);
            }
        }
    }
    // TestPlan testname 作顶层分组名
    let plan_name = root_el
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "TestPlan")
        .and_then(|n| n.attribute("testname"))
        .unwrap_or("JMeter 导入")
        .to_string();
    let folder = unique_path(root, &plan_name, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    let src_name = file
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    // 用户变量中的 host 作为 base_url
    let host_var = vars.get("host").cloned().unwrap_or_default();
    let base_url = if host_var.is_empty() {
        None
    } else {
        Some(host_var.clone())
    };
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some(plan_name.clone()),
            description: format!("从 JMeter 测试计划导入（{src_name}）"),
            base_url,
            mock_port: None,
            order: None,
            collapsed: None,
            deprecated: None,
        },
    )?;
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    // 递归处理所有 hashTree（TestPlan 级 sampler 罕见；ThreadGroup 为分组）
    let mut pending_headers: Vec<KeyValue> = Vec::new();
    let mut pending_group: Option<String> = None;
    count += jmeter_walk_hash_tree(
        &root_el,
        &folder,
        &vars,
        &mut pending_headers,
        &mut pending_group,
        &mut stats,
    )?;
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

/// 把裸 & 转义为 &amp;（保留合法实体）
fn sanitize_jmx_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while !rest.is_empty() {
        let ch = rest.chars().next().unwrap();
        if ch == '&' {
            let after = &rest[ch.len_utf8()..];
            let valid = after.starts_with("amp;")
                || after.starts_with("lt;")
                || after.starts_with("gt;")
                || after.starts_with("quot;")
                || after.starts_with("apos;")
                || after.starts_with('#');
            if valid {
                out.push('&');
            } else {
                out.push_str("&amp;");
            }
        } else {
            out.push(ch);
        }
        rest = &rest[ch.len_utf8()..];
    }
    out
}

/// 递归遍历元素树：HeaderManager 更新作用域 headers，HTTPSamplerProxy 写接口，ThreadGroup 建分组，hashTree 递归
fn jmeter_walk_hash_tree(
    el: &roxmltree::Node,
    dir: &Path,
    vars: &HashMap<String, String>,
    pending_headers: &mut Vec<KeyValue>,
    pending_group: &mut Option<String>,

    stats: &mut ImportStats) -> Result<usize, String> {
    let mut count = 0usize;
    for child in el.children().filter(|n| n.is_element()) {
        match child.tag_name().name() {
            "HeaderManager" => {
                let mut hs: Vec<KeyValue> = Vec::new();
                for h in child
                    .descendants()
                    .filter(|n| n.is_element() && n.attribute("elementType") == Some("Header"))
                {
                    let key = jmeter_child_string(h, "Header.name");
                    let value = jmeter_child_string(h, "Header.value");
                    if !key.is_empty() {
                        hs.push(KeyValue {
                            key,
                            value,
                            enabled: true,
                            is_file: false,
                            description: String::new(),
                        });
                    }
                }
                if !hs.is_empty() {
                    *pending_headers = hs;
                }
            }
            "ThreadGroup" => {
                if let Some(n) = child.attribute("testname") {
                    *pending_group = Some(n.to_string());
                }
            }
            "HTTPSamplerProxy" => {
                count += jmeter_sampler_to_api(child, dir, vars, pending_headers, stats)?;
            }
            "hashTree" => {
                if let Some(gname) = pending_group.take() {
                    let sub_base = sanitize_filename(&gname);
                    let sub_base = if sub_base.is_empty() {
                        "线程组".to_string()
                    } else {
                        sub_base
                    };
                    let sub_dir = dir.join(&sub_base);
                    if !sub_dir.is_dir() {
                        fs::create_dir_all(&sub_dir).map_err(|e| format!("创建分组失败: {e}"))?;
                        write_pretty(
                            &sub_dir.join(INFO_FILE),
                            &InfoJson {
                                name: Some(gname),
                                description: String::new(),
                                base_url: None,
                                mock_port: None,
                                order: None,
                                collapsed: None,
                                deprecated: None,
                            },
                        )?;
                    }
                    count += jmeter_walk_hash_tree(
                        &child,
                        &sub_dir,
                        vars,
                        pending_headers,
                        pending_group,
                        stats,
                    )?;
                } else {
                    count += jmeter_walk_hash_tree(
                        &child,
                        dir,
                        vars,
                        pending_headers,
                        pending_group,
                        stats,
                    )?;
                }
            }
            _ => {}
        }
    }
    Ok(count)
}

/// 读取元素下指定名称的 stringProp 子节点文本
fn jmeter_child_string(el: roxmltree::Node, prop: &str) -> String {
    el.children()
        .find(|n| n.is_element() && n.tag_name().name() == "stringProp" && n.attribute("name") == Some(prop))
        .map(|n| n.text().unwrap_or("").to_string())
        .unwrap_or_default()
}

/// JMeter sampler → ApiFile
fn jmeter_sampler_to_api(
    el: roxmltree::Node,
    dir: &Path,
    vars: &HashMap<String, String>,
    headers: &[KeyValue],

    stats: &mut ImportStats) -> Result<usize, String> {
    let name = el.attribute("testname").unwrap_or("未命名接口").to_string();
    let mut method = jmeter_child_string(el, "HTTPSampler.method");
    if method.is_empty() {
        method = "GET".to_string();
    }
    let method = method.to_uppercase();
    let mut domain = jmeter_child_string(el, "HTTPSampler.domain");
    let mut path = jmeter_child_string(el, "HTTPSampler.path");
    if path.is_empty() {
        return Ok(0);
    }
    // ${var} 替换
    for (k, v) in vars {
        domain = domain.replace(&format!("${{{k}}}"), v);
        path = path.replace(&format!("${{{k}}}"), v);
    }
    let protocol = jmeter_child_string(el, "HTTPSampler.protocol");
    let is_ws = domain.starts_with("ws://") || domain.starts_with("wss://");
    let api_protocol = if is_ws {
        "websocket".to_string()
    } else {
        "http".to_string()
    };
    let (clean_path, params) = if is_ws {
        (path.clone(), Vec::new())
    } else {
        extract_path(&path)
    };
    let mut query = Vec::new();
    // path 中的 ?a=b（extract_path 已剥离 query，从原始 path 提取）
    if let Some(qi) = path.find('?') {
        let qs = &path[qi + 1..];
        for pair in qs.split('&') {
            let mut it = pair.splitn(2, '=');
            let key = it.next().unwrap_or("").trim().to_string();
            if key.is_empty() {
                continue;
            }
            let value = it.next().unwrap_or("").to_string();
            query.push(KeyValue {
                key,
                value,
                enabled: true,
                is_file: false,
                description: String::new(),
            });
        }
    }
    // HTTPsampler.Arguments：body（POST）或 query 参数（GET）
    let mut body = BodyData::default();
    if let Some(args) = el
        .children()
        .find(|n| n.is_element() && n.attribute("name") == Some("HTTPsampler.Arguments"))
    {
        let mut http_args: Vec<(String, String, String)> = Vec::new();
        for a in args
            .descendants()
            .filter(|n| n.is_element() && n.attribute("elementType") == Some("HTTPArgument"))
        {
            let aname = a.attribute("name").unwrap_or("").to_string();
            let aval = jmeter_child_string(a, "Argument.value");
            let meta = jmeter_child_string(a, "Argument.metadata");
            http_args.push((aname, aval, meta));
        }
        let is_write = matches!(method.as_str(), "POST" | "PUT" | "PATCH" | "DELETE");
        if is_write && !http_args.is_empty() {
            // 单个无名参数且值为 JSON → json body
            let single = http_args.len() == 1 && http_args[0].0.is_empty();
            let first = http_args[0].1.trim();
            if single && (first.starts_with('{') || first.starts_with('[')) {
                if !first.is_empty() {
                    body.mode = "json".into();
                    body.raw = first.to_string();
                }
            } else if http_args.iter().all(|(n, _, _)| !n.is_empty()) {
                let mut form = Vec::new();
                for (n, v, _) in http_args {
                    if n.is_empty() {
                        continue;
                    }
                    form.push(KeyValue {
                        key: n,
                        value: v,
                        enabled: true,
                        is_file: false,
                        description: String::new(),
                    });
                }
                if !form.is_empty() {
                    body.mode = "form".into();
                    body.form = form;
                }
            }
        } else if !is_write {
            for (n, v, _) in http_args {
                if n.is_empty() {
                    continue;
                }
                query.push(KeyValue {
                    key: n,
                    value: v,
                    enabled: true,
                    is_file: false,
                    description: String::new(),
                });
            }
        }
    }
    let mut url = String::new();
    if is_ws {
        url = clean_path.clone();
    } else if !domain.is_empty() {
        if domain.contains("://") {
            url = format!("{domain}{clean_path}");
        } else if !protocol.is_empty() && !domain.is_empty() {
            url = format!("{protocol}://{domain}{clean_path}");
        } else {
            url = format!("https://{domain}{clean_path}");
        }
    }
    let api_name = if name.trim().is_empty() {
        format!("{} {}", method, clean_path)
    } else {
        name.trim().to_string()
    };
    let file_base = sanitize_filename(&api_name);
    let file_path = unique_path(dir, &file_base, ".json");
    let api_file = ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name: api_name.clone(),
        method: method.clone(),
        path: clean_path.clone(),
        url,
        description: jmeter_child_string(el, "HTTPSampler.comments"),
        headers: headers.to_vec(),
        query,
        params,
        body,
        mock: MockConfig::default(),
        examples: vec![],
        responses: vec![],
        doc_params: vec![],
        deprecated: false,
        protocol: api_protocol,
    };
    write_pretty(&file_path, &api_file)?;
        stats.add(&api_file.protocol);
    Ok(1)
}

// ==================== Eolink 导入 ====================

/// 导入 Eolink 导出文件（.json）
#[tauri::command]
pub(crate) fn import_eolink(
    app: AppHandle,
    state: State<'_, WorkspaceState>,
) -> Result<Option<OpenApiImportResult>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app
        .dialog()
        .file()
        .add_filter("Eolink", &["json"])
        .blocking_pick_file();
    let Some(path) = picked else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    let root = workspace_root(&state)?;
    let result = import_eolink_file(&root, &path)?;
    Ok(Some(result))
}

pub(crate) fn import_eolink_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let content = fs::read_to_string(file).map_err(|e| format!("读取文件失败: {e}"))?;
    let json: Value =
        serde_json::from_str(&content).map_err(|e| format!("解析 Eolink 文件失败: {e}"))?;
    let project = json.get("projectInfo").cloned().unwrap_or(Value::Null);
    let project_name = project
        .get("projectName")
        .and_then(|v| v.as_str())
        .unwrap_or("Eolink 导入")
        .to_string();
    let folder = unique_path(root, &project_name, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    let src_name = file
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    // 环境 host 作为 base_url
    let env_host = json
        .get("environmentList")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|e| e.get("envHost"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let base_url = if env_host.is_empty() {
        None
    } else {
        Some(env_host)
    };
    let desc = project
        .get("projectDesc")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some(project_name.clone()),
            description: if desc.is_empty() {
                format!("从 Eolink 导出文件导入（{src_name}）")
            } else {
                desc
            },
            base_url,
            mock_port: None,
            order: None,
            collapsed: None,
            deprecated: None,
        },
    )?;
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    if let Some(groups) = json.get("apiGroupList").and_then(|v| v.as_array()) {
        for g in groups {
            count += eolink_group_to_apis(&folder, g, &mut stats)?;
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

/// Eolink 分组递归：本组建目录，apiList 写入本组，childGroupList 递归子目录
fn eolink_group_to_apis(dir: &Path, group: &Value,
    stats: &mut ImportStats) -> Result<usize, String> {
    let name = group
        .get("groupName")
        .and_then(|v| v.as_str())
        .unwrap_or("未命名分组")
        .to_string();
    let sub_base = sanitize_filename(&name);
    let sub_base = if sub_base.is_empty() {
        "子分组".to_string()
    } else {
        sub_base
    };
    let sub_dir = dir.join(&sub_base);
    if !sub_dir.is_dir() {
        fs::create_dir_all(&sub_dir).map_err(|e| format!("创建分组失败: {e}"))?;
        write_pretty(
            &sub_dir.join(INFO_FILE),
            &InfoJson {
                name: Some(name.clone()),
                description: String::new(),
                base_url: None,
                mock_port: None,
                order: None,
                collapsed: None,
                deprecated: None,
            },
        )?;
    }
    let mut count = 0usize;
    if let Some(apis) = group.get("apiList").and_then(|v| v.as_array()) {
        for a in apis {
            count += eolink_api_to_api(&sub_dir, a, stats)?;
        }
    }
    if let Some(children) = group.get("childGroupList").and_then(|v| v.as_array()) {
        for c in children {
            count += eolink_group_to_apis(&sub_dir, c, stats)?;
        }
    }
    Ok(count)
}

/// Eolink API 对象 → ApiFile
fn eolink_api_to_api(dir: &Path, api: &Value,
    stats: &mut ImportStats) -> Result<usize, String> {
    let method = api
        .get("apiMethod")
        .and_then(|v| v.as_str())
        .unwrap_or("GET")
        .to_uppercase();
    let raw_uri = api
        .get("apiUri")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if raw_uri.is_empty() {
        return Ok(0);
    }
    let protocol = match api.get("apiProtocol").and_then(|v| v.as_str()) {
        Some(p) if p.eq_ignore_ascii_case("ws") || p.eq_ignore_ascii_case("websocket") => {
            "websocket".to_string()
        }
        _ => "http".to_string(),
    };
    let (path, mut params) = if protocol == "websocket" {
        (raw_uri.clone(), Vec::new())
    } else {
        extract_path(&raw_uri)
    };
    let info = api.get("requestInfo").cloned().unwrap_or(Value::Null);
    // 请求头
    let mut headers = Vec::new();
    if let Some(hs) = info.get("requestHeaderList").and_then(|v| v.as_array()) {
        for h in hs {
            let key = h.get("key").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if key.is_empty() {
                continue;
            }
            let value = h
                .get("example")
                .and_then(|v| v.as_str())
                .or_else(|| h.get("value").and_then(|v| v.as_str()))
                .unwrap_or("")
                .to_string();
            let desc = h.get("desc").and_then(|v| v.as_str()).unwrap_or("").to_string();
            headers.push(KeyValue {
                key,
                value,
                enabled: true,
                is_file: false,
                description: desc,
            });
        }
    }
    // 查询参数
    let mut query = Vec::new();
    if let Some(qs) = info.get("requestQueryList").and_then(|v| v.as_array()) {
        for q in qs {
            let key = q.get("key").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if key.is_empty() {
                continue;
            }
            let value = q
                .get("example")
                .and_then(|v| v.as_str())
                .or_else(|| q.get("value").and_then(|v| v.as_str()))
                .unwrap_or("")
                .to_string();
            let desc = q.get("desc").and_then(|v| v.as_str()).unwrap_or("").to_string();
            query.push(KeyValue {
                key,
                value,
                enabled: true,
                is_file: false,
                description: desc,
            });
        }
    }
    // 路径参数（requestRestList）补充 value/desc
    if let Some(rs) = info.get("requestRestList").and_then(|v| v.as_array()) {
        for r in rs {
            let key = r.get("key").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if key.is_empty() {
                continue;
            }
            let value = r
                .get("example")
                .and_then(|v| v.as_str())
                .or_else(|| r.get("value").and_then(|v| v.as_str()))
                .unwrap_or("")
                .to_string();
            let desc = r.get("desc").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if let Some(p) = params.iter_mut().find(|p| p.key == key) {
                p.value = value;
                p.description = desc;
            } else {
                params.push(KeyValue {
                    key,
                    value,
                    enabled: true,
                    is_file: false,
                    description: desc,
                });
            }
        }
    }
    // 请求体
    let mut body = BodyData::default();
    let body_type = info
        .get("requestBodyType")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    match body_type.as_str() {
        "json" => {
            if let Some(list) = info.get("requestBodyJsonList").and_then(|v| v.as_array()) {
                let v = eolink_json_list_to_value(list);
                if v.as_object().map(|m| !m.is_empty()).unwrap_or(false) {
                    body.mode = "json".into();
                    body.raw = serde_json::to_string_pretty(&v).unwrap_or_default();
                }
            }
        }
        "x-www-form-urlencoded" | "form" => {
            if let Some(fl) = info.get("requestBodyFormList").and_then(|v| v.as_array()) {
                let mut form = Vec::new();
                for f in fl {
                    let key = f.get("key").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                    if key.is_empty() {
                        continue;
                    }
                    let value = f
                        .get("example")
                        .and_then(|v| v.as_str())
                        .or_else(|| f.get("value").and_then(|v| v.as_str()))
                        .unwrap_or("")
                        .to_string();
                    let desc = f.get("desc").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let is_file = f
                        .get("type")
                        .and_then(|v| v.as_str())
                        .map(|t| t.eq_ignore_ascii_case("file"))
                        .unwrap_or(false);
                    form.push(KeyValue {
                        key,
                        value,
                        enabled: true,
                        is_file,
                        description: desc,
                    });
                }
                if !form.is_empty() {
                    body.mode = "form".into();
                    body.form = form;
                }
            }
        }
        "raw" | "text" => {
            if let Some(text) = info.get("requestBodyRaw").and_then(|v| v.as_str()) {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    body.mode = "raw".into();
                    body.raw = trimmed.to_string();
                }
            }
        }
        _ => {}
    }
    // 响应示例
    let mut responses = Vec::new();
    if let Some(list) = api.get("responseInfoList").and_then(|v| v.as_array()) {
        for r in list {
            let status = r
                .get("responseCode")
                .and_then(|v| v.as_u64())
                .unwrap_or(200)
                .min(u64::from(u16::MAX)) as u16;
            let content_type = match r
                .get("responseContentType")
                .and_then(|v| v.as_str())
                .unwrap_or("")
            {
                s if s.eq_ignore_ascii_case("json") => "application/json".to_string(),
                s if s.eq_ignore_ascii_case("xml") => "application/xml".to_string(),
                s if s.eq_ignore_ascii_case("html") => "text/html".to_string(),
                s if s.eq_ignore_ascii_case("text") => "text/plain".to_string(),
                _ => "application/json".to_string(),
            };
            let mut rbody = String::new();
            if let Some(rl) = r.get("responseBodyJsonList").and_then(|v| v.as_array()) {
                let v = eolink_json_list_to_value(rl);
                if v.as_object().map(|m| !m.is_empty()).unwrap_or(false) {
                    rbody = serde_json::to_string_pretty(&v).unwrap_or_default();
                }
            } else if let Some(text) = r.get("responseBodyRaw").and_then(|v| v.as_str()) {
                rbody = text.trim().to_string();
            }
            let rname = r
                .get("responseName")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if !rbody.is_empty() {
                responses.push(ResponseItem {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: if rname.is_empty() {
                        format!("HTTP {status}")
                    } else {
                        rname
                    },
                    status,
                    content_type,
                    body: rbody,
                });
            }
        }
    }
    let name = api
        .get("apiName")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let name = if name.is_empty() {
        format!("{} {}", method, path)
    } else {
        name
    };
    let mut desc = api
        .get("apiDesc")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if let Some(note) = api.get("apiNote").and_then(|v| v.as_str()) {
        if !note.trim().is_empty() {
            if !desc.is_empty() {
                desc.push('\n');
            }
            desc.push_str(note);
        }
    }
    let file_base = sanitize_filename(&name);
    let file_path = unique_path(dir, &file_base, ".json");
    let api_file = ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name: name.clone(),
        method: method.clone(),
        path: path.clone(),
        url: path.clone(),
        description: desc,
        headers,
        query,
        params,
        body,
        mock: MockConfig::default(),
        examples: vec![],
        responses,
        doc_params: vec![],
        deprecated: false,
        protocol,
    };
    write_pretty(&file_path, &api_file)?;
        stats.add(&api_file.protocol);
    Ok(1)
}

/// Eolink JSON 字段列表（含嵌套 children）→ serde_json Value
fn eolink_json_list_to_value(list: &[Value]) -> Value {
    let mut map = serde_json::Map::new();
    for item in list {
        let key = item.get("key").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        if key.is_empty() {
            continue;
        }
        let kids = item.get("children").and_then(|v| v.as_array());
        let value = if let Some(kids) = kids {
            if kids.is_empty() {
                Value::Null
            } else {
                eolink_json_list_to_value(kids)
            }
        } else {
            match item.get("example") {
                Some(v) => v.clone(),
                None => match item.get("value") {
                    Some(v) => v.clone(),
                    None => Value::Null,
                },
            }
        };
        map.insert(key, value);
    }
    Value::Object(map)
}

// ==================== Insomnia 导入 ====================

/// 导入 Insomnia 导出文件（.yml / .json）
#[tauri::command]
pub(crate) fn import_insomnia(
    app: AppHandle,
    state: State<'_, WorkspaceState>,
) -> Result<Option<OpenApiImportResult>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app
        .dialog()
        .file()
        .add_filter("Insomnia", &["yml", "yaml", "json"])
        .blocking_pick_file();
    let Some(path) = picked else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    let root = workspace_root(&state)?;
    let result = import_insomnia_file(&root, &path)?;
    Ok(Some(result))
}

pub(crate) fn import_insomnia_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let content = fs::read_to_string(file).map_err(|e| format!("读取文件失败: {e}"))?;
    let doc: Value = serde_yaml::from_str(&content).map_err(|e| format!("解析 Insomnia 文件失败: {e}"))?;
    let coll_name = doc
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Insomnia 导入")
        .to_string();
    let folder = unique_path(root, &coll_name, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    // 集合级环境变量 baseUrl
    let base_url = doc
        .get("environment")
        .and_then(|e| e.get("baseUrl"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let src_name = file
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some(coll_name.clone()),
            description: format!("从 Insomnia 导出文件导入（{src_name}）"),
            base_url,
            mock_port: None,
            order: None,
            collapsed: None,
            deprecated: None,
        },
    )?;
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    let coll_env = doc
        .get("environment")
        .cloned()
        .unwrap_or(Value::Null);
    if let Some(children) = doc.get("children").and_then(|v| v.as_array()) {
        for c in children {
            count += insomnia_node_to_apis(&folder, c, &coll_env, &mut stats)?;
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

/// Insomnia 节点递归：有 url/method 的是请求，否则是文件夹
fn insomnia_node_to_apis(dir: &Path, node: &Value, coll_env: &Value,
    stats: &mut ImportStats) -> Result<usize, String> {
    let name = node
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("未命名")
        .to_string();
    let url = node
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let method = node
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if !url.is_empty() && !method.is_empty() {
        return insomnia_request_to_api(dir, &name, node, &url, &method, coll_env, stats);
    }
    let kids = node.get("children").and_then(|v| v.as_array());
    let Some(kids) = kids else {
        return Ok(0);
    };
    if kids.is_empty() {
        return Ok(0); // 空文件夹不创建
    }
    let sub_base = sanitize_filename(&name);
    let sub_base = if sub_base.is_empty() {
        "子文件夹".to_string()
    } else {
        sub_base
    };
    let sub_dir = dir.join(&sub_base);
    if !sub_dir.is_dir() {
        fs::create_dir_all(&sub_dir).map_err(|e| format!("创建分组失败: {e}"))?;
        write_pretty(
            &sub_dir.join(INFO_FILE),
            &InfoJson {
                name: Some(name.clone()),
                description: String::new(),
                base_url: None,
                mock_port: None,
                order: None,
                collapsed: None,
                deprecated: None,
            },
        )?;
    }
    let mut count = 0usize;
    for c in kids {
        count += insomnia_node_to_apis(&sub_dir, c, coll_env, stats)?;
    }
    Ok(count)
}

/// Insomnia 请求对象 → ApiFile
fn insomnia_request_to_api(
    dir: &Path,
    name: &str,
    node: &Value,
    url: &str,
    method: &str,
    coll_env: &Value,

    stats: &mut ImportStats) -> Result<usize, String> {
    let protocol = if url.starts_with("ws://") || url.starts_with("wss://") {
        "websocket".to_string()
    } else {
        "http".to_string()
    };
    // {{baseUrl}} 等模板变量替换（节点级环境优先，集合级兜底）
    let mut env_map: Vec<(String, String)> = Vec::new();
    let mut collect_env = |env: &Value| {
        if let Some(m) = env.as_object() {
            for (k, v) in m {
                if let Some(s) = v.as_str() {
                    env_map.push((k.clone(), s.to_string()));
                }
            }
        }
    };
    if let Some(env) = node.get("environment") {
        collect_env(env);
    }
    collect_env(coll_env);
    let replace_vars = |s: &str| -> String {
        let mut out = s.to_string();
        for (k, v) in &env_map {
            out = out.replace(&format!("{{{{{k}}}}}"), v);
        }
        out
    };
    let url2 = replace_vars(url);
    let (path, params) = if protocol == "websocket" {
        (url2.clone(), Vec::new())
    } else {
        extract_path(&url2)
    };
    // 请求头
    let mut headers = Vec::new();
    if let Some(hs) = node.get("headers").and_then(|v| v.as_array()) {
        for h in hs {
            let key = h.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if key.is_empty() {
                continue;
            }
            let value = h.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let desc = h.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
            headers.push(KeyValue {
                key,
                value,
                enabled: true,
                is_file: false,
                description: desc,
            });
        }
    }
    // 查询参数
    let mut query = Vec::new();
    if let Some(ps) = node.get("parameters").and_then(|v| v.as_array()) {
        for p in ps {
            let key = p.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if key.is_empty() {
                continue;
            }
            let value = p.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let desc = p.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
            query.push(KeyValue {
                key,
                value,
                enabled: true,
                is_file: false,
                description: desc,
            });
        }
    }
    // 请求体
    let mut body = BodyData::default();
    if let Some(b) = node.get("body") {
        let mime = b.get("mimeType").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
        let text = b.get("text").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        if mime.contains("json") && !text.is_empty() {
            body.mode = "json".into();
            body.raw = text;
        } else if (mime.contains("form") || mime.contains("urlencoded")) && !text.is_empty() {
            // x-www-form-urlencoded → 表单字段
            let mut form = Vec::new();
            for pair in text.split('&') {
                let mut it = pair.splitn(2, '=');
                let key = it.next().unwrap_or("").trim().to_string();
                if key.is_empty() {
                    continue;
                }
                let value = it.next().unwrap_or("").to_string();
                form.push(KeyValue {
                    key,
                    value,
                    enabled: true,
                    is_file: false,
                    description: String::new(),
                });
            }
            if !form.is_empty() {
                body.mode = "form".into();
                body.form = form;
            }
        } else if !text.is_empty() {
            body.mode = "raw".into();
            body.raw = text;
        }
    }
    // 鉴权：bearer → Authorization 头
    if let Some(auth) = node.get("authentication") {
        let atype = auth.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if atype == "bearer" {
            if let Some(token) = auth.get("token").and_then(|v| v.as_str()) {
                let token = replace_vars(token);
                if !headers.iter().any(|h| h.key.eq_ignore_ascii_case("authorization")) {
                    headers.push(KeyValue {
                        key: "Authorization".into(),
                        value: format!("Bearer {token}"),
                        enabled: true,
                        is_file: false,
                        description: String::new(),
                    });
                }
            }
        }
    }
    let api_name = if name.trim().is_empty() {
        format!("{} {}", method.to_uppercase(), path)
    } else {
        name.trim().to_string()
    };
    let file_base = sanitize_filename(&api_name);
    let file_path = unique_path(dir, &file_base, ".json");
    let api_file = ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name: api_name.clone(),
        method: method.to_uppercase(),
        path: path.clone(),
        url: path.clone(),
        description: node
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        headers,
        query,
        params,
        body,
        mock: MockConfig::default(),
        examples: vec![],
        responses: vec![],
        doc_params: vec![],
        deprecated: false,
        protocol,
    };
    write_pretty(&file_path, &api_file)?;
        stats.add(&api_file.protocol);
    Ok(1)
}

#[tauri::command]
pub(crate) fn import_openapi(
    app: AppHandle,
    state: State<'_, WorkspaceState>,
) -> Result<Option<OpenApiImportResult>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app
        .dialog()
        .file()
        .add_filter("OpenAPI / Swagger", &["json", "yml", "yaml"])
        .blocking_pick_file();
    let Some(path) = picked else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    let root = workspace_root(&state)?;
    let result = import_openapi_file(&root, &path)?;
    Ok(Some(result))
}

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
            order: None,
            collapsed: None,
            deprecated: None,
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
                                order: None,
                                collapsed: None,
                                deprecated: None,
                            },
                        );
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
