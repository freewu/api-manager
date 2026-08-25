mod mock;
mod markdown;
mod export;
mod objects;
mod history;
mod import;
mod update;
mod request;
mod tray;

// 供 invoke_handler 与 tests（use super::*）引用的跨模块命令/函数
#[allow(unused_imports)]
use crate::export::*;
#[allow(unused_imports)]
use crate::history::*;
#[allow(unused_imports)]
use crate::import::*;
#[allow(unused_imports)]
use crate::markdown::*;
#[allow(unused_imports)]
use crate::mock::*;
#[allow(unused_imports)]
use crate::objects::*;
#[allow(unused_imports)]
use crate::request::*;
#[allow(unused_imports)]
use crate::tray::*;
#[allow(unused_imports)]
use crate::update::*;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

pub const INFO_FILE: &str = "__info.json";
pub const ENV_FILE: &str = "__envs.json";

// ==================== 状态 ====================

#[derive(Default)]
pub struct WorkspaceState {
    pub root: Mutex<Option<PathBuf>>,
}

#[derive(Default)]
pub struct MockRunState {
    pub running: Mutex<bool>,
    pub addr: Mutex<Option<String>>,
    pub port: Mutex<Option<u16>>,
    pub route_count: Mutex<usize>,
    pub abort: Mutex<Option<tokio::task::AbortHandle>>,
    pub routes: Mutex<Option<std::sync::Arc<std::sync::RwLock<Vec<crate::mock::MockRoute>>>>>,
    /// 当前生效的全局环境变量（供 reload 时热更新）
    pub envs: Mutex<Option<std::sync::Arc<std::sync::RwLock<HashMap<String, String>>>>>,
}

/// 系统托盘相关状态
pub struct TrayState {
    /// 托盘菜单中“启动/停止 Mock”菜单项，用于动态更新文字
    pub mock_item: Mutex<Option<tauri::menu::IconMenuItem<tauri::Wry>>>,
    /// 托盘菜单中“环境变量”菜单项，显示当前环境名，点击可打开编辑器
    pub env_item: Mutex<Option<tauri::menu::IconMenuItem<tauri::Wry>>>,
    /// 显示窗口菜单项（语言切换时更新文字）
    pub show_item: Mutex<Option<tauri::menu::IconMenuItem<tauri::Wry>>>,
    /// GitHub 仓库菜单项
    pub github_item: Mutex<Option<tauri::menu::IconMenuItem<tauri::Wry>>>,
    /// 提交 Issue 菜单项
    pub issue_item: Mutex<Option<tauri::menu::IconMenuItem<tauri::Wry>>>,
    /// 退出菜单项
    pub quit_item: Mutex<Option<tauri::menu::IconMenuItem<tauri::Wry>>>,
    /// 语言子菜单（单行入口，展开后勾选简体中文 / 繁體中文 / English）
    pub lang_submenu: Mutex<Option<tauri::menu::Submenu<tauri::Wry>>>,
    /// 语言子菜单项（CheckMenuItem，勾选态表示当前语言）
    pub lang_zh_item: Mutex<Option<tauri::menu::CheckMenuItem<tauri::Wry>>>,
    pub lang_tw_item: Mutex<Option<tauri::menu::CheckMenuItem<tauri::Wry>>>,
    pub lang_en_item: Mutex<Option<tauri::menu::CheckMenuItem<tauri::Wry>>>,
    /// 「检查更新」菜单项，发现新版本后文字改为「发现新版本 vX.Y.Z」
    pub update_item: Mutex<Option<tauri::menu::IconMenuItem<tauri::Wry>>>,
    /// 最近一次发现的最新版本号（Some 时点击「检查更新」直接打开发布页）
    pub latest_version: Mutex<Option<String>>,
    /// 是否正在退出（退出时不拦截窗口关闭）
    pub exiting: AtomicBool,
}

// ==================== 数据结构 ====================

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoJson {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mock_port: Option<u16>,
    #[serde(default)]
    pub order: Option<i32>,
    #[serde(default)]
    pub collapsed: Option<bool>,
    /// 标记该分组是否已废弃
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
}

fn default_method() -> String {
    "GET".into()
}
fn default_true() -> bool {
    true
}
fn default_status() -> u16 {
    200
}
fn default_body_mode() -> String {
    "none".into()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub description: String,
    /// 是否文件字段（表单上传用，value 为文件路径）
    #[serde(default)]
    pub is_file: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BodyData {
    #[serde(default = "default_body_mode")]
    pub mode: String,
    #[serde(default)]
    pub raw: String,
    #[serde(default)]
    pub form: Vec<KeyValue>,
    /// 二进制模式：本地文件路径（发送时读取文件字节）
    #[serde(default)]
    pub binary_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MockConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_status")]
    pub status: u16,
    #[serde(default)]
    pub headers: Vec<KeyValue>,
    #[serde(default)]
    pub delay: u64,
    #[serde(default)]
    pub body: String,
}

impl Default for MockConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            status: 200,
            headers: vec![],
            delay: 0,
            body: String::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiFile {
    /// 接口唯一标识（用于版本管理等），旧文件无此字段时自动生成
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub headers: Vec<KeyValue>,
    #[serde(default)]
    pub query: Vec<KeyValue>,
    #[serde(default)]
    pub params: Vec<KeyValue>,
    #[serde(default)]
    pub body: BodyData,
    #[serde(default)]
    pub mock: MockConfig,
    #[serde(default)]
    pub examples: Vec<Value>,
    /// 响应页签条目：返回成功 / 返回失败 / 自定义错误返回（名称、状态码、示例体）
    #[serde(default)]
    pub responses: Vec<ResponseItem>,
    /// 入参文档：请求参数的补充说明（类型 / 说明），按 source+key 关联到请求配置
    #[serde(default)]
    pub doc_params: Vec<DocParam>,
    /// 是否已标记废弃
    #[serde(default)]
    pub deprecated: bool,
    /// 接口协议：http（HTTP 接口）、websocket（WebSocket 接口）或 socketio（Socket.IO 接口）
    #[serde(default = "default_protocol")]
    pub protocol: String,
}

pub(crate) fn default_protocol() -> String {
    "http".to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ResponseItem {
    pub id: String,
    /// 返回名称（返回成功 / 返回失败 / 自定义名称）
    pub name: String,
    /// HTTP 状态码，0 表示未填写
    pub status: u16,
    pub content_type: String,
    /// 响应体示例（JSON / XML / 文本）
    pub body: String,
}

impl Default for ResponseItem {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            status: 0,
            content_type: "application/json".to_string(),
            body: String::new(),
        }
    }
}

/// 新建接口的默认返回条目（返回成功 + 返回失败）
pub(crate) fn default_responses() -> Vec<ResponseItem> {
    vec![
        ResponseItem {
            id: uuid::Uuid::new_v4().to_string(),
            name: "返回成功".into(),
            status: 200,
            content_type: "application/json".into(),
            body: "{\n  \"code\": 0,\n  \"message\": \"success\"\n}".into(),
        },
        ResponseItem {
            id: uuid::Uuid::new_v4().to_string(),
            name: "返回失败".into(),
            status: 400,
            content_type: "application/json".into(),
            body: "{\n  \"code\": 1,\n  \"message\": \"error\"\n}".into(),
        },
    ]
}

/// 旧文件兼容：无 responses 字段时按旧数据补全默认返回
/// 返回成功取 mock.body（合法 JSON 时），返回失败由 resp_fail 文档条目生成示例；
/// 同时把 docParams 的 resp_success / resp_fail 重键到对应条目 id
pub(crate) fn ensure_responses(api: &mut ApiFile) {
    if !api.responses.is_empty() {
        return;
    }
    let success_id = uuid::Uuid::new_v4().to_string();
    let fail_id = uuid::Uuid::new_v4().to_string();
    let success_body = if serde_json::from_str::<Value>(&api.mock.body).is_ok() {
        api.mock.body.clone()
    } else {
        String::new()
    };
    let fail_docs: Vec<DocParam> = api
        .doc_params
        .iter()
        .filter(|d| d.source == "resp_fail")
        .cloned()
        .collect();
    let mut flat: Vec<(String, String, String)> = Vec::new();
    for d in &fail_docs {
        crate::markdown::flatten_doc(d, "", &mut flat);
    }
    let fail_body = if flat.is_empty() {
        String::new()
    } else {
        crate::markdown::sample_json_from_rows(&flat)
    };
    api.responses = vec![
        ResponseItem {
            id: success_id.clone(),
            name: "返回成功".into(),
            status: 200,
            content_type: "application/json".into(),
            body: success_body,
        },
        ResponseItem {
            id: fail_id.clone(),
            name: "返回失败".into(),
            status: 400,
            content_type: "application/json".into(),
            body: fail_body,
        },
    ];
    for d in &mut api.doc_params {
        if d.source == "resp_success" {
            d.source = format!("resp:{success_id}");
        } else if d.source == "resp_fail" {
            d.source = format!("resp:{fail_id}");
        }
    }
}

/// 入参/出参文档条目：位置（header / query / path / body / resp_success / resp_fail）
/// + 参数名 + 类型 + 说明；List 可带元素类型，Object 可带对象名称与下级字段（树状）
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DocParam {
    pub source: String,
    pub key: String,
    pub r#type: String,
    pub description: String,
    /// List 类型的元素类型
    #[serde(default)]
    pub item_type: String,
    /// Object 类型的对象名称
    #[serde(default)]
    pub object_name: String,
    /// 下级字段（树状，递归）
    #[serde(default)]
    pub children: Vec<DocParam>,
}

impl Default for DocParam {
    fn default() -> Self {
        Self {
            source: String::new(),
            key: String::new(),
            r#type: String::new(),
            description: String::new(),
            item_type: String::new(),
            object_name: String::new(),
            children: vec![],
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeNode {
    pub kind: String, // "folder" | "api"
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapsed: Option<bool>,
    /// 该分组下接口总数（含子分组），前端用于显示数量角标
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_count: Option<u32>,
    /// 是否已标记废弃（分组无此字段，接口对应其文件中的 deprecated 字段）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
    /// 接口协议：http / websocket（分组无此字段）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<TreeNode>>,
}

/// 版本文件信息（.version/<uuid>/<名称>.<n>.json）
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub version: u32,
    pub name: String,
    pub path: String,
    /// 文件修改时间（Unix 秒）
    pub modified: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequestData {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<KeyValue>,
    #[serde(default)]
    pub body: Option<String>,
    /// 二进制模式：本地文件路径，存在时按原始字节发送
    #[serde(default)]
    pub body_file: Option<String>,
    /// 表单字段（含文件字段 isFile=true，值为文件路径），存在时按 multipart/form-data 发送
    #[serde(default)]
    pub form: Option<Vec<KeyValue>>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 {
    30000
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResult {
    pub ok: bool,
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub time_ms: u64,
    pub size: usize,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MockStatus {
    pub running: bool,
    pub url: Option<String>,
    pub port: Option<u16>,
    pub route_count: usize,
}

// ==================== 环境变量（全局） ====================

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvVariable {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub default_value: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Environment {
    pub name: String,
    pub variables: Vec<EnvVariable>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct EnvStore {
    pub active: String,
    pub environments: Vec<Environment>,
}

/// 从工作区读取环境配置（不存在则返回空）
fn read_env_file(dir: &Path) -> EnvStore {
    let p = dir.join(ENV_FILE);
    fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// 取当前激活环境的所有已启用变量 -> HashMap
pub fn read_env_map(root: &Path) -> HashMap<String, String> {
    let store = read_env_file(root);
    let active = store.active.clone();
    store
        .environments
        .into_iter()
        .find(|e| e.name == active)
        .map(|e| {
            e.variables
                .into_iter()
                .filter(|v| v.enabled && !v.key.trim().is_empty())
                .map(|v| {
                    let val = if v.value.trim().is_empty() {
                        v.default_value
                    } else {
                        v.value
                    };
                    (v.key.trim().to_string(), val)
                })
                .collect()
        })
        .unwrap_or_default()
}

// ==================== 工具 ====================

fn read_info_file(dir: &Path) -> InfoJson {
    let p = dir.join(INFO_FILE);
    fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_pretty(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let json = serde_json::to_string_pretty(value).map_err(|e| format!("序列化失败: {e}"))?;
    fs::write(path, json + "\n").map_err(|e| format!("写入失败: {e}"))
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if "\\/:*?\"<>|".contains(c) { '_' } else { c })
        .collect::<String>()
        .trim()
        .to_string()
}

/// 生成不冲突的文件路径：xxx.json / xxx (2).json ...
fn unique_path(dir: &Path, base: &str, ext: &str) -> PathBuf {
    let mut candidate = dir.join(format!("{base}{ext}"));
    let mut i = 2;
    while candidate.exists() {
        candidate = dir.join(format!("{base} ({i}){ext}"));
        i += 1;
    }
    candidate
}

fn build_folder_node(dir: &Path) -> Result<TreeNode, String> {
    let info = read_info_file(dir);
    let dir_name = dir
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| dir.to_string_lossy().to_string());

    let mut folders: Vec<(i32, TreeNode)> = Vec::new();
    let mut apis: Vec<TreeNode> = Vec::new();

    for entry in fs::read_dir(dir).map_err(|e| format!("读取目录失败: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            if let Ok(node) = build_folder_node(&path) {
                let child_info = read_info_file(&path);
                folders.push((child_info.order.unwrap_or(1000), node));
            }
        } else if path.extension().map(|e| e == "json").unwrap_or(false)
            && file_name != INFO_FILE
            && file_name != ENV_FILE
        {
            apis.push(build_api_node(&path));
        }
    }

    folders.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.name.cmp(&b.1.name)));
    apis.sort_by(|a, b| a.name.cmp(&b.name));
    let mut children = folders.into_iter().map(|(_, n)| n).collect::<Vec<_>>();
    let api_count = children
        .iter()
        .map(|c| c.api_count.unwrap_or(0))
        .sum::<u32>()
        + apis.len() as u32;
    children.extend(apis);

    Ok(TreeNode {
        kind: "folder".into(),
        name: info.name.clone().unwrap_or(dir_name),
        path: dir.to_string_lossy().to_string(),
        method: None,
        endpoint: None,
        mock_enabled: None,
        description: Some(info.description),
        collapsed: info.collapsed,
        deprecated: info.deprecated,
        protocol: None,
        api_count: Some(api_count),
        children: Some(children),
    })
}

fn build_api_node(path: &Path) -> TreeNode {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut name = stem.clone();
    let mut method = None;
    let mut endpoint = None;
    let mut mock_enabled = None;
    let mut deprecated = None;
    let mut protocol = None;

    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(v) = serde_json::from_str::<Value>(&content) {
            if let Some(n) = v.get("name").and_then(|x| x.as_str()) {
                if !n.trim().is_empty() {
                    name = n.to_string();
                }
            }
            method = v.get("method").and_then(|x| x.as_str()).map(String::from);
            endpoint = v.get("path").and_then(|x| x.as_str()).map(String::from);
            mock_enabled = v
                .get("mock")
                .and_then(|m| m.get("enabled"))
                .and_then(|e| e.as_bool());
            deprecated = v.get("deprecated").and_then(|d| d.as_bool());
            protocol = v.get("protocol").and_then(|x| x.as_str()).map(String::from);
        }
    }

    TreeNode {
        kind: "api".into(),
        name,
        path: path.to_string_lossy().to_string(),
        method,
        endpoint,
        mock_enabled,
        description: None,
        collapsed: None,
        deprecated,
        protocol,
        api_count: None,
        children: None,
    }
}

fn workspace_root(state: &State<'_, WorkspaceState>) -> Result<PathBuf, String> {
    state
        .root
        .lock()
        .map_err(|_| "状态锁错误".to_string())?
        .clone()
        .ok_or_else(|| "尚未选择工作目录".to_string())
}

fn ensure_inside_workspace(root: &Path, target: &Path) -> Result<(), String> {
    if target == root {
        return Err("不能操作工作区根目录".into());
    }
    if target.starts_with(root) {
        Ok(())
    } else {
        Err("路径不在工作区内".into())
    }
}

// ==================== 应用设置 ====================

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    /// "dark" | "light" | "system"（跟随系统）
    pub display_mode: String,
    /// 是否启用接口版本（主页面显示「保存」与「查看版本信息」）
    pub enable_version: bool,
    /// 是否启用 Mock 功能（主页面显示 Mock 开关）
    pub enable_mock: bool,
    /// Mock 服务默认端口
    pub mock_port: u32,
    /// 是否同步远程（工作目录为 Git/SVN 仓库时显示同步与提交按钮）
    pub sync_remote: bool,
    /// 是否启用代码生成（编辑区显示「代码」页签）
    pub enable_codegen: bool,
    /// 代码生成默认开发语言（bash / python / c / cpp / java / csharp / ...）
    pub codegen_lang: String,
    /// 是否启用默认 Header（新增接口时自动附带）
    pub enable_default_headers: bool,
    /// 默认 Header 列表
    pub default_headers: Vec<KeyValue>,
    /// 导出默认格式（postman / openapi / docsify）
    pub export_format: String,
    /// HTML 文档悬浮导航栏位置（off / left / right）
    pub html_nav: String,
    /// 界面语言（zh / zh-tw / en，设置页与托盘菜单同步切换）
    pub language: String,
    /// 最近打开的工作目录数量上限（最少 3）
    pub recent_limit: usize,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            display_mode: "system".into(),
            enable_version: true,
            enable_mock: true,
            mock_port: 5050,
            sync_remote: true,
            enable_codegen: true,
            codegen_lang: "bash".into(),
            enable_default_headers: false,
            default_headers: vec![],
            export_format: "postman".into(),
            html_nav: "right".into(),
            language: "zh".into(),
            recent_limit: 5,
        }
    }
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("获取配置目录失败: {e}"))?;
    Ok(dir.join("settings.json"))
}

#[tauri::command]
fn load_settings(app: AppHandle) -> Result<AppSettings, String> {
    let p = settings_path(&app)?;
    if let Ok(content) = fs::read_to_string(&p) {
        if let Ok(s) = serde_json::from_str::<AppSettings>(&content) {
            return Ok(s);
        }
    }
    Ok(AppSettings::default())
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    let p = settings_path(&app)?;
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {e}"))?;
    }
    write_pretty(&p, &settings)?;
    Ok(())
}

// ==================== 工作区命令 ====================

/// 工作区版本控制信息（检测 .git / .svn）
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VcsInfo {
    /// "git" | "svn" | null
    pub vcs: Option<String>,
}

/// 检测工作区根目录使用的版本控制系统
fn detect_vcs(root: &Path) -> Option<String> {
    if root.join(".git").exists() {
        Some("git".into())
    } else if root.join(".svn").exists() {
        Some("svn".into())
    } else {
        None
    }
}

#[tauri::command]
fn vcs_info(state: State<'_, WorkspaceState>) -> Result<VcsInfo, String> {
    let root = workspace_root(&state)?;
    Ok(VcsInfo {
        vcs: detect_vcs(&root),
    })
}

/// 执行外部命令，合并 stdout/stderr；退出码非 0 时返回错误信息
fn run_cmd(cmd: &str, args: &[&str], cwd: &Path) -> Result<String, String> {
    let out = std::process::Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| {
            format!("执行 {cmd} 失败（请确认已安装 {cmd} 并加入 PATH）: {e}")
        })?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    let combined = match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => String::new(),
        (false, true) => stdout,
        (true, false) => stderr,
        (false, false) => format!("{stdout}\n{stderr}"),
    };
    if !out.status.success() {
        return Err(if combined.is_empty() {
            format!("{cmd} 执行失败（退出码 {:?}）", out.status.code())
        } else {
            combined
        });
    }
    Ok(combined)
}

fn now_stamp() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 同步：git pull / svn update；remote=false 时仅 git fetch（不触碰工作区）
#[tauri::command]
fn vcs_sync(state: State<'_, WorkspaceState>, remote: bool) -> Result<String, String> {
    let root = workspace_root(&state)?;
    match detect_vcs(&root).as_deref() {
        Some("git") => {
            if remote {
                run_cmd("git", &["pull"], &root)
            } else {
                run_cmd("git", &["fetch"], &root)
            }
        }
        Some("svn") => run_cmd("svn", &["update"], &root),
        _ => Err("当前工作目录不是 Git / SVN 仓库".into()),
    }
}

