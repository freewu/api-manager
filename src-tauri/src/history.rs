//! 请求历史（.history 目录）与请求示例（.examples 目录）

use crate::{sanitize_filename, unique_path, workspace_root, write_pretty, WorkspaceState};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, State};

// ==================== 请求历史 ====================

pub const HISTORY_DIR: &str = crate::HISTORY_DATA_DIR;

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

pub(crate) fn save_history_to(root: &Path, input: HistoryInput) -> Result<String, String> {
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

pub(crate) fn history_records_from(root: &Path, offset: u32, limit: u32) -> Result<Vec<HistoryRecord>, String> {
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

pub(crate) fn history_detail_from(root: &Path, id: &str) -> Result<HistoryDetail, String> {
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

pub(crate) fn history_days_from(root: &Path) -> Result<Vec<HistoryDay>, String> {
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
pub(crate) fn save_history(state: State<'_, WorkspaceState>, input: HistoryInput) -> Result<String, String> {
    let root = workspace_root(&state)?;
    save_history_to(&root, input)
}

#[tauri::command]
pub(crate) fn history_records(
    state: State<'_, WorkspaceState>,
    offset: u32,
    limit: u32,
) -> Result<Vec<HistoryRecord>, String> {
    let root = workspace_root(&state)?;
    history_records_from(&root, offset, limit)
}

#[tauri::command]
pub(crate) fn history_detail(state: State<'_, WorkspaceState>, id: String) -> Result<HistoryDetail, String> {
    let root = workspace_root(&state)?;
    history_detail_from(&root, &id)
}

#[tauri::command]
pub(crate) fn history_days(state: State<'_, WorkspaceState>) -> Result<Vec<HistoryDay>, String> {
    let root = workspace_root(&state)?;
    history_days_from(&root)
}

#[tauri::command]
pub(crate) fn history_clear(state: State<'_, WorkspaceState>) -> Result<(), String> {
    let root = workspace_root(&state)?;
    let hist_dir = root.join(HISTORY_DIR);
    if hist_dir.exists() {
        fs::remove_dir_all(&hist_dir).map_err(|e| format!("清空历史失败: {e}"))?;
    }
    Ok(())
}

// ==================== 请求示例 ====================

pub const EXAMPLES_DIR: &str = crate::EXAMPLES_DATA_DIR;

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

pub(crate) fn save_example_to(root: &Path, uuid: &str, name: &str, data: ExampleFile) -> Result<String, String> {
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

pub(crate) fn list_examples_from(root: &Path, uuid: &str) -> Result<Vec<ExampleSummary>, String> {
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

pub(crate) fn example_path(root: &Path, uuid: &str, file: &str) -> Result<PathBuf, String> {
    // 防目录穿越：文件名必须是纯文件名
    if file.contains('/') || file.contains('\\') || file.contains("..") {
        return Err("非法的示例文件名".into());
    }
    Ok(examples_dir(root, uuid)?.join(file))
}

#[tauri::command]
pub(crate) fn save_example(
    state: State<'_, WorkspaceState>,
    uuid: String,
    name: String,
    data: ExampleFile,
) -> Result<String, String> {
    let root = workspace_root(&state)?;
    save_example_to(&root, &uuid, &name, data)
}

#[tauri::command]
pub(crate) fn list_examples(
    state: State<'_, WorkspaceState>,
    uuid: String,
) -> Result<Vec<ExampleSummary>, String> {
    let root = workspace_root(&state)?;
    list_examples_from(&root, &uuid)
}

pub(crate) fn read_example_file(root: &Path, uuid: &str, file: &str) -> Result<ExampleFile, String> {
    let p = example_path(root, uuid, file)?;
    let content = fs::read_to_string(&p).map_err(|e| format!("读取示例失败: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("解析示例失败: {e}"))
}

#[tauri::command]
pub(crate) fn read_example(
    state: State<'_, WorkspaceState>,
    uuid: String,
    file: String,
) -> Result<ExampleFile, String> {
    let root = workspace_root(&state)?;
    read_example_file(&root, &uuid, &file)
}

#[tauri::command]
pub(crate) fn delete_example(
    state: State<'_, WorkspaceState>,
    uuid: String,
    file: String,
) -> Result<(), String> {
    let root = workspace_root(&state)?;
    let p = example_path(&root, &uuid, &file)?;
    fs::remove_file(&p).map_err(|e| format!("删除示例失败: {e}"))
}

/// 重命名示例：更新 name 字段；新名称哈希不同时把文件改名
pub(crate) fn rename_example_to(
    root: &Path,
    uuid: &str,
    file: &str,
    new_name: &str,
) -> Result<String, String> {
    let new_name = new_name.trim();
    if new_name.is_empty() {
        return Err("示例名称不能为空".into());
    }
    let mut f = read_example_file(root, uuid, file)?;
    if f.name == new_name {
        return Ok(file.to_string());
    }
    let new_file = format!("{}.json", example_name_hash(new_name));
    // 防止覆盖：目标哈希文件已存在且不是当前文件 → 已存在同名示例
    if new_file != file && example_path(root, uuid, &new_file)?.exists() {
        return Err("已存在同名示例".into());
    }
    f.name = new_name.to_string();
    write_pretty(&example_path(root, uuid, &new_file)?, &f)?;
    if new_file != file {
        fs::remove_file(example_path(root, uuid, file)?).map_err(|e| format!("删除原示例失败: {e}"))?;
    }
    Ok(new_file)
}

#[tauri::command]
pub(crate) fn rename_example(
    state: State<'_, WorkspaceState>,
    uuid: String,
    file: String,
    new_name: String,
) -> Result<String, String> {
    let root = workspace_root(&state)?;
    rename_example_to(&root, &uuid, &file, &new_name)
}

/// 把示例的请求部分转为 .http 文本（VS Code REST Client / JetBrains HTTP Client 兼容）
fn example_to_http(f: &ExampleFile) -> String {
    let mut out = String::new();
    out.push_str(&format!("### {}\n", f.name));
    out.push_str(&format!("{} {}\n", f.method, f.url));
    for (k, v) in f.req_headers.iter() {
        if !k.trim().is_empty() {
            out.push_str(&format!("{}: {}\n", k, v));
        }
    }
    if let Some(body) = f.req_body.as_deref() {
        if !body.trim().is_empty() {
            out.push('\n');
            out.push_str(body.trim_end());
            out.push('\n');
        }
    }
    out
}

/// 导出示例为 .http 文件：弹系统保存框，默认文件名为「示例名.http」；取消返回 None
#[tauri::command]
pub(crate) fn export_example_http(
    app: AppHandle,
    state: State<'_, WorkspaceState>,
    uuid: String,
    file: String,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let root = workspace_root(&state)?;
    let f = read_example_file(&root, &uuid, &file)?;
    let base = sanitize_filename(&f.name);
    let default_name = if base.is_empty() {
        "示例.http".to_string()
    } else {
        format!("{base}.http")
    };
    let picked = app
        .dialog()
        .file()
        .set_title("导出示例为 .http 文件")
        .set_file_name(&default_name)
        .add_filter("HTTP", &["http"])
        .blocking_save_file();
    let Some(fp) = picked else {
        return Ok(None);
    };
    let path = fp.into_path().map_err(|e| e.to_string())?;
    fs::write(&path, example_to_http(&f)).map_err(|e| format!("写入失败: {e}"))?;
    Ok(Some(path.to_string_lossy().to_string()))
}

// ==================== 对象管理命令 ====================
