mod mock;

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
    pub mock_item: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
    /// 托盘菜单中“环境变量”菜单项，显示当前环境名，点击可打开编辑器
    pub env_item: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
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
    /// 是否启用接口版本管理（主页面显示「保存」与「查看版本信息」）
    pub enable_version: bool,
    /// 是否启用 Mock 功能（主页面显示 Mock 开关）
    pub enable_mock: bool,
    /// Mock 服务默认端口
    pub mock_port: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            display_mode: "system".into(),
            enable_version: true,
            enable_mock: true,
            mock_port: 5050,
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
            // 如果根目录没有 __info.json，自动生成一份
            let info_path = Path::new(&s).join(INFO_FILE);
            if !info_path.exists() {
                let info = serde_json::json!({
                    "name": "我的 API 集合",
                    "description": "",
                    "baseUrl": "",
                    "mockPort": 5050
                });
                let _ = write_pretty(&info_path, &info);
            }
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

#[tauri::command]
fn workspace_is_empty(state: State<'_, WorkspaceState>) -> Result<bool, String> {
    let root = workspace_root(&state)?;
    is_workspace_empty(&root)
}

/// 在空工作区中生成演示案例（示例分组 + 接口 + 环境变量）
#[tauri::command]
fn create_demo(state: State<'_, WorkspaceState>) -> Result<(), String> {
    let root = workspace_root(&state)?;
    if !is_workspace_empty(&root)? {
        return Err("工作区非空，不生成演示案例".into());
    }
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
    write("用户管理", "创建用户.json", &create_user)?;

    let mut get_user = api_file("获取用户信息", "GET", "/api/users/{id}", "查询单个用户信息");
    get_user["params"] = serde_json::json!([{ "key": "id", "value": "1", "enabled": true, "description": "用户ID" }]);
    get_user["mock"] = serde_json::json!({ "enabled": true, "status": 200, "headers": [], "delay": 0, "body": "{\n  \"code\": 0,\n  \"data\": {\n    \"id\": 1,\n    \"name\": \"张三\",\n    \"email\": \"zhangsan@example.com\"\n  },\n  \"message\": \"成功\"\n}" });
    write("用户管理", "获取用户信息.json", &get_user)?;

    let mut del_user = api_file("删除用户", "DELETE", "/api/users/{id}", "删除指定用户");
    del_user["params"] = serde_json::json!([{ "key": "id", "value": "1", "enabled": true, "description": "用户ID" }]);
    del_user["mock"] = serde_json::json!({ "enabled": true, "status": 200, "headers": [], "delay": 0, "body": "{\n  \"code\": 0,\n  \"message\": \"删除成功\"\n}" });
    write("用户管理", "删除用户.json", &del_user)?;

    // 订单管理分组
    write("订单管理", INFO_FILE, &serde_json::json!({ "name": "订单管理", "description": "订单相关接口" }))?;
    let mut list_orders = api_file("获取订单列表", "GET", "/api/orders", "分页查询订单列表");
    list_orders["query"] = serde_json::json!([
        { "key": "page", "value": "1", "enabled": true, "description": "页码" },
        { "key": "pageSize", "value": "10", "enabled": true, "description": "每页数量" }
    ]);
    list_orders["mock"] = serde_json::json!({ "enabled": true, "status": 200, "headers": [], "delay": 0, "body": "{\n  \"code\": 0,\n  \"data\": {\n    \"list\": [\n      { \"id\": 1001, \"no\": \"SO20240101001\", \"amount\": 99.5 },\n      { \"id\": 1002, \"no\": \"SO20240101002\", \"amount\": 199.0 }\n    ],\n    \"total\": 2\n  },\n  \"message\": \"成功\"\n}" });
    write("订单管理", "获取订单列表.json", &list_orders)?;

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
                    out.form.push(KeyValue {
                        key,
                        value: f
                            .get("value")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        enabled: !f.get("disabled").and_then(|v| v.as_bool()).unwrap_or(false),
                        description: String::new(),
                    });
                }
            }
        }
        _ => {}
    }
    out
}

