mod mock;
mod markdown;
mod export;
mod objects;
mod history;
mod import;
mod update;
mod request;
mod tray;
mod demo;

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
            crate::demo::create_demo,
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
#[path = "lib_test.rs"]
mod tests;
