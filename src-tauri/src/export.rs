//! 导出：Postman Collection v2.1 / OpenAPI 3.0 / Docsify 文档目录。
//! 收集选中路径（接口或分组）下的全部接口，按格式生成内容。

use crate::markdown;
use crate::{read_api, read_info_file, ApiFile, ENV_FILE, INFO_FILE};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

mod apidoc;
mod apifox;
mod apipost;
mod batch;
mod docsify;
mod eolink;
mod insomnia;
mod jmeter;
mod openapi;
mod postman;
mod raml;
mod wadl;
mod yapi;
pub use self::apidoc::to_apidoc;
pub use self::apifox::to_apifox;
pub use self::apipost::to_apipost;
pub use self::batch::{to_apidog, to_apizza, to_bruno, to_doclever, to_docway, to_easydoc, to_hoppscotch, to_io_docs, to_metersphere, to_nei, to_rap2_project};
pub use self::docsify::{docsify_files, markdown_single_file};
pub use self::eolink::to_eolink;
pub use self::insomnia::to_insomnia;
pub use self::jmeter::to_jmeter;
pub use self::openapi::to_openapi;
pub use self::postman::to_postman;
pub use self::raml::to_raml;
pub use self::wadl::to_wadl;
pub use self::yapi::to_yapi;

/// 收集选中路径下的全部接口。
/// 返回 (分组路径段, ApiFile)：分组路径段为各层分组的显示名称（不含工作区根）。
pub fn collect_apis(root: &Path, paths: &[String]) -> Result<Vec<(Vec<(String, bool)>, ApiFile)>, String> {
    let mut out: Vec<(Vec<(String, bool)>, ApiFile)> = Vec::new();
    // 已选中的分组目录：其下接口由目录递归收集，单独的文件路径命中目录时跳过，避免重复
    let dirs: Vec<PathBuf> = paths
        .iter()
        .filter(|p| Path::new(p).is_dir())
        .map(PathBuf::from)
        .collect();
    // 仅遍历未被其他已选分组覆盖的「顶层」分组：嵌套分组已随外层目录递归收集，
    // 若再单独遍历会重复导出（导出弹窗勾选分组时同时勾选整棵子树）
    let top_dirs: Vec<&PathBuf> = dirs
        .iter()
        .filter(|d| !dirs.iter().any(|o| o != *d && d.starts_with(o)))
        .collect();
    for p in paths {
        let abs = Path::new(p);
        if abs.is_dir() {
            if top_dirs.iter().any(|d| d.as_path() == abs) {
                let mut segs = Vec::new();
                walk_dir(abs, &mut segs, &mut out)?;
            }
        } else if abs.is_file() {
            if dirs.iter().any(|d| abs.starts_with(d)) {
                continue; // 已随分组目录收集，跳过避免重复
            }
            let api = read_api(p.clone())?;
            out.push((Vec::new(), api));
        }
    }
    let _ = root;
    Ok(out)
}

/// 递归遍历分组目录，收集其下所有接口文件（跳过 .examples / .version 等点开头目录）
/// 括号第二项为分组是否已废弃（来自该目录 __info.json 的 deprecated 字段）
fn walk_dir(
    dir: &Path,
    segs: &mut Vec<(String, bool)>,
    out: &mut Vec<(Vec<(String, bool)>, ApiFile)>,
) -> Result<(), String> {
    let info = read_info_file(dir);
    let dep = info.deprecated.unwrap_or(false);
    let dir_name = dir
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    segs.push((info.name.clone().unwrap_or(dir_name), dep));
    for entry in fs::read_dir(dir).map_err(|e| format!("读取目录失败: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            walk_dir(&path, segs, out)?;
        } else if path.extension().map(|e| e == "json").unwrap_or(false)
            && file_name != INFO_FILE
            && file_name != ENV_FILE
        {
            let api = read_api(path.to_string_lossy().to_string())?;
            out.push((segs.clone(), api));
        }
    }
    segs.pop();
    Ok(())
}

// ==================== Postman Collection v2.1 ====================

struct PNode<'a> {
    name: String,
    apis: Vec<&'a ApiFile>,
    children: BTreeMap<String, PNode<'a>>,
}

/// 简单 URL 拆分：host 按点分段、path 按 / 分段
fn parse_url(url: &str) -> (Vec<String>, Vec<String>) {
    let no_q = url.split(['?', '#']).next().unwrap_or("");
    let after = no_q
        .split_once("://")
        .map(|(_, r)| r)
        .unwrap_or(no_q);
    if let Some((h, p)) = after.split_once('/') {
        let host = h.split('.').map(|s| s.to_string()).collect();
        let path = p
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        (host, path)
    } else {
        let host = after.split('.').map(|s| s.to_string()).collect();
        (host, Vec::new())
    }
}

// ==================== OpenAPI 3.0 ====================

/// XML 转义
fn xml_escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&apos;".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

// ==================== YApi 导出 ====================

pub fn export_extra(
    apis: &[(Vec<(String, bool)>, ApiFile)],
    format: &str,
) -> Result<(String, String, String), String> {
    let (val, fname, ext) = match format {
        "apidog" => (to_apidog(apis), "api-collection", "json"),
        "bruno" => (to_bruno(apis), "bruno-collection", "json"),
        "apizza" => (to_apizza(apis), "apizza-project", "json"),
        "nei" => (to_nei(apis), "nei-project", "json"),
        "doclever" => (to_doclever(apis), "DOClever", "json"),
        "io-docs" => (to_io_docs(apis), "io-docs", "json"),
        "easydoc" => (to_easydoc(apis), "easydoc", "json"),
        "docway" => (to_docway(apis), "docway", "mjson"),
        "hoppscotch" => (to_hoppscotch(apis), "hoppscotch", "json"),
        "metersphere" => (to_metersphere(apis), "metersphere", "json"),
        "rap2-project" => (to_rap2_project(apis), "rap2-project", "json"),
        _ => return Err(format!("不支持的格式: {format}")),
    };
    let content = serde_json::to_string_pretty(&val).map_err(|e| format!("序列化失败: {e}"))?;
    Ok((content, fname.to_string(), ext.to_string()))
}

use crate::{workspace_root, WorkspaceState};
use tauri::{AppHandle, State};

/// 导出选中接口/分组为 Postman / OpenAPI / Docsify 格式：弹窗选择保存位置并写入
#[tauri::command]
pub(crate) fn export_selection(
    app: AppHandle,
    state: State<'_, WorkspaceState>,
    paths: Vec<String>,
    format: String,
    nav: String,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let root = workspace_root(&state)?;
    let apis = collect_apis(&root, &paths)?;
    if apis.is_empty() {
        return Err("所选内容中没有接口".into());
    }
    match format.as_str() {
        "postman" => {
            let v = to_postman(&apis);
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
            let v = to_openapi(&ws_name, &apis);
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
            let v = to_apifox(&apis);
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
            let v = to_apipost(&apis);
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
            let v = to_raml(&apis);
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
            let content = to_wadl(&apis);
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
            let v = to_yapi(&apis);
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
            let v = to_eolink(&apis);
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
            let v = to_insomnia(&apis);
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
            let content = to_jmeter(&apis);
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
            let (proj, data) = to_apidoc(&apis);
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
            let (content, fname, ext) = export_extra(&apis, &format)?;
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
            let files = docsify_files(&apis);
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
            let md = markdown_single_file(&title, &apis);
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

#[cfg(test)]
#[path = "export_test.rs"]
mod tests;