/// 提交并推送远程：git add -A + commit + push / svn add + commit；remote=false 时只提交不推送
#[tauri::command]
fn vcs_commit_push(state: State<'_, WorkspaceState>, remote: bool) -> Result<String, String> {
    let root = workspace_root(&state)?;
    let msg = format!("接口文档更新 {}", now_stamp());
    match detect_vcs(&root).as_deref() {
        Some("git") => {
            run_cmd("git", &["add", "-A"], &root)?;
            match run_cmd("git", &["commit", "-m", &msg], &root) {
                Ok(out) => {
                    if remote {
                        let push = run_cmd("git", &["push"], &root)?;
                        Ok(format!("{out}\n{push}"))
                    } else {
                        Ok(format!("{out}\n（未开启同步远程，已跳过 push）"))
                    }
                }
                Err(e) => {
                    // 没有改动时视为成功
                    if e.contains("nothing to commit")
                        || e.contains("no changes added")
                        || e.contains("没有要提交的内容")
                    {
                        Ok("没有需要提交的变更".into())
                    } else {
                        Err(e)
                    }
                }
            }
        }
        Some("svn") => {
            run_cmd("svn", &["add", "--force", "."], &root)?;
            run_cmd("svn", &["commit", "-m", &msg], &root)
        }
        _ => Err("当前工作目录不是 Git / SVN 仓库".into()),
    }
}

// ==================== 最近打开的工作目录 ====================

fn recent_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("获取配置目录失败: {e}"))?;
    Ok(dir.join("recent_workspaces.json"))
}

fn read_recent(app: &AppHandle) -> Vec<String> {
    let p = recent_path(app).ok();
    match p.and_then(|p| fs::read_to_string(p).ok()) {
        Some(content) => serde_json::from_str(&content).unwrap_or_default(),
        None => Vec::new(),
    }
}

/// 记录最近打开的工作目录（去重、最新的排最前；保留全部历史，数量只影响前端展示）
fn record_recent(app: &AppHandle, path: &str) {
    let Ok(p) = recent_path(app) else { return };
    let mut list = read_recent(app);
    list.retain(|x| x != path);
    list.insert(0, path.to_string());
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = write_pretty(&p, &list);
}

#[tauri::command]
fn get_recent_workspaces(app: AppHandle) -> Vec<String> {
    // 过滤掉已不存在的目录；返回全部存在的记录，由前端按设置限制展示数量（历史记录不删除）
    read_recent(&app)
        .into_iter()
        .filter(|p| PathBuf::from(p).is_dir())
        .collect()
}

/// 按路径直接打开工作目录（开始页「最近打开」点击时调用）
#[tauri::command]
fn open_workspace(
    app: AppHandle,
    state: State<'_, WorkspaceState>,
    path: String,
) -> Result<String, String> {
    let p = PathBuf::from(&path);
    if !p.is_dir() {
        return Err(format!("目录不存在: {path}"));
    }
    if let Ok(mut guard) = state.root.lock() {
        *guard = Some(p);
    }
    record_recent(&app, &path);
    Ok(path)
}

#[tauri::command]
fn get_workspace(state: State<'_, WorkspaceState>) -> Option<String> {
    state
        .root
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
fn pick_workspace(app: AppHandle, state: State<'_, WorkspaceState>) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app.dialog().file().blocking_pick_folder();
    match picked {
        Some(path) => {
            let p = path.into_path().map_err(|e| e.to_string())?;
            let s = p.to_string_lossy().to_string();
            if let Ok(mut guard) = state.root.lock() {
                *guard = Some(p);
            }
            // 记录最近打开
            record_recent(&app, &s);
            Ok(Some(s))
        }
        None => Ok(None),
    }
}

/// 工作区是否为空（仅含自动生成的 __info.json / __envs.json 也算空）
fn is_workspace_empty(root: &Path) -> Result<bool, String> {
    let mut count = 0;
    for entry in fs::read_dir(root).map_err(|e| format!("读取目录失败: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name == INFO_FILE || name == ENV_FILE {
            continue;
        }
        count += 1;
    }
    Ok(count == 0)
}

/// 工作区根目录是否已存在 __info.json（判断是否为全新工作目录）
#[tauri::command]
fn has_workspace_info(state: State<'_, WorkspaceState>) -> Result<bool, String> {
    let root = workspace_root(&state)?;
    Ok(root.join(INFO_FILE).exists())
}

#[tauri::command]
fn workspace_is_empty(state: State<'_, WorkspaceState>) -> Result<bool, String> {
    let root = workspace_root(&state)?;
    is_workspace_empty(&root)
}

/// 在空工作区中生成演示案例（示例分组 + 接口 + 环境变量）
#[tauri::command]
fn create_demo(state: State<'_, WorkspaceState>) -> Result<(), String> {
    let root = workspace_root(&state)?;
    // 不判断工作区是否为空：演示案例直接生成（同名文件会被覆盖）
    let api_file = |name: &str, method: &str, path: &str, description: &str| {
        serde_json::json!({
            "uuid": uuid::Uuid::new_v4().to_string(),
            "name": name,
            "method": method,
            "path": path,
            "url": "",
            "description": description,
            "headers": [],
            "query": [],
            "params": [],
            "body": { "mode": "none", "raw": "", "form": [] },
            "mock": { "enabled": false, "status": 200, "headers": [], "delay": 0, "body": "" },
            "examples": []
        })
    };
    let write = |dir: &str, file: &str, value: &serde_json::Value| -> Result<(), String> {
        let dir_path = if dir.is_empty() {
            root.clone()
        } else {
            root.join(dir)
        };
        fs::create_dir_all(&dir_path).map_err(|e| format!("创建目录失败: {e}"))?;
        write_pretty(&dir_path.join(file), value)
    };

    // docParams 快捷构造：位置 + 字段名 + 类型 + 说明（children 可嵌套下级字段）
    let d = |source: &str, key: &str, ty: &str, desc: &str, children: Vec<serde_json::Value>| -> serde_json::Value {
        serde_json::json!({
            "source": source, "key": key, "type": ty, "description": desc,
            "itemType": "", "objectName": key, "children": children
        })
    };

    // 根信息 + 环境变量
    write("", INFO_FILE, &serde_json::json!({
        "name": "演示 API 集合",
        "description": "这是一个示例工作区，展示了 API Manager 的目录组织方式",
        "baseUrl": "{{baseUrl}}",
        "mockPort": 5050
    }))?;
    write("", ENV_FILE, &serde_json::json!({
        "active": "开发环境",
        "environments": [
            {
                "name": "开发环境",
                "variables": [
                    { "key": "baseUrl", "value": "http://127.0.0.1:5050", "defaultValue": "https://api.example.com", "description": "接口服务地址", "enabled": true },
                    { "key": "token", "value": "dev-token-123456", "defaultValue": "demo-token", "description": "鉴权令牌", "enabled": true }
                ]
            },
            {
                "name": "生产环境",
                "variables": [
                    { "key": "baseUrl", "value": "https://api.example.com", "defaultValue": "https://api.example.com", "description": "接口服务地址", "enabled": true },
                    { "key": "token", "value": "prod-token-abcdef", "defaultValue": "demo-token", "description": "鉴权令牌", "enabled": true }
                ]
            }
        ]
    }))?;

    // 用户管理分组
    write("用户管理", INFO_FILE, &serde_json::json!({ "name": "用户管理", "description": "用户相关接口" }))?;
    let mut create_user = api_file("创建用户", "POST", "/api/users", "创建一个新用户");
    create_user["headers"] = serde_json::json!([{ "key": "Content-Type", "value": "application/json", "enabled": true, "description": "" }]);
    create_user["body"] = serde_json::json!({ "mode": "json", "raw": "{\n  \"name\": \"张三\",\n  \"email\": \"zhangsan@example.com\",\n  \"role\": \"user\"\n}", "form": [] });
    create_user["mock"] = serde_json::json!({ "enabled": true, "status": 201, "headers": [], "delay": 0, "body": "{\n  \"code\": 0,\n  \"data\": {\n    \"id\": 1001,\n    \"name\": \"张三\",\n    \"email\": \"zhangsan@example.com\"\n  },\n  \"message\": \"创建成功\"\n}" });
    create_user["docParams"] = serde_json::json!([
        d("body", "name", "String", "用户名", vec![]),
        d("body", "email", "String", "邮箱地址", vec![]),
        d("body", "role", "String", "用户角色（user / admin / vip）", vec![]),
        d("resp_success", "code", "Integer", "状态码，0 表示成功", vec![]),
        d("resp_success", "data", "Object", "创建成功的用户数据", vec![
            d("resp_success", "id", "Integer", "用户ID", vec![]),
            d("resp_success", "name", "String", "用户名", vec![]),
            d("resp_success", "email", "String", "邮箱地址", vec![]),
        ]),
        d("resp_success", "message", "String", "提示信息", vec![]),
        d("resp_fail", "code", "Integer", "错误码，非 0 表示失败", vec![]),
        d("resp_fail", "message", "String", "错误描述", vec![]),
        d("resp_fail", "errors", "Object", "字段校验错误明细", vec![
            d("resp_fail", "field", "String", "出错的字段名", vec![]),
            d("resp_fail", "reason", "String", "出错原因", vec![]),
        ]),
    ]);
    write("用户管理", "创建用户.json", &create_user)?;

    let mut get_user = api_file("获取用户信息", "GET", "/api/users/{id}", "查询单个用户信息");
    get_user["params"] = serde_json::json!([{ "key": "id", "value": "1", "enabled": true, "description": "用户ID" }]);
    get_user["mock"] = serde_json::json!({ "enabled": true, "status": 200, "headers": [], "delay": 0, "body": "{\n  \"code\": 0,\n  \"data\": {\n    \"id\": 1,\n    \"name\": \"张三\",\n    \"email\": \"zhangsan@example.com\"\n  },\n  \"message\": \"成功\"\n}" });
    get_user["docParams"] = serde_json::json!([
        d("path", "id", "Integer", "用户ID", vec![]),
        d("resp_success", "code", "Integer", "状态码，0 表示成功", vec![]),
        d("resp_success", "data", "Object", "用户信息", vec![
            d("resp_success", "id", "Integer", "用户ID", vec![]),
            d("resp_success", "name", "String", "用户名", vec![]),
            d("resp_success", "email", "String", "邮箱地址", vec![]),
        ]),
        d("resp_success", "message", "String", "提示信息", vec![]),
        d("resp_fail", "code", "Integer", "错误码（404 表示用户不存在）", vec![]),
        d("resp_fail", "message", "String", "错误描述", vec![]),
    ]);
    write("用户管理", "获取用户信息.json", &get_user)?;

    let mut del_user = api_file("删除用户", "DELETE", "/api/users/{id}", "删除指定用户");
    del_user["params"] = serde_json::json!([{ "key": "id", "value": "1", "enabled": true, "description": "用户ID" }]);
    del_user["mock"] = serde_json::json!({ "enabled": true, "status": 200, "headers": [], "delay": 0, "body": "{\n  \"code\": 0,\n  \"message\": \"删除成功\"\n}" });
    del_user["docParams"] = serde_json::json!([
        d("path", "id", "Integer", "用户ID", vec![]),
        d("resp_success", "code", "Integer", "状态码，0 表示成功", vec![]),
        d("resp_success", "message", "String", "提示信息", vec![]),
        d("resp_fail", "code", "Integer", "错误码（404 表示用户不存在）", vec![]),
        d("resp_fail", "message", "String", "错误描述", vec![]),
    ]);
    write("用户管理", "删除用户.json", &del_user)?;

    let mut update_user = api_file("更新用户", "PUT", "/api/users/{id}", "全量更新用户信息");
    update_user["params"] = serde_json::json!([{ "key": "id", "value": "1", "enabled": true, "description": "用户ID" }]);
    update_user["headers"] = serde_json::json!([{ "key": "Content-Type", "value": "application/json", "enabled": true, "description": "" }]);
    update_user["body"] = serde_json::json!({ "mode": "json", "raw": "{\n  \"name\": \"张三\",\n  \"email\": \"zhangsan@example.com\",\n  \"role\": \"admin\"\n}", "form": [] });
    update_user["mock"] = serde_json::json!({ "enabled": true, "status": 200, "headers": [], "delay": 0, "body": "{\n  \"code\": 0,\n  \"data\": {\n    \"id\": 1,\n    \"name\": \"张三\",\n    \"email\": \"zhangsan@example.com\",\n    \"role\": \"admin\"\n  },\n  \"message\": \"更新成功\"\n}" });
    update_user["docParams"] = serde_json::json!([
        d("path", "id", "Integer", "用户ID", vec![]),
        d("body", "name", "String", "用户名", vec![]),
        d("body", "email", "String", "邮箱地址", vec![]),
        d("body", "role", "String", "用户角色（user / admin / vip）", vec![]),
        d("resp_success", "code", "Integer", "状态码，0 表示成功", vec![]),
        d("resp_success", "data", "Object", "更新后的用户数据", vec![
            d("resp_success", "id", "Integer", "用户ID", vec![]),
            d("resp_success", "name", "String", "用户名", vec![]),
            d("resp_success", "email", "String", "邮箱地址", vec![]),
            d("resp_success", "role", "String", "用户角色", vec![]),
        ]),
        d("resp_success", "message", "String", "提示信息", vec![]),
        d("resp_fail", "code", "Integer", "错误码（404 表示用户不存在）", vec![]),
        d("resp_fail", "message", "String", "错误描述", vec![]),
    ]);
    write("用户管理", "更新用户.json", &update_user)?;

    let mut patch_user = api_file("部分更新用户", "PATCH", "/api/users/{id}", "仅更新用户的指定字段");
    patch_user["params"] = serde_json::json!([{ "key": "id", "value": "1", "enabled": true, "description": "用户ID" }]);
    patch_user["headers"] = serde_json::json!([{ "key": "Content-Type", "value": "application/json", "enabled": true, "description": "" }]);
    patch_user["body"] = serde_json::json!({ "mode": "json", "raw": "{\n  \"role\": \"vip\"\n}", "form": [] });
    patch_user["mock"] = serde_json::json!({ "enabled": true, "status": 200, "headers": [], "delay": 0, "body": "{\n  \"code\": 0,\n  \"data\": {\n    \"id\": 1,\n    \"role\": \"vip\"\n  },\n  \"message\": \"更新成功\"\n}" });
    patch_user["docParams"] = serde_json::json!([
        d("path", "id", "Integer", "用户ID", vec![]),
        d("body", "role", "String", "要更新的字段（仅传需要修改的字段）", vec![]),
        d("resp_success", "code", "Integer", "状态码，0 表示成功", vec![]),
        d("resp_success", "data", "Object", "更新后的用户数据（仅包含更新的字段）", vec![
            d("resp_success", "id", "Integer", "用户ID", vec![]),
            d("resp_success", "role", "String", "更新后的角色", vec![]),
        ]),
        d("resp_success", "message", "String", "提示信息", vec![]),
        d("resp_fail", "code", "Integer", "错误码", vec![]),
        d("resp_fail", "message", "String", "错误描述", vec![]),
    ]);
    write("用户管理", "部分更新用户.json", &patch_user)?;

    // 订单管理分组
    write("订单管理", INFO_FILE, &serde_json::json!({ "name": "订单管理", "description": "订单相关接口" }))?;
    let mut list_orders = api_file("获取订单列表", "GET", "/api/orders", "分页查询订单列表");
    list_orders["query"] = serde_json::json!([
        { "key": "page", "value": "1", "enabled": true, "description": "页码" },
        { "key": "pageSize", "value": "10", "enabled": true, "description": "每页数量" }
    ]);
    list_orders["mock"] = serde_json::json!({ "enabled": true, "status": 200, "headers": [], "delay": 0, "body": "{\n  \"code\": 0,\n  \"data\": {\n    \"list\": [\n      { \"id\": 1001, \"no\": \"SO20240101001\", \"amount\": 99.5 },\n      { \"id\": 1002, \"no\": \"SO20240101002\", \"amount\": 199.0 }\n    ],\n    \"total\": 2\n  },\n  \"message\": \"成功\"\n}" });
    list_orders["docParams"] = serde_json::json!([
        d("query", "page", "Integer", "页码，从 1 开始", vec![]),
        d("query", "pageSize", "Integer", "每页数量，最大 100", vec![]),
        d("resp_success", "code", "Integer", "状态码，0 表示成功", vec![]),
        d("resp_success", "data", "Object", "分页数据", vec![
            d("resp_success", "list", "List", "订单列表", vec![
                d("resp_success", "items", "Object", "订单信息", vec![
                    d("resp_success", "id", "Integer", "订单ID", vec![]),
                    d("resp_success", "no", "String", "订单编号", vec![]),
                    d("resp_success", "amount", "Float", "订单金额", vec![]),
                ]),
            ]),
            d("resp_success", "total", "Integer", "总记录数", vec![]),
        ]),
        d("resp_success", "message", "String", "提示信息", vec![]),
        d("resp_fail", "code", "Integer", "错误码", vec![]),
        d("resp_fail", "message", "String", "错误描述", vec![]),
    ]);
    write("订单管理", "获取订单列表.json", &list_orders)?;

    let mut head_order = api_file("检查订单状态", "HEAD", "/api/orders/{id}", "仅获取响应头，不返回响应体");
    head_order["params"] = serde_json::json!([{ "key": "id", "value": "1001", "enabled": true, "description": "订单ID" }]);
    head_order["mock"] = serde_json::json!({ "enabled": true, "status": 200, "headers": [{ "key": "X-Order-Status", "value": "paid", "enabled": true }], "delay": 0, "body": "" });
    write("订单管理", "检查订单状态.json", &head_order)?;

    let mut options_orders = api_file("订单接口预检", "OPTIONS", "/api/orders", "跨域预检请求（CORS）");
    options_orders["mock"] = serde_json::json!({ "enabled": true, "status": 204, "headers": [{ "key": "Access-Control-Allow-Methods", "value": "GET,POST,PUT,PATCH,DELETE,HEAD,OPTIONS", "enabled": true }], "delay": 0, "body": "" });
    write("订单管理", "订单接口预检.json", &options_orders)?;

    // WebSocket 分组（与 tests/websocket-server.py 一一对应）：仅保留一个回显示例，
    // 服务器会回传该连接获取到的 query / header 参数供核对
    write("WebSocket", INFO_FILE, &serde_json::json!({ "name": "WebSocket", "description": "WebSocket 接口示例（与 tests/websocket-server.py 一一对应）" }))?;

    let ws_desc = "WebSocket 回显演示接口，配合测试服务 tests/websocket-server.py 使用。\n\n【启动测试服务】\n1. 安装依赖：pip install websockets\n2. 启动服务：python tests/websocket-server.py\n   - 默认监听 ws://127.0.0.1:8765\n   - 自定义端口：python tests/websocket-server.py 9999\n\n【接口说明】\n- 连接地址：ws://127.0.0.1:8765/echo\n- 连接时携带 Query 参数：token={{token}}（开发环境下值为 dev-token-123456）\n- 浏览器 WebSocket API 无法自定义请求头：Header 页签中配置的值不会发送，服务器回传的 header 为连接时的标准请求头（host、user-agent 等）\n\n【测试步骤】\n1. 点击「发送」建立连接，连接成功后会先收到一条欢迎消息（type: welcome，含本次连接的 query / header）\n2. 在消息输入框输入任意内容并发送\n3. 服务器回传消息内容及本次连接收到的 query / header，例如：\n{\"type\":\"message\",\"query\":{\"token\":\"dev-token-123456\"},\"header\":{\"host\":\"127.0.0.1:8765\",\"user-agent\":\"<客户端 User-Agent>\"},\"message\":\"hello\"}";
    let mut ws_echo = api_file("WebSocket 回显", "GET", "/echo", ws_desc);
    ws_echo["protocol"] = serde_json::json!("websocket");
    ws_echo["url"] = serde_json::json!("ws://127.0.0.1:8765/echo?token={{token}}");
    ws_echo["query"] = serde_json::json!([{ "key": "token", "value": "{{token}}", "enabled": true, "description": "鉴权令牌" }]);
    ws_echo["body"] = serde_json::json!({ "mode": "raw", "raw": "hello, this is a websocket echo message", "form": [], "binaryPath": "" });
    ws_echo["responses"] = serde_json::json!([
        { "id": format!("ws-echo-{}", uuid::Uuid::new_v4()), "name": "回显成功", "status": 0, "content_type": "application/json", "body": "{\n  \"type\": \"message\",\n  \"query\": {\"token\": \"dev-token-123456\"},\n  \"header\": {\"host\": \"127.0.0.1:8765\", \"user-agent\": \"<客户端 User-Agent>\"},\n  \"message\": \"hello, this is a websocket echo message\"\n}" }
    ]);
    write("WebSocket", "WebSocket 回显.json", &ws_echo)?;

    // GraphQL 分组（与 tests/graphql-server.py 一一对应）：仅支持 POST + JSON body，不支持 Mock
    write("GraphQL", INFO_FILE, &serde_json::json!({ "name": "GraphQL", "description": "GraphQL 接口示例（与 tests/graphql-server.py 一一对应）" }))?;

    let gql_desc = "GraphQL 接口演示，配合测试服务 tests/graphql-server.py 使用。\n\n【启动测试服务】\n1. 无需安装第三方依赖（纯 Python 标准库）\n2. 启动服务：python tests/graphql-server.py\n   - 默认监听 http://127.0.0.1:8080/graphql\n   - 自定义端口：python tests/graphql-server.py 9999\n\n【接口说明】\n- GraphQL 接口固定使用 POST 方法，Body 仅支持 JSON 格式\n- 不支持 Mock（GraphQL 无法按路径生成路由）\n- 请求体结构：{ \"query\": \"...\", \"variables\": {} }\n\n【测试步骤】\n1. 点击「发送」执行下方 query / mutation 语句\n2. 服务端返回对应数据（data 字段）或错误信息（errors 字段）";

    let mut gql_query_user = api_file("查询用户", "POST", "/graphql", gql_desc);
    gql_query_user["protocol"] = serde_json::json!("graphql");
    gql_query_user["url"] = serde_json::json!("http://127.0.0.1:8080/graphql");
    gql_query_user["headers"] = serde_json::json!([{ "key": "Content-Type", "value": "application/json", "enabled": true, "description": "" }]);
    gql_query_user["body"] = serde_json::json!({ "mode": "json", "raw": "{\n  \"query\": \"query { user(id: 1) { id name email role } }\"\n}", "form": [], "binaryPath": "" });
    gql_query_user["responses"] = serde_json::json!([
        { "id": format!("gql-user-{}", uuid::Uuid::new_v4()), "name": "返回成功", "status": 200, "content_type": "application/json", "body": "{\n  \"data\": {\n    \"user\": {\n      \"id\": 1,\n      \"name\": \"张三\",\n      \"email\": \"zhangsan@example.com\",\n      \"role\": \"user\"\n    }\n  }\n}" }
    ]);
    gql_query_user["docParams"] = serde_json::json!([
        d("body", "query", "String", "GraphQL 查询语句（query / mutation）", vec![]),
        d("body", "variables", "Object", "查询变量（可选）", vec![]),
        d("resp_success", "data", "Object", "查询结果数据", vec![
            d("resp_success", "user", "Object", "用户信息", vec![
                d("resp_success", "id", "Integer", "用户ID", vec![]),
                d("resp_success", "name", "String", "用户名", vec![]),
                d("resp_success", "email", "String", "邮箱地址", vec![]),
                d("resp_success", "role", "String", "用户角色", vec![]),
            ]),
        ]),
        d("resp_fail", "errors", "List", "GraphQL 错误列表（如用户不存在）", vec![]),
    ]);
    write("GraphQL", "查询用户.json", &gql_query_user)?;

    let mut gql_list_users = api_file("用户列表", "POST", "/graphql", "查询全部用户（GraphQL query）");
    gql_list_users["protocol"] = serde_json::json!("graphql");
    gql_list_users["url"] = serde_json::json!("http://127.0.0.1:8080/graphql");
    gql_list_users["headers"] = serde_json::json!([{ "key": "Content-Type", "value": "application/json", "enabled": true, "description": "" }]);
    gql_list_users["body"] = serde_json::json!({ "mode": "json", "raw": "{\n  \"query\": \"query { users { id name email role } }\"\n}", "form": [], "binaryPath": "" });
    gql_list_users["responses"] = serde_json::json!([
        { "id": format!("gql-users-{}", uuid::Uuid::new_v4()), "name": "返回成功", "status": 200, "content_type": "application/json", "body": "{\n  \"data\": {\n    \"users\": [\n      { \"id\": 1, \"name\": \"张三\", \"email\": \"zhangsan@example.com\", \"role\": \"user\" },\n      { \"id\": 2, \"name\": \"李四\", \"email\": \"lisi@example.com\", \"role\": \"admin\" }\n    ]\n  }\n}" }
    ]);
    write("GraphQL", "用户列表.json", &gql_list_users)?;

    let mut gql_create_user = api_file("创建用户", "POST", "/graphql", "通过 mutation 创建用户");
    gql_create_user["protocol"] = serde_json::json!("graphql");
    gql_create_user["url"] = serde_json::json!("http://127.0.0.1:8080/graphql");
    gql_create_user["headers"] = serde_json::json!([{ "key": "Content-Type", "value": "application/json", "enabled": true, "description": "" }]);
    gql_create_user["body"] = serde_json::json!({ "mode": "json", "raw": "{\n  \"query\": \"mutation { createUser(name: \\\"王五\\\", email: \\\"wangwu@example.com\\\") { id name email } }\"\n}", "form": [], "binaryPath": "" });
    gql_create_user["responses"] = serde_json::json!([
        { "id": format!("gql-create-{}", uuid::Uuid::new_v4()), "name": "返回成功", "status": 200, "content_type": "application/json", "body": "{\n  \"data\": {\n    \"createUser\": {\n      \"id\": 3,\n      \"name\": \"王五\",\n      \"email\": \"wangwu@example.com\"\n    }\n  }\n}" }
    ]);
    write("GraphQL", "创建用户.json", &gql_create_user)?;

    let mut gql_order = api_file("查询订单", "POST", "/graphql", "查询订单详情（含嵌套字段）");
    gql_order["protocol"] = serde_json::json!("graphql");
    gql_order["url"] = serde_json::json!("http://127.0.0.1:8080/graphql");
    gql_order["headers"] = serde_json::json!([{ "key": "Content-Type", "value": "application/json", "enabled": true, "description": "" }]);
    gql_order["body"] = serde_json::json!({ "mode": "json", "raw": "{\n  \"query\": \"query { order(id: 1001) { id no amount items { name price } } }\"\n}", "form": [], "binaryPath": "" });
    gql_order["responses"] = serde_json::json!([
        { "id": format!("gql-order-{}", uuid::Uuid::new_v4()), "name": "返回成功", "status": 200, "content_type": "application/json", "body": "{\n  \"data\": {\n    \"order\": {\n      \"id\": 1001,\n      \"no\": \"SO20240101001\",\n      \"amount\": 99.5,\n      \"items\": [\n        { \"name\": \"鼠标\", \"price\": 49.5 },\n        { \"name\": \"键盘\", \"price\": 50.0 }\n      ]\n    }\n  }\n}" }
    ]);
    write("GraphQL", "查询订单.json", &gql_order)?;

    // Socket.IO 分组（与 tests/socketio-server.py 一一对应）：实时消息交互，展示与 WebSocket 一致
    write("Socket.IO", INFO_FILE, &serde_json::json!({ "name": "Socket.IO", "description": "Socket.IO 接口示例（与 tests/socketio-server.py 一一对应）" }))?;
    let sio_desc = "Socket.IO 实时消息接口演示，配合测试服务 tests/socketio-server.py 使用。\n\n【启动测试服务】\n1. 安装依赖：pip install python-socketio simple-websocket\n2. 启动服务：python tests/socketio-server.py\n   - 默认监听 http://127.0.0.1:8090\n   - 自定义端口：python tests/socketio-server.py 9999\n\n【接口说明】\n- Socket.IO 连接地址为 http://127.0.0.1:8090（不提供 ws/wss 切换，由库内部协商传输方式）\n- 消息事件名固定为 message：发送的消息会原样回显，并附带本次连接的 query 参数\n- 浏览器端不可自定义请求头，Header 页签中的配置不会发送\n\n【测试步骤】\n1. 点击「发送」建立连接，连接成功后会先收到一条欢迎消息（type: welcome）\n2. 在消息输入框输入任意内容并发送\n3. 服务器回传消息内容及本次连接的 query 参数，例如：\n{\"type\":\"message\",\"query\":{\"token\":\"dev-token-123456\"},\"message\":\"hello\"}";
    let mut sio_chat = api_file("实时聊天", "GET", "/", sio_desc);
    sio_chat["protocol"] = serde_json::json!("socketio");
    sio_chat["url"] = serde_json::json!("http://127.0.0.1:8090");
    sio_chat["body"] = serde_json::json!({ "mode": "text", "raw": "hello socket.io", "form": [], "binaryPath": "" });
    sio_chat["responses"] = serde_json::json!([]);
    write("Socket.IO", "实时聊天.json", &sio_chat)?;

    let mut sio_broadcast = api_file("广播通知", "GET", "/", "向所有已连接客户端广播一条消息（Socket.IO broadcast 事件）。\n\n【测试步骤】\n1. 先启动 tests/socketio-server.py（默认 http://127.0.0.1:8090）\n2. 点击「发送」建立连接并收到欢迎消息\n3. 发送消息：{\"cmd\":\"broadcast\",\"msg\":\"hello everyone\"}\n4. 所有连接的客户端都会收到这条广播（type: broadcast）");
    sio_broadcast["protocol"] = serde_json::json!("socketio");
    sio_broadcast["url"] = serde_json::json!("http://127.0.0.1:8090");
    sio_broadcast["body"] = serde_json::json!({ "mode": "json", "raw": "{\n  \"cmd\": \"broadcast\",\n  \"msg\": \"hello everyone\"\n}", "form": [], "binaryPath": "" });
    sio_broadcast["responses"] = serde_json::json!([]);
    write("Socket.IO", "广播通知.json", &sio_broadcast)?;

    // 对象示例：工作区 .object/ 下生成「用户管理 / 订单管理」分组与几个对象，
    // 与上面的接口演示呼应（属性含 mock 示例值，可配合数据生成体验）
    let now = chrono::Local::now().timestamp();
    let prop = |key: &str, kind: &str, item_kind: &str, description: &str, mock: &str| {
        crate::objects::ObjectProp {
            key: key.into(),
            kind: kind.into(),
            item_kind: item_kind.into(),
            ref_hash: String::new(),
            description: description.into(),
            mock: mock.into(),
        }
    };
    let obj_def = |name: &str, object_name: &str, group: &str, description: &str, properties: Vec<crate::objects::ObjectProp>| {
        crate::objects::ObjectDef {
            uuid: uuid::Uuid::new_v4().to_string(),
            hash: String::new(), // save_objects 会重算
            name: name.into(),
            object_name: object_name.into(),
            package_name: String::new(),
            group: group.into(),
            deprecated: false,
            description: description.into(),
            properties,
            created_at: now,
            updated_at: now,
        }
    };
    let demo_store = crate::objects::ObjectStore {
        groups: vec![
            crate::objects::ObjectGroup { id: "用户管理".into(), name: "用户管理".into(), deprecated: false },
            crate::objects::ObjectGroup { id: "订单管理".into(), name: "订单管理".into(), deprecated: false },
        ],
        objects: vec![
            obj_def("用户", "User", "用户管理", "系统用户信息", vec![
                prop("id", "Integer", "Integer", "主键", ""),
                prop("name", "String", "String", "用户名", "@cname"),
                prop("email", "String", "String", "邮箱地址", "@email"),
                prop("role", "String", "String", "用户角色（user / admin / vip）", "user"),
                prop("createdAt", "Datetime", "String", "创建时间", "@datetime"),
            ]),
            obj_def("订单", "Order", "订单管理", "用户订单", vec![
                prop("id", "Integer", "Integer", "订单ID", ""),
                prop("no", "String", "String", "订单编号", "SO2024"),
                prop("amount", "Float", "Float", "订单金额（元）", "99.5"),
                prop("status", "String", "String", "订单状态（pending/paid/shipped/done）", "paid"),
                prop("userId", "Integer", "Integer", "下单用户ID", "1001"),
                prop("createdAt", "Datetime", "String", "下单时间", "@datetime"),
            ]),
            obj_def("订单明细", "OrderItem", "订单管理", "订单包含的商品明细", vec![
                prop("id", "Integer", "Integer", "明细ID", ""),
                prop("productName", "String", "String", "商品名称", "@ctitle(6)"),
                prop("price", "Float", "Float", "单价（元）", "19.9"),
                prop("quantity", "Integer", "Integer", "数量", "2"),
            ]),
        ],
    };
    crate::objects::save_objects_impl(&root, &demo_store)?;

    Ok(())
}

#[tauri::command]
fn read_tree(state: State<'_, WorkspaceState>) -> Result<TreeNode, String> {
    let root = workspace_root(&state)?;
    build_folder_node(&root)
}

#[tauri::command]
fn read_api(path: String) -> Result<ApiFile, String> {
    let content = fs::read_to_string(&path).map_err(|e| format!("读取失败: {e}"))?;
    // 仅旧文件（无 responses 字段）需要迁移补全；显式保存过空列表的接口保持原样
    let has_responses = serde_json::from_str::<Value>(&content)
        .ok()
        .map(|v| v.get("responses").is_some())
        .unwrap_or(false);
    let mut api: ApiFile =
        serde_json::from_str(&content).map_err(|e| format!("JSON 解析失败: {e}"))?;
    if !has_responses {
        ensure_responses(&mut api);
    }
    Ok(api)
}

#[tauri::command]
fn save_api(path: String, data: ApiFile) -> Result<String, String> {
    let mut data = data;
    if data.uuid.trim().is_empty() {
        data.uuid = uuid::Uuid::new_v4().to_string();
    }
    if let Some(parent) = Path::new(&path).parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
    }
    write_pretty(Path::new(&path), &data)?;
    Ok(path)
}

