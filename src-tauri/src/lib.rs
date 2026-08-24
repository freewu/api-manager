mod mock;
mod markdown;
mod export;
mod objects;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager, State};

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

    Ok(())
}

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
}

#[tauri::command]
fn import_postman(
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
fn import_postman_file(root: &Path, file: &Path) -> Result<PostmanImportResult, String> {
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
    import_postman_items(&folder, &items)?;
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
fn import_postman_items(dir: &Path, items: &[Value]) -> Result<(), String> {
    for item in items {
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("未命名")
            .to_string();
        if item.get("request").is_some() {
            let api = postman_request_to_api(&name, &item["request"])?;
            let file_base = sanitize_filename(&name);
            let file_base = if file_base.is_empty() {
                "未命名接口".to_string()
            } else {
                file_base
            };
            let file_path = unique_path(dir, &file_base, ".json");
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
            import_postman_items(&sub_dir, sub)?;
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
        protocol: "http".into(),
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
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiImportResult {
    folder: String,
    count: usize,
}

/// 接口 Markdown 文档（供前端预览）
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownDoc {
    name: String,
    md: String,
    html: String,
}

/// 从接口文件路径推导分组名（父目录名；接口直接在工作区根目录下时为空）
fn group_of(path: &str, root: &str) -> String {
    let parent = Path::new(path).parent().unwrap_or(Path::new(""));
    let norm = |p: &str| p.trim_end_matches(['/', '\\']).trim_end_matches('/').to_string();
    if norm(&parent.to_string_lossy()) == norm(root) {
        return String::new();
    }
    parent
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// 将任意 Markdown 文本渲染为 HTML 片段（用于接口描述预览）
#[tauri::command]
fn render_markdown(text: String) -> String {
    markdown::md_to_html(&text)
}

/// 渲染接口的 Markdown 文档（含 HTML 预览版）
#[tauri::command]
fn render_api_markdown(state: State<'_, WorkspaceState>, path: String) -> Result<MarkdownDoc, String> {
    let root = workspace_root(&state)?;
    let group = group_of(&path, &root.to_string_lossy());
    let api = read_api(path)?;
    let md = markdown::render(&api, &group, false);
    let html = markdown::md_to_html(&md);
    Ok(MarkdownDoc { name: api.name, md, html })
}

/// 生成分组（含其下全部子分组/接口）的单个 Markdown：返回 (分组名, markdown)
fn group_markdown_doc(root: &Path, path: &str) -> Result<(String, String), String> {
    let apis = export::collect_apis(root, &[path.to_string()])?;
    if apis.is_empty() {
        return Err("所选内容中没有接口".into());
    }
    let name = read_info_file(Path::new(path))
        .name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| {
            Path::new(path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        });
    let md = export::markdown_single_file(&name, &apis);
    Ok((name, md))
}

/// 渲染分组（含其下全部子分组/接口）为单个 Markdown 文档
#[tauri::command]
fn render_group_markdown(state: State<'_, WorkspaceState>, path: String) -> Result<MarkdownDoc, String> {
    let root = workspace_root(&state)?;
    let (name, md) = group_markdown_doc(&root, &path)?;
    let html = markdown::md_to_html(&md);
    Ok(MarkdownDoc { name, md, html })
}

/// 导出接口 Markdown / HTML：弹出目录选择框，写入 <接口名>.md 或 <接口名>.html
#[tauri::command]
fn export_api_markdown(
    app: AppHandle,
    state: State<'_, WorkspaceState>,
    path: String,
    format: String,
    nav: String,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let root = workspace_root(&state)?;
    // 分组（目录）走分组 Markdown；接口文件走单接口 Markdown
    let (name, md) = if Path::new(&path).is_dir() {
        group_markdown_doc(&root, &path)?
    } else {
        let group = group_of(&path, &root.to_string_lossy());
        let api = read_api(path)?;
        (api.name.clone(), markdown::render(&api, &group, false))
    };
    let fmt = if format.eq_ignore_ascii_case("html") { "html" } else { "md" };
    let picked = app
        .dialog()
        .file()
        .set_title("选择保存目录")
        .blocking_pick_folder();
    let Some(dir) = picked else {
        return Ok(None);
    };
    let dir = dir.into_path().map_err(|e| e.to_string())?;
    let base = sanitize_filename(&name);
    let base = if base.trim().is_empty() {
        "未命名文档".to_string()
    } else {
        base
    };
    let target = unique_path(&dir, &base, &format!(".{fmt}"));
    let content = if fmt == "html" {
        markdown::wrap_html(&name, &md, &nav)
    } else {
        md
    };
    fs::write(&target, content).map_err(|e| format!("写入失败: {e}"))?;
    Ok(Some(target.to_string_lossy().to_string()))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownImportResult {
    folder: String,
    count: usize,
}

/// 导出选中接口/分组为 Postman / OpenAPI / Docsify 格式：弹窗选择保存位置并写入
#[tauri::command]
fn export_selection(
    app: AppHandle,
    state: State<'_, WorkspaceState>,
    paths: Vec<String>,
    format: String,
    nav: String,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let root = workspace_root(&state)?;
    let apis = export::collect_apis(&root, &paths)?;
    if apis.is_empty() {
        return Err("所选内容中没有接口".into());
    }
    match format.as_str() {
        "postman" => {
            let v = export::to_postman(&apis);
            let content = serde_json::to_string_pretty(&v).map_err(|e| format!("序列化失败: {e}"))?;
            let picked = app
                .dialog()
                .file()
                .set_title("导出 Postman Collection")
                .set_file_name("api-collection.postman_collection.json")
                .add_filter("Postman Collection", &["json"])
                .blocking_save_file();
            let Some(p) = picked else {
                return Ok(None);
            };
            let path = p.into_path().map_err(|e| e.to_string())?;
            fs::write(&path, content).map_err(|e| format!("写入失败: {e}"))?;
            Ok(Some(path.to_string_lossy().to_string()))
        }
        "openapi" => {
            let ws_name = read_info_file(&root).name.unwrap_or_default();
            let v = export::to_openapi(&ws_name, &apis);
            let content = serde_json::to_string_pretty(&v).map_err(|e| format!("序列化失败: {e}"))?;
            let picked = app
                .dialog()
                .file()
                .set_title("导出 OpenAPI 规范")
                .set_file_name("openapi.json")
                .add_filter("OpenAPI", &["json"])
                .blocking_save_file();
            let Some(p) = picked else {
                return Ok(None);
            };
            let path = p.into_path().map_err(|e| e.to_string())?;
            fs::write(&path, content).map_err(|e| format!("写入失败: {e}"))?;
            Ok(Some(path.to_string_lossy().to_string()))
        }
        "apifox" => {
            let v = export::to_apifox(&apis);
            let content = serde_json::to_string_pretty(&v).map_err(|e| format!("序列化失败: {e}"))?;
            let picked = app
                .dialog()
                .file()
                .set_title("导出 Apifox 项目")
                .set_file_name("apifox-project.json")
                .add_filter("Apifox 项目", &["json"])
                .blocking_save_file();
            let Some(p) = picked else {
                return Ok(None);
            };
            let path = p.into_path().map_err(|e| e.to_string())?;
            fs::write(&path, content).map_err(|e| format!("写入失败: {e}"))?;
            Ok(Some(path.to_string_lossy().to_string()))
        }
        "apipost" => {
            let v = export::to_apipost(&apis);
            let content = serde_json::to_string_pretty(&v).map_err(|e| format!("序列化失败: {e}"))?;
            let picked = app
                .dialog()
                .file()
                .set_title("导出 Apipost 项目")
                .set_file_name("apipost-project.json")
                .add_filter("Apipost 项目", &["json"])
                .blocking_save_file();
            let Some(p) = picked else {
                return Ok(None);
            };
            let path = p.into_path().map_err(|e| e.to_string())?;
            fs::write(&path, content).map_err(|e| format!("写入失败: {e}"))?;
            Ok(Some(path.to_string_lossy().to_string()))
        }
        "raml" => {
            let v = export::to_raml(&apis);
            let content =
                serde_yaml::to_string(&v).map_err(|e| format!("序列化失败: {e}"))?;
            let picked = app
                .dialog()
                .file()
                .set_title("导出 RAML")
                .set_file_name("api.raml")
                .add_filter("RAML", &["raml"])
                .blocking_save_file();
            let Some(p) = picked else {
                return Ok(None);
            };
            let path = p.into_path().map_err(|e| e.to_string())?;
            fs::write(&path, content).map_err(|e| format!("写入失败: {e}"))?;
            Ok(Some(path.to_string_lossy().to_string()))
        }
        "wadl" => {
            let content = export::to_wadl(&apis);
            let picked = app
                .dialog()
                .file()
                .set_title("导出 WADL")
                .set_file_name("api.wadl")
                .add_filter("WADL", &["wadl"])
                .blocking_save_file();
            let Some(p) = picked else {
                return Ok(None);
            };
            let path = p.into_path().map_err(|e| e.to_string())?;
            fs::write(&path, content).map_err(|e| format!("写入失败: {e}"))?;
            Ok(Some(path.to_string_lossy().to_string()))
        }
        "yapi" => {
            let v = export::to_yapi(&apis);
            let content = serde_json::to_string_pretty(&v).map_err(|e| format!("序列化失败: {e}"))?;
            let picked = app
                .dialog()
                .file()
                .set_title("导出 YApi")
                .set_file_name("yapi-project.json")
                .add_filter("YApi 项目", &["json"])
                .blocking_save_file();
            let Some(p) = picked else {
                return Ok(None);
            };
            let path = p.into_path().map_err(|e| e.to_string())?;
            fs::write(&path, content).map_err(|e| format!("写入失败: {e}"))?;
            Ok(Some(path.to_string_lossy().to_string()))
        }
        "eolink" => {
            let v = export::to_eolink(&apis);
            let content = serde_json::to_string_pretty(&v).map_err(|e| format!("序列化失败: {e}"))?;
            let picked = app
                .dialog()
                .file()
                .set_title("导出 Eolink")
                .set_file_name("eolink-project.json")
                .add_filter("Eolink 项目", &["json"])
                .blocking_save_file();
            let Some(p) = picked else {
                return Ok(None);
            };
            let path = p.into_path().map_err(|e| e.to_string())?;
            fs::write(&path, content).map_err(|e| format!("写入失败: {e}"))?;
            Ok(Some(path.to_string_lossy().to_string()))
        }
        "insomnia" => {
            let v = export::to_insomnia(&apis);
            let content = serde_yaml::to_string(&v).map_err(|e| format!("序列化失败: {e}"))?;
            let picked = app
                .dialog()
                .file()
                .set_title("导出 Insomnia")
                .set_file_name("insomnia-collection.yml")
                .add_filter("Insomnia", &["yml", "yaml"])
                .blocking_save_file();
            let Some(p) = picked else {
                return Ok(None);
            };
            let path = p.into_path().map_err(|e| e.to_string())?;
            fs::write(&path, content).map_err(|e| format!("写入失败: {e}"))?;
            Ok(Some(path.to_string_lossy().to_string()))
        }
        "jmeter" => {
            let content = export::to_jmeter(&apis);
            let picked = app
                .dialog()
                .file()
                .set_title("导出 JMeter")
                .set_file_name("api-test.jmx")
                .add_filter("JMeter 测试计划", &["jmx"])
                .blocking_save_file();
            let Some(p) = picked else {
                return Ok(None);
            };
            let path = p.into_path().map_err(|e| e.to_string())?;
            fs::write(&path, content).map_err(|e| format!("写入失败: {e}"))?;
            Ok(Some(path.to_string_lossy().to_string()))
        }
        "apidoc" => {
            let (proj, data) = export::to_apidoc(&apis);
            let picked = app
                .dialog()
                .file()
                .set_title("导出 apiDoc")
                .set_file_name("api_project.json")
                .add_filter("apiDoc", &["json"])
                .blocking_save_file();
            let Some(p) = picked else {
                return Ok(None);
            };
            let path = p.into_path().map_err(|e| e.to_string())?;
            let dir = path.parent().unwrap_or(Path::new("."));
            let proj_json = serde_json::to_string_pretty(&proj).map_err(|e| format!("序列化失败: {e}"))?;
            let data_json = serde_json::to_string_pretty(&data).map_err(|e| format!("序列化失败: {e}"))?;
            fs::write(&path, proj_json).map_err(|e| format!("写入失败: {e}"))?;
            let data_path = dir.join("api_data.json");
            fs::write(&data_path, data_json).map_err(|e| format!("写入失败: {e}"))?;
            Ok(Some(path.to_string_lossy().to_string()))
        }
        "apidog" | "bruno" | "apizza" | "nei" | "doclever" | "io-docs" | "easydoc" | "docway" | "hoppscotch" | "metersphere" | "rap2-project" | "rap2-single" => {
            let (content, fname, ext) = export::export_extra(&apis, &format)?;
            let title = match format.as_str() {
                "apidog" => "导出 apiDog",
                "bruno" => "导出 Bruno",
                "apizza" => "导出 Apizza",
                "nei" => "导出 NEI",
                "doclever" => "导出 DOClever",
                "io-docs" => "导出 IO-Docs",
                "easydoc" => "导出 EasyDoc",
                "docway" => "导出 DocWay",
                "hoppscotch" => "导出 Hoppscotch",
                "rap2-project" => "导出 RAP2 项目",
                _ => "导出 RAP2 单接口",
            };
            let picked = app
                .dialog()
                .file()
                .set_title(title)
                .set_file_name(format!("{fname}.{ext}"))
                .add_filter(title, &["json", "mjson"])
                .blocking_save_file();
            let Some(p) = picked else {
                return Ok(None);
            };
            let path = p.into_path().map_err(|e| e.to_string())?;
            fs::write(&path, content).map_err(|e| format!("写入失败: {e}"))?;
            Ok(Some(path.to_string_lossy().to_string()))
        }
        "docsify" => {
            let picked = app
                .dialog()
                .file()
                .set_title("选择 Docsify 文档目录")
                .blocking_pick_folder();
            let Some(dir) = picked else {
                return Ok(None);
            };
            let dir = dir.into_path().map_err(|e| e.to_string())?;
            let files = export::docsify_files(&apis);
            for (rel, content) in &files {
                let target = dir.join(rel);
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
                }
                fs::write(&target, content).map_err(|e| format!("写入失败: {e}"))?;
            }
            Ok(Some(dir.to_string_lossy().to_string()))
        }
        "markdown" | "html" => {
            // 单个 Markdown 文件（html 由该 Markdown 渲染生成）：含全部选中接口
            let title = read_info_file(&root).name.unwrap_or_default();
            let title = if title.trim().is_empty() {
                "接口文档".to_string()
            } else {
                title.trim().to_string()
            };
            let md = export::markdown_single_file(&title, &apis);
            let is_html = format == "html";
            let picked = app
                .dialog()
                .file()
                .set_title(if is_html {
                    "导出 HTML 文档"
                } else {
                    "导出 Markdown 文档"
                })
                .set_file_name(if is_html {
                    "api-docs.html"
                } else {
                    "接口文档.md"
                })
                .add_filter(
                    if is_html { "HTML" } else { "Markdown" },
                    if is_html { &["html"] } else { &["md"] },
                )
                .blocking_save_file();
            let Some(p) = picked else {
                return Ok(None);
            };
            let path = p.into_path().map_err(|e| e.to_string())?;
            let content = if is_html {
                markdown::wrap_html(&title, &md, &nav)
            } else {
                md
            };
            fs::write(&path, content).map_err(|e| format!("写入失败: {e}"))?;
            Ok(Some(path.to_string_lossy().to_string()))
        }
        _ => Err(format!("不支持的导出格式: {format}")),
    }
}

/// 导入 Markdown 接口文档：弹窗选 .md 文件，在工作区根新建分组并逐个保存接口
#[tauri::command]
fn import_markdown(
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
        for mut api in parsed.apis {
            api.uuid = uuid::Uuid::new_v4().to_string();
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
    for mut api in parsed.apis {
        api.uuid = uuid::Uuid::new_v4().to_string();
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
    }))
}

// ==================== Apifox / Apipost 导入 ====================

/// 导入 Apifox 项目（apifox-project.json）：弹窗选文件，工作区根新建同名分组
#[tauri::command]
fn import_apifox(
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
fn import_apifox_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
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
    // HTTP 接口：apiCollection 为数组（可能多个集合）或对象 {items:[...]}
    match json.get("apiCollection") {
        Some(Value::Array(arr)) => {
            for c in arr {
                count += import_apifox_items(&folder, c)?;
            }
        }
        Some(obj) => {
            if let Some(items) = obj.get("items").and_then(|v| v.as_array()) {
                count += import_apifox_items_arr(&folder, items)?;
            }
        }
        _ => {}
    }
    // WebSocket 接口：webSocketCollection（api 无 method，path 即 ws url，消息体在 requestBody.message）
    if let Some(arr) = json.get("webSocketCollection").and_then(|v| v.as_array()) {
        for c in arr {
            count += import_apifox_items(&folder, c)?;
        }
    }
    Ok(OpenApiImportResult {
        folder: folder.to_string_lossy().to_string(),
        count,
    })
}

/// 单个 Apifox 集合：取 items 递归导入
fn import_apifox_items(dir: &Path, collection: &Value) -> Result<usize, String> {
    let items = collection
        .get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    import_apifox_items_arr(dir, &items)
}

/// Apifox items 递归：带 api 的为接口，带 items 的为分组
fn import_apifox_items_arr(dir: &Path, items: &[Value]) -> Result<usize, String> {
    let mut count = 0usize;
    for item in items {
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("未命名")
            .to_string();
        if let Some(api_obj) = item.get("api") {
            let api = apifox_api_to_api(&name, api_obj)?;
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
            count += import_apifox_items_arr(&sub_dir, sub)?;
        }
    }
    Ok(count)
}

/// 将 Apifox api 对象转换为 ApiFile（WebSocket 接口无 method，path 即地址）
fn apifox_api_to_api(name: &str, api_obj: &Value) -> Result<ApiFile, String> {
    let is_ws = api_obj.get("method").is_none() && api_obj.get("path").and_then(|v| v.as_str()).map_or(true, |p| p.contains("ws://") || p.contains("wss://"));
    let method = if is_ws {
        "WS".to_string()
    } else {
        api_obj
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_uppercase()
    };
    let path = api_obj
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
        protocol: if is_ws { "websocket".into() } else { "http".into() },
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
fn import_apipost(
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
fn import_apipost_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
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
    for r in roots {
        count += import_apipost_node(&folder, r, &by_id)?;
    }
    Ok(OpenApiImportResult {
        folder: folder.to_string_lossy().to_string(),
        count,
    })
}

/// Apipost 节点递归：folder 建分组，api/graphql 写接口文件
fn import_apipost_node(
    dir: &Path,
    node: &Value,
    by_id: &HashMap<String, &Value>,
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
                count += import_apipost_node(&sub_dir, c, by_id)?;
            }
            Ok(count)
        }
        _ => {
            let api = apipost_request_to_api(&name, node)?;
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
    let is_ws = protocol.contains("websocket") || protocol.contains("ws");
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
        protocol: if is_ws { "websocket".into() } else { "http".into() },
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
fn import_raml(
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
fn import_raml_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
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
    // 顶层 key：以 / 开头的为资源路径，其余为元数据（title/version/baseUri/mediaType/types/...）
    if let Some(obj) = json.as_object() {
        for (key, val) in obj {
            if !key.starts_with('/') {
                continue;
            }
            count += raml_resource_to_apis(&folder, key, val, &base_url)?;
        }
    }
    Ok(OpenApiImportResult {
        folder: folder.to_string_lossy().to_string(),
        count,
    })
}

/// RAML 资源节点 → 接口文件（路径 key 为资源，值为方法对象或嵌套资源）
fn raml_resource_to_apis(
    dir: &Path,
    path: &str,
    node: &Value,
    base_url: &str,
) -> Result<usize, String> {
    let mut count = 0usize;
    let Some(obj) = node.as_object() else {
        return Ok(0);
    };
    // 子资源：key 不以 HTTP 方法开头且值为对象（含 / 前缀的路径）
    for (key, val) in obj {
        if key.starts_with('/') {
            let joined = format!("{}{}", path.trim_end_matches('/'), key);
            count += raml_resource_to_apis(dir, &joined, val, base_url)?;
            continue;
        }
        let method = key.to_uppercase();
        if !matches!(
            method.as_str(),
            "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "HEAD" | "OPTIONS"
        ) {
            continue;
        }
        count += raml_method_to_api(dir, &method, path, val, base_url)?;
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
) -> Result<usize, String> {
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
    Ok(1)
}

/// 导入 WADL 文件（.wadl）：弹窗选文件，工作区根新建同名分组
#[tauri::command]
fn import_wadl(
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
fn import_wadl_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
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
    for res in root_el.descendants().filter(|n| n.is_element() && n.has_tag_name("resources")) {
        for child in res.children().filter(|n| n.is_element() && n.has_tag_name("resource")) {
            count += wadl_resource_to_apis(&folder, "", child)?;
        }
    }
    Ok(OpenApiImportResult {
        folder: folder.to_string_lossy().to_string(),
        count,
    })
}

/// WADL resource 递归：子 resource 拼接 path，method 写接口文件
fn wadl_resource_to_apis(dir: &Path, parent_path: &str, res: roxmltree::Node) -> Result<usize, String> {
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
            count += wadl_resource_to_apis(dir, &path, child)?;
        } else if child.has_tag_name("method") {
            count += wadl_method_to_api(dir, &path, child)?;
        }
    }
    Ok(count)
}

/// WADL method 元素 → ApiFile
fn wadl_method_to_api(dir: &Path, path: &str, method_el: roxmltree::Node) -> Result<usize, String> {
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
fn import_har(
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
fn import_har_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
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
            count += har_entry_to_api(&sub, e)?;
        }
    }
    Ok(OpenApiImportResult {
        folder: folder.to_string_lossy().to_string(),
        count,
    })
}

/// HAR entry → ApiFile：method/url/headers/queryString/postData，响应存为返回示例
fn har_entry_to_api(dir: &Path, entry: &Value) -> Result<usize, String> {
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
    Ok(1)
}

// ==================== YApi 导入 ====================

/// 导入 YApi 导出文件（.json）：自动识别 Swagger（复用 openapi 导入）与 YApi 原生树格式
#[tauri::command]
fn import_yapi(
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
fn import_yapi_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
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
    for g in arr {
        count += yapi_node_to_apis(&folder, g)?;
    }
    Ok(OpenApiImportResult {
        folder: folder.to_string_lossy().to_string(),
        count,
    })
}

/// YApi 节点递归：有 children 建分组，有 api 写接口文件
fn yapi_node_to_apis(dir: &Path, node: &Value) -> Result<usize, String> {
    let name = node
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("未命名")
        .to_string();
    if let Some(api) = node.get("api") {
        return yapi_api_to_api(dir, &name, api);
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
        count += yapi_node_to_apis(&sub_dir, c)?;
    }
    Ok(count)
}

/// YApi api 对象 → ApiFile
fn yapi_api_to_api(dir: &Path, title: &str, api: &Value) -> Result<usize, String> {
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
    if let Some(folders) = v.get("folders").and_then(|x| x.as_array()) {
        for f in folders {
            let fname = str_field(f, "name");
            if fname.is_empty() {
                continue;
            }
            let sub = mk_group_dir(&folder, &fname, &str_field(f, "description"))?;
            if let Some(apis) = f.get("apis").and_then(|x| x.as_array()) {
                for a in apis {
                    count += apidog_api_to_api(&sub, a)?;
                }
            }
        }
    }
    Ok(OpenApiImportResult { folder: folder.to_string_lossy().to_string(), count })
}

fn apidog_api_to_api(dir: &Path, a: &Value) -> Result<usize, String> {
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
    if let Some(folders) = v.get("folders").and_then(|x| x.as_array()) {
        for f in folders {
            let fname = str_field(&f.get("info").cloned().unwrap_or(Value::Null), "name");
            if fname.is_empty() {
                continue;
            }
            let sub = mk_group_dir(&folder, &fname, "")?;
            count += bruno_walk(&sub, f, &vars)?;
        }
    }
    Ok(OpenApiImportResult { folder: folder.to_string_lossy().to_string(), count })
}

/// 递归处理 bruno 分组（requests + 嵌套 folders）
fn bruno_walk(dir: &Path, f: &Value, vars: &HashMap<String, String>) -> Result<usize, String> {
    let mut count = 0usize;
    if let Some(reqs) = f.get("requests").and_then(|x| x.as_array()) {
        for r in reqs {
            count += bruno_req_to_api(dir, r, vars)?;
        }
    }
    if let Some(subs) = f.get("folders").and_then(|x| x.as_array()) {
        for sf in subs {
            let fname = str_field(&sf.get("info").cloned().unwrap_or(Value::Null), "name");
            if fname.is_empty() {
                continue;
            }
            let sub = mk_group_dir(dir, &fname, "")?;
            count += bruno_walk(&sub, sf, vars)?;
        }
    }
    Ok(count)
}

fn bruno_req_to_api(dir: &Path, r: &Value, vars: &HashMap<String, String>) -> Result<usize, String> {
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
    if let Some(folders) = v.get("folders").and_then(|x| x.as_array()) {
        count += apizza_walk(&folder, folders, &vars)?;
    }
    Ok(OpenApiImportResult { folder: folder.to_string_lossy().to_string(), count })
}

fn apizza_walk(dir: &Path, folders: &[Value], vars: &HashMap<String, String>) -> Result<usize, String> {
    let mut count = 0usize;
    for f in folders {
        let fname = str_field(f, "folderName");
        if fname.is_empty() {
            continue;
        }
        let sub = mk_group_dir(dir, &fname, &str_field(f, "folderDesc"))?;
        if let Some(apis) = f.get("apis").and_then(|x| x.as_array()) {
            for a in apis {
                count += apizza_api_to_api(&sub, a, vars)?;
            }
        }
        if let Some(ch) = f.get("children").and_then(|x| x.as_array()) {
            count += apizza_walk(&sub, ch, vars)?;
        }
    }
    Ok(count)
}

fn apizza_api_to_api(dir: &Path, a: &Value, vars: &HashMap<String, String>) -> Result<usize, String> {
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
    if let Some(ifs) = v.get("interfaces").and_then(|x| x.as_array()) {
        for it in ifs {
            let gid = it.get("group").and_then(|x| x.as_i64()).unwrap_or(0);
            let dir = group_dirs.get(&gid).cloned().unwrap_or_else(|| folder.clone());
            count += nei_api_to_api(&dir, it, &datatypes)?;
        }
    }
    Ok(OpenApiImportResult { folder: folder.to_string_lossy().to_string(), count })
}

fn nei_api_to_api(dir: &Path, it: &Value, datatypes: &HashMap<i64, &Value>) -> Result<usize, String> {
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
    count += doclever_walk(&folder, arr)?;
    Ok(OpenApiImportResult { folder: folder.to_string_lossy().to_string(), count })
}

fn doclever_walk(dir: &Path, items: &[Value]) -> Result<usize, String> {
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
                count += doclever_walk(&sub, ch)?;
            }
        } else {
            count += doclever_api_to_api(dir, it)?;
        }
    }
    Ok(count)
}

fn doclever_api_to_api(dir: &Path, a: &Value) -> Result<usize, String> {
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
    if let Some(res) = v.get("resources").and_then(|x| x.as_object()) {
        for (rname, rv) in res {
            if rname.is_empty() {
                continue;
            }
            let sub = mk_group_dir(&folder, rname, &str_field(rv, "description"))?;
            if let Some(ms) = rv.get("methods").and_then(|x| x.as_object()) {
                for (_, mv) in ms {
                    count += io_docs_api_to_api(&sub, mv)?;
                }
            }
        }
    }
    Ok(OpenApiImportResult { folder: folder.to_string_lossy().to_string(), count })
}

fn io_docs_api_to_api(dir: &Path, a: &Value) -> Result<usize, String> {
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
    if let Some(apis) = data.get("api_list").and_then(|x| x.as_array()) {
        for a in apis {
            let cid = a.get("catalog_id").and_then(|x| x.as_i64()).unwrap_or(0);
            let dir = cat_dirs.get(&cid).cloned().unwrap_or_else(|| folder.clone());
            count += easydoc_api_to_api(&dir, a)?;
        }
    }
    Ok(OpenApiImportResult { folder: folder.to_string_lossy().to_string(), count })
}

fn easydoc_api_to_api(dir: &Path, a: &Value) -> Result<usize, String> {
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
                        count += docway_api_to_api(&sub, c)?;
                    } else {
                        let cname = str_field(c, "name");
                        if !cname.is_empty() {
                            let sub2 = mk_group_dir(&sub, &cname, "")?;
                            if let Some(ch2) = c.get("children").and_then(|x| x.as_array()) {
                                for c2 in ch2 {
                                    count += docway_api_to_api(&sub2, c2)?;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(OpenApiImportResult { folder: folder.to_string_lossy().to_string(), count })
}

fn docway_api_to_api(dir: &Path, a: &Value) -> Result<usize, String> {
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
    if let Some(folders) = v.get("folders").and_then(|x| x.as_array()) {
        count += hoppscotch_walk(&folder, folders)?;
    }
    if let Some(reqs) = v.get("requests").and_then(|x| x.as_array()) {
        for r in reqs {
            count += hoppscotch_req_to_api(&folder, r)?;
        }
    }
    Ok(OpenApiImportResult { folder: folder.to_string_lossy().to_string(), count })
}

fn hoppscotch_walk(dir: &Path, folders: &[Value]) -> Result<usize, String> {
    let mut count = 0usize;
    for f in folders {
        let fname = str_field(f, "name");
        if fname.is_empty() {
            continue;
        }
        let sub = mk_group_dir(dir, &fname, &str_field(f, "description"))?;
        if let Some(reqs) = f.get("requests").and_then(|x| x.as_array()) {
            for r in reqs {
                count += hoppscotch_req_to_api(&sub, r)?;
            }
        }
        if let Some(subs) = f.get("folders").and_then(|x| x.as_array()) {
            count += hoppscotch_walk(&sub, subs)?;
        }
    }
    Ok(count)
}

fn hoppscotch_req_to_api(dir: &Path, r: &Value) -> Result<usize, String> {
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
    if let Some(apis) = v.get("data").and_then(|x| x.as_array()) {
        for a in apis {
            let mid = str_field(a, "moduleId");
            let dir = node_dirs.get(&mid).cloned().unwrap_or_else(|| folder.clone());
            count += metersphere_api_to_api(&dir, a)?;
        }
    }
    Ok(OpenApiImportResult { folder: folder.to_string_lossy().to_string(), count })
}

fn metersphere_api_to_api(dir: &Path, a: &Value) -> Result<usize, String> {
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
    let mut order = 0i32;
    let modules = data.get("modules").and_then(|x| x.as_array()).cloned().unwrap_or_default();
    for m in modules {
        let mname = str_field(&m, "name");
        let dir = mk_group_dir(&folder, &if mname.is_empty() { "未分组".to_string() } else { mname.clone() }, &str_field(&m, "description"))?;
        let interfaces = m.get("interfaces").and_then(|x| x.as_array()).cloned().unwrap_or_default();
        for it in interfaces {
            order += 1;
            let api = rap2_interface_to_api(&it);
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
    })
}

/// 单接口格式：data 直接是接口
fn import_rap2_single(root: &Path, data: &Value) -> Result<OpenApiImportResult, String> {
    let folder = unique_path(root, "RAP2 导入", "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    let api = rap2_interface_to_api(data);
    let fname = sanitize_filename(&api.name);
    write_pretty(&folder.join(format!("{fname}.json")), &api)?;
    Ok(OpenApiImportResult {
        folder: folder.to_string_lossy().to_string(),
        count: 1,
    })
}

/// 自动识别：data.modules 存在 → 项目格式，否则单接口
fn import_rap2_files(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
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

fn import_extra_files(root: &Path, file: &Path, format: &str) -> Result<OpenApiImportResult, String> {
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
fn import_extra(
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
fn import_apidoc(
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

fn import_apidoc_files(
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
    if let Some(apis) = data.get("apis").and_then(|v| v.as_array()) {
        for a in apis {
            let gname = a.get("group").and_then(|v| v.as_str()).unwrap_or("");
            let dir = group_dirs.get(gname).unwrap_or(&folder);
            count += apidoc_api_to_api(dir, a)?;
        }
    }
    Ok(OpenApiImportResult {
        folder: folder.to_string_lossy().to_string(),
        count,
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
fn apidoc_api_to_api(dir: &Path, a: &Value) -> Result<usize, String> {
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
fn import_jmeter(
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

fn import_jmeter_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
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
    // 递归处理所有 hashTree（TestPlan 级 sampler 罕见；ThreadGroup 为分组）
    let mut pending_headers: Vec<KeyValue> = Vec::new();
    let mut pending_group: Option<String> = None;
    count += jmeter_walk_hash_tree(
        &root_el,
        &folder,
        &vars,
        &mut pending_headers,
        &mut pending_group,
    )?;
    Ok(OpenApiImportResult {
        folder: folder.to_string_lossy().to_string(),
        count,
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
) -> Result<usize, String> {
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
                count += jmeter_sampler_to_api(child, dir, vars, pending_headers)?;
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
                    )?;
                } else {
                    count += jmeter_walk_hash_tree(
                        &child,
                        dir,
                        vars,
                        pending_headers,
                        pending_group,
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
) -> Result<usize, String> {
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
    Ok(1)
}

// ==================== Eolink 导入 ====================

/// 导入 Eolink 导出文件（.json）
#[tauri::command]
fn import_eolink(
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

fn import_eolink_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
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
    if let Some(groups) = json.get("apiGroupList").and_then(|v| v.as_array()) {
        for g in groups {
            count += eolink_group_to_apis(&folder, g)?;
        }
    }
    Ok(OpenApiImportResult {
        folder: folder.to_string_lossy().to_string(),
        count,
    })
}

/// Eolink 分组递归：本组建目录，apiList 写入本组，childGroupList 递归子目录
fn eolink_group_to_apis(dir: &Path, group: &Value) -> Result<usize, String> {
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
            count += eolink_api_to_api(&sub_dir, a)?;
        }
    }
    if let Some(children) = group.get("childGroupList").and_then(|v| v.as_array()) {
        for c in children {
            count += eolink_group_to_apis(&sub_dir, c)?;
        }
    }
    Ok(count)
}

/// Eolink API 对象 → ApiFile
fn eolink_api_to_api(dir: &Path, api: &Value) -> Result<usize, String> {
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
fn import_insomnia(
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

fn import_insomnia_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
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
    let coll_env = doc
        .get("environment")
        .cloned()
        .unwrap_or(Value::Null);
    if let Some(children) = doc.get("children").and_then(|v| v.as_array()) {
        for c in children {
            count += insomnia_node_to_apis(&folder, c, &coll_env)?;
        }
    }
    Ok(OpenApiImportResult {
        folder: folder.to_string_lossy().to_string(),
        count,
    })
}

/// Insomnia 节点递归：有 url/method 的是请求，否则是文件夹
fn insomnia_node_to_apis(dir: &Path, node: &Value, coll_env: &Value) -> Result<usize, String> {
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
        return insomnia_request_to_api(dir, &name, node, &url, &method, coll_env);
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
        count += insomnia_node_to_apis(&sub_dir, c, coll_env)?;
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
) -> Result<usize, String> {
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
    Ok(1)
}

#[tauri::command]
fn import_openapi(
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
fn import_openapi_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
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
                let api = openapi_op_to_api(method, path_str, op, &shared_params, &base_url, &defs)?;
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
                write_pretty(&unique_path(&target, &file_base, ".json"), &api)?;
                count += 1;
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

// ==================== 检查更新（GitHub Releases） ====================

/// GitHub 仓库与发布页地址
const RELEASES_PAGE: &str = "https://github.com/freewu/api-manager/releases";
const LATEST_RELEASE_API: &str = "https://api.github.com/repos/freewu/api-manager/releases/latest";

/// 更新检查结果
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    /// 最新版本号（去掉 v 前缀，如 "0.2.0"）
    pub latest: String,
    /// 当前应用版本号
    pub current: String,
    /// 是否发现更新（latest > current）
    pub has_update: bool,
    /// 最新版本发布页地址
    pub url: String,
}

/// 解析版本号 "v0.1.5" / "0.1.5-beta" -> 数字段 [0, 1, 5]；忽略非数字部分
fn parse_version(v: &str) -> Vec<u32> {
    v.trim()
        .trim_start_matches(['v', 'V'])
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<u32>().ok())
        .collect()
}

/// 比较两个版本号，a 大于 b 返回 true（数值逐段比较，段数多的更大）
fn version_gt(a: &str, b: &str) -> bool {
    let pa = parse_version(a);
    let pb = parse_version(b);
    for (x, y) in pa.iter().zip(pb.iter()) {
        if x != y {
            return x > y;
        }
    }
    pa.len() > pb.len()
}

/// 异步访问 GitHub Releases API，获取最新版本号并判断是否有更新
async fn fetch_latest_release() -> Result<UpdateInfo, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("api-manager/update-check")
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    let resp = client
        .get(LATEST_RELEASE_API)
        .send()
        .await
        .map_err(|e| format!("访问 GitHub Releases 失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "GitHub Releases 接口返回 {}",
            resp.status().as_u16()
        ));
    }
    let json: Value = resp
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {e}"))?;
    let tag = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .trim_start_matches(['v', 'V'])
        .to_string();
    let current = env!("CARGO_PKG_VERSION").to_string();
    let has_update = !tag.is_empty() && version_gt(&tag, &current);
    let url = json
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or(RELEASES_PAGE)
        .to_string();
    Ok(UpdateInfo {
        latest: tag,
        current,
        has_update,
        url,
    })
}

/// 前端手动触发检查更新
#[tauri::command]
async fn check_update() -> Result<UpdateInfo, String> {
    fetch_latest_release().await
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

// ==================== 请求测试 ====================

fn decode_body(bytes: &[u8], headers: &[(String, String)]) -> String {
    let charset = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .and_then(|(_, v)| v.split(';').nth(1))
        .and_then(|s| s.trim().strip_prefix("charset="))
        .map(|s| s.trim().trim_matches('"').to_lowercase());
    match charset.as_deref() {
        Some("gbk") | Some("gb2312") | Some("gb18030") | Some("cp936") | Some("gbk-2312") => {
            let (cow, _, _) = encoding_rs::GBK.decode(bytes);
            cow.to_string()
        }
        Some("big5") => {
            let (cow, _, _) = encoding_rs::BIG5.decode(bytes);
            cow.to_string()
        }
        Some("latin1") | Some("iso-8859-1") | Some("windows-1252") => {
            let (cow, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
            cow.to_string()
        }
        Some("utf-8") | Some("utf8") | None => String::from_utf8_lossy(bytes).to_string(),
        Some(other) => {
            let (cow, _, _) = encoding_rs::Encoding::for_label(other.as_bytes())
                .map(|enc| enc.decode(bytes))
                .unwrap_or_else(|| encoding_rs::UTF_8.decode(bytes));
            cow.to_string()
        }
    }
}

#[tauri::command]
fn pick_file(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app.dialog().file().blocking_pick_file();
    Ok(picked
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().to_string()))
}

#[tauri::command]
async fn send_request(req: HttpRequestData) -> Result<HttpResult, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(req.timeout_ms.max(1000)))
        .redirect(reqwest::redirect::Policy::limited(10))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("创建客户端失败: {e}"))?;

    let method =
        reqwest::Method::from_bytes(req.method.as_bytes()).map_err(|e| format!("非法请求方法: {e}"))?;
    let mut rb = client.request(method, &req.url);

    for h in req.headers.iter().filter(|h| h.enabled && !h.key.trim().is_empty()) {
        rb = rb.header(h.key.trim(), h.value.trim());
    }
    // 表单（含文件字段）：multipart/form-data；否则按原始 body 发送
    if let Some(form) = &req.form {
        if !form.is_empty() {
            let mut mp = reqwest::multipart::Form::new();
            for f in form.iter().filter(|f| f.enabled && !f.key.trim().is_empty()) {
                if f.is_file {
                    let path = f.value.trim();
                    if path.is_empty() {
                        return Err(format!("表单文件字段 [{}] 未选择文件", f.key.trim()));
                    }
                    let bytes = tokio::fs::read(path)
                        .await
                        .map_err(|e| format!("读取文件失败 [{}]: {e}", path))?;
                    let fname = std::path::Path::new(path)
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "file".to_string());
                    mp = mp.part(
                        f.key.trim().to_string(),
                        reqwest::multipart::Part::bytes(bytes).file_name(fname),
                    );
                } else {
                    mp = mp.text(f.key.trim().to_string(), f.value.clone());
                }
            }
            rb = rb.multipart(mp);
        }
    } else if let Some(path) = &req.body_file {
        // 二进制模式：读取本地文件字节作为请求体
        if !path.trim().is_empty() {
            let bytes = tokio::fs::read(path.trim())
                .await
                .map_err(|e| format!("读取文件失败 [{path}]: {e}"))?;
            let has_ct = req
                .headers
                .iter()
                .any(|h| h.enabled && h.key.eq_ignore_ascii_case("content-type"));
            if !has_ct {
                rb = rb.header("Content-Type", "application/octet-stream");
            }
            rb = rb.body(bytes);
        }
    } else if let Some(body) = &req.body {
        if !body.is_empty() {
            let has_ct = req
                .headers
                .iter()
                .any(|h| h.enabled && h.key.eq_ignore_ascii_case("content-type"));
            if !has_ct {
                rb = rb.header("Content-Type", "application/json; charset=utf-8");
            }
            rb = rb.body(body.clone());
        }
    }

    let start = Instant::now();
    match rb.send().await {
        Ok(resp) => {
            let status = resp.status();
            let headers: Vec<(String, String)> = resp
                .headers()
                .iter()
                .map(|(k, v)| {
                    (
                        k.as_str().to_string(),
                        v.to_str().unwrap_or("").to_string(),
                    )
                })
                .collect();
            let bytes = resp.bytes().await.unwrap_or_default();
            let time_ms = start.elapsed().as_millis() as u64;
            let text = decode_body(&bytes, &headers);
            Ok(HttpResult {
                ok: true,
                status: status.as_u16(),
                status_text: status
                    .canonical_reason()
                    .unwrap_or("")
                    .to_string(),
                headers,
                body: text,
                time_ms,
                size: bytes.len(),
                url: req.url.clone(),
                error: None,
            })
        }
        Err(e) => {
            let time_ms = start.elapsed().as_millis() as u64;
            let err = if e.is_timeout() {
                "请求超时".to_string()
            } else if e.is_connect() {
                format!("连接失败: {e}")
            } else if e.is_builder() {
                // URL 无法解析为合法地址（缺少 http(s):// 前缀，或包含未替换的 {{变量}}）
                format!("URL 格式不正确: {}（请检查是否缺少 http:// 前缀或存在未替换的 {{变量}}）", req.url)
            } else {
                e.to_string()
            };
            Ok(HttpResult {
                ok: false,
                status: 0,
                status_text: String::new(),
                headers: vec![],
                body: String::new(),
                time_ms,
                size: 0,
                url: req.url.clone(),
                error: Some(err),
            })
        }
    }
}

// ==================== 请求历史 ====================

pub const HISTORY_DIR: &str = ".history";

/// 单条历史记录文件内容（.history/<日期>/<时间戳>_<uuid>.json）
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryFile {
    pub id: String,
    /// 记录时间（Unix 秒）
    pub time: u64,
    pub method: String,
    pub url: String,
    /// 所属接口 uuid（用于 Diff 比对时限定同接口；旧记录无此字段）
    #[serde(default)]
    pub api_uuid: String,
    /// 所属接口名称（旧记录无此字段）
    #[serde(default)]
    pub api_name: String,
    #[serde(default)]
    pub req_headers: Vec<(String, String)>,
    #[serde(default)]
    pub req_body: Option<String>,
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub status: u16,
    #[serde(default)]
    pub status_text: String,
    #[serde(default)]
    pub resp_headers: Vec<(String, String)>,
    #[serde(default)]
    pub resp_body: String,
    #[serde(default)]
    pub time_ms: u64,
    #[serde(default)]
    pub size: usize,
    #[serde(default)]
    pub error: Option<String>,
}

impl HistoryFile {
    fn summary(&self) -> HistoryRecord {
        HistoryRecord {
            id: self.id.clone(),
            time: self.time,
            method: self.method.clone(),
            url: self.url.clone(),
            api_uuid: self.api_uuid.clone(),
            api_name: self.api_name.clone(),
            ok: self.ok,
            status: self.status,
            status_text: self.status_text.clone(),
            time_ms: self.time_ms,
            size: self.size,
            error: self.error.clone(),
        }
    }
}

/// 历史列表摘要（不含请求/响应全文，便于分页加载）
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecord {
    pub id: String,
    pub time: u64,
    pub method: String,
    pub url: String,
    pub api_uuid: String,
    pub api_name: String,
    pub ok: bool,
    pub status: u16,
    pub status_text: String,
    pub time_ms: u64,
    pub size: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 单条历史详情（含请求与响应全文）
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryDetail {
    pub id: String,
    pub time: u64,
    pub method: String,
    pub url: String,
    pub api_uuid: String,
    pub api_name: String,
    pub ok: bool,
    pub status: u16,
    pub status_text: String,
    pub time_ms: u64,
    pub size: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub req_headers: Vec<(String, String)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub req_body: Option<String>,
    pub resp_headers: Vec<(String, String)>,
    pub resp_body: String,
}

/// 某天的记录数量（用于按天分组显示）
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryDay {
    pub day: String,
    pub count: u32,
}

/// 前端保存一条请求历史（发送请求后由前端调用）
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryInput {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub api_uuid: String,
    #[serde(default)]
    pub api_name: String,
    #[serde(default)]
    pub req_headers: Vec<(String, String)>,
    #[serde(default)]
    pub req_body: Option<String>,
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub status: u16,
    #[serde(default)]
    pub status_text: String,
    #[serde(default)]
    pub resp_headers: Vec<(String, String)>,
    #[serde(default)]
    pub resp_body: String,
    #[serde(default)]
    pub time_ms: u64,
    #[serde(default)]
    pub size: usize,
    #[serde(default)]
    pub error: Option<String>,
}

/// 列出 .history 下全部记录文件（跨天，最新在前）
fn list_history_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let hist_dir = root.join(HISTORY_DIR);
    if !hist_dir.exists() {
        return Ok(vec![]);
    }
    let mut files: Vec<PathBuf> = Vec::new();
    for day_entry in fs::read_dir(&hist_dir).map_err(|e| format!("读取历史目录失败: {e}"))? {
        let day_entry = day_entry.map_err(|e| e.to_string())?;
        let day_path = day_entry.path();
        if !day_path.is_dir() {
            continue;
        }
        for f in fs::read_dir(&day_path).map_err(|e| format!("读取历史目录失败: {e}"))? {
            let f = f.map_err(|e| e.to_string())?;
            if f.path().extension().map(|e| e == "json").unwrap_or(false) {
                files.push(f.path());
            }
        }
    }
    // 按修改时间倒序（最新在前），同秒时按文件名倒序
    files.sort_by(|a, b| {
        let ta = a.metadata().and_then(|m| m.modified()).ok();
        let tb = b.metadata().and_then(|m| m.modified()).ok();
        tb.cmp(&ta).then_with(|| b.file_name().cmp(&a.file_name()))
    });
    Ok(files)
}

fn save_history_to(root: &Path, input: HistoryInput) -> Result<String, String> {
    let now = chrono::Local::now();
    let day = now.format("%Y-%m-%d").to_string();
    let dir = root.join(HISTORY_DIR).join(&day);
    fs::create_dir_all(&dir).map_err(|e| format!("创建历史目录失败: {e}"))?;
    let id = uuid::Uuid::new_v4().to_string();
    let secs = now.timestamp() as u64;
    let file = HistoryFile {
        id: id.clone(),
        time: secs,
        method: input.method,
        url: input.url,
        api_uuid: input.api_uuid,
        api_name: input.api_name,
        req_headers: input.req_headers,
        req_body: input.req_body,
        ok: input.ok,
        status: input.status,
        status_text: input.status_text,
        resp_headers: input.resp_headers,
        resp_body: input.resp_body,
        time_ms: input.time_ms,
        size: input.size,
        error: input.error,
    };
    let name = unique_path(&dir, &format!("{secs}_{id}"), ".json");
    write_pretty(&name, &file)?;
    Ok(id)
}

fn history_records_from(root: &Path, offset: u32, limit: u32) -> Result<Vec<HistoryRecord>, String> {
    let files = list_history_files(root)?;
    let start = (offset as usize).min(files.len());
    let end = (start + limit as usize).min(files.len());
    let mut out = Vec::new();
    for p in &files[start..end] {
        if let Ok(content) = fs::read_to_string(p) {
            if let Ok(f) = serde_json::from_str::<HistoryFile>(&content) {
                out.push(f.summary());
            }
        }
    }
    Ok(out)
}

fn history_detail_from(root: &Path, id: &str) -> Result<HistoryDetail, String> {
    let files = list_history_files(root)?;
    for p in files {
        let Ok(content) = fs::read_to_string(&p) else {
            continue;
        };
        let Ok(rec) = serde_json::from_str::<HistoryFile>(&content) else {
            continue;
        };
        if rec.id == id {
            return Ok(HistoryDetail {
                id: rec.id,
                time: rec.time,
                method: rec.method,
                url: rec.url,
                api_uuid: rec.api_uuid,
                api_name: rec.api_name,
                ok: rec.ok,
                status: rec.status,
                status_text: rec.status_text,
                time_ms: rec.time_ms,
                size: rec.size,
                error: rec.error,
                req_headers: rec.req_headers,
                req_body: rec.req_body,
                resp_headers: rec.resp_headers,
                resp_body: rec.resp_body,
            });
        }
    }
    Err("记录不存在".into())
}

fn history_days_from(root: &Path) -> Result<Vec<HistoryDay>, String> {
    let hist_dir = root.join(HISTORY_DIR);
    if !hist_dir.exists() {
        return Ok(vec![]);
    }
    let mut days = Vec::new();
    for day_entry in fs::read_dir(&hist_dir).map_err(|e| format!("读取历史目录失败: {e}"))? {
        let day_entry = day_entry.map_err(|e| e.to_string())?;
        let p = day_entry.path();
        if !p.is_dir() {
            continue;
        }
        let mut count = 0u32;
        for f in fs::read_dir(&p).map_err(|e| format!("读取历史目录失败: {e}"))? {
            if let Ok(f) = f {
                if f.path().extension().map(|e| e == "json").unwrap_or(false) {
                    count += 1;
                }
            }
        }
        if count > 0 {
            days.push(HistoryDay {
                day: day_entry.file_name().to_string_lossy().to_string(),
                count,
            });
        }
    }
    days.sort_by(|a, b| b.day.cmp(&a.day));
    Ok(days)
}

#[tauri::command]
fn save_history(state: State<'_, WorkspaceState>, input: HistoryInput) -> Result<String, String> {
    let root = workspace_root(&state)?;
    save_history_to(&root, input)
}

#[tauri::command]
fn history_records(
    state: State<'_, WorkspaceState>,
    offset: u32,
    limit: u32,
) -> Result<Vec<HistoryRecord>, String> {
    let root = workspace_root(&state)?;
    history_records_from(&root, offset, limit)
}

#[tauri::command]
fn history_detail(state: State<'_, WorkspaceState>, id: String) -> Result<HistoryDetail, String> {
    let root = workspace_root(&state)?;
    history_detail_from(&root, &id)
}

#[tauri::command]
fn history_days(state: State<'_, WorkspaceState>) -> Result<Vec<HistoryDay>, String> {
    let root = workspace_root(&state)?;
    history_days_from(&root)
}

#[tauri::command]
fn history_clear(state: State<'_, WorkspaceState>) -> Result<(), String> {
    let root = workspace_root(&state)?;
    let hist_dir = root.join(HISTORY_DIR);
    if hist_dir.exists() {
        fs::remove_dir_all(&hist_dir).map_err(|e| format!("清空历史失败: {e}"))?;
    }
    Ok(())
}

// ==================== 请求示例 ====================

pub const EXAMPLES_DIR: &str = ".examples";

/// 示例文件内容（.examples/<接口uuid>/<示例名称hash值>.json）
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExampleFile {
    /// 示例名称
    pub name: String,
    /// 保存时间（Unix 秒）
    pub time: u64,
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub req_headers: Vec<(String, String)>,
    /// 路径参数（发送时的取值）
    #[serde(default)]
    pub req_path: Vec<(String, String)>,
    /// Query 参数（发送时的取值）
    #[serde(default)]
    pub req_query: Vec<(String, String)>,
    #[serde(default)]
    pub req_body: Option<String>,
    #[serde(default)]
    pub status: u16,
    #[serde(default)]
    pub status_text: String,
    #[serde(default)]
    pub resp_headers: Vec<(String, String)>,
    #[serde(default)]
    pub resp_body: String,
    #[serde(default)]
    pub time_ms: u64,
    #[serde(default)]
    pub size: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 示例列表摘要（不含请求/响应全文）
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExampleSummary {
    pub name: String,
    /// 文件名（不含目录），用于读取/删除
    pub file: String,
    pub time: u64,
    pub method: String,
    pub url: String,
    pub status: u16,
}

/// 示例名称 -> 稳定哈希（同名示例覆盖保存；FNV-1a 64 位）
fn example_name_hash(name: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in name.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

fn examples_dir(root: &Path, uuid: &str) -> Result<PathBuf, String> {
    if uuid.trim().is_empty() {
        return Err("接口标识为空，无法保存示例".into());
    }
    Ok(root.join(EXAMPLES_DIR).join(uuid.trim()))
}

fn save_example_to(root: &Path, uuid: &str, name: &str, data: ExampleFile) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("示例名称不能为空".into());
    }
    let dir = examples_dir(root, uuid)?;
    fs::create_dir_all(&dir).map_err(|e| format!("创建示例目录失败: {e}"))?;
    let file = format!("{}.json", example_name_hash(name));
    write_pretty(&dir.join(&file), &data)?;
    Ok(file)
}

fn list_examples_from(root: &Path, uuid: &str) -> Result<Vec<ExampleSummary>, String> {
    let dir = examples_dir(root, uuid)?;
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| format!("读取示例目录失败: {e}"))? {
        let p = entry.map_err(|e| format!("读取示例目录失败: {e}"))?.path();
        if p.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(content) = fs::read_to_string(&p) {
            if let Ok(f) = serde_json::from_str::<ExampleFile>(&content) {
                out.push(ExampleSummary {
                    name: f.name,
                    file: p
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    time: f.time,
                    method: f.method,
                    url: f.url,
                    status: f.status,
                });
            }
        }
    }
    out.sort_by(|a, b| b.time.cmp(&a.time));
    Ok(out)
}

fn example_path(root: &Path, uuid: &str, file: &str) -> Result<PathBuf, String> {
    // 防目录穿越：文件名必须是纯文件名
    if file.contains('/') || file.contains('\\') || file.contains("..") {
        return Err("非法的示例文件名".into());
    }
    Ok(examples_dir(root, uuid)?.join(file))
}

#[tauri::command]
fn save_example(
    state: State<'_, WorkspaceState>,
    uuid: String,
    name: String,
    data: ExampleFile,
) -> Result<String, String> {
    let root = workspace_root(&state)?;
    save_example_to(&root, &uuid, &name, data)
}

#[tauri::command]
fn list_examples(
    state: State<'_, WorkspaceState>,
    uuid: String,
) -> Result<Vec<ExampleSummary>, String> {
    let root = workspace_root(&state)?;
    list_examples_from(&root, &uuid)
}

fn read_example_file(root: &Path, uuid: &str, file: &str) -> Result<ExampleFile, String> {
    let p = example_path(root, uuid, file)?;
    let content = fs::read_to_string(&p).map_err(|e| format!("读取示例失败: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("解析示例失败: {e}"))
}

#[tauri::command]
fn read_example(
    state: State<'_, WorkspaceState>,
    uuid: String,
    file: String,
) -> Result<ExampleFile, String> {
    let root = workspace_root(&state)?;
    read_example_file(&root, &uuid, &file)
}

#[tauri::command]
fn delete_example(
    state: State<'_, WorkspaceState>,
    uuid: String,
    file: String,
) -> Result<(), String> {
    let root = workspace_root(&state)?;
    let p = example_path(&root, &uuid, &file)?;
    fs::remove_file(&p).map_err(|e| format!("删除示例失败: {e}"))
}

// ==================== 对象管理命令 ====================

/// 列出对象存储（分组 + 对象定义）
#[tauri::command]
fn list_objects(state: State<'_, WorkspaceState>) -> Result<objects::ObjectStore, String> {
    let root = workspace_root(&state)?;
    objects::list_objects(&root)
}

/// 保存对象存储（整体覆盖写）
#[tauri::command]
fn save_objects(
    state: State<'_, WorkspaceState>,
    store: objects::ObjectStore,
) -> Result<String, String> {
    let root = workspace_root(&state)?;
    objects::save_objects(&root, &store)
}

/// 数据生成结果
#[derive(serde::Serialize)]
struct GenDataResult {
    file: String,
    dir: String,
    count: usize,
    elapsed_ms: u64,
}

/// 数据生成提交的属性配置（写入日志）
#[derive(serde::Serialize, serde::Deserialize)]
struct GenPropItem {
    key: String,
    kind: String,
    mock: String,
    enabled: bool,
    #[serde(default)]
    desc: Option<String>,
}

/// 单条生成记录（.gen_log/<时间戳>_<object-uuid>.json）
#[derive(serde::Serialize, serde::Deserialize)]
struct GenLogItem {
    file: String,
    time: i64,
    time_str: String,
    object_uuid: String,
    object_name: String,
    dir: String,
    format: String,
    table: String,
    count: usize,
    elapsed_ms: u64,
    props: Vec<GenPropItem>,
}

/// 写入生成的数据文件，并在工作区 .gen_log/<时间戳>_<object-uuid>.json 保存一条生成记录
/// （含提交的数据与耗时）。
#[tauri::command]
fn gen_data(
    state: State<'_, WorkspaceState>,
    dir: String,
    file_name: String,
    content: String,
    format: String,
    table: String,
    count: usize,
    elapsed_ms: u64,
    object_uuid: String,
    object_name: String,
    props: Vec<GenPropItem>,
) -> Result<GenDataResult, String> {
    let dir_path = std::path::Path::new(&dir);
    if !dir_path.is_dir() {
        return Err(format!("导出目录不存在: {dir}"));
    }
    let path = dir_path.join(&file_name);
    std::fs::write(&path, content).map_err(|e| format!("写入文件失败: {e}"))?;

    // 生成记录：工作区根 .gen_log/<时间戳>_<object-uuid>.json（每条记录一个文件）
    let root = workspace_root(&state)?;
    let log_dir = root.join(".gen_log");
    std::fs::create_dir_all(&log_dir).map_err(|e| format!("创建 .gen_log 失败: {e}"))?;
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let log_path = log_dir.join(format!("{ts}_{object_uuid}.json"));
    let record = GenLogItem {
        file: file_name.clone(),
        time: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0),
        time_str: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        object_uuid,
        object_name,
        dir: dir.clone(),
        format,
        table,
        count,
        elapsed_ms,
        props,
    };
    let text = serde_json::to_string_pretty(&record).map_err(|e| format!("序列化生成记录失败: {e}"))?;
    std::fs::write(&log_path, text).map_err(|e| format!("写入生成记录失败: {e}"))?;

    Ok(GenDataResult { file: file_name, dir, count, elapsed_ms })
}

/// 读取 .gen_log 下全部生成记录（按时间倒序）。
#[tauri::command]
fn list_gen_logs(state: State<'_, WorkspaceState>) -> Result<Vec<GenLogItem>, String> {
    let root = workspace_root(&state)?;
    let log_dir = root.join(".gen_log");
    if !log_dir.is_dir() {
        return Ok(vec![]);
    }
    let mut items: Vec<GenLogItem> = vec![];
    let read = std::fs::read_dir(&log_dir).map_err(|e| format!("读取 .gen_log 失败: {e}"))?;
    for entry in read.flatten() {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Ok(t) = std::fs::read_to_string(&p) {
                if let Ok(v) = serde_json::from_str::<GenLogItem>(&t) {
                    items.push(v);
                }
            }
        }
    }
    items.sort_by(|a, b| b.time.cmp(&a.time));
    Ok(items)
}

/// 从 JSON 文本生成对象（嵌套 object 提取为独立对象，hash 相同则复用已有对象）
#[tauri::command]
fn import_json_object(
    state: State<'_, WorkspaceState>,
    name: String,
    group: String,
    json: String,
) -> Result<objects::ObjectImportResult, String> {
    let root = workspace_root(&state)?;
    objects::import_json_object(&root, &name, &group, &json)
}

/// 从 SQL CREATE TABLE 建表语句生成对象（每个表一个对象）
#[tauri::command]
fn import_ddl(
    state: State<'_, WorkspaceState>,
    group: String,
    ddl: String,
) -> Result<objects::ObjectImportResult, String> {
    let root = workspace_root(&state)?;
    objects::import_ddl(&root, &group, &ddl)
}

/// 对象被接口文档引用的统计（接口数量 + 引用接口列表）
#[tauri::command]
fn object_usage(
    state: State<'_, WorkspaceState>,
    store: objects::ObjectStore,
) -> Result<Vec<objects::ObjectUsageItem>, String> {
    let root = workspace_root(&state)?;
    objects::object_usage(&root, &store)
}

// ==================== Mock 服务 ====================

#[tauri::command]
async fn mock_start(app: AppHandle, port: u16) -> Result<MockStatus, String> {
    let res = mock::start_mock(&app, port).await;
    update_tray_mock_item(&app);
    res
}

#[tauri::command]
async fn mock_stop(app: AppHandle) -> Result<MockStatus, String> {
    mock::stop_mock(&app);
    update_tray_mock_item(&app);
    Ok(mock::status(&app))
}

#[tauri::command]
async fn mock_status(app: AppHandle) -> Result<MockStatus, String> {
    Ok(mock::status(&app))
}

#[tauri::command]
async fn mock_reload(app: AppHandle) -> Result<MockStatus, String> {
    mock::reload_mock(&app)?;
    Ok(mock::status(&app))
}

// ==================== 系统托盘 ====================

fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// 当前激活环境名（从工作区 __envs.json 读取）
fn active_env_name(app: &AppHandle) -> String {
    let root = app
        .state::<WorkspaceState>()
        .root
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    match root {
        Some(r) => read_env_file(&r).active,
        None => String::new(),
    }
}

/// 当前设置语言（"zh" / "zh-tw" / "en"，读取 settings.json，兼容旧值）
fn settings_lang(app: &AppHandle) -> String {
    let lang = load_settings(app.clone()).unwrap_or_default().language;
    normalize_lang(&lang)
}

/// 归一化语言：旧配置 "" / "zh" / "en" 与新值 "zh-tw" 统一处理
fn normalize_lang(lang: &str) -> String {
    let l = lang.trim().to_lowercase().replace('_', "-");
    if l == "en" {
        "en".into()
    } else if l == "zh-tw" || l == "zh-hant" || l == "zh-cht" || l == "tw" || l == "cht" {
        "zh-tw".into()
    } else {
        "zh".into()
    }
}

/// 按语言取托盘文案（简体中文 / 繁體中文 / English）
fn tray_text(lang: &str, zh: &str, tw: &str, en: &str) -> String {
    if lang == "en" {
        en.into()
    } else if lang == "zh-tw" {
        tw.into()
    } else {
        zh.into()
    }
}

/// 更新托盘菜单中的环境变量菜单项文字
pub fn update_tray_env_item(app: &AppHandle) {
    let lang = settings_lang(app);
    let name = active_env_name(app);
    let text = if name.trim().is_empty() {
        tray_text(&lang, "环境：未设置（点击编辑）", "環境：未設置（點擊編輯）", "Env: unset (click to edit)")
    } else {
        tray_text(&lang, "环境：{name}（点击编辑）", "環境：{name}（點擊編輯）", "Env: {name} (click to edit)")
            .replace("{name}", name.trim())
    };
    let state = app.state::<TrayState>();
    let guard = state.env_item.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(item) = guard.as_ref() {
        let _ = item.set_text(&text);
    }
}

/// 前端保存/切换环境后同步托盘文字
#[tauri::command]
fn update_tray_env(app: AppHandle, name: String) {
    let lang = settings_lang(&app);
    let text = if name.trim().is_empty() {
        tray_text(&lang, "环境：未设置（点击编辑）", "環境：未設置（點擊編輯）", "Env: unset (click to edit)")
    } else {
        tray_text(&lang, "环境：{name}（点击编辑）", "環境：{name}（點擊編輯）", "Env: {name} (click to edit)")
            .replace("{name}", name.trim())
    };
    let state = app.state::<TrayState>();
    let guard = state.env_item.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(item) = guard.as_ref() {
        let _ = item.set_text(&text);
    }
}

/// 更新托盘菜单中 Mock 菜单项文字
pub fn update_tray_mock_item(app: &AppHandle) {
    let state = app.state::<TrayState>();
    let running = *app
        .state::<MockRunState>()
        .running
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let lang = settings_lang(app);
    let guard = state.mock_item.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(item) = guard.as_ref() {
        let _ = item.set_text(if running {
            tray_text(&lang, "停止 Mock 服务", "停止 Mock 服務", "Stop Mock Server")
        } else {
            tray_text(&lang, "启动 Mock 服务", "啟動 Mock 服務", "Start Mock Server")
        });
    }
}

/// 按当前语言刷新托盘菜单全部文字（语言切换后调用）
pub fn update_tray_language(app: &AppHandle) {
    let lang = settings_lang(app);
    let st = app.state::<TrayState>();
    if let Some(i) = st.show_item.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        let _ = i.set_text(&tray_text(&lang, "显示窗口", "顯示窗口", "Show Window"));
    }
    if let Some(i) = st.github_item.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        let _ = i.set_text(&tray_text(&lang, "GitHub 仓库", "GitHub 倉庫", "GitHub Repository"));
    }
    if let Some(i) = st.issue_item.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        let _ = i.set_text(&tray_text(&lang, "提交 Issue", "提交 Issue", "Submit Issue"));
    }
    if let Some(i) = st.quit_item.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        let _ = i.set_text(&tray_text(&lang, "退出", "退出", "Quit"));
    }
    // 语言子菜单标题 + 子项勾选态（单行入口，展开后勾选当前语言）
    if let Some(m) = st
        .lang_submenu
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
    {
        let _ = m.set_text(&tray_text(&lang, "语言", "語言", "Language"));
    }
    let set_checked = |item: &Mutex<Option<tauri::menu::CheckMenuItem<tauri::Wry>>>, active: bool| {
        if let Some(i) = item.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            let _ = i.set_checked(active);
        }
    };
    set_checked(&st.lang_zh_item, lang == "zh");
    set_checked(&st.lang_tw_item, lang == "zh-tw");
    set_checked(&st.lang_en_item, lang == "en");
    update_tray_env_item(app);
    update_tray_mock_item(app);
    update_tray_update_item(app);
}

/// 按当前语言刷新「检查更新」菜单项文字（无待提醒版本时显示默认文字）
pub fn update_tray_update_item(app: &AppHandle) {
    let lang = settings_lang(app);
    let st = app.state::<TrayState>();
    let pending = st
        .latest_version
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let guard = st.update_item.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(i) = guard.as_ref() {
        let text = match pending {
            Some(v) => tray_text(
                &lang,
                &format!("发现新版本 v{v}"),
                &format!("發現新版本 v{v}"),
                &format!("New version v{v} available"),
            ),
            None => tray_text(&lang, "检查更新", "檢查更新", "Check for Updates"),
        };
        let _ = i.set_text(&text);
    }
}

/// 切换界面语言：保存设置 + 刷新托盘菜单 + 通知前端刷新文案
#[tauri::command]
fn set_language(app: AppHandle, lang: String) -> Result<(), String> {
    let normalized = normalize_lang(&lang);
    let mut s = load_settings(app.clone())?;
    if s.language == normalized {
        // 已是当前语言，无需重复刷新
        return Ok(());
    }
    s.language = normalized.clone();
    save_settings(app.clone(), s)?;
    update_tray_language(&app);
    let _ = app.emit("language-changed", normalized);
    Ok(())
}

/// 托盘菜单：启动/停止 Mock 服务
fn tray_toggle_mock(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let status = mock::status(&app);
        if status.running {
            mock::stop_mock(&app);
        } else {
            // 从工作区 __info.json 读取端口，默认 5050
            let port = {
                let root = app
                    .state::<WorkspaceState>()
                    .root
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                match root {
                    Some(r) => read_info_file(&r).mock_port.unwrap_or(5050),
                    None => 5050,
                }
            };
            let _ = mock::start_mock(&app, port).await;
        }
        update_tray_mock_item(&app);
        // 托盘操作 Mock 后通知主页面刷新状态（启动/停止联动）
        let _ = app.emit("mock-status-changed", ());
    });
}

