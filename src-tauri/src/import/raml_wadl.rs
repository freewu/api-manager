//! 由 import.rs 拆分：RAML / WADL
use super::*;
#[allow(unused_imports)]
use crate::{ApiFile, BodyData, DocParam, EnvVariable, KeyValue, MockConfig, ResponseItem, sanitize_filename, unique_path, workspace_root, ENV_FILE, INFO_FILE};
#[allow(unused_imports)]
use serde::Serialize;
#[allow(unused_imports)]
use serde_json::Value;
#[allow(unused_imports)]
use std::collections::{BTreeMap, HashMap};
#[allow(unused_imports)]
use std::fs;
#[allow(unused_imports)]
use std::path::{Path, PathBuf};

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
            collapsed: None,
            deprecated: None,
            dirs: vec![],
            apis: vec![],
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
        prescript: String::new(),
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
            collapsed: None,
            deprecated: None,
            dirs: vec![],
            apis: vec![],
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
        prescript: String::new(),
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