/// 保存接口新版本：写入工作区根目录 .version/<uuid>/<名称>.<版本号>.json
#[tauri::command]
fn save_api_version(
    state: State<'_, WorkspaceState>,
    data: ApiFile,
) -> Result<String, String> {
    let root = workspace_root(&state)?;
    save_api_version_at(&root, data)
}

/// 纯函数：把版本写入 root 下的 .version 目录（根目录下，不随接口所在子目录变化）
fn save_api_version_at(root: &Path, data: ApiFile) -> Result<String, String> {
    let mut data = data;
    if !valid_uuid(data.uuid.trim()) {
        data.uuid = uuid::Uuid::new_v4().to_string();
    }
    let uuid = data.uuid.trim().to_string();
    let name = sanitize_filename(&data.name);
    let name = if name.trim().is_empty() {
        "未命名接口".to_string()
    } else {
        name.trim().to_string()
    };

    let ver_dir = root.join(".version").join(&uuid);
    fs::create_dir_all(&ver_dir).map_err(|e| format!("创建版本目录失败: {e}"))?;

    // 计算下一个版本号：<名称>.1.json / .2.json ...
    let version = next_version(&ver_dir, &name);

    let target = ver_dir.join(format!("{name}.{version}.json"));
    write_pretty(&target, &data)?;
    Ok(format!(".version/{uuid}/{name}.{version}.json"))
}

/// uuid 仅允许十六进制字符与连字符，防止路径穿越
fn valid_uuid(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

/// 计算下一个版本号：扫描 <name>.<n>.json 取最大 n + 1
fn next_version(ver_dir: &Path, name: &str) -> u32 {
    let mut max: u32 = 0;
    if let Ok(rd) = fs::read_dir(ver_dir) {
        for entry in rd.flatten() {
            let fname = entry.file_name().to_string_lossy().to_string();
            if let Some(rest) = fname.strip_prefix(&format!("{name}.")) {
                if let Some(num) = rest.strip_suffix(".json") {
                    if let Ok(n) = num.parse::<u32>() {
                        max = max.max(n);
                    }
                }
            }
        }
    }
    max + 1
}

/// 列出某个接口（按 uuid）的所有历史版本，按版本号从大到小排序
#[tauri::command]
fn list_versions(state: State<'_, WorkspaceState>, uuid: String) -> Result<Vec<VersionInfo>, String> {
    let root = workspace_root(&state)?;
    let uuid = uuid.trim().to_string();
    if !valid_uuid(&uuid) {
        return Ok(vec![]);
    }
    let dir = root.join(".version").join(&uuid);
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut list: Vec<VersionInfo> = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| format!("读取版本目录失败: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().map(|e| e != "json").unwrap_or(true) {
            continue;
        }
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        // <名称>.<版本号>
        let (name, version) = match stem.rfind('.') {
            Some(idx) => match stem[idx + 1..].parse::<u32>() {
                Ok(n) => (stem[..idx].to_string(), n),
                Err(_) => continue,
            },
            None => continue,
        };
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let mut method = None;
        let mut endpoint = None;
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(v) = serde_json::from_str::<Value>(&content) {
                method = v.get("method").and_then(|x| x.as_str()).map(String::from);
                endpoint = v.get("path").and_then(|x| x.as_str()).map(String::from);
            }
        }
        list.push(VersionInfo {
            version,
            name,
            path: path.to_string_lossy().to_string(),
            modified,
            method,
            endpoint,
        });
    }
    list.sort_by(|a, b| b.version.cmp(&a.version));
    Ok(list)
}

/// 查询接口当前版本号：.version/<uuid> 目录下的最大版本号（未保存过版本返回 0）
#[tauri::command]
fn get_current_version(state: State<'_, WorkspaceState>, uuid: String) -> Result<u32, String> {
    let root = workspace_root(&state)?;
    let uuid = uuid.trim().to_string();
    if !valid_uuid(&uuid) {
        return Ok(0);
    }
    let dir = root.join(".version").join(&uuid);
    if !dir.exists() {
        return Ok(0);
    }
    let mut max: u32 = 0;
    for entry in fs::read_dir(&dir).map_err(|e| format!("读取版本目录失败: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let fname = entry.file_name().to_string_lossy().to_string();
        if let Some(stem) = fname.strip_suffix(".json") {
            if let Some(idx) = stem.rfind('.') {
                if let Ok(n) = stem[idx + 1..].parse::<u32>() {
                    max = max.max(n);
                }
            }
        }
    }
    Ok(max)
}

/// 读取某个历史版本文件的原始内容（用于 diff）
#[tauri::command]
fn read_api_version(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("读取版本失败: {e}"))
}

/// 保存对象版本快照（.object_version/<uuid>/<版本号>.json）
#[tauri::command]
fn save_object_version(
    state: State<'_, WorkspaceState>,
    uuid: String,
    snapshot: crate::objects::ObjectDef,
) -> Result<String, String> {
    let root = workspace_root(&state)?;
    crate::objects::save_object_version(&root, &uuid, &snapshot)
}

/// 对象版本列表
#[tauri::command]
fn list_object_versions(
    state: State<'_, WorkspaceState>,
    uuid: String,
) -> Result<Vec<crate::objects::ObjectVersionInfo>, String> {
    let root = workspace_root(&state)?;
    crate::objects::list_object_versions(&root, &uuid)
}

/// 读取指定版本的对象快照
#[tauri::command]
fn read_object_version(
    state: State<'_, WorkspaceState>,
    uuid: String,
    version: u32,
) -> Result<crate::objects::ObjectDef, String> {
    let root = workspace_root(&state)?;
    crate::objects::read_object_version(&root, &uuid, version)
}

/// 遍历工作区内的接口 json 文件（跳过 .version / .examples 等点开头目录），返回首个满足条件的路径
fn walk_api_files<F: FnMut(&Path, &Value) -> bool>(root: &Path, mut pred: F) -> Option<PathBuf> {
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let p = entry.path();
            let fname = entry.file_name().to_string_lossy().to_string();
            if fname.starts_with('.') {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            if p.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = fs::read_to_string(&p) {
                    if let Ok(v) = serde_json::from_str::<Value>(&content) {
                        if pred(&p, &v) {
                            return Some(p);
                        }
                    }
                }
            }
        }
    }
    None
}

/// 按 uuid 查找接口主文件路径
fn find_api_path_by_uuid(root: &Path, uuid: &str) -> Option<PathBuf> {
    if !valid_uuid(uuid) {
        return None;
    }
    walk_api_files(root, |_, v| v.get("uuid").and_then(|x| x.as_str()) == Some(uuid))
}