/// 标记发现新版本：刷新托盘菜单文字 + 记录版本号 + 通知前端弹窗提醒
fn mark_update_available(app: &AppHandle, info: &UpdateInfo) {
    *app.state::<TrayState>()
        .latest_version
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = Some(info.latest.clone());
    update_tray_update_item(app);
    let _ = app.emit("update-available", info);
}

/// 清除「发现新版本」状态，恢复默认「检查更新」文字
fn reset_update_item(app: &AppHandle) {
    *app.state::<TrayState>()
        .latest_version
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = None;
    update_tray_update_item(app);
}

/// 托盘菜单：检查更新（异步访问 GitHub Releases，发现新版本时提醒）
pub fn tray_check_update(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        match fetch_latest_release().await {
            Ok(info) if info.has_update => mark_update_available(&app, &info),
            // 已是最新或检查失败：恢复默认文字
            _ => reset_update_item(&app),
        }
    });
}

/// 创建系统托盘图标与菜单
fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::{CheckMenuItem, IconMenuItem, Menu, PredefinedMenuItem, Submenu};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    use tauri_plugin_opener::OpenerExt;

    app.manage(TrayState {
        mock_item: Mutex::new(None),
        env_item: Mutex::new(None),
        show_item: Mutex::new(None),
        github_item: Mutex::new(None),
        issue_item: Mutex::new(None),
        quit_item: Mutex::new(None),
        lang_submenu: Mutex::new(None),
        lang_zh_item: Mutex::new(None),
        lang_tw_item: Mutex::new(None),
        lang_en_item: Mutex::new(None),
        update_item: Mutex::new(None),
        latest_version: Mutex::new(None),
        exiting: AtomicBool::new(false),
    });

    // 托盘菜单图标：由 gen-tray-icons.mjs 生成的 16x16 单色图标
    let icon_info = tauri::image::Image::from_bytes(include_bytes!("../tray-icons/info.png"))?;
    let icon_window =
        tauri::image::Image::from_bytes(include_bytes!("../tray-icons/window.png"))?;
    let icon_env = tauri::image::Image::from_bytes(include_bytes!("../tray-icons/env.png"))?;
    let icon_mock = tauri::image::Image::from_bytes(include_bytes!("../tray-icons/mock.png"))?;
    let icon_github =
        tauri::image::Image::from_bytes(include_bytes!("../tray-icons/github.png"))?;
    let icon_issue = tauri::image::Image::from_bytes(include_bytes!("../tray-icons/issue.png"))?;
    let icon_quit = tauri::image::Image::from_bytes(include_bytes!("../tray-icons/quit.png"))?;

    let version = IconMenuItem::with_id(
        app,
        "tray_version",
        format!("API Manager v{}", env!("CARGO_PKG_VERSION")),
        false,
        Some(icon_info.clone()),
        None::<&str>,
    )?;
    let show = IconMenuItem::with_id(app, "show", "显示窗口", true, Some(icon_window), None::<&str>)?;
    let env_item = IconMenuItem::with_id(
        app,
        "edit_env",
        "环境：未设置（点击编辑）",
        true,
        Some(icon_env),
        None::<&str>,
    )?;
    let toggle_mock = IconMenuItem::with_id(
        app,
        "toggle_mock",
        "启动 Mock 服务",
        true,
        Some(icon_mock),
        None::<&str>,
    )?;
    let github = IconMenuItem::with_id(
        app,
        "open_github",
        "GitHub 仓库",
        true,
        Some(icon_github),
        None::<&str>,
    )?;
    let issue = IconMenuItem::with_id(
        app,
        "open_issue",
        "提交 Issue",
        true,
        Some(icon_issue),
        None::<&str>,
    )?;
    let quit = IconMenuItem::with_id(app, "quit", "退出", true, Some(icon_quit), None::<&str>)?;
    // 语言切换：单行「语言」子菜单，内含简体中文 / 繁體中文 / English 勾选项
    let lang_zh = CheckMenuItem::with_id(app, "lang_zh", "简体中文", true, false, None::<&str>)?;
    let lang_tw = CheckMenuItem::with_id(app, "lang_tw", "繁體中文", true, false, None::<&str>)?;
    let lang_en = CheckMenuItem::with_id(app, "lang_en", "English", true, false, None::<&str>)?;
    let lang_menu = Submenu::with_items(app, "语言", true, &[&lang_zh, &lang_tw, &lang_en])?;
    // 检查更新（异步访问 GitHub Releases；发现新版本时文字变为「发现新版本 vX.Y.Z」）
    let check_update = IconMenuItem::with_id(
        app,
        "check_update",
        "检查更新",
        true,
        Some(icon_info.clone()),
        None::<&str>,
    )?;
    let menu = Menu::with_items(
        app,
        &[
            &version,
            &PredefinedMenuItem::separator(app)?,
            &show,
            &PredefinedMenuItem::separator(app)?,
            &env_item,
            &PredefinedMenuItem::separator(app)?,
            &toggle_mock,
            &PredefinedMenuItem::separator(app)?,
            &check_update,
            &PredefinedMenuItem::separator(app)?,
            &github,
            &issue,
            &PredefinedMenuItem::separator(app)?,
            &lang_menu,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    *app.state::<TrayState>().mock_item.lock().unwrap() = Some(toggle_mock.clone());
    *app.state::<TrayState>().env_item.lock().unwrap() = Some(env_item.clone());
    *app.state::<TrayState>().show_item.lock().unwrap() = Some(show.clone());
    *app.state::<TrayState>().github_item.lock().unwrap() = Some(github.clone());
    *app.state::<TrayState>().issue_item.lock().unwrap() = Some(issue.clone());
    *app.state::<TrayState>().quit_item.lock().unwrap() = Some(quit.clone());
    *app.state::<TrayState>().lang_submenu.lock().unwrap() = Some(lang_menu.clone());
    *app.state::<TrayState>().lang_zh_item.lock().unwrap() = Some(lang_zh.clone());
    *app.state::<TrayState>().lang_tw_item.lock().unwrap() = Some(lang_tw.clone());
    *app.state::<TrayState>().lang_en_item.lock().unwrap() = Some(lang_en.clone());
    *app.state::<TrayState>().update_item.lock().unwrap() = Some(check_update.clone());
    // 用当前设置语言 + 工作区环境名刷新托盘文字
    update_tray_language(app.handle());

    TrayIconBuilder::with_id("main")
        // 使用项目 logo 生成的 32px 方形图标作为托盘图标（小尺寸显示更清晰）
        .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))?)
        .menu(&menu)
        .tooltip(format!("API Manager v{}", env!("CARGO_PKG_VERSION")))
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app),
            "edit_env" => {
                // 显示窗口并通知前端打开环境变量编辑器
                show_main_window(app);
                let _ = app.emit("open-env-editor", ());
            }
            "toggle_mock" => tray_toggle_mock(app),
            "check_update" => {
                // 已发现新版本时点击直接打开 GitHub 发布页；否则发起检查
                let pending = app
                    .state::<TrayState>()
                    .latest_version
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .is_some();
                if pending {
                    use tauri_plugin_opener::OpenerExt;
                    let _ = app.opener().open_url(RELEASES_PAGE, None::<&str>);
                } else {
                    tray_check_update(app);
                }
            }
            "open_github" => {
                // 打开项目 GitHub 仓库
                let _ = app
                    .opener()
                    .open_url("https://github.com/freewu/api-manager", None::<&str>);
            }
            "open_issue" => {
                // 快速跳转到新建 Issue 页面
                let _ = app
                    .opener()
                    .open_url("https://github.com/freewu/api-manager/issues/new", None::<&str>);
            }
            "quit" => {
                app.state::<TrayState>()
                    .exiting
                    .store(true, Ordering::Relaxed);
                app.exit(0);
            }
            "lang_zh" => {
                let _ = crate::set_language(app.clone(), "zh".to_string());
            }
            "lang_tw" => {
                let _ = crate::set_language(app.clone(), "zh-tw".to_string());
            }
            "lang_en" => {
                let _ = crate::set_language(app.clone(), "en".to_string());
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // 左键单击托盘图标 -> 显示窗口
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
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
            setup_tray(app)?;
            // 启动后异步检查 GitHub Releases（延迟 3 秒避免与启动抢资源，失败静默）
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                tray_check_update(&handle);
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
            set_language,
            get_workspace,
            open_workspace,
            get_recent_workspaces,
            pick_workspace,
            workspace_is_empty,
            has_workspace_info,
            create_demo,
            import_postman,
            import_openapi,
            import_apifox,
            import_apipost,
            import_raml,
            import_wadl,
            import_har,
            import_yapi,
            import_eolink,
            import_insomnia,
            import_jmeter,
            import_apidoc,
            import_extra,
            render_api_markdown,
            render_group_markdown,
            render_markdown,
            export_api_markdown,
            import_markdown,
            export_selection,
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
            update_tray_env,
            get_app_version,
            check_update,
            send_request,
            pick_file,
            save_history,
            history_records,
            history_detail,
            history_days,
            history_clear,
            save_example,
            list_examples,
            read_example,
            delete_example,
            mock_start,
            mock_stop,
            mock_status,
            list_objects,
            save_objects,
            gen_data,
            list_gen_logs,
            import_json_object,
            import_ddl,
            object_usage,
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