#[tauri::command]
fn read_tree(state: State<'_, WorkspaceState>) -> Result<TreeNode, String> {
    let root = workspace_root(&state)?;
    build_folder_node(&root)
}

#[tauri::command]
fn read_api(path: String) -> Result<ApiFile, String> {
    let content = fs::read_to_string(&path).map_err(|e| format!("读取失败: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("JSON 解析失败: {e}"))
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

/// 读取某个历史版本文件的原始内容（用于 diff）
#[tauri::command]
fn read_api_version(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("读取版本失败: {e}"))
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
    write_pretty(&p.join(INFO_FILE), &merged)
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
    if let Some(body) = &req.body {
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

fn hide_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
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

/// 更新托盘菜单中的环境变量菜单项文字
pub fn update_tray_env_item(app: &AppHandle) {
    let name = active_env_name(app);
    let text = if name.trim().is_empty() {
        "环境：未设置（点击编辑）".to_string()
    } else {
        format!("环境：{}（点击编辑）", name.trim())
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
    let text = if name.trim().is_empty() {
        "环境：未设置（点击编辑）".to_string()
    } else {
        format!("环境：{}（点击编辑）", name.trim())
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
    let guard = state.mock_item.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(item) = guard.as_ref() {
        let _ = item.set_text(if running {
            "停止 Mock 服务"
        } else {
            "启动 Mock 服务"
        });
    }
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
    });
}

/// 创建系统托盘图标与菜单
fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    app.manage(TrayState {
        mock_item: Mutex::new(None),
        env_item: Mutex::new(None),
        exiting: AtomicBool::new(false),
    });

    let show = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "隐藏窗口", true, None::<&str>)?;
    let env_item =
        MenuItem::with_id(app, "edit_env", "环境：未设置（点击编辑）", true, None::<&str>)?;
    let toggle_mock =
        MenuItem::with_id(app, "toggle_mock", "启动 Mock 服务", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &hide,
            &PredefinedMenuItem::separator(app)?,
            &env_item,
            &PredefinedMenuItem::separator(app)?,
            &toggle_mock,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    *app.state::<TrayState>().mock_item.lock().unwrap() = Some(toggle_mock.clone());
    *app.state::<TrayState>().env_item.lock().unwrap() = Some(env_item.clone());
    // 用当前工作区的环境名刷新托盘文字
    update_tray_env_item(app.handle());

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().expect("缺少默认应用图标").clone())
        .menu(&menu)
        .tooltip("API Manager")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app),
            "hide" => hide_main_window(app),
            "edit_env" => {
                // 显示窗口并通知前端打开环境变量编辑器
                show_main_window(app);
                let _ = app.emit("open-env-editor", ());
            }
            "toggle_mock" => tray_toggle_mock(app),
            "quit" => {
                app.state::<TrayState>()
                    .exiting
                    .store(true, Ordering::Relaxed);
                app.exit(0);
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
            get_workspace,
            pick_workspace,
            workspace_is_empty,
            create_demo,
            import_postman,
            read_tree,
            read_api,
            save_api,
            save_api_version,
            list_versions,
            read_api_version,
            create_api,
            create_folder,
            rename_entry,
            move_entry,
            delete_entry,
            read_info,
            save_info,
            read_envs,
            save_envs,
            update_tray_env,
            get_app_version,
            send_request,
            save_history,
            history_records,
            history_detail,
            history_days,
            history_clear,
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
    async fn test_send_request_bad_url() {
        // 未替换的 {{变量}} 会产生 reqwest builder error，应给出中文提示而不是裸的 builder error
        for url in ["http://{{host}}:8080/api", "127.0.0.1:8080/api"] {
            let req = HttpRequestData {
                method: "GET".into(),
                url: url.to_string(),
                headers: vec![],
                body: None,
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
}