/// 按名称查找接口主文件路径（旧文件未持久化 uuid 时的兜底）
fn find_api_path_by_name(root: &Path, name: &str) -> Option<PathBuf> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    walk_api_files(root, |_, v| {
        v.get("name")
            .and_then(|x| x.as_str())
            .map(str::trim)
            == Some(name)
    })
}

/// 恢复到指定历史版本：先把当前状态默认保存为一个新版本（备份），再把版本内容写回接口主文件
#[tauri::command]
fn restore_api_version(
    state: State<'_, WorkspaceState>,
    version_path: String,
    uuid: String,
) -> Result<String, String> {
    let root = workspace_root(&state)?;
    restore_api_version_at(&root, &version_path, &uuid)
}

fn restore_api_version_at(root: &Path, version_path: &str, uuid: &str) -> Result<String, String> {
    // 1. 读取选中版本内容
    let raw = fs::read_to_string(version_path).map_err(|e| format!("读取版本失败: {e}"))?;
    let restored: ApiFile =
        serde_json::from_str(&raw).map_err(|e| format!("版本内容解析失败: {e}"))?;
    // 2. 定位主文件：优先 uuid，其次版本文件自带的 uuid，最后按名称兜底
    let main = find_api_path_by_uuid(root, uuid)
        .or_else(|| find_api_path_by_uuid(root, restored.uuid.trim()))
        .or_else(|| find_api_path_by_name(root, &restored.name))
        .ok_or("未找到该接口文件".to_string())?;
    // 3. 先把当前状态保存为新版本（备份），再写回版本内容
    let current = read_api(main.to_string_lossy().to_string())?;
    save_api_version_at(root, current)?;
    save_api(main.to_string_lossy().to_string(), restored)?;
    Ok(main.to_string_lossy().to_string())
}

#[tauri::command]
fn create_api(
    state: State<'_, WorkspaceState>,
    dir: String,
    name: String,
    protocol: Option<String>,
) -> Result<String, String> {
    let root = workspace_root(&state)?;
    let dir_path = if dir.trim().is_empty() {
        root.clone()
    } else {
        PathBuf::from(&dir)
    };
    if !dir_path.starts_with(&root) {
        return Err("保存位置不在工作区内".into());
    }
    fs::create_dir_all(&dir_path).map_err(|e| format!("创建目录失败: {e}"))?;
    // 显示名保留原始输入（支持 /xxx/xxx 路径风格名称），仅文件名做安全化处理
    let display_name = if name.trim().is_empty() {
        "未命名接口".to_string()
    } else {
        name.trim().to_string()
    };
    let file_base = sanitize_filename(&display_name);
    let file_path = unique_path(&dir_path, &file_base, ".json");

    let data = ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name: display_name,
        // GraphQL 接口固定使用 POST
        method: if protocol.as_deref() == Some("graphql") {
            "POST".into()
        } else {
            "GET".into()
        },
        path: if protocol.as_deref() == Some("graphql") {
            "/graphql".into()
        } else {
            "/".into()
        },
        url: String::new(),
        description: String::new(),
        headers: vec![],
        query: vec![],
        params: vec![],
        body: if protocol.as_deref() == Some("graphql") {
            // GraphQL 固定 JSON body
            BodyData {
                mode: "json".into(),
                raw: String::new(),
                form: vec![],
                binary_path: String::new(),
            }
        } else {
            BodyData::default()
        },
        mock: MockConfig::default(),
        examples: vec![],
        responses: default_responses(),
        doc_params: vec![],
        deprecated: false,
        protocol: match protocol.as_deref() {
            Some("websocket") => "websocket".into(),
            Some("socketio") => "socketio".into(),
            Some("graphql") => "graphql".into(),
            _ => "http".into(),
        },
    };    write_pretty(&file_path, &data)?;
    Ok(file_path.to_string_lossy().to_string())
}

#[tauri::command]
fn create_folder(
    state: State<'_, WorkspaceState>,
    parent: String,
    name: String,
) -> Result<String, String> {
    let root = workspace_root(&state)?;
    let parent_path = if parent.trim().is_empty() {
        root.clone()
    } else {
        PathBuf::from(&parent)
    };
    if !parent_path.starts_with(&root) {
        return Err("保存位置不在工作区内".into());
    }
    fs::create_dir_all(&parent_path).map_err(|e| format!("创建目录失败: {e}"))?;
    let base = sanitize_filename(&name);
    let base = if base.is_empty() { "新分组".to_string() } else { base };
    let dir_path = unique_path(&parent_path, &base, "");
    fs::create_dir_all(&dir_path).map_err(|e| format!("创建目录失败: {e}"))?;
    let info = InfoJson {
        name: Some(base),
        description: String::new(),
        base_url: None,
        mock_port: None,
        order: None,
        collapsed: None,
        deprecated: None,
    };
    write_pretty(&dir_path.join(INFO_FILE), &info)?;
    Ok(dir_path.to_string_lossy().to_string())
}

#[tauri::command]
fn rename_entry(
    state: State<'_, WorkspaceState>,
    path: String,
    new_name: String,
) -> Result<(), String> {
    let root = workspace_root(&state)?;
    let old = PathBuf::from(&path);
    ensure_inside_workspace(&root, &old)?;
    // 显示名保留原始输入（支持 /xxx/xxx），仅用于文件系统的名称做安全化处理
    let display_name = new_name.trim().to_string();
    if display_name.is_empty() {
        return Err("名称不能为空".into());
    }
    let fs_name = sanitize_filename(&display_name);
    if fs_name.is_empty() {
        return Err("名称不能为空".into());
    }
    let parent = old.parent().ok_or("无上级目录")?;
    let (base, ext) = if old.is_dir() {
        (fs_name, String::new())
    } else {
        let ext = old
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_else(|| ".json".into());
        let base = if fs_name.to_lowercase().ends_with(&ext.to_lowercase()) {
            fs_name.trim_end_matches(&ext).to_string()
        } else {
            fs_name
        };
        (base, ext)
    };
    let new_path = unique_path(parent, &base, &ext);
    fs::rename(&old, &new_path).map_err(|e| format!("重命名失败: {e}"))?;
    if old.is_dir() {
        // 目录重命名时同步更新 __info.json 的 name（显示名，可含 / 路径风格）
        let mut info = read_info_file(&new_path);
        info.name = Some(display_name);
        let _ = write_pretty(&new_path.join(INFO_FILE), &info);
    } else {
        // 接口文件重命名时同步更新 JSON 内的 name 字段（显示名，可含 / 路径风格）
        if let Ok(content) = fs::read_to_string(&new_path) {
            if let Ok(mut v) = serde_json::from_str::<Value>(&content) {
                if let Some(obj) = v.as_object_mut() {
                    obj.insert("name".into(), Value::String(display_name));
                    let _ = write_pretty(&new_path, &v);
                }
            }
        }
    }
    Ok(())
}

#[tauri::command]
fn delete_entry(state: State<'_, WorkspaceState>, path: String) -> Result<(), String> {
    let root = workspace_root(&state)?;
    let target = PathBuf::from(&path);
    ensure_inside_workspace(&root, &target)?;
    if target.is_dir() {
        fs::remove_dir_all(&target).map_err(|e| format!("删除失败: {e}"))
    } else {
        fs::remove_file(&target).map_err(|e| format!("删除失败: {e}"))
    }
}

/// 复制接口/分组到其所在目录：接口重新生成 uuid（分组则递归复制整棵树，
/// 其中每个接口都重新生成 uuid），名称追加「 副本」，重名自动加序号。
#[tauri::command]
fn copy_entry(state: State<'_, WorkspaceState>, path: String) -> Result<String, String> {
    let root = workspace_root(&state)?;
    let src = PathBuf::from(&path);
    ensure_inside_workspace(&root, &src)?;
    if src == root {
        return Err("不能复制工作区根目录".into());
    }
    let parent = src.parent().ok_or("无上级目录")?;
    let fs_name = src
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let copy_base = format!("{fs_name} 副本");
    let (base, ext) = if src.is_dir() {
        (sanitize_filename(&copy_base), String::new())
    } else {
        let ext = src
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_else(|| ".json".into());
        (sanitize_filename(&copy_base), ext)
    };
    let dst = unique_path(parent, &base, &ext);

    if src.is_dir() {
        copy_dir_with_new_uuids(&src, &dst)?;
    } else {
        copy_api_file(&src, &dst)?;
    }
    Ok(dst.to_string_lossy().to_string())
}

/// 复制单个接口文件：重新生成 uuid，显示名追加「 副本」
fn copy_api_file(src: &Path, dst: &Path) -> Result<(), String> {
    let content = fs::read_to_string(src).map_err(|e| format!("读取失败: {e}"))?;
    let mut v: Value = serde_json::from_str(&content).map_err(|e| format!("JSON 解析失败: {e}"))?;
    if let Some(obj) = v.as_object_mut() {
        obj.insert("uuid".into(), Value::String(uuid::Uuid::new_v4().to_string()));
        if let Some(Some(n)) = obj.get("name").map(|x| x.as_str()) {
            if !n.trim().is_empty() {
                obj.insert("name".into(), Value::String(format!("{n} 副本")));
            }
        }
    }
    write_pretty(dst, &v)
}

/// 递归复制目录：所有接口 JSON 重新生成 uuid，分组 __info.json 的 name 追加「 副本」，
/// 跳过 .examples / .version 等点开头目录（内容与旧 uuid 绑定，不随复制携带）
fn copy_dir_with_new_uuids(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("创建目录失败: {e}"))?;
    for entry in fs::read_dir(src).map_err(|e| format!("读取目录失败: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let sp = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.starts_with('.') {
            continue;
        }
        let dp = dst.join(&file_name);
        if sp.is_dir() {
            copy_dir_with_new_uuids(&sp, &dp)?;
        } else if file_name == INFO_FILE {
            // 分组信息：name 追加「 副本」
            if let Ok(content) = fs::read_to_string(&sp) {
                if let Ok(mut v) = serde_json::from_str::<Value>(&content) {
                    if let Some(obj) = v.as_object_mut() {
                        if let Some(Some(n)) = obj.get("name").map(|x| x.as_str()) {
                            obj.insert("name".into(), Value::String(format!("{n} 副本")));
                        }
                        let _ = write_pretty(&dp, &v);
                        continue;
                    }
                }
            }
            fs::copy(&sp, &dp).map_err(|e| format!("复制失败: {e}"))?;
        } else {
            // 接口 JSON：重新生成 uuid 与显示名；其余文件原样复制
            if file_name.ends_with(".json") && file_name != ENV_FILE {
                copy_api_file(&sp, &dp)?;
            } else {
                fs::copy(&sp, &dp).map_err(|e| format!("复制失败: {e}"))?;
            }
        }
    }
    Ok(())
}

/// 移动接口/目录到目标目录（跨目录拖拽）。目录不能移入自身或其子目录。
#[tauri::command]
fn move_entry(
    state: State<'_, WorkspaceState>,
    src: String,
    dst_dir: String,
) -> Result<String, String> {
    let root = workspace_root(&state)?;
    move_entry_inner(&root, &src, &dst_dir)
}

fn move_entry_inner(root: &Path, src: &str, dst_dir: &str) -> Result<String, String> {
    let src_path = PathBuf::from(src);
    let dst_path = if dst_dir.trim().is_empty() {
        root.to_path_buf()
    } else {
        PathBuf::from(dst_dir)
    };
    if !dst_path.starts_with(root) {
        return Err("目标位置不在工作区内".into());
    }
    if !src_path.starts_with(root) || src_path == root {
        return Err("路径不在工作区内".into());
    }
    if src_path == dst_path {
        return Ok(src_path.to_string_lossy().to_string());
    }
    // 目录不能移入自身或其子目录
    if src_path.is_dir() && dst_path.starts_with(&src_path) {
        return Err("不能将目录移动到自身或其子目录".into());
    }
    fs::create_dir_all(&dst_path).map_err(|e| format!("创建目标目录失败: {e}"))?;
    // 目标位置重名时自动加序号（如 "xxx (2)"），文件内容/显示名保持不变
    let (base, ext) = if src_path.is_dir() {
        let name = src_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        (name, String::new())
    } else {
        let stem = src_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = src_path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_else(|| ".json".into());
        (stem, ext)
    };
    let target = unique_path(&dst_path, &base, &ext);
    fs::rename(&src_path, &target).map_err(|e| format!("移动失败: {e}"))?;
    Ok(target.to_string_lossy().to_string())
}

#[tauri::command]
fn read_info(state: State<'_, WorkspaceState>, path: String) -> Result<InfoJson, String> {
    let root = workspace_root(&state)?;
    let p = PathBuf::from(&path);
    if p == root {
        return Ok(read_info_file(&p));
    }
    if !p.starts_with(&root) {
        return Err("路径不在工作区内".into());
    }
    Ok(read_info_file(&p))
}

#[tauri::command]
fn save_info(
    state: State<'_, WorkspaceState>,
    path: String,
    data: InfoJson,
) -> Result<(), String> {
    let root = workspace_root(&state)?;
    let p = PathBuf::from(&path);
    if !p.starts_with(&root) {
        return Err("路径不在工作区内".into());
    }
    fs::create_dir_all(&p).map_err(|e| format!("创建目录失败: {e}"))?;
    // 与已有信息合并，避免丢失 order / collapsed 等字段
    let mut merged = read_info_file(&p);
    if data.name.is_some() {
        merged.name = data.name;
    }
    merged.description = data.description;
    if data.base_url.is_some() {
        merged.base_url = data.base_url;
    }
    if data.mock_port.is_some() {
        merged.mock_port = data.mock_port;
    }
    if data.deprecated.is_some() {
        merged.deprecated = data.deprecated;
    }
    write_pretty(&p.join(INFO_FILE), &merged)
}

/// 标记 / 取消标记“已废弃”：接口写入其 JSON 文件的 deprecated 字段，
/// 分组写入其目录下 __info.json 的 deprecated 字段。返回新的废弃状态。
#[tauri::command]
fn toggle_deprecated(
    state: State<'_, WorkspaceState>,
    path: String,
) -> Result<bool, String> {
    let root = workspace_root(&state)?;
    let target = PathBuf::from(&path);
    ensure_inside_workspace(&root, &target)?;

    if target.is_dir() {
        let mut info = read_info_file(&target);
        let next = !info.deprecated.unwrap_or(false);
        info.deprecated = Some(next);
        write_pretty(&target.join(INFO_FILE), &info)?;
        Ok(next)
    } else {
        let content = fs::read_to_string(&target).map_err(|e| format!("读取失败: {e}"))?;
        let mut v: Value =
            serde_json::from_str(&content).map_err(|e| format!("JSON 解析失败: {e}"))?;
        let cur = v.get("deprecated").and_then(|d| d.as_bool()).unwrap_or(false);
        let next = !cur;
        if let Some(obj) = v.as_object_mut() {
            obj.insert("deprecated".into(), Value::Bool(next));
        }
        write_pretty(&target, &v)?;
        Ok(next)
    }
}

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}


// ==================== 全局环境变量命令 ====================

#[tauri::command]
fn read_envs(state: State<'_, WorkspaceState>) -> Result<EnvStore, String> {
    let root = workspace_root(&state)?;
    Ok(read_env_file(&root))
}

#[tauri::command]
fn save_envs(state: State<'_, WorkspaceState>, data: EnvStore) -> Result<(), String> {
    let root = workspace_root(&state)?;
    write_pretty(&root.join(ENV_FILE), &data)
}

