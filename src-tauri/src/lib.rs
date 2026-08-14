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
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            display_mode: "system".into(),
            enable_version: true,
            enable_mock: true,
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

/// 保存接口新版本：写入工作区 .version/<uuid>/<名称>.<版本号>.json
#[tauri::command]
fn save_api_version(
    state: State<'_, WorkspaceState>,
    data: ApiFile,
) -> Result<String, String> {
    let root = workspace_root(&state)?;
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
            read_tree,
            read_api,
            save_api,
            save_api_version,
            list_versions,
            read_api_version,
            create_api,
            create_folder,
            rename_entry,
            delete_entry,
            read_info,
            save_info,
            read_envs,
            save_envs,
            update_tray_env,
            get_app_version,
            send_request,
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
}
