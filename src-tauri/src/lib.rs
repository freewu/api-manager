mod mock;
mod markdown;
mod export;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
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
    // 返回全部记录，由前端按设置限制展示数量（历史记录不删除）
    read_recent(&app)
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

/// 渲染接口的 Markdown 文档（含 HTML 预览版）
#[tauri::command]
fn render_api_markdown(state: State<'_, WorkspaceState>, path: String) -> Result<MarkdownDoc, String> {
    let root = workspace_root(&state)?;
    let group = group_of(&path, &root.to_string_lossy());
    let api = read_api(path)?;
    let md = markdown::render(&api, &group);
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
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let root = workspace_root(&state)?;
    // 分组（目录）走分组 Markdown；接口文件走单接口 Markdown
    let (name, md) = if Path::new(&path).is_dir() {
        group_markdown_doc(&root, &path)?
    } else {
        let group = group_of(&path, &root.to_string_lossy());
        let api = read_api(path)?;
        (api.name.clone(), markdown::render(&api, &group))
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
        markdown::wrap_html(&name, &md)
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
                markdown::wrap_html(&title, &md)
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
        method: "GET".into(),
        path: "/".into(),
        url: String::new(),
        description: String::new(),
        headers: vec![],
        query: vec![],
        params: vec![],
        body: BodyData::default(),
        mock: MockConfig::default(),
        examples: vec![],
        responses: default_responses(),
        doc_params: vec![],
        deprecated: false,
    };
    write_pretty(&file_path, &data)?;
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
            render_api_markdown,
            render_group_markdown,
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
        let mut a = ApiFile {
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
        };
        let main = base.join("接口").join("接口A.json");
        save_api(main.to_string_lossy().to_string(), make("接口A", "v1 描述"))
            .unwrap();
        // 保存两个版本：v1（描述 v1 描述）与 v2（描述 v2 描述）
        save_api_version_at(&base, make("接口A", "v1 描述")).unwrap();
        let v2 = save_api_version_at(&base, make("接口A", "v2 描述")).unwrap();
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

    #[test]
    fn test_history_roundtrip() {
        // 保存 -> 分页列表 -> 详情 -> 按天统计 全链路
        let root = std::env::temp_dir().join(format!("history-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let input = HistoryInput {
            method: "GET".into(),
            url: "http://127.0.0.1:8080/api/users".into(),
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


