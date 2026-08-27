//! 全部导入格式：OpenAPI / Postman / Apifox / Apipost / RAML / WADL / HAR / YApi /
//! apiDoc / 批量 10 格式 / JMeter / Eolink / Insomnia / Markdown。

use crate::{
    read_env_file, sanitize_filename, unique_path, workspace_root, write_pretty, Environment, InfoJson,
    KeyValue, WorkspaceState, INFO_FILE,
};
use crate::markdown;
use crate::markdown::MarkdownImportResult;
use serde::Serialize;
use std::fs;
use std::path::Path;
use tauri::{AppHandle, State};

mod apifox;
mod batch;
mod eolink;
mod har;
mod insomnia;
mod jmeter;
mod openapi;
mod postman;
mod raml_wadl;
mod yapi;
pub(crate) use self::apifox::{import_apifox_file, import_apipost_file};
pub(crate) use self::batch::{import_apidoc_files, import_extra_files};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use self::batch::import_rap2_files;
pub(crate) use self::eolink::import_eolink_file;
pub(crate) use self::har::import_har_file;
pub(crate) use self::insomnia::import_insomnia_file;
pub(crate) use self::jmeter::import_jmeter_file;
pub(crate) use self::openapi::import_openapi_file;
pub(crate) use self::postman::import_postman_file;
pub(crate) use self::raml_wadl::{import_raml_file, import_wadl_file};
pub(crate) use self::yapi::import_yapi_file;

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
            collapsed: None,
            deprecated: None,
            dirs: vec![],
            apis: vec![],
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

/// 从 URL 提取路径（:id → {id}）与路径参数
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