// ==================== 入口 ====================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(WorkspaceState::default())
        .manage(MockRunState::default())
        .setup(|app| {
            crate::tray::setup_tray(app)?;
            // 启动后异步检查 GitHub Releases（延迟 3 秒避免与启动抢资源，失败静默）
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                crate::tray::tray_check_update(&handle);
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            // 点击窗口关闭按钮 -> 隐藏到托盘（而非退出）
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let exiting = window
                    .app_handle()
                    .try_state::<TrayState>()
                    .map(|s| s.exiting.load(Ordering::Relaxed))
                    .unwrap_or(false);
                if !exiting {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            load_settings,
            save_settings,
            crate::tray::set_language,
            get_workspace,
            open_workspace,
            get_recent_workspaces,
            pick_workspace,
            workspace_is_empty,
            has_workspace_info,
            create_demo,
            crate::import::import_postman,
            crate::import::import_openapi,
            crate::import::import_apifox,
            crate::import::import_apipost,
            crate::import::import_raml,
            crate::import::import_wadl,
            crate::import::import_har,
            crate::import::import_yapi,
            crate::import::import_eolink,
            crate::import::import_insomnia,
            crate::import::import_jmeter,
            crate::import::import_apidoc,
            crate::import::import_extra,
            crate::markdown::render_api_markdown,
            crate::markdown::render_group_markdown,
            crate::markdown::render_markdown,
            crate::markdown::export_api_markdown,
            crate::import::import_markdown,
            crate::export::export_selection,
            vcs_info,
            vcs_sync,
            vcs_commit_push,
            read_tree,
            read_api,
            save_api,
            save_api_version,
            list_versions,
            get_current_version,
            read_api_version,
            restore_api_version,
            save_object_version,
            list_object_versions,
            read_object_version,
            create_api,
            create_folder,
            rename_entry,
            copy_entry,
            move_entry,
            delete_entry,
            read_info,
            save_info,
            toggle_deprecated,
            read_envs,
            save_envs,
            crate::tray::update_tray_env,
            get_app_version,
            crate::update::check_update,
            crate::request::send_request,
            crate::request::pick_file,
            crate::history::save_history,
            crate::history::history_records,
            crate::history::history_detail,
            crate::history::history_days,
            crate::history::history_clear,
            crate::history::save_example,
            crate::history::list_examples,
            crate::history::read_example,
            crate::history::delete_example,
            crate::mock::mock_start,
            crate::mock::mock_stop,
            crate::mock::mock_status,
            crate::mock::list_custom_mocks,
            crate::mock::save_custom_mock,
            crate::mock::delete_custom_mock,
            crate::objects::list_objects,
            crate::objects::save_objects,
            crate::objects::gen_data,
            crate::objects::list_gen_logs,
            crate::objects::import_json_object,
            crate::objects::import_ddl,
            crate::objects::object_usage,
            mock_reload
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 极简 HTTP 服务器：对所有请求返回固定 JSON
    fn start_test_server() -> (std::net::SocketAddr, std::thread::JoinHandle<()>, std::sync::Arc<std::sync::atomic::AtomicBool>) {
        use std::io::ErrorKind;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use std::time::Duration;

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = stop.clone();
        let handle = std::thread::spawn(move || {
            loop {
                if stop2.load(Ordering::Relaxed) {
                    break;
                }
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let _ = std::thread::spawn(move || {
                            use std::io::{Read, Write};
                            let mut buf = [0u8; 4096];
                            let _ = stream.read(&mut buf);
                            let body = r#"{"hello":"world","n":42}"#;
                            let resp = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nX-Test: yes\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(),
                                body
                            );
                            let _ = stream.write_all(resp.as_bytes());
                        });
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });
        (addr, handle, stop)
    }

    #[tokio::test]
    async fn test_send_request_ok() {
        let (addr, handle, stop) = start_test_server();
        let req = HttpRequestData {
            method: "GET".into(),
            url: format!("http://{addr}/api/users/1001?page=1"),
            headers: vec![],
            body: None,
            body_file: None,
            form: None,
            timeout_ms: 5000,
        };
        let res = send_request(req).await.unwrap();
        assert!(res.ok);
        assert_eq!(res.status, 200);
        assert_eq!(res.status_text, "OK");
        assert!(res.body.contains("\"hello\":\"world\""));
        assert!(res.headers.iter().any(|(k, v)| k == "x-test" && v == "yes"));
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        handle.join().unwrap();
    }

    #[tokio::test]
    async fn test_send_request_multipart_file() {
        use axum::extract::Multipart;
        use axum::response::IntoResponse;

        async fn upload(mut mp: Multipart) -> impl IntoResponse {
            let mut text = String::new();
            let mut files = Vec::new();
            while let Some(field) = mp.next_field().await.unwrap() {
                let name = field.name().unwrap_or("").to_string();
                let data = field.bytes().await.unwrap();
                if name == "file" {
                    files.push(String::from_utf8_lossy(&data).to_string());
                } else {
                    text.push_str(&format!("{name}={}", String::from_utf8_lossy(&data)));
                }
            }
            format!("text:{text};files:{}", files.join(","))
        }

        let app = axum::Router::new().route("/upload", axum::routing::post(upload));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // 准备一个待上传的临时文件
        let file = std::env::temp_dir().join(format!("upload-test-{}.txt", std::process::id()));
        fs::write(&file, "hello-multipart").unwrap();

        let req = HttpRequestData {
            method: "POST".into(),
            url: format!("http://{addr}/upload"),
            headers: vec![],
            body: None,
            body_file: None,
            form: Some(vec![
                KeyValue {
                    key: "name".into(),
                    value: "张三".into(),
                    enabled: true,
                    is_file: false,
                    description: String::new(),
                },
                KeyValue {
                    key: "file".into(),
                    value: file.to_string_lossy().to_string(),
                    enabled: true,
                    is_file: true,
                    description: String::new(),
                },
            ]),
            timeout_ms: 5000,
        };
        let res = send_request(req).await.unwrap();
        server.abort();
        let _ = server.await;
        let _ = fs::remove_file(&file);

        assert!(res.ok, "multipart 请求应成功: {:?}", res.error);
        assert_eq!(res.status, 200);
        assert!(res.body.contains("name=张三"), "应包含文本字段: {}", res.body);
        assert!(res.body.contains("hello-multipart"), "应包含文件内容: {}", res.body);
    }

    #[tokio::test]
    async fn test_send_request_binary_file() {
        // 二进制模式：读取本地文件字节作为请求体发送
        async fn echo_body(body: axum::body::Bytes) -> impl axum::response::IntoResponse {
            format!("bytes:{}", String::from_utf8_lossy(&body))
        }

        let app = axum::Router::new().route("/echo", axum::routing::post(echo_body));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let file = std::env::temp_dir().join(format!("binary-test-{}.bin", std::process::id()));
        fs::write(&file, b"\x00\x01binary-body\xff").unwrap();

        let req = HttpRequestData {
            method: "POST".into(),
            url: format!("http://{addr}/echo"),
            headers: vec![],
            body: None,
            body_file: Some(file.to_string_lossy().to_string()),
            form: None,
            timeout_ms: 5000,
        };
        let res = send_request(req).await.unwrap();
        server.abort();
        let _ = server.await;
        let _ = fs::remove_file(&file);

        assert!(res.ok, "二进制请求应成功: {:?}", res.error);
        assert_eq!(res.status, 200);
        assert!(
            res.body.contains("binary-body"),
            "应发送文件字节: {}",
            res.body
        );
    }

    #[tokio::test]
    async fn test_send_request_bad_url() {
        // 未替换的 {{变量}} 会产生 reqwest builder error，应给出中文提示而不是裸的 builder error
        for url in ["http://{{host}}:8080/api", "127.0.0.1:8080/api"] {
            let req = HttpRequestData {
                method: "GET".into(),
                url: url.to_string(),
                headers: vec![],
                body: None,
                body_file: None,
                form: None,
                timeout_ms: 3000,
            };
            let res = send_request(req).await.unwrap();
            assert!(!res.ok, "url [{url}] 应失败");
            let err = res.error.unwrap_or_default();
            assert!(
                err.contains("URL 格式不正确") && !err.starts_with("builder"),
                "url [{url}] 错误信息不友好: {err}"
            );
        }
    }

    #[tokio::test]
    async fn test_send_request_connection_refused() {
        // 绑定一个端口后立刻释放，用于模拟连接失败
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let req = HttpRequestData {
            method: "GET".into(),
            url: format!("http://{addr}/"),
            headers: vec![],
            body: None,
            body_file: None,
            form: None,
            timeout_ms: 3000,
        };
        let res = send_request(req).await.unwrap();
        assert!(!res.ok);
        assert!(res.error.is_some());
    }

    #[test]
    fn test_read_env_map() {
        // 示例工作区：激活“开发环境”
        let root =
            Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../examples/demo-workspace"));
        let map = read_env_map(root);
        assert_eq!(
            map.get("baseUrl").map(|s| s.as_str()),
            Some("http://127.0.0.1:5050")
        );
        assert_eq!(map.get("token").map(|s| s.as_str()), Some("dev-token-123456"));
        // 不存在的环境 -> 空
        assert!(!map.contains_key("nope"));
    }

    #[test]
    fn test_import_postman() {
        let root = std::env::temp_dir().join(format!("apimgr-postman-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let coll = root.join("collection.json");
        fs::write(
            &coll,
            r#"{
                "info": { "name": "示例集合" },
                "variable": [
                    { "key": "baseUrl", "value": "https://api.example.com", "type": "string", "description": "接口基地址" },
                    { "key": "token", "value": "dev-token-123456", "type": "string" },
                    { "key": "timeout", "value": 30, "type": "number", "description": { "content": "超时秒数", "type": "text/plain" } }
                ],
                "item": [
                    {
                        "name": "获取用户",
                        "request": {
                            "method": "GET",
                            "url": {
                                "raw": "https://api.example.com/users/:id?page=1",
                                "query": [{ "key": "page", "value": "1", "disabled": false }]
                            },
                            "header": [{ "key": "Authorization", "value": "Bearer {{token}}", "disabled": false }]
                        }
                    },
                    {
                        "name": "订单",
                        "item": [
                            {
                                "name": "创建订单",
                                "request": {
                                    "method": "POST",
                                    "url": { "raw": "https://api.example.com/orders" },
                                    "body": { "mode": "raw", "raw": "{\"no\":\"1\"}" }
                                }
                            }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();
        let result = import_postman_file(&root, &coll).unwrap();
        assert!(result.folder.ends_with("示例集合"));
        // 集合变量 -> 环境变量集
        assert_eq!(result.env, "示例集合");
        assert_eq!(result.vars, 3);
        let env_store: EnvStore =
            serde_json::from_str(&fs::read_to_string(root.join("__envs.json")).unwrap()).unwrap();
        assert_eq!(env_store.active, "示例集合");
        assert_eq!(env_store.environments.len(), 1);
        let env = &env_store.environments[0];
        assert_eq!(env.name, "示例集合");
        assert_eq!(env.variables.len(), 3);
        let find = |k: &str| env.variables.iter().find(|v| v.key == k).unwrap();
        assert_eq!(find("baseUrl").value, "https://api.example.com");
        assert_eq!(find("baseUrl").description, "接口基地址");
        assert_eq!(find("token").value, "dev-token-123456");
        // 数字 value 转字符串、结构化 description 取 content
        assert_eq!(find("timeout").value, "30");
        assert_eq!(find("timeout").description, "超时秒数");
        // 顶层接口 + 子分组 + 子接口
        assert!(root.join("示例集合/获取用户.json").exists());
        assert!(root.join("示例集合/订单/创建订单.json").exists());
        // 校验内容：方法 / 路径变量 / query / header / body
        let api: ApiFile = serde_json::from_str(
            &fs::read_to_string(root.join("示例集合/获取用户.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(api.method, "GET");
        assert_eq!(api.path, "/users/{id}");
        assert_eq!(api.params.len(), 1);
        assert_eq!(api.params[0].key, "id");
        assert_eq!(api.query.len(), 1);
        assert_eq!(api.query[0].key, "page");
        assert_eq!(api.headers.len(), 1);
        assert_eq!(api.headers[0].key, "Authorization");
        let api2: ApiFile = serde_json::from_str(
            &fs::read_to_string(root.join("示例集合/订单/创建订单.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(api2.method, "POST");
        assert_eq!(api2.body.mode, "json");
        // 重复导入同一集合：变量按 key 合并，不产生重复集
        import_postman_file(&root, &coll).unwrap();
        let env_store2: EnvStore =
            serde_json::from_str(&fs::read_to_string(root.join("__envs.json")).unwrap()).unwrap();
        assert_eq!(env_store2.environments.len(), 1);
        assert_eq!(env_store2.environments[0].variables.len(), 3);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_env_store_roundtrip() {
        let dir = std::env::temp_dir().join(format!("env-test-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let store = EnvStore {
            active: "dev".into(),
            environments: vec![
                Environment {
                    name: "dev".into(),
                    variables: vec![EnvVariable {
                        key: "k".into(),
                        value: "v".into(),
                        default_value: "".into(),
                        description: "".into(),
                        enabled: true,
                    }],
                },
                Environment {
                    name: "prod".into(),
                    variables: vec![],
                },
            ],
        };
        write_pretty(&dir.join(ENV_FILE), &store).unwrap();
        let back = read_env_file(&dir);
        assert_eq!(back.active, "dev");
        assert_eq!(back.environments.len(), 2);
        assert_eq!(back.environments[0].variables[0].key, "k");
        assert_eq!(read_env_map(&dir).get("k").map(|s| s.as_str()), Some("v"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_env_default_value_fallback() {
        // 现有值为空时，自动使用默认值
        let dir = std::env::temp_dir().join(format!("env-default-test-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let store = EnvStore {
            active: "dev".into(),
            environments: vec![Environment {
                name: "dev".into(),
                variables: vec![
                    EnvVariable {
                        key: "empty_value".into(),
                        value: "".into(),
                        default_value: "fallback".into(),
                        description: "".into(),
                        enabled: true,
                    },
                    EnvVariable {
                        key: "has_value".into(),
                        value: "real".into(),
                        default_value: "fallback".into(),
                        description: "".into(),
                        enabled: true,
                    },
                ],
            }],
        };
        write_pretty(&dir.join(ENV_FILE), &store).unwrap();
        let map = read_env_map(&dir);
        assert_eq!(map.get("empty_value").map(|s| s.as_str()), Some("fallback"));
        assert_eq!(map.get("has_value").map(|s| s.as_str()), Some("real"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_next_version() {
        // 版本号递增：<name>.1.json / .2.json ...
        let dir = std::env::temp_dir().join(format!("version-test-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        for f in ["x.1.json", "x.3.json", "other.9.json"] {
            fs::write(dir.join(f), "{}").unwrap();
        }
        assert_eq!(next_version(&dir, "x"), 4);
        assert_eq!(next_version(&dir, "y"), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_version_saved_at_workspace_root() {
        // .version 必须创建在工作区根目录下，而不是接口文件所在的子目录
        let root = std::env::temp_dir().join(format!("version-root-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        // 接口文件位于根目录的子目录中
        let sub = root.join("some-folder");
        fs::create_dir_all(&sub).unwrap();

        let api = ApiFile {
            uuid: "11111111-2222-3333-4444-555555555555".into(),
            name: "创建用户".into(),
            method: "POST".into(),
            path: "/api/users".into(),
            url: "".into(),
            description: "".into(),
            headers: vec![],
            query: vec![],
            params: vec![],
            body: BodyData::default(),
            mock: MockConfig::default(),
            examples: vec![],
            responses: vec![],
            doc_params: vec![],
        deprecated: false,
        protocol: "http".into(),
        };
        let rel = save_api_version_at(&root, api).unwrap();
        assert!(rel.starts_with(".version/11111111-2222-3333-4444-555555555555/"));
        let ver_file = root
            .join(".version")
            .join("11111111-2222-3333-4444-555555555555")
            .join("创建用户.1.json");
        assert!(ver_file.exists(), "版本文件应写入根目录 .version 下");
        assert!(
            !sub.join(".version").exists(),
            "版本目录不应出现在接口所在子目录中"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("v0.1.5"), vec![0, 1, 5]);
        assert_eq!(parse_version("0.2.0"), vec![0, 2, 0]);
        assert_eq!(parse_version("V1.2.3-beta.4"), vec![1, 2, 3, 4]);
        assert_eq!(parse_version("9.9.9"), vec![9, 9, 9]);
        assert_eq!(parse_version(""), Vec::<u32>::new());
    }

    #[test]
    fn test_version_gt() {
        assert!(version_gt("0.2.0", "0.1.5"));
        assert!(version_gt("1.0.0", "0.9.9"));
        assert!(version_gt("0.1.10", "0.1.9"));
        assert!(version_gt("0.2", "0.1.9")); // 段数多
        assert!(!version_gt("0.1.5", "0.1.5"));
        assert!(!version_gt("0.1.4", "0.1.5"));
        assert!(!version_gt("0.1.9", "0.2.0"));
        assert!(!version_gt("", "0.1.5")); // 空版本不视为更新
    }

    /// 旧文件无 responses 字段时：返回成功取 mock 体、返回失败由 resp_fail 文档生成，docParams 重键到 resp:<id>
    #[test]
    fn ensure_responses_migrates_old_files() {
        let mut api = ApiFile {
            uuid: "u".into(),
            name: "测试".into(),
            method: "GET".into(),
            path: "/x".into(),
            url: String::new(),
            description: String::new(),
            headers: vec![],
            query: vec![],
            params: vec![],
            body: BodyData::default(),
            mock: MockConfig {
                enabled: false,
                status: 200,
                headers: vec![],
                delay: 0,
                body: r#"{"code":0}"#.into(),
            },
            examples: vec![],
            responses: vec![],
            doc_params: vec![DocParam {
                source: "resp_fail".into(),
                key: "message".into(),
                r#type: "String".into(),
                description: "错误描述".into(),
                item_type: String::new(),
                object_name: String::new(),
                children: vec![],
            }],
            deprecated: false,
            protocol: "http".into(),
        };
        ensure_responses(&mut api);
        assert_eq!(api.responses.len(), 2);
        assert_eq!(api.responses[0].name, "返回成功");
        assert_eq!(api.responses[0].status, 200);
        assert_eq!(api.responses[0].body, r#"{"code":0}"#);
        assert_eq!(api.responses[1].name, "返回失败");
        assert!(api.responses[1].body.contains("message"), "fail body: {}", api.responses[1].body);
        // docParams 已重键到新条目 id
        assert!(api.doc_params.iter().all(|d| d.source == format!("resp:{}", api.responses[1].id)));
    }

    /// 分组目录保存 .md/.html：export_api_markdown 的分支走 group_markdown_doc，目录不再是 read_api 目标
    #[test]
    fn group_markdown_doc_renders_group_dir() {
        let base = std::env::temp_dir().join(format!("apim-gmdoc-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let g = base.join("用户管理");
        fs::create_dir_all(&g).unwrap();
        let a = ApiFile {
            uuid: "u".into(),
            name: "接口A".into(),
            method: "GET".into(),
            path: "/a".into(),
            url: String::new(),
            description: String::new(),
            headers: vec![],
            query: vec![],
            params: vec![],
            body: BodyData::default(),
            mock: MockConfig {
                enabled: false,
                status: 200,
                headers: vec![],
                delay: 0,
                body: String::new(),
            },
            examples: vec![],
            responses: vec![],
            doc_params: vec![],
        deprecated: false,
        protocol: "http".into(),
        };
        fs::write(
            g.join("接口A.json"),
            serde_json::to_string(&a).unwrap(),
        )
        .unwrap();
        // 空分组直接报错（与导出逻辑一致）
        let empty = base.join("空分组");
        fs::create_dir_all(&empty).unwrap();
        assert!(group_markdown_doc(&base, &empty.to_string_lossy()).is_err());
        let (name, md) = group_markdown_doc(&base, &g.to_string_lossy()).expect("group doc");
        assert_eq!(name, "用户管理");
        assert!(md.contains("## 接口A"), "md: {md}");
        // 分组名即标题：不再重复输出 # 用户管理
        assert_eq!(md.matches("# 用户管理").count(), 1, "md: {md}");
        let _ = fs::remove_dir_all(&base);
    }

    /// 恢复到历史版本：先自动备份当前状态为新版本，再把版本内容写回主文件
    #[test]
    fn restore_api_version_backs_up_then_restores() {
        let base = std::env::temp_dir().join(format!("apim-restore-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("接口"));
        let uuid = "a1b2c3d4-1111-2222-3333-444455556666".to_string();
        let make = |name: &str, desc: &str| ApiFile {
            uuid: uuid.clone(),
            name: name.into(),
            method: "GET".into(),
            path: "/x".into(),
            url: String::new(),
            description: desc.into(),
            headers: vec![],
            query: vec![],
            params: vec![],
            body: BodyData::default(),
            mock: MockConfig {
                enabled: false,
                status: 200,
                headers: vec![],
                delay: 0,
                body: String::new(),
            },
            examples: vec![],
            responses: vec![],
            doc_params: vec![],
        deprecated: false,
        protocol: "http".into(),
        };
        let main = base.join("接口").join("接口A.json");
        save_api(main.to_string_lossy().to_string(), make("接口A", "v1 描述"))
            .unwrap();
        // 保存两个版本：v1（描述 v1 描述）与 v2（描述 v2 描述）
        save_api_version_at(&base, make("接口A", "v1 描述")).unwrap();
        let _v2 = save_api_version_at(&base, make("接口A", "v2 描述")).unwrap();
        // 主文件当前是 v2 描述
        let mut current = read_api(main.to_string_lossy().to_string()).unwrap();
        current.description = "v2 描述".into();
        save_api(main.to_string_lossy().to_string(), current).unwrap();
        // 列出版本：v2、v1（从大到小）
        let dir = base.join(".version").join(&uuid);
        let files: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(files.len(), 2, "files: {files:?}");
        // 恢复到 v1
        let v1_path = dir.join("接口A.1.json");
        let restored_main = restore_api_version_at(&base, &v1_path.to_string_lossy(), &uuid);
        let main_str = restored_main.unwrap();
        assert_eq!(main_str, main.to_string_lossy().to_string());
        let restored = read_api(main_str).unwrap();
        assert_eq!(restored.description, "v1 描述");
        // 恢复前自动保存了当前（v2）为新版本 → 现在 3 个版本文件
        let files2: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(files2.len(), 3, "files: {files2:?}");
        let backup = read_api(dir.join("接口A.3.json").to_string_lossy().to_string()).unwrap();
        assert_eq!(backup.description, "v2 描述");
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn test_detect_vcs() {
        // 检测 .git / .svn；都没有则返回 None
        let root = std::env::temp_dir().join(format!("vcs-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        assert_eq!(detect_vcs(&root), None);
        fs::create_dir_all(root.join(".git")).unwrap();
        assert_eq!(detect_vcs(&root).as_deref(), Some("git"));
        fs::remove_dir_all(root.join(".git")).unwrap();
        fs::create_dir_all(root.join(".svn")).unwrap();
        assert_eq!(detect_vcs(&root).as_deref(), Some("svn"));
        // .git 优先于 .svn
        fs::create_dir_all(root.join(".git")).unwrap();
        assert_eq!(detect_vcs(&root).as_deref(), Some("git"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_import_openapi_file() {
        let root = std::env::temp_dir().join(format!("oas-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let spec = serde_json::json!({
            "openapi": "3.0.0",
            "info": { "title": "PetStore", "version": "1.0" },
            "servers": [ { "url": "https://api.example.com/v1" } ],
            "paths": {
                "/pets/{id}": {
                    "get": {
                        "tags": ["pets"],
                        "summary": "按 ID 获取宠物",
                        "parameters": [
                            { "name": "id", "in": "path", "required": true, "schema": { "type": "integer" } },
                            { "name": "verbose", "in": "query", "schema": { "type": "boolean", "default": true } }
                        ],
                        "responses": { "200": { "description": "ok" } }
                    }
                },
                "/pets": {
                    "post": {
                        "tags": ["pets"],
                        "summary": "新建宠物",
                        "requestBody": {
                            "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Pet" } } }
                        },
                        "responses": { "200": { "description": "ok" } }
                    }
                }
            },
            "components": {
                "schemas": {
                    "Pet": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "integer" },
                            "name": { "type": "string" },
                            "status": { "type": "string", "enum": ["available", "sold"] }
                        }
                    }
                }
            }
        });
        let spec_file = root.join("swagger.json");
        fs::write(&spec_file, serde_json::to_string(&spec).unwrap()).unwrap();

        let result = import_openapi_file(&root, &spec_file).unwrap();
        assert_eq!(result.count, 2);
        let folder = PathBuf::from(&result.folder);
        assert!(folder.join("__info.json").exists());

        // 按 tag 分组到 pets 子目录
        let pets = folder.join("pets");
        assert!(pets.exists());
        let get_api: ApiFile = serde_json::from_str(
            &fs::read_to_string(pets.join("GET _pets_{id}.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(get_api.method, "GET");
        assert_eq!(get_api.path, "/pets/{id}");
        assert_eq!(get_api.url, "https://api.example.com/v1/pets/{id}");
        assert_eq!(get_api.params.len(), 1);
        assert_eq!(get_api.query.len(), 1);
        assert_eq!(get_api.description, "按 ID 获取宠物");

        let post_api: ApiFile = serde_json::from_str(
            &fs::read_to_string(pets.join("POST _pets.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(post_api.body.mode, "json");
        assert!(post_api.body.raw.contains("\"name\""));

        // YAML 格式同样支持（.yaml / .yml）
        let yaml_content = serde_yaml::to_string(&spec).unwrap();
        let yaml_file = root.join("swagger.yaml");
        fs::write(&yaml_file, yaml_content).unwrap();
        let result2 = import_openapi_file(&root, &yaml_file).unwrap();
        assert_eq!(result2.count, 2);
        assert!(PathBuf::from(&result2.folder).join("pets").exists());

        let _ = fs::remove_dir_all(&root);
    }

    /// 使用 tests/data/apifox.json 真实文件验证 Apifox 项目导入
    #[test]
    fn test_import_apifox_file() {
        let root = std::env::temp_dir().join(format!("apimgr-apifox-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let file = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/data/apifox.json"
        ));
        let result = import_apifox_file(&root, &file).expect("apifox 导入失败");
        assert!(result.count > 0, "应至少导入 1 个接口，实际 {}", result.count);
        let folder = PathBuf::from(&result.folder);
        assert!(folder.join(INFO_FILE).exists());
        // 宠物商店示例：根集合下含「宠物」分组与 GET 接口
        let pets = folder.join("宠物");
        assert!(pets.exists(), "应生成「宠物」分组");
        // 直接读取「获取宠物」接口文件验证转换结果
        let api_file = pets.join("获取宠物.json");
        assert!(api_file.exists(), "应生成「获取宠物.json」");
        let api: ApiFile =
            serde_json::from_str(&fs::read_to_string(api_file).unwrap()).unwrap();
        assert_eq!(api.method, "GET");
        assert_eq!(api.path, "/pets/{id}");
        assert_eq!(api.params.len(), 1);
        assert_eq!(api.params[0].key, "id");
        // 同名分组合并：apiCollection 两个集合的「宠物」应合并，不生成「宠物 (2)」
        assert!(!folder.join("宠物 (2)").exists(), "不应生成重复分组「宠物 (2)」");
        assert!(
            folder.join("宠物").join("批量创建宠物.json").exists(),
            "第二个集合的接口应合并进「宠物」分组"
        );
        // webSocketCollection 的空分组占位（宠物/商店/用户 无接口）不应创建
        assert!(
            !folder.join("商店 (2)").exists(),
            "不应生成空分组「商店 (2)」"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// 使用 tests/data/apipost.json 真实文件验证 Apipost 项目导入
    #[test]
    fn test_import_apipost_file() {
        let root = std::env::temp_dir().join(format!("apimgr-apipost-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let file = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/data/apipost.json"
        ));
        let result = import_apipost_file(&root, &file).expect("apipost 导入失败");
        // 文件含 406 个 api + 15 个 graphql，folder 只建分组不计数
        assert_eq!(result.count, 421, "接口数应为 421，实际 {}", result.count);
        let folder = PathBuf::from(&result.folder);
        assert!(folder.join(INFO_FILE).exists());
        // Auth 分组（parent_id=0 的根分组）应存在
        assert!(folder.join("Auth").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_import_raml_file() {
        let root = std::env::temp_dir().join(format!("apimgr-raml-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let file = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/data/demo.raml"
        ));
        let result = import_raml_file(&root, &file).expect("raml 导入失败");
        // demo.raml 含 /users 的 get/post 两个接口
        assert_eq!(result.count, 2, "接口数应为 2，实际 {}", result.count);
        let folder = PathBuf::from(&result.folder);
        assert!(folder.join(INFO_FILE).exists());
        assert!(
            folder.join("GET _users.json").exists(),
            "GET _users.json 应存在"
        );
        let api: ApiFile = serde_json::from_str(
            &fs::read_to_string(folder.join("GET _users.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(api.method, "GET");
        assert_eq!(api.path, "/users");
        assert_eq!(api.url, "https://api.example.com/v1/users");
        // queryParameters page 应导入为查询参数
        assert_eq!(api.query.len(), 1);
        assert_eq!(api.query[0].key, "page");
        assert_eq!(api.query[0].value, "1");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_import_wadl_file() {
        let root = std::env::temp_dir().join(format!("apimgr-wadl-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let file = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/data/demo.wadl"
        ));
        let result = import_wadl_file(&root, &file).expect("wadl 导入失败");
        // demo.wadl 含 /users 的 GET/POST 两个接口
        assert_eq!(result.count, 2, "接口数应为 2，实际 {}", result.count);
        let folder = PathBuf::from(&result.folder);
        assert!(folder.join(INFO_FILE).exists());
        assert!(
            folder.join("GET _users.json").exists(),
            "GET _users.json 应存在"
        );
        let api: ApiFile = serde_json::from_str(
            &fs::read_to_string(folder.join("GET _users.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(api.method, "GET");
        assert_eq!(api.path, "/users");
        // query param page（style=query）应导入为查询参数
        assert_eq!(api.query.len(), 1);
        assert_eq!(api.query[0].key, "page");
        assert_eq!(api.query[0].value, "1");
        // 分组 INFO_FILE 应记录 base
        let info: InfoJson =
            serde_json::from_str(&fs::read_to_string(folder.join(INFO_FILE)).unwrap()).unwrap();
        assert_eq!(info.base_url.as_deref(), Some("https://api.example.com/v1"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_export_raml_wadl() {
        // 用导出的 RAML/WADL 再导回：round-trip 冒烟
        let root = std::env::temp_dir().join(format!("apimgr-rw-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        // 构造两个接口
        let make = |name: &str, method: &str, path: &str, is_ws: bool| ApiFile {
            uuid: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            method: method.into(),
            path: path.into(),
            url: format!("https://api.example.com{path}"),
            description: format!("{name} 描述"),
            headers: vec![KeyValue {
                key: "X-Token".into(),
                value: "abc".into(),
                enabled: true,
                is_file: false,
                description: "令牌".into(),
            }],
            query: vec![KeyValue {
                key: "page".into(),
                value: "1".into(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            params: vec![KeyValue {
                key: "id".into(),
                value: String::new(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            body: BodyData {
                mode: "json".into(),
                raw: "{\"a\":1}".into(),
                form: vec![],
                binary_path: String::new(),
            },
            mock: MockConfig::default(),
            examples: vec![],
            responses: vec![],
            doc_params: vec![],
            deprecated: false,
            protocol: if is_ws { "websocket".into() } else { "http".into() },
        };
        let apis = vec![
            (vec![], make("get", "GET", "/users", false)),
            (vec![], make("post", "POST", "/users", false)),
            (vec![], make("ws", "GET", "/ws/chat", true)),
        ];
        // RAML 导出：ws 应被过滤
        let raml = export::to_raml(&apis);
        assert!(raml.get("/users").is_some(), "RAML 应包含 /users");
        assert!(raml.get("/ws/chat").is_none(), "RAML 不应包含 WS 接口");
        assert_eq!(raml["/users"]["get"]["queryParameters"]["page"]["default"], "1");
        let yaml = serde_yaml::to_string(&raml).unwrap();
        assert!(yaml.contains("baseUri: https://api.example.com"));
        // WADL 导出
        let wadl = export::to_wadl(&apis);
        assert!(wadl.contains("<resource path=\"users\">"));
        assert!(wadl.contains("<method name=\"GET\">"));
        assert!(!wadl.contains("ws/chat"), "WADL 不应包含 WS 接口");
        // WADL 可再解析回接口
        let tmp = root.join("round.wadl");
        fs::write(&tmp, &wadl).unwrap();
        let re = import_wadl_file(&root, &tmp).expect("wadl round-trip 失败");
        assert_eq!(re.count, 2, "round-trip 接口数应为 2");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_import_har_file() {
        let root = std::env::temp_dir().join(format!("apimgr-har-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        // 内联迷你 HAR：两个 host、浏览器自动头应被过滤、json body、响应示例、urlencoded 表单
        let har = serde_json::json!({
            "log": {
                "version": "1.2",
                "creator": { "name": "t", "version": "1" },
                "pages": [{ "title": "示例站点" }],
                "entries": [
                    {
                        "request": {
                            "method": "GET",
                            "url": "https://api.example.com/users?page=2",
                            "headers": [
                                { "name": "Accept", "value": "*/*" },
                                { "name": "User-Agent", "value": "Mozilla/5.0" },
                                { "name": "X-Api-Key", "value": "secret123" },
                                { "name": "Cookie", "value": "sid=abc" }
                            ],
                            "queryString": [
                                { "name": "page", "value": "2" }
                            ],
                            "postData": null
                        },
                        "response": {
                            "status": 200,
                            "content": {
                                "mimeType": "application/json",
                                "text": "{\"list\":[]}"
                            }
                        }
                    },
                    {
                        "request": {
                            "method": "POST",
                            "url": "https://api.example.com/users",
                            "headers": [
                                { "name": "Content-Type", "value": "application/json" }
                            ],
                            "queryString": [],
                            "postData": {
                                "mimeType": "application/json",
                                "text": "{\"name\":\"张三\"}"
                            }
                        },
                        "response": {
                            "status": 201,
                            "content": {
                                "mimeType": "application/json",
                                "text": "{\"id\":1}"
                            }
                        }
                    },
                    {
                        "request": {
                            "method": "POST",
                            "url": "https://track.other.com/event",
                            "headers": [],
                            "queryString": [],
                            "postData": {
                                "mimeType": "application/x-www-form-urlencoded",
                                "text": "a=1&name=%E5%BC%A0%E4%B8%89"
                            }
                        },
                        "response": {
                            "status": 200,
                            "content": { "mimeType": "text/plain", "text": "ok" }
                        }
                    }
                ]
            }
        });
        let file = root.join("sample.har");
        fs::write(&file, serde_json::to_string_pretty(&har).unwrap()).unwrap();
        let result = import_har_file(&root, &file).expect("har 导入失败");
        assert_eq!(result.count, 3, "接口数应为 3，实际 {}", result.count);
        let folder = PathBuf::from(&result.folder);
        assert!(folder.join(INFO_FILE).exists());
        // 按 host 分小组
        let api_host = folder.join("api.example.com");
        let track_host = folder.join("track.other.com");
        assert!(api_host.is_dir(), "api.example.com 分组应存在");
        assert!(track_host.is_dir(), "track.other.com 分组应存在");
        // GET /users 的接口：query 参数、浏览器头被过滤、X-Api-Key 保留
        let get_file = api_host.join("GET _users.json");
        assert!(get_file.exists(), "GET _users.json 应存在");
        let api: ApiFile =
            serde_json::from_str(&fs::read_to_string(get_file).unwrap()).unwrap();
        assert_eq!(api.method, "GET");
        assert_eq!(api.path, "/users");
        assert_eq!(api.query.len(), 1);
        assert_eq!(api.query[0].key, "page");
        assert_eq!(api.query[0].value, "2");
        assert_eq!(api.headers.len(), 1, "浏览器自动头应被过滤");
        assert_eq!(api.headers[0].key, "x-api-key");
        assert_eq!(api.headers[0].value, "secret123");
        // 响应示例已存储
        assert_eq!(api.responses.len(), 1);
        assert_eq!(api.responses[0].status, 200);
        assert!(api.responses[0].body.contains("list"));
        // POST json body
        let post_file = api_host.join("POST _users.json");
        let post: ApiFile =
            serde_json::from_str(&fs::read_to_string(post_file).unwrap()).unwrap();
        assert_eq!(post.body.mode, "json");
        assert!(post.body.raw.contains("张三"));
        assert_eq!(post.responses[0].status, 201);
        // urlencoded 表单 → form 列表 + 解码
        let ev_file = track_host.join("POST _event.json");
        let ev: ApiFile = serde_json::from_str(&fs::read_to_string(ev_file).unwrap()).unwrap();
        assert_eq!(ev.body.mode, "form");
        assert_eq!(ev.body.form.len(), 2);
        assert_eq!(ev.body.form[0].key, "a");
        assert_eq!(ev.body.form[0].value, "1");
        assert_eq!(ev.body.form[1].value, "张三");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_import_yapi_swagger() {
        // YApi 的 swagger 数据导出（tests/data/yapi.json）应走 openapi 导入
        let root = std::env::temp_dir().join(format!("apimgr-yapi-s-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let file = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/data/yapi.json"
        ));
        let result = import_yapi_file(&root, &file).expect("yapi(swagger) 导入失败");
        // paths: /user/{uid} get、/user/add post、/order/list get
        assert_eq!(result.count, 3, "接口数应为 3，实际 {}", result.count);
        let folder = PathBuf::from(&result.folder);
        assert!(folder.join(INFO_FILE).exists());
        // 按 tag 分组的「用户模块」应存在，且含 GET _user_{uid}.json
        let um = folder.join("用户模块");
        assert!(um.is_dir(), "用户模块分组应存在");
        assert!(um.join("GET _user_{uid}.json").exists());
        let api: ApiFile = serde_json::from_str(
            &fs::read_to_string(um.join("GET _user_{uid}.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(api.method, "GET");
        assert_eq!(api.path, "/user/{uid}");
        // path 参数 uid + query 参数 withExtra
        assert!(api.params.iter().any(|p| p.key == "uid"));
        assert!(api.query.iter().any(|q| q.key == "withExtra"));
        // Authorization header
        assert!(api.headers.iter().any(|h| h.key.eq_ignore_ascii_case("Authorization")));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_import_yapi_native() {
        // YApi 原生导出树：分组/接口/表单/json body/WS
        let root = std::env::temp_dir().join(format!("apimgr-yapi-n-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let doc = serde_json::json!([
            {
                "name": "用户模块",
                "desc": "",
                "children": [
                    {
                        "name": "获取用户",
                        "api": {
                            "method": "GET",
                            "path": "/user/:uid",
                            "title": "获取用户",
                            "desc": "根据ID查询",
                            "req_query": [
                                { "name": "withExtra", "value": "", "desc": "是否扩展", "example": "true" }
                            ],
                            "req_headers": [
                                { "name": "X-Token", "value": "abc", "desc": "令牌" }
                            ],
                            "req_body_type": null,
                            "req_body_other": "",
                            "req_body_form": [],
                            "res_body_type": "json",
                            "res_body": "{\"code\":0}",
                            "protocol": "http"
                        }
                    },
                    {
                        "name": "新增用户",
                        "api": {
                            "method": "POST",
                            "path": "/user/add",
                            "title": "新增用户",
                            "desc": "",
                            "req_body_type": "json",
                            "req_body_other": "{\"name\":\"张三\"}",
                            "res_body_type": null,
                            "res_body": "",
                            "protocol": "http"
                        }
                    },
                    {
                        "name": "上传头像",
                        "api": {
                            "method": "POST",
                            "path": "/user/avatar",
                            "title": "上传头像",
                            "desc": "",
                            "req_body_type": "form",
                            "req_body_form": [
                                { "name": "file", "type": "file", "desc": "图片" },
                                { "name": "tag", "value": "avatar", "type": "text" }
                            ],
                            "res_body_type": null,
                            "res_body": "",
                            "protocol": "http"
                        }
                    },
                    {
                        "name": "消息推送",
                        "api": {
                            "method": "GET",
                            "path": "ws://example.com/chat",
                            "title": "消息推送",
                            "desc": "",
                            "req_body_type": null,
                            "req_body_other": "",
                            "res_body_type": null,
                            "res_body": "",
                            "protocol": "ws"
                        }
                    },
                    {
                        "name": "空分组",
                        "desc": "",
                        "children": []
                    }
                ]
            }
        ]);
        let file = root.join("yapi-native.json");
        fs::write(&file, serde_json::to_string_pretty(&doc).unwrap()).unwrap();
        let result = import_yapi_file(&root, &file).expect("yapi 原生导入失败");
        assert_eq!(result.count, 4, "接口数应为 4，实际 {}", result.count);
        let folder = PathBuf::from(&result.folder);
        let um = folder.join("用户模块");
        assert!(um.is_dir(), "用户模块分组应存在");
        assert!(!um.join("空分组").exists(), "空分组不应创建");
        // 获取用户：query/header/路径参数 :uid → {uid}
        let get: ApiFile = serde_json::from_str(
            &fs::read_to_string(um.join("获取用户.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(get.method, "GET");
        assert_eq!(get.path, "/user/{uid}");
        assert!(get.params.iter().any(|p| p.key == "uid"));
        assert!(get.query.iter().any(|q| q.key == "withExtra" && q.value == "true"));
        assert!(get.headers.iter().any(|h| h.key == "X-Token"));
        assert_eq!(get.responses[0].body, "{\"code\":0}");
        // 新增用户：json body 格式化
        let add: ApiFile = serde_json::from_str(
            &fs::read_to_string(um.join("新增用户.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(add.body.mode, "json");
        assert!(add.body.raw.contains("张三"));
        // 上传头像：form 文件字段
        let up: ApiFile = serde_json::from_str(
            &fs::read_to_string(um.join("上传头像.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(up.body.mode, "form");
        assert!(up.body.form.iter().any(|f| f.key == "file" && f.is_file));
        assert!(up.body.form.iter().any(|f| f.key == "tag" && f.value == "avatar"));
        // WS 接口
        let ws: ApiFile = serde_json::from_str(
            &fs::read_to_string(um.join("消息推送.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(ws.protocol, "websocket");
        assert!(ws.path.starts_with("ws://"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_export_yapi_roundtrip() {
        // 导出 YApi 原生格式后能再导入回来
        let root = std::env::temp_dir().join(format!("apimgr-yapi-e-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let make = |name: &str, method: &str, path: &str| ApiFile {
            uuid: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            method: method.into(),
            path: path.into(),
            url: path.into(),
            description: format!("{name} 描述"),
            headers: vec![KeyValue {
                key: "X-Token".into(),
                value: "abc".into(),
                enabled: true,
                is_file: false,
                description: "令牌".into(),
            }],
            query: vec![KeyValue {
                key: "page".into(),
                value: "1".into(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            params: vec![KeyValue {
                key: "id".into(),
                value: String::new(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            body: BodyData {
                mode: "json".into(),
                raw: "{\"a\":1}".into(),
                form: vec![],
                binary_path: String::new(),
            },
            mock: MockConfig::default(),
            examples: vec![],
            responses: vec![ResponseItem {
                id: "r1".into(),
                name: "HTTP 200".into(),
                status: 200,
                content_type: "application/json".into(),
                body: "{\"ok\":true}".into(),
            }],
            doc_params: vec![],
            deprecated: false,
            protocol: "http".into(),
        };
        let apis: Vec<(Vec<(String, bool)>, ApiFile)> = vec![
            (vec![("用户模块".to_string(), true)], make("获取用户", "GET", "/user/{id}")),
            (vec![], make("订单列表", "GET", "/order/list")),
        ];
let v = export::to_yapi(&apis);
        let arr = v.as_array().expect("yapi 导出应为数组");
        // 根级「订单列表」+ 分组「用户模块」
        assert_eq!(arr.len(), 2);
        let um = arr
            .iter()
            .find(|n| n.get("name").and_then(|x| x.as_str()) == Some("用户模块"))
            .expect("用户模块分组");
        let api_item = um["children"][0].clone();
        assert_eq!(api_item["api"]["method"], "GET");
        assert_eq!(api_item["api"]["path"], "/user/:id", "路径参数应为 :id 语法");
        assert_eq!(api_item["api"]["req_query"][0]["name"], "page");
        assert_eq!(api_item["api"]["res_body"], "{\"ok\":true}");
        // round-trip：导出 → 再导入
        let tmp = root.join("round.json");
        fs::write(&tmp, serde_json::to_string_pretty(&v).unwrap()).unwrap();
        let re = import_yapi_file(&root, &tmp).expect("yapi round-trip 失败");
        assert_eq!(re.count, 2, "round-trip 接口数应为 2");
        let folder = PathBuf::from(&re.folder);
        assert!(folder.join("用户模块").join("获取用户.json").exists());
        assert!(folder.join("订单列表.json").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_import_eolink_file() {
        let root = std::env::temp_dir().join(format!("apimgr-eolink-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let file = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/data/eolink.json"
        ));
        let result = import_eolink_file(&root, &file).expect("eolink 导入失败");
        assert_eq!(result.count, 1, "接口数应为 1，实际 {}", result.count);
        let folder = PathBuf::from(&result.folder);
        assert_eq!(folder.file_name().unwrap().to_string_lossy(), "订单管理服务");
        // 顶层组「订单模块」→ 子组「订单操作」→ 创建订单.json
        let om = folder.join("订单模块");
        assert!(om.is_dir(), "订单模块分组应存在");
        let op = om.join("订单操作");
        assert!(op.is_dir(), "订单操作分组应存在");
        let api: ApiFile = serde_json::from_str(
            &fs::read_to_string(op.join("创建订单.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(api.method, "POST");
        assert_eq!(api.path, "/order/{orderType}/create");
        // 路径参数 orderType（requestRestList 补了 example）
        assert!(api.params.iter().any(|p| p.key == "orderType" && p.value == "normal"));
        // 查询参数 channel
        assert!(api.query.iter().any(|q| q.key == "channel" && q.value == "app"));
        // 请求头 Authorization
        assert!(api.headers.iter().any(|h| h.key == "Authorization"));
        // json body 嵌套结构
        assert_eq!(api.body.mode, "json");
        assert!(api.body.raw.contains("userId"));
        assert!(api.body.raw.contains("receiverName"));
        // 描述合并 apiDesc + apiNote
        assert!(api.description.contains("批量下单"));
        assert!(api.description.contains("鉴权token"));
        // 2 个响应示例（200/400）
        assert_eq!(api.responses.len(), 2);
        assert!(api.responses.iter().any(|r| r.status == 200 && r.body.contains("orderId")));
        assert!(api.responses.iter().any(|r| r.status == 400));
        // INFO_FILE base_url 来自环境 host
        let info: InfoJson =
            serde_json::from_str(&fs::read_to_string(folder.join(INFO_FILE)).unwrap()).unwrap();
        assert!(info.base_url.as_deref().unwrap_or("").contains("api.local"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_import_insomnia_file() {
        let root = std::env::temp_dir().join(format!("apimgr-ins-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let file = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/data/Insomnia.yml"
        ));
        let result = import_insomnia_file(&root, &file).expect("insomnia 导入失败");
        assert_eq!(result.count, 1, "接口数应为 1，实际 {}", result.count);
        let folder = PathBuf::from(&result.folder);
        assert_eq!(folder.file_name().unwrap().to_string_lossy(), "Project API");
        let um = folder.join("用户模块");
        assert!(um.is_dir(), "用户模块分组应存在");
        let api: ApiFile = serde_json::from_str(
            &fs::read_to_string(um.join("创建用户 POST.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(api.method, "POST");
        // {{baseUrl}} 由集合级 environment 替换
        assert_eq!(api.path, "/user");
        assert_eq!(api.body.mode, "json");
        assert!(api.body.raw.contains("test"));
        // bearer token → Authorization 头
        assert!(api.headers.iter().any(|h| {
            h.key.eq_ignore_ascii_case("authorization") && h.value.contains("demo-token")
        }));
        // INFO_FILE base_url
        let info: InfoJson =
            serde_json::from_str(&fs::read_to_string(folder.join(INFO_FILE)).unwrap()).unwrap();
        assert_eq!(info.base_url.as_deref(), Some("https://api.example.com"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_export_eolink_insomnia_roundtrip() {
        let root = std::env::temp_dir().join(format!("apimgr-ei-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let make = |name: &str, method: &str, path: &str| ApiFile {
            uuid: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            method: method.into(),
            path: path.into(),
            url: path.into(),
            description: format!("{name} 描述"),
            headers: vec![KeyValue {
                key: "Authorization".into(),
                value: "Bearer tok123".into(),
                enabled: true,
                is_file: false,
                description: "鉴权".into(),
            }],
            query: vec![KeyValue {
                key: "page".into(),
                value: "1".into(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            params: vec![KeyValue {
                key: "orderType".into(),
                value: "normal".into(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            body: BodyData {
                mode: "json".into(),
                raw: "{\"userId\":1,\"addr\":{\"city\":\"赣州\"}}".into(),
                form: vec![],
                binary_path: String::new(),
            },
            mock: MockConfig::default(),
            examples: vec![],
            responses: vec![ResponseItem {
                id: "r1".into(),
                name: "HTTP 200".into(),
                status: 200,
                content_type: "application/json".into(),
                body: "{\"code\":0}".into(),
            }],
            doc_params: vec![],
            deprecated: false,
            protocol: "http".into(),
        };
        let apis: Vec<(Vec<(String, bool)>, ApiFile)> = vec![(
            vec![("订单模块".to_string(), true), ("订单操作".to_string(), true)],
            make("创建订单", "POST", "/order/{orderType}/create"),
        )];
        // Eolink 导出 → 再导入
        let ev = export::to_eolink(&apis);
        assert_eq!(ev["apiGroupList"][0]["groupName"], "订单模块");
        let eapi = &ev["apiGroupList"][0]["childGroupList"][0]["apiList"][0];
        assert_eq!(eapi["apiMethod"], "POST");
        assert_eq!(eapi["apiUri"], "/order/{orderType}/create");
        assert_eq!(eapi["requestInfo"]["requestRestList"][0]["key"], "orderType");
        assert_eq!(eapi["requestInfo"]["requestQueryList"][0]["key"], "page");
        // serde_json Map 键按字母序，用 find 断言
        let bl = eapi["requestInfo"]["requestBodyJsonList"]
            .as_array()
            .unwrap();
        assert!(bl.iter().any(|x| x["key"] == "addr"));
        let addr = bl.iter().find(|x| x["key"] == "addr").unwrap();
        assert_eq!(addr["children"][0]["key"], "city");
        assert!(bl.iter().any(|x| x["key"] == "userId"));
        assert_eq!(eapi["responseInfoList"][0]["responseCode"], 200);
        let etmp = root.join("eolink-out.json");
        fs::write(&etmp, serde_json::to_string_pretty(&ev).unwrap()).unwrap();
        let re = import_eolink_file(&root, &etmp).expect("eolink round-trip 失败");
        assert_eq!(re.count, 1, "eolink round-trip 接口数应为 1");
        // Insomnia 导出 → 再导入
        let iv = export::to_insomnia(&apis);
        assert_eq!(iv["type"], "collection.insomnia.rest/5.0");
        assert_eq!(iv["children"][0]["name"], "订单模块");
        let req = &iv["children"][0]["children"][0]["children"][0];
        assert_eq!(req["method"], "POST");
        assert!(req["url"].as_str().unwrap().contains("baseUrl"));
        assert_eq!(req["authentication"]["type"], "bearer");
        assert_eq!(req["authentication"]["token"], "tok123");
        assert_eq!(req["body"]["mimeType"], "application/json");
        let itmp = root.join("insomnia-out.yml");
        fs::write(&itmp, serde_yaml::to_string(&iv).unwrap()).unwrap();
        let ri = import_insomnia_file(&root, &itmp).expect("insomnia round-trip 失败");
        assert_eq!(ri.count, 1, "insomnia round-trip 接口数应为 1");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_import_jmeter_file() {
        let root = std::env::temp_dir().join(format!("apimgr-jmx-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let file = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/data/jmeter.jmx"
        ));
        let result = import_jmeter_file(&root, &file).expect("jmeter 导入失败");
        assert_eq!(result.count, 5, "接口数应为 5，实际 {}", result.count);
        let folder = PathBuf::from(&result.folder);
        assert_eq!(
            folder.file_name().unwrap().to_string_lossy(),
            "综合业务接口测试计划"
        );
        // ThreadGroup「业务线程组」→ 分组目录
        let tg = folder.join("业务线程组");
        assert!(tg.is_dir(), "业务线程组分组应存在");
        // 登录接口：POST + json body + 变量替换
        let login: ApiFile = serde_json::from_str(
            &fs::read_to_string(tg.join("1-登录获取token.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(login.method, "POST");
        assert_eq!(login.path, "/api/login");
        assert_eq!(login.body.mode, "json");
        assert!(login.body.raw.contains("username"));
        // HeaderManager 的 Content-Type 应用到接口
        assert!(login.headers.iter().any(|h| {
            h.key.eq_ignore_ascii_case("content-type") && h.value.contains("application/json")
        }));
        // GET 接口：path 中 query 拆出
        let info: ApiFile = serde_json::from_str(
            &fs::read_to_string(tg.join("2-获取用户信息.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(info.method, "GET");
        assert_eq!(info.path, "/api/user/info");
        assert!(info.query.iter().any(|q| q.key == "token"));
        // DELETE 接口
        let del: ApiFile = serde_json::from_str(
            &fs::read_to_string(tg.join("5-删除订单.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(del.method, "DELETE");
        assert_eq!(del.path, "/api/order/del");
        // INFO_FILE base_url 来自 host 变量
        let info_f: InfoJson =
            serde_json::from_str(&fs::read_to_string(folder.join(INFO_FILE)).unwrap()).unwrap();
        assert_eq!(info_f.base_url.as_deref(), Some("http://127.0.0.1:8080"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_export_jmeter_roundtrip() {
        let root = std::env::temp_dir().join(format!("apimgr-jmx-e-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let make = |name: &str, method: &str, path: &str| ApiFile {
            uuid: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            method: method.into(),
            path: path.into(),
            url: path.into(),
            description: String::new(),
            headers: vec![KeyValue {
                key: "X-Token".into(),
                value: "abc".into(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            query: vec![KeyValue {
                key: "page".into(),
                value: "2".into(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            params: vec![],
            body: BodyData {
                mode: "json".into(),
                raw: "{\"name\":\"测试\"}".into(),
                form: vec![],
                binary_path: String::new(),
            },
            mock: MockConfig::default(),
            examples: vec![],
            responses: vec![],
            doc_params: vec![],
            deprecated: false,
            protocol: "http".into(),
        };
        let apis: Vec<(Vec<(String, bool)>, ApiFile)> = vec![
            (vec![("用户模块".to_string(), true)], make("创建用户", "POST", "/user")),
            (vec![], make("订单列表", "GET", "/order/list")),
        ];
        let xml = export::to_jmeter(&apis);
        assert!(xml.contains("<jmeterTestPlan"));
        assert!(xml.contains("testname=\"用户模块\""));
        assert!(xml.contains("testname=\"API Manager\""));
        assert!(xml.contains("HTTPSampler.path\">/user"));
        assert!(xml.contains("HTTPSampler.method\">POST"));
        // query 拼进 path
        assert!(xml.contains("/order/list?page=2"));
        // HeaderManager 保留 X-Token
        assert!(xml.contains("Header.name\">X-Token"));
        // round-trip：导出 → 再导入
        let tmp = root.join("round.jmx");
        fs::write(&tmp, &xml).unwrap();
        let re = import_jmeter_file(&root, &tmp).expect("jmeter round-trip 失败");
        assert_eq!(re.count, 2, "jmeter round-trip 接口数应为 2");
        let folder = PathBuf::from(&re.folder);
        let created: ApiFile = serde_json::from_str(
            &fs::read_to_string(folder.join("用户模块").join("创建用户.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(created.body.mode, "json");
        assert!(created.body.raw.contains("测试"));
        assert!(created.headers.iter().any(|h| h.key == "X-Token"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_import_apidoc_files() {
        let root = std::env::temp_dir().join(format!("apimgr-apidoc-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let base = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/data"));
        let result = import_apidoc_files(&root, &base.join("api_project.json"), &base.join("api_data.json"))
            .expect("apidoc 导入失败");
        assert_eq!(result.count, 7, "接口数应为 7，实际 {}", result.count);
        let folder = PathBuf::from(&result.folder);
        assert_eq!(folder.file_name().unwrap().to_string_lossy(), "后端API接口文档");
        // INFO base_url = sampleUrl
        let info_f: InfoJson =
            serde_json::from_str(&fs::read_to_string(folder.join(INFO_FILE)).unwrap()).unwrap();
        assert_eq!(info_f.base_url.as_deref(), Some("http://127.0.0.1:8080/api"));
        // 分组：用户模块 / 订单模块
        let user_dir = folder.join("用户模块");
        assert!(user_dir.is_dir(), "用户模块分组应存在");
        // 登录接口：POST json body + 嵌套字段展开
        let login: ApiFile = serde_json::from_str(
            &fs::read_to_string(user_dir.join("用户登录.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(login.method, "POST");
        assert_eq!(login.path, "/api/user/login");
        assert_eq!(login.body.mode, "json");
        assert!(login.body.raw.contains("username"));
        assert!(login.body.raw.contains("password"));
        // 响应：successExamples → status 200，error.examples → 返回失败
        assert!(login.responses.iter().any(|r| r.status == 200 && r.name == "登录成功"));
        assert!(login.responses.iter().any(|r| r.status == 0 && r.name == "登录失败"));
        // docParams：body 字段 + resp_success 字段
        assert!(login.doc_params.iter().any(|d| d.source == "body" && d.key == "username"));
        assert!(login.doc_params.iter().any(|d| d.source == "resp_success" && d.key == "data.token"));
        // header 字段 → 请求头
        let info: ApiFile = serde_json::from_str(
            &fs::read_to_string(user_dir.join("获取当前登录用户信息.json")).unwrap(),
        )
        .unwrap();
        assert!(info.headers.iter().any(|h| h.key == "Authorization"));
        // 路径参数 :orderId → {orderId}
        let detail: ApiFile = serde_json::from_str(
            &fs::read_to_string(folder.join("订单模块").join("获取订单详情.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(detail.path, "/api/order/{orderId}");
        assert!(detail.params.iter().any(|p| p.key == "orderId"));
        // 创建订单：数组字段 goodsList 展开
        let create: ApiFile = serde_json::from_str(
            &fs::read_to_string(folder.join("订单模块").join("创建订单.json")).unwrap(),
        )
        .unwrap();
        assert!(create.body.raw.contains("goodsList"));
        assert!(create.body.raw.contains("goodsId"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_export_apidoc_roundtrip() {
        let root = std::env::temp_dir().join(format!("apimgr-apidoc-e-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let make = |name: &str, method: &str, path: &str| ApiFile {
            uuid: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            method: method.into(),
            path: path.into(),
            url: path.into(),
            description: String::new(),
            headers: vec![KeyValue {
                key: "Authorization".into(),
                value: String::new(),
                enabled: true,
                is_file: false,
                description: "Bearer token".into(),
            }],
            query: vec![KeyValue {
                key: "page".into(),
                value: "1".into(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            params: vec![KeyValue {
                key: "orderId".into(),
                value: String::new(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            body: BodyData {
                mode: "json".into(),
                raw: "{\"username\":\"zhangsan\",\"info\":{\"age\":18},\"tags\":[\"a\"]}".into(),
                form: vec![],
                binary_path: String::new(),
            },
            mock: MockConfig::default(),
            examples: vec![],
            responses: vec![ResponseItem {
                id: uuid::Uuid::new_v4().to_string(),
                name: "成功".into(),
                status: 200,
                content_type: "application/json".into(),
                body: "{\"code\":0}".into(),
            }],
            doc_params: vec![],
            deprecated: false,
            protocol: "http".into(),
        };
        let apis: Vec<(Vec<(String, bool)>, ApiFile)> = vec![
            (vec![("订单模块".to_string(), true)], make("订单详情", "GET", "/api/order/{orderId}")),
        ];
        let (proj, data) = export::to_apidoc(&apis);
        assert_eq!(proj["name"].as_str(), Some("订单模块"));
        let groups = data["groups"].as_array().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0]["name"].as_str(), Some("订单模块"));
        let apis_out = data["apis"].as_array().unwrap();
        assert_eq!(apis_out.len(), 1);
        let a = &apis_out[0];
        assert_eq!(a["group"].as_str(), Some("订单模块"));
        assert_eq!(a["url"].as_str(), Some("/api/order/:orderId"));
        // header 字段
        assert_eq!(
            a["header"]["fields"]["Header"][0]["field"].as_str(),
            Some("Authorization")
        );
        // query → Query 字段
        assert_eq!(a["parameter"]["fields"]["Query"][0]["field"].as_str(), Some("page"));
        // body 嵌套展开：username / info.age / tags[]
        let pf = a["parameter"]["fields"]["Parameter"].as_array().unwrap();
        let fields: Vec<&str> = pf.iter().map(|f| f["field"].as_str().unwrap()).collect();
        assert!(fields.contains(&"username"));
        assert!(fields.contains(&"info.age"));
        assert!(fields.contains(&"tags"));
        // successExamples
        assert_eq!(a["successExamples"][0]["content"].as_str(), Some("{\"code\":0}"));
        // round-trip：导出 → 写文件 → 再导入
        let dir = root.join("out");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("api_project.json"), serde_json::to_string_pretty(&proj).unwrap()).unwrap();
        fs::write(dir.join("api_data.json"), serde_json::to_string_pretty(&data).unwrap()).unwrap();
        let re = import_apidoc_files(&root, &dir.join("api_project.json"), &dir.join("api_data.json"))
            .expect("apidoc round-trip 失败");
        assert_eq!(re.count, 1, "apidoc round-trip 接口数应为 1");
        let folder = PathBuf::from(&re.folder);
        let created: ApiFile = serde_json::from_str(
            &fs::read_to_string(folder.join("订单模块").join("订单详情.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(created.path, "/api/order/{orderId}");
        assert_eq!(created.body.mode, "json");
        assert!(created.body.raw.contains("username"));
        assert!(created.body.raw.contains("info"));
        assert!(created.headers.iter().any(|h| h.key == "Authorization"));
        assert!(created.query.iter().any(|q| q.key == "page"));
        assert!(created.params.iter().any(|p| p.key == "orderId"));
        assert!(created.responses.iter().any(|r| r.status == 200));
        let _ = fs::remove_dir_all(&root);
    }

        #[test]
    fn test_history_roundtrip() {
        // 保存 -> 分页列表 -> 详情 -> 按天统计 全链路
        let root = std::env::temp_dir().join(format!("history-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let input = HistoryInput {
            method: "GET".into(),
            url: "http://127.0.0.1:8080/api/users".into(),
            api_uuid: "abc-123".into(),
            api_name: "用户列表".into(),
            req_headers: vec![("Content-Type".into(), "application/json".into())],
            req_body: Some("{\"a\":1}".into()),
            ok: true,
            status: 200,
            status_text: "OK".into(),
            resp_headers: vec![("X-Test".into(), "yes".into())],
            resp_body: "{\"hello\":\"world\"}".into(),
            time_ms: 12,
            size: 100,
            error: None,
        };
        let id = save_history_to(&root, input).unwrap();
        assert!(root.join(HISTORY_DIR).exists());

        // 列表分页
        let page = history_records_from(&root, 0, 100).unwrap();
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].id, id);
        assert_eq!(page[0].status, 200);
        assert_eq!(page[0].api_uuid, "abc-123");
        // offset 越界返回空
        assert!(history_records_from(&root, 5, 100).unwrap().is_empty());

        // 详情
        let detail = history_detail_from(&root, &id).unwrap();
        assert_eq!(detail.req_headers[0].0, "Content-Type");
        assert_eq!(detail.req_body.as_deref(), Some("{\"a\":1}"));
        assert_eq!(detail.resp_body, "{\"hello\":\"world\"}");
        // 不存在的 id
        assert!(history_detail_from(&root, "nope").is_err());

        // 按天统计
        let days = history_days_from(&root).unwrap();
        assert_eq!(days.len(), 1);
        assert_eq!(days[0].count, 1);

        // 清空
        fs::remove_dir_all(root.join(HISTORY_DIR)).unwrap();
        assert!(history_records_from(&root, 0, 100).unwrap().is_empty());
        assert!(history_days_from(&root).unwrap().is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_example_roundtrip() {
        // 保存（同名覆盖）-> 列表 -> 读取 -> 删除 全链路
        let root = std::env::temp_dir().join(format!("example-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let make = |name: &str, url: &str| ExampleFile {
            name: name.into(),
            time: 1700000000,
            method: "GET".into(),
            url: url.into(),
            req_headers: vec![("Accept".into(), "*/*".into())],
            req_path: vec![("id".into(), "42".into())],
            req_query: vec![("page".into(), "1".into())],
            req_body: None,
            status: 200,
            status_text: "OK".into(),
            resp_headers: vec![("X-Test".into(), "yes".into())],
            resp_body: "{\"ok\":true}".into(),
            time_ms: 8,
            size: 64,
            error: None,
        };

        // 名称哈希稳定：同名两次保存得到相同文件名（覆盖）
        let f1 = save_example_to(&root, "uuid-1", "登录成功", make("登录成功", "http://a/b")).unwrap();
        let f2 = save_example_to(&root, "uuid-1", "登录成功", make("登录成功", "http://a/b?x=2")).unwrap();
        assert_eq!(f1, f2);
        // 不同名 -> 不同文件
        let f3 = save_example_to(&root, "uuid-1", "查询列表", make("查询列表", "http://a/c")).unwrap();
        assert_ne!(f1, f3);
        // 不同接口 -> 不同目录
        let f4 = save_example_to(&root, "uuid-2", "登录成功", make("登录成功", "http://a/b")).unwrap();
        assert_eq!(f1, f4);

        let list = list_examples_from(&root, "uuid-1").unwrap();
        assert_eq!(list.len(), 2);
        // 最新在前
        assert_eq!(list[0].name, "查询列表");
        assert!(list.iter().all(|s| s.file.ends_with(".json")));

        // 读取详情
        let detail = read_example_file(&root, "uuid-1", &f3).unwrap();
        assert_eq!(detail.url, "http://a/c");
        assert_eq!(detail.resp_body, "{\"ok\":true}");
        assert_eq!(detail.req_path[0], ("id".to_string(), "42".to_string()));
        assert_eq!(detail.req_query[0], ("page".to_string(), "1".to_string()));

        // 空 uuid / 空名称报错
        assert!(save_example_to(&root, "", "x", make("x", "")).is_err());
        assert!(save_example_to(&root, "uuid-1", "   ", make("x", "")).is_err());

        // 防目录穿越
        assert!(example_path(&root, "uuid-1", "../evil.json").is_err());

        // 删除后列表为空
        fs::remove_file(root.join(EXAMPLES_DIR).join("uuid-1").join(&f3)).unwrap();
        assert_eq!(list_examples_from(&root, "uuid-1").unwrap().len(), 1);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_move_entry() {
        // 移动接口文件与目录到目标目录；重名时自动加序号；禁止移入自身子目录
        let root = std::env::temp_dir().join(format!("move-test-{}", std::process::id()));
        let a = root.join("a");
        let b = root.join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        fs::create_dir_all(&a.join("sub")).unwrap();
        fs::write(a.join("api.json"), "{}").unwrap();
        fs::write(a.join("sub").join("deep.json"), "{}").unwrap();

        // 接口移入 b
        let new_path = move_entry_inner(
            &root,
            &a.join("api.json").to_string_lossy(),
            &b.to_string_lossy(),
        )
        .unwrap();
        assert_eq!(new_path, b.join("api.json").to_string_lossy());
        assert!(b.join("api.json").exists());

        // 目录 sub 移入 b（含内部文件）
        let new_path = move_entry_inner(&root, &a.join("sub").to_string_lossy(), &b.to_string_lossy())
            .unwrap();
        assert!(b.join("sub").join("deep.json").exists());
        assert_eq!(new_path, b.join("sub").to_string_lossy());

        // 目录不能移入自身子目录
        let err = move_entry_inner(&root, &b.to_string_lossy(), &b.join("sub").to_string_lossy())
            .unwrap_err();
        assert!(err.contains("子目录"));

        let _ = fs::remove_dir_all(&root);
    }

    /// 复制接口：uuid 重新生成、名称追加「 副本」、同目录重名自动加序号
    #[test]
    fn copy_api_regenerates_uuid() {
        let root = std::env::temp_dir().join(format!("apim-copy-api-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let src = root.join("创建用户.json");
        let api = ApiFile {
            uuid: "old-uuid".into(),
            name: "创建用户".into(),
            method: "POST".into(),
            path: "/api/users".into(),
            url: String::new(),
            description: String::new(),
            headers: vec![],
            query: vec![],
            params: vec![],
            body: BodyData::default(),
            mock: MockConfig::default(),
            examples: vec![],
            responses: vec![],
            doc_params: vec![],
        deprecated: false,
        protocol: "http".into(),
        };
        write_pretty(&src, &api).unwrap();

        let dst = root.join("创建用户 副本.json");
        copy_api_file(&src, &dst).unwrap();
        let copied: ApiFile = serde_json::from_str(&fs::read_to_string(&dst).unwrap()).unwrap();
        assert_ne!(copied.uuid, "old-uuid");
        assert_eq!(copied.name, "创建用户 副本");
        assert_eq!(copied.method, "POST");
        assert_eq!(copied.path, "/api/users");
        let _ = fs::remove_dir_all(&root);
    }

    /// 复制分组：递归复制整棵树，每个接口 uuid 重新生成，分组 __info.json 名称追加「 副本」
    #[test]
    fn copy_dir_regenerates_all_uuids() {
        let root = std::env::temp_dir().join(format!("apim-copy-dir-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let g = root.join("用户管理");
        let sub = g.join("子分组");
        fs::create_dir_all(&sub).unwrap();
        fs::write(
            g.join(INFO_FILE),
            r#"{"name":"用户管理","description":""}"#,
        )
        .unwrap();
        let mk = |p: &std::path::Path, uuid: &str, name: &str| {
            let api = ApiFile {
                uuid: uuid.into(),
                name: name.into(),
                method: "GET".into(),
                path: "/x".into(),
                url: String::new(),
                description: String::new(),
                headers: vec![],
                query: vec![],
                params: vec![],
                body: BodyData::default(),
                mock: MockConfig::default(),
                examples: vec![],
                responses: vec![],
                doc_params: vec![],
        deprecated: false,
        protocol: "http".into(),
            };
            write_pretty(p, &api).unwrap();
        };
        mk(&g.join("接口A.json"), "uuid-a", "接口A");
        mk(&sub.join("接口B.json"), "uuid-b", "接口B");
        // 点目录不应被复制（.examples 与旧 uuid 绑定）
        fs::create_dir_all(g.join(".examples")).unwrap();
        fs::write(g.join(".examples").join("x.json"), "{}").unwrap();

        let dst = root.join("用户管理 副本");
        copy_dir_with_new_uuids(&g, &dst).unwrap();

        let a: ApiFile = serde_json::from_str(&fs::read_to_string(dst.join("接口A.json")).unwrap()).unwrap();
        assert_ne!(a.uuid, "uuid-a");
        assert_eq!(a.name, "接口A 副本");
        let b: ApiFile = serde_json::from_str(&fs::read_to_string(dst.join("子分组").join("接口B.json")).unwrap()).unwrap();
        assert_ne!(b.uuid, "uuid-b");
        assert!(!dst.join(".examples").exists());
        let info: Value = serde_json::from_str(&fs::read_to_string(dst.join(INFO_FILE)).unwrap()).unwrap();
        assert_eq!(info["name"], "用户管理 副本");
        let _ = fs::remove_dir_all(&root);
    }
}



    #[test]
    fn test_import_extra_formats() {
        let root = std::env::temp_dir().join(format!("apimgr-extra-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let base = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/data"));
        // (格式, 文件名, 期望接口数, 关键断言闭包)
        let cases: Vec<(&str, &str, usize)> = vec![
            ("apidog", "demo.apidog.json", 2),
            ("bruno", "bruno.json", 3),
            ("apizza", "apizza.json", 4),
            ("nei", "nei.json", 2),
            ("doclever", "DOClever.json", 2),
            ("io-docs", "io-docs.json", 8),
            ("easydoc", "easydoc.json", 3),
            ("docway", "docway.mjson", 3),
            ("hoppscotch", "Hoppscotch.json", 6),
            ("metersphere", "MeterSphere.json", 2),
        ];
        for (format, fname, expected) in cases {
            let sub = root.join(format);
            fs::create_dir_all(&sub).unwrap();
            let result = import_extra_files(&sub, &base.join(fname), format)
                .unwrap_or_else(|e| panic!("{format} 导入失败: {e}"));
            assert_eq!(result.count, expected, "{format} 接口数应为 {expected}，实际 {}", result.count);
            let folder = PathBuf::from(&result.folder);
            assert!(folder.join(INFO_FILE).is_file(), "{format} INFO 文件应存在");
            // 至少有一个接口文件
            let mut found = 0usize;
            fn walk_count(dir: &Path, found: &mut usize) {
                if let Ok(rd) = fs::read_dir(dir) {
                    for e in rd.flatten() {
                        let p = e.path();
                        if p.is_dir() {
                            walk_count(&p, found);
                        } else if p.extension().map(|x| x == "json").unwrap_or(false) && p.file_name().map(|n| n != INFO_FILE).unwrap_or(false) {
                            *found += 1;
                        }
                    }
                }
            }
            walk_count(&folder, &mut found);
            assert_eq!(found, expected, "{format} 磁盘接口文件数应为 {expected}");
            // 抽查读取第一个接口文件可解析
            if let Some(first) = std::fs::read_dir(&folder).unwrap().flatten().find(|e| {
                let p = e.path();
                p.is_file() && p.extension().map(|x| x == "json").unwrap_or(false)
                    && p.file_name().map(|n| n != INFO_FILE).unwrap_or(false)
            }) {
                let _: ApiFile = serde_json::from_str(&fs::read_to_string(first.path()).unwrap())
                    .unwrap_or_else(|er| panic!("{format} 接口文件解析失败: {er}"));
            }
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_export_extra_roundtrip() {
        let root = std::env::temp_dir().join(format!("apimgr-extra-e-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let mk = |name: &str, method: &str, path: &str, body_mode: &str| ApiFile {
            uuid: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            method: method.into(),
            path: path.into(),
            url: path.into(),
            description: "测试接口".into(),
            headers: vec![KeyValue {
                key: "Authorization".into(),
                value: "Bearer token".into(),
                enabled: true,
                is_file: false,
                description: "鉴权".into(),
            }],
            query: vec![KeyValue {
                key: "page".into(),
                value: "1".into(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            params: vec![KeyValue {
                key: "orderId".into(),
                value: String::new(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            body: if body_mode == "json" {
                BodyData {
                    mode: "json".into(),
                    raw: "{\"username\":\"zhangsan\",\"age\":18}".into(),
                    form: vec![],
                    binary_path: String::new(),
                }
            } else {
                BodyData {
                    mode: "form".into(),
                    raw: String::new(),
                    form: vec![KeyValue {
                        key: "file".into(),
                        value: "a.txt".into(),
                        enabled: true,
                        is_file: true,
                        description: String::new(),
                    }],
                    binary_path: String::new(),
                }
            },
            mock: MockConfig::default(),
            examples: vec![],
            responses: vec![ResponseItem {
                id: uuid::Uuid::new_v4().to_string(),
                name: "成功".into(),
                status: 200,
                content_type: "application/json".into(),
                body: "{\"code\":0}".into(),
            }],
            doc_params: vec![],
            deprecated: false,
            protocol: "http".into(),
        };
        let apis: Vec<(Vec<(String, bool)>, ApiFile)> = vec![
            (vec![("用户模块".to_string(), true)], mk("用户登录", "POST", "/api/user/login", "json")),
            (vec![("订单模块".to_string(), true)], mk("上传文件", "POST", "/api/order/upload", "form")),
        ];
        let formats = [
            "apidog", "bruno", "apizza", "nei", "doclever", "io-docs", "easydoc", "docway", "hoppscotch", "metersphere",
        ];
        for format in formats {
            let (content, fname, ext) = export::export_extra(&apis, format)
                .unwrap_or_else(|e| panic!("{format} 导出失败: {e}"));
            assert!(!content.is_empty(), "{format} 导出内容不应为空");
            let out_dir = root.join(format);
            fs::create_dir_all(&out_dir).unwrap();
            let out_file = out_dir.join(format!("{fname}.{ext}"));
            fs::write(&out_file, &content).unwrap();
            let re = import_extra_files(&root, &out_file, format)
                .unwrap_or_else(|e| panic!("{format} round-trip 导入失败: {e}"));
            assert_eq!(re.count, 2, "{format} round-trip 接口数应为 2，实际 {}", re.count);
            let folder = PathBuf::from(&re.folder);
            let mut created: Vec<ApiFile> = Vec::new();
            fn walk_read(dir: &Path, out: &mut Vec<ApiFile>) {
                if let Ok(rd) = fs::read_dir(dir) {
                    for e in rd.flatten() {
                        let p = e.path();
                        if p.is_dir() {
                            walk_read(&p, out);
                        } else if p.extension().map(|x| x == "json").unwrap_or(false) && p.file_name().map(|n| n != INFO_FILE).unwrap_or(false) {
                            if let Ok(v) = serde_json::from_str::<ApiFile>(&fs::read_to_string(&p).unwrap()) {
                                out.push(v);
                            }
                        }
                    }
                }
            }
            walk_read(&folder, &mut created);
            assert_eq!(created.len(), 2, "{format} round-trip 磁盘接口数应为 2");
            let login = created.iter().find(|a| a.path.contains("/api/user/login")).expect(&format!("{format} 应含登录接口"));
            assert_eq!(login.method, "POST", "{format} 登录接口 method");
            assert!(login.headers.iter().any(|h| h.key == "Authorization"), "{format} 登录接口 header 保留");
            if format != "io-docs" && format != "docway" {
                // io-docs/docway 无 query/body 区分，参数全部归入 body
                assert!(login.query.iter().any(|q| q.key == "page"), "{format} 登录接口 query 保留");
            }
            let upload = created.iter().find(|a| a.path.contains("/api/order/upload")).expect(&format!("{format} 应含上传接口"));
            if format != "io-docs" && format != "docway" && format != "metersphere" {
                assert_eq!(upload.body.mode, "form", "{format} 上传接口 body mode");
                assert!(!upload.body.form.is_empty(), "{format} 上传接口 form 字段保留");
            }
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_import_rap2() {
        let root = std::env::temp_dir().join(format!("apimgr-rap2-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let base = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/data"));
        // 项目格式
        let r = import_rap2_files(&root, &base.join("rap2-project.json")).expect("rap2 项目导入失败");
        assert_eq!(r.count, 6, "项目格式接口数应为 6，实际 {}", r.count);
        let folder = PathBuf::from(&r.folder);
        // 三个分组目录
        for mod_name in ["用户管理", "商品管理", "订单管理"] {
            assert!(folder.join(sanitize_filename(mod_name)).is_dir(), "缺少分组 {mod_name}");
        }
        // 用户管理分组下：获取用户列表 GET /api/user/list，响应含 code/msg/data
        let ulist = folder.join(sanitize_filename("用户管理")).join(sanitize_filename("获取用户列表.json"));
        let api: ApiFile = serde_json::from_str(&fs::read_to_string(&ulist).unwrap()).unwrap();
        assert_eq!(api.method, "GET");
        assert_eq!(api.path, "/api/user/list");
        assert!(api.headers.iter().any(|h| h.key == "Authorization"), "Authorization 应识别为 header");
        assert!(api.query.iter().any(|q| q.key == "page"), "page 应为 query");
        assert!(api.responses.iter().any(|r| r.body.contains("\"code\"") && r.body.contains("\"data\"")), "响应示例应含 code/data");
        // DELETE 接口 path 参数
        let del = folder.join(sanitize_filename("用户管理")).join(sanitize_filename("删除用户.json"));
        let api: ApiFile = serde_json::from_str(&fs::read_to_string(&del).unwrap()).unwrap();
        assert_eq!(api.method, "DELETE");
        assert!(api.path.contains("{userId}"), "删除用户 path 应保留 {{userId}}，实际 {}", api.path);
        assert!(api.params.iter().any(|p| p.key == "userId"), "userId 应为 path 参数");
        // 订单管理 POST /api/order body json
        let ord = folder.join(sanitize_filename("订单管理")).join(sanitize_filename("创建订单.json"));
        let api: ApiFile = serde_json::from_str(&fs::read_to_string(&ord).unwrap()).unwrap();
        assert_eq!(api.method, "POST");
        assert_eq!(api.path, "/api/order");
        assert_eq!(api.body.mode, "json", "订单接口应有 json body");
        assert!(api.body.raw.contains("receiverInfo") && api.body.raw.contains("goodsItems"), "body 应含嵌套 receiverInfo/goodsItems");
        // 单接口格式
        let r2 = import_rap2_files(&root, &base.join("rap2-single.json")).expect("rap2 单接口导入失败");
        assert_eq!(r2.count, 1, "单接口格式接口数应为 1");
        let folder2 = PathBuf::from(&r2.folder);
        let single = fs::read_dir(&folder2).unwrap().flatten()
            .find(|e| e.path().extension().map(|x| x == "json").unwrap_or(false) && e.file_name() != INFO_FILE)
            .unwrap();
        let api: ApiFile = serde_json::from_str(&fs::read_to_string(single.path()).unwrap()).unwrap();
        assert_eq!(api.method, "PUT");
        assert!(api.path.contains("{orderId}"), "单接口 path 应含 {{orderId}}，实际 {}", api.path);
        assert!(api.params.iter().any(|p| p.key == "orderId"), "orderId 应为 path 参数");
        assert_eq!(api.body.mode, "json", "单接口应有 json body（receiver/goodsList）");
        assert!(api.body.raw.contains("receiverName"), "body 应含 receiverName 嵌套");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_export_rap2_roundtrip() {
        let root = std::env::temp_dir().join(format!("apimgr-rap2-e-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let mk = |name: &str, method: &str, path: &str| ApiFile {
            uuid: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            method: method.into(),
            path: path.into(),
            url: path.into(),
            description: "测试".into(),
            headers: vec![KeyValue { key: "Authorization".into(), value: "Bearer x".into(), enabled: true, is_file: false, description: String::new() }],
            query: vec![KeyValue { key: "page".into(), value: "1".into(), enabled: true, is_file: false, description: String::new() }],
            params: vec![KeyValue { key: "orderId".into(), value: String::new(), enabled: true, is_file: false, description: String::new() }],
            body: BodyData {
                mode: "json".into(),
                raw: "{\"username\":\"zhangsan\",\"info\":{\"age\":18}}".into(),
                form: vec![],
                binary_path: String::new(),
            },
            mock: MockConfig::default(),
            examples: vec![],
            responses: vec![ResponseItem {
                id: uuid::Uuid::new_v4().to_string(),
                name: "成功".into(),
                status: 200,
                content_type: "application/json".into(),
                body: "{\"code\":0,\"data\":{\"total\":5}}".into(),
            }],
            doc_params: vec![],
            deprecated: false,
            protocol: "http".into(),
        };
        let apis: Vec<(Vec<(String, bool)>, ApiFile)> = vec![
            (vec![("用户模块".to_string(), true)], mk("登录", "POST", "/api/login")),
            (vec![("订单模块".to_string(), true)], mk("删除订单", "DELETE", "/api/order/{orderId}")),
        ];
        // 项目格式闭环
        let proj = export::to_rap2_project(&apis);
        let file = root.join("rap2-project.json");
        fs::write(&file, serde_json::to_string_pretty(&proj).unwrap()).unwrap();
        let re = import_rap2_files(&root, &file).expect("rap2 项目 round-trip 导入失败");
        assert_eq!(re.count, 2);
        let folder = PathBuf::from(&re.folder);
        let mut apis2: Vec<ApiFile> = Vec::new();
        for dir in fs::read_dir(&folder).unwrap().flatten() {
            if dir.path().is_dir() {
                for f in fs::read_dir(dir.path()).unwrap().flatten() {
                    if f.path().extension().map(|x| x == "json").unwrap_or(false) && f.file_name() != INFO_FILE {
                        apis2.push(serde_json::from_str(&fs::read_to_string(f.path()).unwrap()).unwrap());
                    }
                }
            }
        }
        assert_eq!(apis2.len(), 2);
        let login = apis2.iter().find(|a| a.path == "/api/login").unwrap();
        assert_eq!(login.method, "POST");
        assert!(login.headers.iter().any(|h| h.key == "Authorization"), "round-trip header 保留");
        assert!(login.query.iter().any(|q| q.key == "page"), "round-trip query 保留");
        assert!(login.body.raw.contains("info"), "round-trip 嵌套 body 保留");
        assert!(login.responses.iter().any(|r| r.body.contains("total")), "round-trip 响应保留");
        let del = apis2.iter().find(|a| a.path.contains("/api/order/")).unwrap();
        assert!(del.params.iter().any(|p| p.key == "orderId"), "round-trip path 参数保留");
        let _ = fs::remove_dir_all(&root);
    }
