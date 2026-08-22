//! 导出：Postman Collection v2.1 / OpenAPI 3.0 / Docsify 文档目录。
//! 收集选中路径（接口或分组）下的全部接口，按格式生成内容。

use crate::{read_api, read_info_file, sanitize_filename, ApiFile, KeyValue, ENV_FILE, INFO_FILE};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

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

/// 生成 Postman Collection v2.1 JSON
pub fn to_postman(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let mut root = PNode {
        name: "API Manager".to_string(),
        apis: Vec::new(),
        children: BTreeMap::new(),
    };
    for (segs, api) in apis {
        let mut cur = &mut root;
        for (s, _dep) in segs {
            cur = cur.children.entry(s.clone()).or_insert_with(|| PNode {
                name: s.clone(),
                apis: Vec::new(),
                children: BTreeMap::new(),
            });
        }
        cur.apis.push(api);
    }
    let mut items = Vec::new();
    for api in &root.apis {
        items.push(api_to_postman(api));
    }
    for (_, c) in &root.children {
        items.push(pnode_to_postman(c));
    }
    json!({
        "info": {
            "name": "API Manager 导出",
            "description": "由 API Manager 导出的 Postman Collection",
            "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
        },
        "item": items
    })
}

struct PNode<'a> {
    name: String,
    apis: Vec<&'a ApiFile>,
    children: BTreeMap<String, PNode<'a>>,
}

fn pnode_to_postman(n: &PNode) -> Value {
    let mut item = Vec::new();
    for api in &n.apis {
        item.push(api_to_postman(api));
    }
    for (_, c) in &n.children {
        item.push(pnode_to_postman(c));
    }
    json!({ "name": n.name, "item": item })
}

/// 单个接口 → Postman request item
fn api_to_postman(api: &ApiFile) -> Value {
    let headers: Vec<Value> = api
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .map(|h| json!({ "key": h.key, "value": h.value, "type": "text", "description": h.description }))
        .collect();
    let query: Vec<Value> = api
        .query
        .iter()
        .filter(|q| q.enabled && !q.key.trim().is_empty())
        .map(|q| json!({ "key": q.key, "value": q.value, "description": q.description }))
        .collect();
    let url_raw = if !api.url.trim().is_empty() {
        api.url.trim().to_string()
    } else {
        api.path.trim().to_string()
    };
    let (host, path) = parse_url(&url_raw);
    let mut url = json!({
        "raw": url_raw,
        "host": host,
        "path": path,
    });
    if !query.is_empty() {
        url["query"] = Value::Array(query);
    }
    let mut request = json!({
        "method": api.method,
        "header": headers,
        "url": url,
    });
    match api.body.mode.as_str() {
        "json" | "raw" => {
            request["body"] = json!({ "mode": "raw", "raw": api.body.raw });
        }
        "form" => {
            let fields: Vec<Value> = api
                .body
                .form
                .iter()
                .filter(|f| f.enabled && !f.key.trim().is_empty())
                .map(|f| {
                    if f.is_file {
                        json!({ "key": f.key, "type": "file", "src": Value::Null })
                    } else {
                        json!({ "key": f.key, "value": f.value, "type": "text", "description": f.description })
                    }
                })
                .collect();
            request["body"] = json!({ "mode": "urlencoded", "urlencoded": fields });
        }
        _ => {}
    }
    json!({ "name": api.name, "request": request })
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

/// 生成 OpenAPI 3.0 规范 JSON
pub fn to_openapi(title: &str, apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let mut paths: Map<String, Value> = Map::new();
    for (segs, api) in apis {
        let p = if !api.path.trim().is_empty() {
            api.path.trim().to_string()
        } else {
            api.url.trim().to_string()
        };
        if p.is_empty() {
            continue;
        }
        let method = api.method.trim().to_lowercase();
        if !is_valid_method(&method) {
            continue;
        }
        // 同一路径 + 同一方法重复时追加序号（如 /api/users (2)），保证全部接口都导出
        let mut key = p.clone();
        let mut n = 2;
        while paths.get(&key).and_then(|v| v.get(&method)).is_some() {
            key = format!("{p} ({n})");
            n += 1;
        }
        let entry = paths.entry(key).or_insert_with(|| json!({}));
        let obj = entry.as_object_mut().expect("paths 条目为对象");
        obj.insert(method, openapi_operation(segs, api));
    }
    json!({
        "openapi": "3.0.1",
        "info": {
            "title": if title.trim().is_empty() { "API 文档" } else { title },
            "version": "1.0.0",
            "description": "由 API Manager 导出的 OpenAPI 规范"
        },
        "paths": Value::Object(paths)
    })
}

fn is_valid_method(m: &str) -> bool {
    matches!(m, "get" | "put" | "post" | "delete" | "options" | "head" | "patch" | "trace")
}

fn openapi_operation(segs: &[(String, bool)], api: &ApiFile) -> Value {
    let mut params: Vec<Value> = Vec::new();
    for prm in api.params.iter().filter(|p| !p.key.trim().is_empty()) {
        params.push(json!({
            "name": prm.key,
            "in": "path",
            "required": true,
            "description": prm.description,
            "schema": { "type": "string" }
        }));
    }
    for q in api.query.iter().filter(|q| !q.key.trim().is_empty()) {
        params.push(json!({
            "name": q.key,
            "in": "query",
            "description": q.description,
            "schema": { "type": "string" }
        }));
    }
    for h in api.headers.iter().filter(|h| !h.key.trim().is_empty()) {
        params.push(json!({
            "name": h.key,
            "in": "header",
            "description": h.description,
            "schema": { "type": "string" }
        }));
    }
    let mut responses = Map::new();
    // 优先使用「响应」页签条目（名称 + 状态码 + 示例体）；旧数据回退到 Mock 响应
    for r in &api.responses {
        let status = if r.status > 0 {
            r.status.to_string()
        } else {
            "default".to_string()
        };
        let mut desc = r.name.trim().to_string();
        if desc.is_empty() {
            desc = "响应".to_string();
        }
        let mut content = Map::new();
        if !r.body.trim().is_empty() {
            let example = serde_json::from_str::<Value>(&r.body)
                .unwrap_or_else(|_| Value::String(r.body.clone()));
            content.insert(
                r.content_type.trim().to_string(),
                json!({ "example": example }),
            );
        }
        responses.insert(status, json!({ "description": desc, "content": content }));
    }
    if responses.is_empty() {
        responses.insert(
            "200".to_string(),
            json!({ "description": format!("Mock 响应（状态码 {}）", api.mock.status) }),
        );
    }
    let mut op = json!({
        "summary": api.name,
        "description": api.description,
        "parameters": params,
        "responses": responses
    });
    if !segs.is_empty() {
        let tag = segs
            .iter()
            .map(|(n, dep)| if *dep { format!("{n}（已废弃）") } else { n.clone() })
            .collect::<Vec<_>>()
            .join("/");
        op["tags"] = json!([tag]);
    }
    match api.body.mode.as_str() {
        "json" => {
            let example = serde_json::from_str::<Value>(&api.body.raw).unwrap_or_else(|_| Value::String(api.body.raw.clone()));
            op["requestBody"] = json!({ "content": { "application/json": { "example": example } } });
        }
        "raw" => {
            op["requestBody"] = json!({ "content": { "text/plain": { "example": api.body.raw } } });
        }
        "form" => {
            let mut props = Map::new();
            for f in api.body.form.iter().filter(|f| !f.key.trim().is_empty()) {
                props.insert(
                    f.key.clone(),
                    json!({ "type": "string", "description": f.description }),
                );
            }
            op["requestBody"] = json!({
                "content": {
                    "application/x-www-form-urlencoded": {
                        "schema": { "type": "object", "properties": Value::Object(props) }
                    }
                }
            });
        }
        _ => {}
    }
    op
}

// ==================== Docsify 文档目录 ====================

/// 生成单个 Markdown 文件（导出 / 分组查看用）：根标题 + 全部接口，
/// 分组路径用「 / 」拼接显示（多级分组在单个文件里也能区分层级）
pub fn markdown_single_file(title: &str, apis: &[(Vec<(String, bool)>, ApiFile)]) -> String {
    let mut s = String::new();
    let title = title.trim();
    if !title.is_empty() {
        s.push_str(&format!("# {title}\n\n"));
    }
    // 同一分组只生成一次分组信息；分组名与文档标题相同时不再重复（避免 # 标题 与 # 分组 叠两行）
    let mut seen_groups: Vec<String> = Vec::new();
    for (segs, api) in apis {
        // 废弃分组名加标注，与文档中接口的（已废弃）标识一致
        let group = segs
            .iter()
            .map(|(n, dep)| if *dep { format!("{n}（已废弃）") } else { n.clone() })
            .collect::<Vec<_>>()
            .join(" / ");
        let g = group.trim();
        let emit = !g.is_empty() && g != title && !seen_groups.iter().any(|x| x == g);
        if emit {
            seen_groups.push(g.to_string());
        }
        // 接口所在分组（或其祖先分组）废弃 → 接口标题带「（已废弃）」标注
        let group_dep = segs.iter().any(|(_, d)| *d);
        s.push_str(&crate::markdown::render(api, if emit { &g } else { "" }, group_dep));
        s.push('\n');
    }
    s
}

/// 生成 Docsify 文档目录：返回 (相对路径, 内容) 列表，
/// 含 _sidebar.md、根 README.md（首页）与 index.html（开启 _sidebar 支持）
pub fn docsify_files(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Vec<(PathBuf, String)> {
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    let mut used: Vec<PathBuf> = Vec::new();

    // 根 README.md 是 Docsify 首页，先占位避免顶层接口重名
    used.push(PathBuf::from("README.md"));

    // 接口 .md 文件：<分组路径>/<接口名>.md（重名自动加序号）
    let mut tree: SideNode = SideNode {
        name: String::new(),
        display: String::new(),
        deprecated: false,
        apis: Vec::new(),
        children: BTreeMap::new(),
    };
    for (segs, api) in apis {
        let mut cur = &mut tree;
        let mut dep_inherit = false; // 祖先分组是否已废弃
        for (s, dep) in segs {
            // 分组目录名去掉空格（docsify 链接更稳定），显示名保留原样；
            // 分组自身或其祖先分组已废弃 → 名称加标注
            dep_inherit = dep_inherit || *dep;
            let display = if dep_inherit {
                format!("{}（已废弃）", s.trim())
            } else {
                s.trim().to_string()
            };
            let name = slug_group(s);
            cur = cur
                .children
                .entry(name.clone())
                .or_insert_with(|| SideNode {
                    name,
                    display,
                    deprecated: dep_inherit,
                    apis: Vec::new(),
                    children: BTreeMap::new(),
                });
        }
        cur.apis.push(api);
    }

    // 递归写出接口/分组文件
    write_side(&tree, PathBuf::new(), &mut files, &mut used);

    // 导航列表（根级接口 + 全部分组层级）
    let nav = side_bullets(&tree, PathBuf::new(), 0);

    // _sidebar.md：左侧导航
    let mut sidebar = String::from("# 接口文档\n\n");
    sidebar.push_str(&nav);
    files.push((PathBuf::from("_sidebar.md"), sidebar));

    // README.md：首页
    let mut readme = String::from("# 接口文档\n\n");
    readme.push_str(&nav);
    files.push((PathBuf::from("README.md"), readme));

    // index.html：Docsify 入口，开启 _sidebar 支持
    files.push((PathBuf::from("index.html"), index_html()));
    files
}

/// Docsify 入口页 HTML：加载 _sidebar.md 侧栏
fn index_html() -> String {
    r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>接口文档</title>
  <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/docsify@4/lib/themes/vue.css">
</head>
<body>
  <div id="app"></div>
  <script>
    window.$docsify = {
      name: "接口文档",
      loadSidebar: true,
      subMaxLevel: 2,
      auto2top: true
    };
  </script>
  <script src="https://cdn.jsdelivr.net/npm/docsify@4/lib/docsify.min.js"></script>
</body>
</html>"#
    .to_string()
}

struct SideNode<'a> {
    /// 目录名（已去空格、去非法字符）
    name: String,
    /// 显示名（保留原样，用于标题与链接文字）
    display: String,
    /// 分组自身或其祖先分组是否已废弃（接口继承此标注）
    deprecated: bool,
    apis: Vec<&'a ApiFile>,
    children: BTreeMap<String, SideNode<'a>>,
}

/// 分组目录名：去掉全部空白字符（空格/制表/全角空格），其余非法字符替换为 _
fn slug_group(name: &str) -> String {
    sanitize_filename(name)
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// 转成 Docsify 根目录绝对链接（前导 /，Windows 分隔符转 /）
fn root_link(dir: &Path) -> String {
    let s = dir.to_string_lossy().replace('\\', "/");
    if s.is_empty() {
        "/".to_string()
    } else {
        format!("/{s}")
    }
}

/// 递归生成分组 README.md 与接口 .md
fn write_side(
    n: &SideNode,
    dir: PathBuf,
    files: &mut Vec<(PathBuf, String)>,
    used: &mut Vec<PathBuf>,
) {
    // 接口文件
    for api in &n.apis {
        let base = if api.name.trim().is_empty() {
            "未命名接口".to_string()
        } else {
            sanitize_filename(api.name.trim())
        };
        let mut rel = dir.join(format!("{base}.md"));
        let mut i = 2;
        while used.contains(&rel) {
            rel = dir.join(format!("{base}({i}).md"));
            i += 1;
        }
        used.push(rel.clone());
        files.push((rel, crate::markdown::render(api, &n.display, n.deprecated)));
    }
    // 子分组
    for (_, c) in &n.children {
        let sub = dir.join(&c.name);
        // 分组 README.md：标题 + 子项链接（根目录绝对链接，避免 docsify 相对路径解析出错）
        let mut readme = format!("# {}\n\n", c.display);
        for api in &c.apis {
            let base = if api.name.trim().is_empty() {
                "未命名接口".to_string()
            } else {
                sanitize_filename(api.name.trim())
            };
            readme.push_str(&format!(
                "- [{}]({}/{}.md)\n",
                api.name,
                root_link(&sub),
                base
            ));
        }
        for (_, c2) in &c.children {
            readme.push_str(&format!(
                "- [{}]({}/{}/)\n",
                c2.display,
                root_link(&sub),
                c2.name
            ));
        }
        files.push((sub.join("README.md"), readme));
        write_side(c, sub, files, used);
    }
}

/// 生成侧栏/首页的嵌套列表（路径相对 Docsify 根），含根级接口与全部分组层级
fn side_bullets(n: &SideNode, dir: PathBuf, depth: usize) -> String {
    let mut out = String::new();
    let indent = "  ".repeat(depth);
    // 当前层级的接口（根级接口在此列出）
    for api in &n.apis {
        let base = if api.name.trim().is_empty() {
            "未命名接口".to_string()
        } else {
            sanitize_filename(api.name.trim())
        };
        let rel = dir.join(format!("{base}.md"));
        out.push_str(&format!(
            "{indent}- [{}]({})\n",
            api.name,
            root_link(&rel)
        ));
    }
    // 子分组
    for (_, c) in &n.children {
        let sub = dir.join(&c.name);
        out.push_str(&format!(
            "{indent}- [{}]({}/)\n",
            c.display,
            root_link(&sub)
        ));
        out.push_str(&side_bullets(c, sub, depth + 1));
    }
    out
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BodyData;

    fn sample() -> ApiFile {
        ApiFile {
            uuid: "u1".into(),
            name: "创建用户".into(),
            method: "POST".into(),
            path: "/api/users".into(),
            url: "http://example.com/api/users".into(),
            description: "创建用户".into(),
            headers: vec![],
            query: vec![],
            params: vec![],
            body: BodyData {
                mode: "json".into(),
                raw: "{\"name\":\"张三\"}".into(),
                form: vec![],
                binary_path: String::new(),
            },
            mock: crate::MockConfig::default(),
            examples: vec![],
            responses: vec![],
            doc_params: vec![],
            deprecated: false,
            protocol: "http".into(),
        }
    }

    #[test]
    fn postman_shape() {
        let apis = vec![(vec![("用户管理".to_string(), false)], sample())];
        let v = to_postman(&apis);
        assert_eq!(v["info"]["schema"].as_str().unwrap(), "https://schema.getpostman.com/json/collection/v2.1.0/collection.json");
        let item = &v["item"][0];
        assert_eq!(item["name"], "用户管理");
        assert_eq!(item["item"][0]["request"]["method"], "POST");
        assert_eq!(item["item"][0]["request"]["url"]["raw"], "http://example.com/api/users");
        assert_eq!(item["item"][0]["request"]["body"]["mode"], "raw");
    }

    #[test]
    fn openapi_shape() {
        let apis = vec![(vec![("用户管理".to_string(), false)], sample())];
        let v = to_openapi("测试", &apis);
        assert_eq!(v["openapi"], "3.0.1");
        assert_eq!(v["paths"]["/api/users"]["post"]["summary"], "创建用户");
        assert_eq!(v["paths"]["/api/users"]["post"]["tags"][0], "用户管理");
        assert!(v["paths"]["/api/users"]["post"]["requestBody"].is_object());
    }

    #[test]
    fn docsify_files_ok() {
        let apis = vec![
            (vec![("用户 管理".to_string(), false)], sample()),
            (vec![("用户 管理".to_string(), false)], sample()), // 同名接口 → 加序号
        ];
        let files = docsify_files(&apis);
        let names: Vec<String> = files
            .iter()
            .map(|(p, _)| p.to_string_lossy().replace('\\', "/"))
            .collect();
        // 分组名去掉空格：目录/链接里不含空格
        assert!(names.contains(&"用户管理/创建用户.md".to_string()));
        assert!(names.contains(&"用户管理/创建用户(2).md".to_string()));
        assert!(names.contains(&"用户管理/README.md".to_string()));
        assert!(names.contains(&"_sidebar.md".to_string()));
        assert!(names.contains(&"README.md".to_string()));
        assert!(names.contains(&"index.html".to_string()));
        let sidebar = files.iter().find(|(p, _)| p.to_string_lossy() == "_sidebar.md").unwrap().1.clone();
        assert!(sidebar.contains("[创建用户](/用户管理/创建用户.md)"), "sidebar: {sidebar}");
        // 分组 README 标题保留原名称（含空格），链接为根目录绝对链接
        let gre = files.iter().find(|(p, _)| p.to_string_lossy().replace('\\', "/") == "用户管理/README.md").unwrap().1.clone();
        assert!(gre.starts_with("# 用户 管理"), "group readme: {gre}");
        assert!(gre.contains("[创建用户](/用户管理/创建用户.md)"), "group readme: {gre}");
        let index = files.iter().find(|(p, _)| p.to_string_lossy() == "index.html").unwrap().1.clone();
        assert!(index.contains("loadSidebar: true"));
        let readme = files.iter().find(|(p, _)| p.to_string_lossy() == "README.md").unwrap().1.clone();
        assert!(readme.contains("[创建用户](/用户管理/创建用户.md)"), "readme: {readme}");
    }

    /// 单个 Markdown 文件：根标题 + 分组路径拼接 + 全部接口（分组查看/单文件导出共用）
    #[test]
    fn markdown_single_file_shape() {
        let apis = vec![
            (vec![("用户管理".to_string(), false)], sample()),
            (vec![("用户管理".to_string(), false), ("子组".to_string(), false)], sample()),
        ];
        let md = markdown_single_file("接口文档", &apis);
        assert!(md.starts_with("# 接口文档\n"), "md: {md}");
        assert!(md.contains("# 用户管理"), "md: {md}");
        assert!(md.contains("# 用户管理 / 子组"), "md: {md}");
        assert!(md.contains("## 创建用户"), "md: {md}");
    }

    /// 已废弃接口/分组：markdown 与导出接口文件中带「（已废弃）」标注
    #[test]
    fn markdown_deprecated_badges() {
        // 接口废弃 → 接口标题加标注
        let mut api = sample();
        api.deprecated = true;
        let md = crate::markdown::render(&api, "", false);
        assert!(md.contains("## 创建用户（已废弃）"), "md: {md}");

        // 分组废弃 → 单文件分组名加标注，且其下接口继承「（已废弃）」（接口自身未废弃）
        let apis = vec![(vec![("用户管理".to_string(), true)], sample())];
        let md = markdown_single_file("接口文档", &apis);
        assert!(md.contains("# 用户管理（已废弃）"), "md: {md}");
        assert!(md.contains("## 创建用户（已废弃）"), "md: {md}");

        // docsify：废弃分组名（README 标题 / 侧栏链接）带标注，接口 .md 内接口标题也带标注
        let files = docsify_files(&apis);
        let gre = files
            .iter()
            .find(|(p, _)| {
                p.to_string_lossy().replace('\\', "/") == "用户管理/README.md"
            })
            .unwrap()
            .1
            .clone();
        assert!(gre.starts_with("# 用户管理（已废弃）"), "group readme: {gre}");
        let sidebar = files
            .iter()
            .find(|(p, _)| p.to_string_lossy() == "_sidebar.md")
            .unwrap()
            .1
            .clone();
        assert!(sidebar.contains("用户管理（已废弃）"), "sidebar: {sidebar}");
        // 接口 .md 文件：分组标题带标注 + 接口标题继承标注
        let api_md = files
            .iter()
            .find(|(p, _)| {
                p.to_string_lossy().replace('\\', "/") == "用户管理/创建用户.md"
            })
            .unwrap()
            .1
            .clone();
        assert!(api_md.contains("# 用户管理（已废弃）"), "api md: {api_md}");
        assert!(api_md.contains("## 创建用户（已废弃）"), "api md: {api_md}");

        // openapi：废弃分组 tag 带标注
        let openapi = to_openapi("测试", &apis);
        assert_eq!(
            openapi["paths"]["/api/users"]["post"]["tags"][0],
            "用户管理（已废弃）"
        );
    }

    /// 同一分组下的多个接口：分组信息只生成一次
    #[test]
    fn markdown_single_file_group_heading_once() {
        let mut a = sample();
        a.name = "接口A".into();
        let mut b = sample();
        b.name = "接口B".into();
        let apis = vec![
            (vec![("用户管理".to_string(), false)], a.clone()),
            (vec![("用户管理".to_string(), false)], b.clone()),
        ];
        let md = markdown_single_file("用户管理", &apis);
        // 标题即分组名：不再重复输出 # 用户管理
        assert_eq!(md.matches("# 用户管理").count(), 1, "md: {md}");
        assert!(md.contains("## 接口A"), "md: {md}");
        assert!(md.contains("## 接口B"), "md: {md}");

        // 标题为文档名（整库导出）时：分组信息仍只出现一次
        let md2 = markdown_single_file("接口文档", &apis);
        assert_eq!(md2.matches("# 用户管理").count(), 1, "md: {md2}");
        assert!(md2.contains("## 接口A"), "md: {md2}");
        assert!(md2.contains("## 接口B"), "md: {md2}");
    }

    /// 勾选分组后前端会把分组目录 + 其下全部文件路径一起提交，后端应去重
    #[test]
    fn collect_apis_dedupes_dir_plus_files() {
        let base = std::env::temp_dir().join(format!("apim-dedupe-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let g = base.join("用户管理");
        fs::create_dir_all(&g).unwrap();
        for (name, method) in [("接口A", "GET"), ("接口B", "POST")] {
            let mut a = sample();
            a.name = name.into();
            a.method = method.into();
            fs::write(
                g.join(format!("{name}.json")),
                serde_json::to_string(&a).unwrap(),
            )
            .unwrap();
        }
        let paths = vec![
            g.to_string_lossy().to_string(),
            g.join("接口A.json").to_string_lossy().to_string(),
            g.join("接口B.json").to_string_lossy().to_string(),
        ];
        let apis = collect_apis(&base, &paths).expect("collect");
        // 目录已覆盖整棵子树，文件路径被跳过 → 恰好 2 个，不重复
        assert_eq!(apis.len(), 2);
        assert!(apis.iter().all(|(s, _)| s == &vec![("用户管理".to_string(), false)]));
        let _ = fs::remove_dir_all(&base);
    }

    /// 勾选分组时导出弹窗会同时提交外层分组与嵌套分组路径，嵌套分组不应被重复收集
    #[test]
    fn collect_apis_dedupes_nested_dirs() {
        let base = std::env::temp_dir().join(format!("apim-dedupe2-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let g = base.join("用户管理");
        let sub = g.join("子分组");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join(INFO_FILE), r#"{"name":"子分组"}"#).unwrap();
        let mut a = sample();
        a.name = "接口A".into();
        fs::write(sub.join("接口A.json"), serde_json::to_string(&a).unwrap()).unwrap();
        // 外层分组 + 嵌套分组同时被选中（导出弹窗的实际行为）
        let paths = vec![
            g.to_string_lossy().to_string(),
            sub.to_string_lossy().to_string(),
        ];
        let apis = collect_apis(&base, &paths).expect("collect");
        // 嵌套分组已随外层收集 → 恰好 1 个，不重复
        assert_eq!(apis.len(), 1);
        assert_eq!(
            apis[0].0,
            vec![
                ("用户管理".to_string(), false),
                ("子分组".to_string(), false)
            ]
        );
        let _ = fs::remove_dir_all(&base);
    }

    /// 同路径同方法的不同接口（如重名文件）在 OpenAPI 中不应互相覆盖，追加序号保留全部
    #[test]
    fn openapi_keeps_duplicate_path_method() {
        let mut a2 = sample();
        a2.name = "创建用户(2)".into();
        let apis = vec![
            (vec![("用户管理".to_string(), false)], sample()),
            (vec![("用户管理".to_string(), false)], a2),
        ];
        let v = to_openapi("测试", &apis);
        assert_eq!(v["paths"]["/api/users"]["post"]["summary"], "创建用户");
        assert_eq!(v["paths"]["/api/users (2)"]["post"]["summary"], "创建用户(2)");
    }
}

// ==================== Apifox 导出 ====================

/// 生成 Apifox 项目 JSON（apifox-project.json 结构）
pub fn to_apifox(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let mut root = PNode {
        name: "根目录".to_string(),
        apis: Vec::new(),
        children: BTreeMap::new(),
    };
    for (segs, api) in apis {
        let mut cur = &mut root;
        for (s, _dep) in segs {
            cur = cur
                .children
                .entry(s.clone())
                .or_insert_with(|| PNode {
                    name: s.clone(),
                    apis: Vec::new(),
                    children: BTreeMap::new(),
                });
        }
        cur.apis.push(api);
    }
    let mut items = Vec::new();
    for api in &root.apis {
        items.push(api_to_apifox(api));
    }
    for (_, c) in &root.children {
        items.push(pnode_to_apifox(c));
    }
    json!({
        "$schema": "https://apifox.com/schemas/apifox-project.json",
        "info": {
            "name": "API Manager 导出",
            "description": "由 API Manager 导出的 Apifox 项目",
            "version": "1.0.0"
        },
        "apiCollection": [{ "name": "根目录", "items": items }]
    })
}

fn pnode_to_apifox(n: &PNode) -> Value {
    let mut item = Vec::new();
    for api in &n.apis {
        item.push(api_to_apifox(api));
    }
    for (_, c) in &n.children {
        item.push(pnode_to_apifox(c));
    }
    json!({ "name": n.name, "items": item })
}

/// 单个接口 → Apifox api item
fn api_to_apifox(api: &ApiFile) -> Value {
    let to_param = |kv: &KeyValue| {
        json!({
            "name": kv.key,
            "type": "string",
            "required": kv.enabled,
            "enable": kv.enabled,
            "description": kv.description,
            "value": kv.value
        })
    };
    let path_params: Vec<Value> = api
        .params
        .iter()
        .filter(|p| p.enabled && !p.key.trim().is_empty())
        .map(|p| {
            json!({
                "name": p.key,
                "type": "string",
                "required": true,
                "enable": true,
                "description": p.description
            })
        })
        .collect();
    let query: Vec<Value> = api
        .query
        .iter()
        .filter(|q| q.enabled && !q.key.trim().is_empty())
        .map(&to_param)
        .collect();
    let headers: Vec<Value> = api
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .map(&to_param)
        .collect();
    let (body_type, body_params, body_examples, media_type) = match api.body.mode.as_str() {
        "json" | "raw" => {
            let examples = if api.body.raw.trim().is_empty() {
                Vec::new()
            } else {
                vec![json!({
                    "name": "默认示例",
                    "data": api.body.raw,
                    "mediaType": "application/json"
                })]
            };
            ("json", Vec::<Value>::new(), examples, "application/json")
        }
        "form" => {
            let params: Vec<Value> = api
                .body
                .form
                .iter()
                .filter(|f| f.enabled && !f.key.trim().is_empty())
                .map(|f| {
                    json!({
                        "name": f.key,
                        "type": if f.is_file { "file" } else { "text" },
                        "enable": true,
                        "required": true,
                        "description": f.description
                    })
                })
                .collect();
            ("form-data", params, Vec::new(), "multipart/form-data")
        }
        _ => ("none", Vec::new(), Vec::new(), ""),
    };
    json!({
        "name": api.name,
        "api": {
            "method": api.method.to_lowercase(),
            "path": api.path,
            "parameters": {
                "path": path_params,
                "query": query,
                "header": headers,
                "cookie": []
            },
            "requestBody": {
                "type": body_type,
                "parameters": body_params,
                "examples": body_examples,
                "mediaType": media_type
            },
            "description": api.description
        }
    })
}

// ==================== Apipost 导出 ====================

/// 生成 Apipost 项目 JSON（apis 平铺数组 + target_id/parent_id 组织树）
pub fn to_apipost(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let mut root = PNode {
        name: "根目录".to_string(),
        apis: Vec::new(),
        children: BTreeMap::new(),
    };
    for (segs, api) in apis {
        let mut cur = &mut root;
        for (s, _dep) in segs {
            cur = cur
                .children
                .entry(s.clone())
                .or_insert_with(|| PNode {
                    name: s.clone(),
                    apis: Vec::new(),
                    children: BTreeMap::new(),
                });
        }
        cur.apis.push(api);
    }
    let mut apis_out: Vec<Value> = Vec::new();
    let mut counter = 0usize;
    pnode_to_apipost(&root, "0", &mut counter, &mut apis_out);
    json!({
        "project_id": "apipost-export",
        "name": "API Manager 导出",
        "intro": "",
        "global": {},
        "models": [],
        "apis": apis_out,
        "samples": [],
        "automated_testings": []
    })
}

fn pnode_to_apipost(n: &PNode, parent_id: &str, counter: &mut usize, out: &mut Vec<Value>) {
    for api in &n.apis {
        *counter += 1;
        let id = format!("a{counter}");
        out.push(api_to_apipost(api, &id, parent_id));
    }
    for (_, c) in &n.children {
        *counter += 1;
        let id = format!("f{counter}");
        out.push(json!({
            "target_id": id,
            "project_id": "apipost-export",
            "parent_id": parent_id,
            "target_type": "folder",
            "name": c.name,
            "sort": 0,
            "request": {},
            "description": ""
        }));
        pnode_to_apipost(c, &id, counter, out);
    }
}

fn api_to_apipost(api: &ApiFile, id: &str, parent_id: &str) -> Value {
    let to_param = |kv: &KeyValue| {
        json!({
            "key": kv.key,
            "value": kv.value,
            "description": kv.description,
            "is_checked": if kv.enabled { 1 } else { 0 },
            "not_null": 0,
            "field_type": "string"
        })
    };
    let headers: Vec<Value> = api
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .map(&to_param)
        .collect();
    let query: Vec<Value> = api
        .query
        .iter()
        .filter(|q| q.enabled && !q.key.trim().is_empty())
        .map(&to_param)
        .collect();
    let restful: Vec<Value> = api
        .params
        .iter()
        .filter(|p| p.enabled && !p.key.trim().is_empty())
        .map(&to_param)
        .collect();
    let (mode, raw, form_params) = match api.body.mode.as_str() {
        "json" | "raw" => ("json", api.body.raw.clone(), Vec::<Value>::new()),
        "form" => {
            let params: Vec<Value> = api
                .body
                .form
                .iter()
                .filter(|f| f.enabled && !f.key.trim().is_empty())
                .map(|f| {
                    json!({
                        "key": f.key,
                        "value": f.value,
                        "type": if f.is_file { "file" } else { "text" },
                        "description": f.description
                    })
                })
                .collect();
            ("form-data", String::new(), params)
        }
        _ => ("none", String::new(), Vec::new()),
    };
    let url = if !api.url.trim().is_empty() {
        api.url.trim().to_string()
    } else {
        api.path.clone()
    };
    json!({
        "target_id": id,
        "project_id": "apipost-export",
        "parent_id": parent_id,
        "target_type": "api",
        "name": api.name,
        "method": api.method,
        "url": url,
        "description": api.description,
        "protocol": if api.protocol == "websocket" { "websocket" } else { "http/1.1" },
        "sort": 0,
        "request": {
            "header": { "parameter": headers },
            "query": { "query_add_equal": 1, "parameter": query },
            "restful": { "parameter": restful },
            "cookie": { "parameter": [] },
            "body": { "mode": mode, "parameter": form_params, "raw": raw }
        },
        "response": []
    })
}

// ==================== RAML 导出 ====================

/// 生成 RAML 1.0 文档（YAML Value，调用方用 serde_yaml 序列化并拼接 #%RAML 1.0 头）
pub fn to_raml(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let mut paths: Map<String, Value> = Map::new();
    let mut base_uri = String::new();
    for (_, api) in apis {
        if api.protocol == "websocket" {
            continue;
        }
        let p = if !api.path.trim().is_empty() {
            api.path.trim().to_string()
        } else {
            api.url.trim().to_string()
        };
        if p.is_empty() {
            continue;
        }
        let method = api.method.trim().to_lowercase();
        if !is_valid_method(&method) {
            continue;
        }
        if base_uri.is_empty() {
            base_uri = extract_base_url(&api.url);
        }
        let entry = paths.entry(p.clone()).or_insert_with(|| json!({}));
        let obj = entry.as_object_mut().expect("paths 条目为对象");
        obj.insert(method.clone(), api_to_raml_method(api));
    }
    let mut doc = Map::new();
    doc.insert("title".into(), json!("API Manager 导出"));
    if !base_uri.is_empty() {
        doc.insert("baseUri".into(), json!(base_uri));
    }
    doc.insert("mediaType".into(), json!("application/json"));
    for (k, v) in paths {
        doc.insert(k, v);
    }
    json!(doc)
}

/// 单个接口 → RAML method 对象
fn api_to_raml_method(api: &ApiFile) -> Value {
    let mut op = Map::new();
    if !api.description.trim().is_empty() {
        op.insert("description".into(), json!(api.description));
    }
    let query: Map<String, Value> = api
        .query
        .iter()
        .filter(|q| q.enabled && !q.key.trim().is_empty())
        .map(|q| {
            let mut p = Map::new();
            p.insert("type".into(), json!("string"));
            p.insert("required".into(), json!(false));
            if !q.value.is_empty() {
                p.insert("default".into(), json!(q.value));
            }
            if !q.description.is_empty() {
                p.insert("description".into(), json!(q.description));
            }
            (q.key.clone(), json!(p))
        })
        .collect();
    if !query.is_empty() {
        op.insert("queryParameters".into(), json!(query));
    }
    let headers: Map<String, Value> = api
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .map(|h| {
            let mut p = Map::new();
            p.insert("type".into(), json!("string"));
            p.insert("required".into(), json!(false));
            if !h.description.is_empty() {
                p.insert("description".into(), json!(h.description));
            }
            (h.key.clone(), json!(p))
        })
        .collect();
    if !headers.is_empty() {
        op.insert("headers".into(), json!(headers));
    }
    if matches!(api.body.mode.as_str(), "json" | "raw") && !api.body.raw.trim().is_empty() {
        op.insert(
            "body".into(),
            json!({ "application/json": { "example": api.body.raw } }),
        );
    }
    json!(op)
}

/// 从 URL 提取 base（scheme://host[:port]，去掉路径）
fn extract_base_url(raw: &str) -> String {
    let no_q = raw.split(['?', '#']).next().unwrap_or("");
    let Some((scheme, rest)) = no_q.split_once("://") else {
        return String::new();
    };
    let host = rest.split('/').next().unwrap_or("");
    if host.is_empty() {
        return String::new();
    }
    format!("{scheme}://{host}")
}

// ==================== WADL 导出 ====================

/// WADL 资源树节点：某路径段下的接口列表与子资源
struct WadlNode<'a> {
    apis: Vec<&'a ApiFile>,
    children: BTreeMap<String, WadlNode<'a>>,
}

/// 生成 WADL 文档（XML 字符串）
pub fn to_wadl(apis: &[(Vec<(String, bool)>, ApiFile)]) -> String {
    let mut tree = WadlNode {
        apis: Vec::new(),
        children: BTreeMap::new(),
    };
    let mut base = String::new();
    for (_, api) in apis {
        if api.protocol == "websocket" {
            continue;
        }
        let p = if !api.path.trim().is_empty() {
            api.path.trim().to_string()
        } else {
            api.url.trim().to_string()
        };
        if p.is_empty() {
            continue;
        }
        if base.is_empty() {
            base = extract_base_url(&api.url);
        }
        // 去掉 scheme://host 前缀，只保留路径段
        let path_only = p
            .split("://")
            .nth(1)
            .and_then(|r| r.split_once('/').map(|(_, rest)| format!("/{rest}")))
            .unwrap_or_else(|| p.clone());
        let segs: Vec<String> = path_only
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        let mut node = &mut tree;
        let n = segs.len();
        for (i, seg) in segs.iter().enumerate() {
            node = node.children.entry(seg.clone()).or_insert_with(|| WadlNode {
                apis: Vec::new(),
                children: BTreeMap::new(),
            });
            if i == n - 1 {
                node.apis.push(api);
            }
        }
    }
    let mut body = String::new();
    for (seg, node) in &tree.children {
        body.push_str(&wadl_node_xml(seg, node, 1));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<application xmlns=\"http://wadl.dev.java.net/2009/02\">\n  <resources base=\"{}\">\n{}{}</resources>\n</application>\n",
        xml_escape(&base),
        body,
        ""
    )
}

/// 递归生成 <resource> 元素
fn wadl_node_xml(seg: &str, node: &WadlNode, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    let mut s = format!("{pad}<resource path=\"{}\">\n", xml_escape(seg));
    for api in &node.apis {
        s.push_str(&wadl_method_xml(api, indent + 1));
    }
    for (child_seg, child) in &node.children {
        s.push_str(&wadl_node_xml(child_seg, child, indent + 1));
    }
    s.push_str(&format!("{pad}</resource>\n"));
    s
}

/// 单个接口 → <method> 元素
fn wadl_method_xml(api: &ApiFile, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    let mut s = format!("{pad}<method name=\"{}\">\n", xml_escape(&api.method));
    s.push_str(&format!("{pad}  <request>\n"));
    for h in api
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
    {
        s.push_str(&format!(
            "{pad}    <param name=\"{}\" style=\"header\" type=\"xsd:string\"/>\n",
            xml_escape(&h.key)
        ));
    }
    for q in api
        .query
        .iter()
        .filter(|q| q.enabled && !q.key.trim().is_empty())
    {
        let default = if q.value.is_empty() {
            String::new()
        } else {
            format!(" default=\"{}\"", xml_escape(&q.value))
        };
        s.push_str(&format!(
            "{pad}    <param name=\"{}\" style=\"query\" type=\"xsd:string\"{}/>\n",
            xml_escape(&q.key),
            default
        ));
    }
    for p in api
        .params
        .iter()
        .filter(|p| p.enabled && !p.key.trim().is_empty())
    {
        s.push_str(&format!(
            "{pad}    <param name=\"{}\" style=\"path\" type=\"xsd:string\"/>\n",
            xml_escape(&p.key)
        ));
    }
    if matches!(api.body.mode.as_str(), "json" | "raw") && !api.body.raw.trim().is_empty() {
        s.push_str(&format!("{pad}    <representation mediaType=\"application/json\"/>\n"));
    }
    s.push_str(&format!("{pad}  </request>\n"));
    s.push_str(&format!("{pad}  <response status=\"200\"/>\n"));
    s.push_str(&format!("{pad}</method>\n"));
    s
}

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

/// 生成 YApi 原生导出格式（分组树 + api 对象，YApi 可导入）
pub fn to_yapi(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let mut tree = PNode {
        name: "root".into(),
        apis: Vec::new(),
        children: BTreeMap::new(),
    };
    for (segs, api) in apis {
        if segs.is_empty() {
            // 根级接口（无分组）直接输出为顶层接口项
            tree.apis.push(api);
            continue;
        }
        let mut node = &mut tree;
        for (seg, _dep) in segs {
            node = node
                .children
                .entry(seg.clone())
                .or_insert_with(|| PNode {
                    name: seg.clone(),
                    apis: Vec::new(),
                    children: BTreeMap::new(),
                });
        }
        node.apis.push(api);
    }
    let mut out = Vec::new();
    for api in &tree.apis {
        out.push(api_to_yapi(api));
    }
    for (_, c) in &tree.children {
        out.push(pnode_to_yapi(c));
    }
    json!(out)
}

/// PNode → YApi 分组节点（含 children 与 api 接口项）
fn pnode_to_yapi(n: &PNode) -> Value {
    let mut children = Vec::new();
    for api in &n.apis {
        children.push(api_to_yapi(api));
    }
    for (_, c) in &n.children {
        children.push(pnode_to_yapi(c));
    }
    json!({
        "name": n.name,
        "desc": "",
        "children": children,
    })
}

/// 单个接口 → YApi 接口节点 { name, api: {...} }
fn api_to_yapi(api: &ApiFile) -> Value {
    let path = path_to_yapi(&api.path);
    let req_query: Vec<Value> = api
        .query
        .iter()
        .filter(|q| q.enabled && !q.key.trim().is_empty())
        .map(|q| {
            json!({
                "name": q.key,
                "value": q.value,
                "desc": q.description,
                "required": false,
                "example": q.value,
                "type": "text"
            })
        })
        .collect();
    let req_headers: Vec<Value> = api
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .map(|h| {
            json!({
                "name": h.key,
                "value": h.value,
                "desc": h.description,
                "required": false
            })
        })
        .collect();
    let (req_body_type, req_body_other, req_body_form) = match api.body.mode.as_str() {
        "json" | "raw" => (
            "json",
            api.body.raw.clone(),
            Vec::<Value>::new(),
        ),
        "form" => {
            let form: Vec<Value> = api
                .body
                .form
                .iter()
                .filter(|f| f.enabled && !f.key.trim().is_empty())
                .map(|f| {
                    json!({
                        "name": f.key,
                        "value": f.value,
                        "type": if f.is_file { "file" } else { "text" },
                        "desc": f.description,
                        "required": false
                    })
                })
                .collect();
            ("form", String::new(), form)
        }
        _ => ("null", String::new(), Vec::new()),
    };
    let res_body = api
        .responses
        .first()
        .map(|r| r.body.clone())
        .unwrap_or_default();
    let res_body_type = if res_body.trim().is_empty() {
        "null"
    } else {
        "json"
    };
    json!({
        "name": api.name,
        "api": {
            "method": api.method,
            "path": path,
            "title": api.name,
            "desc": api.description,
            "req_query": req_query,
            "req_headers": req_headers,
            "req_body_type": req_body_type,
            "req_body_other": req_body_other,
            "req_body_form": req_body_form,
            "res_body_type": res_body_type,
            "res_body": res_body,
            "protocol": if api.protocol == "websocket" { "ws" } else { "http" }
        }
    })
}

/// 路径参数 {id} → :id（YApi 语法）
fn path_to_yapi(p: &str) -> String {
    let mut out = String::with_capacity(p.len());
    let mut chars = p.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            let mut var = String::new();
            for c2 in chars.by_ref() {
                if c2 == '}' {
                    break;
                }
                var.push(c2);
            }
            if !var.is_empty() {
                out.push(':');
                out.push_str(&var);
            }
        } else {
            out.push(c);
        }
    }
    out
}

// ==================== Eolink 导出 ====================

/// 生成 Eolink 导出格式（apiGroupList 分组树 + 接口对象）
pub fn to_eolink(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let mut root = PNode {
        name: "root".into(),
        apis: Vec::new(),
        children: BTreeMap::new(),
    };
    for (segs, api) in apis {
        let mut node = &mut root;
        for (s, _dep) in segs {
            node = node.children.entry(s.clone()).or_insert_with(|| PNode {
                name: s.clone(),
                apis: Vec::new(),
                children: BTreeMap::new(),
            });
        }
        node.apis.push(api);
    }
    let mut next_id = 0i64;
    let mut groups = Vec::new();
    // 根级接口 → 「未分组」组
    if !root.apis.is_empty() {
        next_id += 1;
        let api_list: Vec<Value> = root.apis.iter().map(|a| api_to_eolink_api(a)).collect();
        groups.push(json!({
            "groupID": next_id,
            "groupName": "未分组",
            "parentGroupID": 0,
            "sort": 0,
            "apiList": api_list,
            "childGroupList": []
        }));
    }
    for (_, c) in &root.children {
        groups.push(eolink_pnode(&mut next_id, 0, c));
    }
    json!({
        "exportVersion": "1.0",
        "projectInfo": {
            "projectName": "API Manager 导出",
            "projectDesc": "",
            "projectVersion": "1.0.0"
        },
        "apiGroupList": groups,
        "environmentList": [],
        "dataStructureList": [],
        "statusCodeList": [],
        "projectDocList": []
    })
}

/// PNode → Eolink 分组（分配 groupID/parentGroupID）
fn eolink_pnode(next: &mut i64, parent: i64, n: &PNode) -> Value {
    *next += 1;
    let gid = *next;
    let api_list: Vec<Value> = n.apis.iter().map(|a| api_to_eolink_api(a)).collect();
    let child_groups: Vec<Value> = n
        .children
        .iter()
        .map(|(_, c)| eolink_pnode(next, gid, c))
        .collect();
    json!({
        "groupID": gid,
        "groupName": n.name,
        "parentGroupID": parent,
        "sort": 0,
        "apiList": api_list,
        "childGroupList": child_groups
    })
}

/// 单个接口 → Eolink API 对象
fn api_to_eolink_api(api: &ApiFile) -> Value {
    let (api_uri, protocol) = if api.protocol == "websocket" {
        (api.path.clone(), "WS".to_string())
    } else {
        (api.path.clone(), "HTTP".to_string())
    };
    let req_headers: Vec<Value> = api
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .map(|h| {
            json!({
                "key": h.key,
                "type": "string",
                "isRequired": 1,
                "example": h.value,
                "mock": "",
                "desc": h.description
            })
        })
        .collect();
    let req_query: Vec<Value> = api
        .query
        .iter()
        .filter(|q| q.enabled && !q.key.trim().is_empty())
        .map(|q| {
            json!({
                "key": q.key,
                "type": "string",
                "isRequired": 1,
                "example": q.value,
                "mock": "",
                "desc": q.description
            })
        })
        .collect();
    let req_rest: Vec<Value> = api
        .params
        .iter()
        .filter(|p| p.enabled && !p.key.trim().is_empty())
        .map(|p| {
            json!({
                "key": p.key,
                "type": "string",
                "isRequired": 1,
                "example": p.value,
                "mock": "",
                "desc": p.description
            })
        })
        .collect();
    let (req_body_type, req_body_json, req_body_form) = match api.body.mode.as_str() {
        "json" => {
            let list = parse_json_to_eolink_list(&api.body.raw);
            ("json", list, Vec::<Value>::new())
        }
        "form" => {
            let form: Vec<Value> = api
                .body
                .form
                .iter()
                .filter(|f| f.enabled && !f.key.trim().is_empty())
                .map(|f| {
                    json!({
                        "key": f.key,
                        "type": if f.is_file { "file" } else { "text" },
                        "isRequired": 1,
                        "example": f.value,
                        "mock": "",
                        "desc": f.description
                    })
                })
                .collect();
            ("x-www-form-urlencoded", Vec::<Value>::new(), form)
        }
        "raw" => ("raw", Vec::<Value>::new(), Vec::<Value>::new()),
        _ => ("", Vec::<Value>::new(), Vec::<Value>::new()),
    };
    let req_body_raw = if api.body.mode == "raw" {
        api.body.raw.clone()
    } else {
        String::new()
    };
    let response_info: Vec<Value> = api
        .responses
        .iter()
        .map(|r| {
            let list = parse_json_to_eolink_list(&r.body);
            json!({
                "responseName": r.name,
                "responseCode": r.status,
                "responseContentType": if r.content_type.contains("json") { "json" } else { "raw" },
                "responseBodyJsonList": list
            })
        })
        .collect();
    json!({
        "apiID": format!("api_{}", api.uuid),
        "apiName": api.name,
        "apiMethod": api.method,
        "apiUri": api_uri,
        "apiProtocol": protocol,
        "apiStatus": "已完成",
        "apiTagList": [],
        "apiDesc": api.description,
        "apiNote": "",
        "requestInfo": {
            "requestHeaderList": req_headers,
            "requestQueryList": req_query,
            "requestRestList": req_rest,
            "requestBodyType": req_body_type,
            "requestBodyJsonList": req_body_json,
            "requestBodyFormList": req_body_form,
            "requestBodyRaw": req_body_raw
        },
        "responseInfoList": response_info,
        "testCaseList": []
    })
}

/// 解析 JSON 字符串为 Eolink 字段列表（嵌套 children）
fn parse_json_to_eolink_list(raw: &str) -> Vec<Value> {
    match serde_json::from_str::<Value>(raw.trim()) {
        Ok(Value::Object(m)) => m
            .iter()
            .map(|(k, v)| json_value_to_eolink_item(k, v))
            .collect(),
        _ => Vec::new(),
    }
}

/// 单个 JSON 值 → Eolink 字段项
fn json_value_to_eolink_item(key: &str, v: &Value) -> Value {
    let (ty, example, children) = match v {
        Value::Object(m) => {
            let kids: Vec<Value> = m.iter().map(|(k, x)| json_value_to_eolink_item(k, x)).collect();
            ("object", json!({}), kids)
        }
        Value::Array(a) => {
            let kids: Vec<Value> = a
                .iter()
                .enumerate()
                .map(|(i, x)| json_value_to_eolink_item(&format!("[{i}]"), x))
                .collect();
            ("array", json!([]), kids)
        }
        Value::String(s) => ("string", json!(s), Vec::new()),
        Value::Number(n) => ("number", json!(n), Vec::new()),
        Value::Bool(b) => ("boolean", json!(b), Vec::new()),
        Value::Null => ("string", json!(""), Vec::new()),
    };
    json!({
        "key": key,
        "type": ty,
        "isRequired": 1,
        "example": example,
        "mock": "",
        "desc": "",
        "children": children
    })
}

// ==================== Insomnia 导出 ====================

/// 生成 Insomnia 导出格式（collection.insomnia.rest/5.0 YAML）
pub fn to_insomnia(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let mut root = PNode {
        name: "root".into(),
        apis: Vec::new(),
        children: BTreeMap::new(),
    };
    for (segs, api) in apis {
        let mut node = &mut root;
        for (s, _dep) in segs {
            node = node.children.entry(s.clone()).or_insert_with(|| PNode {
                name: s.clone(),
                apis: Vec::new(),
                children: BTreeMap::new(),
            });
        }
        node.apis.push(api);
    }
    let mut children = Vec::new();
    for api in &root.apis {
        children.push(insomnia_request_value(api));
    }
    for (_, c) in &root.children {
        children.push(insomnia_folder_value(c));
    }
    json!({
        "type": "collection.insomnia.rest/5.0",
        "name": "API Manager 导出",
        "meta": {
            "id": "coll_export",
            "created": "2026-01-01T00:00:00.000Z",
            "modified": "2026-01-01T00:00:00.000Z"
        },
        "children": children,
        "environment": {
            "baseUrl": ""
        }
    })
}

/// 分组 → Insomnia 文件夹节点
fn insomnia_folder_value(n: &PNode) -> Value {
    let mut children = Vec::new();
    for api in &n.apis {
        children.push(insomnia_request_value(api));
    }
    for (_, c) in &n.children {
        children.push(insomnia_folder_value(c));
    }
    json!({
        "name": n.name,
        "meta": { "id": format!("fld_{}", n.name) },
        "children": children
    })
}

/// 接口 → Insomnia 请求节点
fn insomnia_request_value(api: &ApiFile) -> Value {
    let is_ws = api.protocol == "websocket";
    let url = if is_ws {
        api.path.clone()
    } else {
        format!("{{{{baseUrl}}}}{}", api.path)
    };
    // 收集 Bearer token（如存在）
    let mut token = String::new();
    let mut headers: Vec<Value> = Vec::new();
    for h in api.headers.iter().filter(|h| h.enabled && !h.key.trim().is_empty()) {
        if h.key.eq_ignore_ascii_case("authorization") && h.value.starts_with("Bearer ") {
            token = h.value.trim_start_matches("Bearer ").trim().to_string();
            continue; // 交给 authentication 表达
        }
        headers.push(json!({ "name": h.key, "value": h.value }));
    }
    let parameters: Vec<Value> = api
        .query
        .iter()
        .filter(|q| q.enabled && !q.key.trim().is_empty())
        .map(|q| json!({ "name": q.key, "value": q.value }))
        .collect();
    let body = match api.body.mode.as_str() {
        "json" => json!({ "mimeType": "application/json", "text": api.body.raw }),
        "form" => {
            // 拼接 urlencoded 文本
            let pairs: Vec<String> = api
                .body
                .form
                .iter()
                .filter(|f| f.enabled && !f.key.trim().is_empty())
                .map(|f| format!("{}={}", f.key, f.value))
                .collect();
            json!({ "mimeType": "application/x-www-form-urlencoded", "text": pairs.join("&") })
        }
        "raw" => json!({ "mimeType": "text/plain", "text": api.body.raw }),
        _ => Value::Null,
    };
    let mut req = json!({
        "name": api.name,
        "meta": { "id": format!("req_{}", api.uuid) },
        "url": url,
        "method": api.method,
        "body": body,
        "headers": headers,
        "parameters": parameters,
        "authentication": { "type": "none" }
    });
    if !token.is_empty() {
        req["authentication"] = json!({ "type": "bearer", "token": token });
    }
    req
}

// ==================== JMeter 导出 ====================

/// 生成 JMeter 测试计划 XML（.jmx）
pub fn to_jmeter(apis: &[(Vec<(String, bool)>, ApiFile)]) -> String {
    let mut root = PNode {
        name: "root".into(),
        apis: Vec::new(),
        children: BTreeMap::new(),
    };
    for (segs, api) in apis {
        let mut node = &mut root;
        for (s, _dep) in segs {
            node = node.children.entry(s.clone()).or_insert_with(|| PNode {
                name: s.clone(),
                apis: Vec::new(),
                children: BTreeMap::new(),
            });
        }
        node.apis.push(api);
    }
    let mut groups_xml = String::new();
    // 根级接口 → 「API Manager」线程组
    if !root.apis.is_empty() {
        groups_xml.push_str(&jmeter_thread_group("API Manager", &root.apis));
    }
    for (_, c) in &root.children {
        groups_xml.push_str(&jmeter_pnode_thread_group(c, &c.name));
    }
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<jmeterTestPlan version=\"1.2\" properties=\"5.0\" jmeter=\"5.6.3\">\n");
    out.push_str("  <hashTree>\n");
    out.push_str("    <TestPlan guiclass=\"TestPlanGui\" testclass=\"TestPlan\" testname=\"API Manager 导出\" enabled=\"true\">\n");
    out.push_str("      <elementProp name=\"TestPlan.user_defined_variables\" elementType=\"Arguments\" guiclass=\"ArgumentsPanel\" testclass=\"Arguments\" testname=\"用户定义的变量\" enabled=\"true\">\n");
    out.push_str("        <collectionProp name=\"Arguments.arguments\">\n");
    out.push_str("          <elementProp name=\"host\" elementType=\"Argument\">\n");
    out.push_str("            <stringProp name=\"Argument.name\">host</stringProp>\n");
    out.push_str("            <stringProp name=\"Argument.value\">http://localhost</stringProp>\n");
    out.push_str("            <stringProp name=\"Argument.metadata\">=</stringProp>\n");
    out.push_str("          </elementProp>\n");
    out.push_str("        </collectionProp>\n");
    out.push_str("      </elementProp>\n");
    out.push_str("      <stringProp name=\"TestPlan.user_define_classpath\"></stringProp>\n");
    out.push_str("    </TestPlan>\n");
    out.push_str("    <hashTree>\n");
    out.push_str(&groups_xml);
    out.push_str("    </hashTree>\n");
    out.push_str("  </hashTree>\n");
    out.push_str("</jmeterTestPlan>\n");
    out
}

/// 分组（含嵌套）→ 线程组（完整路径作 testname），嵌套分组再生成子线程组
fn jmeter_pnode_thread_group(n: &PNode, full_path: &str) -> String {
    let mut out = String::new();
    if !n.apis.is_empty() {
        out.push_str(&jmeter_thread_group(full_path, &n.apis));
    }
    for (_, c) in &n.children {
        let child_path = format!("{full_path} / {}", c.name);
        out.push_str(&jmeter_pnode_thread_group(c, &child_path));
    }
    out
}

/// 一组接口 → ThreadGroup + hashTree（每个接口前带独立 HeaderManager）
fn jmeter_thread_group(name: &str, apis: &[&ApiFile]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "      <ThreadGroup guiclass=\"ThreadGroupGui\" testclass=\"ThreadGroup\" testname=\"{}\" enabled=\"true\">\n",
        xml_escape(name)
    ));
    out.push_str("        <stringProp name=\"ThreadGroup.on_sample_error\">continue</stringProp>\n");
    out.push_str("        <elementProp name=\"ThreadGroup.main_controller\" elementType=\"LoopController\" guiclass=\"LoopControlPanel\" testclass=\"LoopController\" testname=\"循环控制器\" enabled=\"true\">\n");
    out.push_str("          <boolProp name=\"LoopController.continue_forever\">false</boolProp>\n");
    out.push_str("          <stringProp name=\"LoopController.loops\">1</stringProp>\n");
    out.push_str("        </elementProp>\n");
    out.push_str("        <stringProp name=\"ThreadGroup.num_threads\">1</stringProp>\n");
    out.push_str("        <stringProp name=\"ThreadGroup.ramp_time\">1</stringProp>\n");
    out.push_str("        <boolProp name=\"ThreadGroup.scheduler\">false</boolProp>\n");
    out.push_str("        <stringProp name=\"ThreadGroup.duration\"></stringProp>\n");
    out.push_str("        <stringProp name=\"ThreadGroup.delay\"></stringProp>\n");
    out.push_str("      </ThreadGroup>\n");
    out.push_str("      <hashTree>\n");
    for api in apis {
        out.push_str(&jmeter_sampler(api));
    }
    out.push_str("      </hashTree>\n");
    out
}

/// 接口 → HeaderManager（如有头）+ HTTPSamplerProxy
fn jmeter_sampler(api: &ApiFile) -> String {
    let mut out = String::new();
    let enabled_headers: Vec<&KeyValue> = api
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .collect();
    if !enabled_headers.is_empty() {
        out.push_str("        <HeaderManager guiclass=\"HeaderPanel\" testclass=\"HeaderManager\" testname=\"HTTP信息头管理器\" enabled=\"true\">\n");
        out.push_str("          <collectionProp name=\"HeaderManager.headers\">\n");
        for h in &enabled_headers {
            out.push_str(&format!(
                "            <elementProp name=\"\" elementType=\"Header\">\n              <stringProp name=\"Header.name\">{}</stringProp>\n              <stringProp name=\"Header.value\">{}</stringProp>\n            </elementProp>\n",
                xml_escape(&h.key),
                xml_escape(&h.value)
            ));
        }
        out.push_str("          </collectionProp>\n");
        out.push_str("        </HeaderManager>\n");
        out.push_str("        <hashTree/>\n");
    }
    let method = if api.method.is_empty() {
        "GET".to_string()
    } else {
        api.method.to_uppercase()
    };
    let is_ws = api.protocol == "websocket";
    let (domain, path) = if is_ws {
        // WS：domain 为空，path 放完整地址
        ("".to_string(), api.path.clone())
    } else {
        ("${host}".to_string(), jmeter_path_with_query(api))
    };
    out.push_str(&format!(
        "        <HTTPSamplerProxy guiclass=\"HttpSamplerGui\" testclass=\"HTTPSamplerProxy\" testname=\"{}\" enabled=\"true\">\n",
        xml_escape(&api.name)
    ));
    // Arguments：body
    let args_xml = match api.body.mode.as_str() {
        "json" => {
            if api.body.raw.trim().is_empty() {
                String::new()
            } else {
                format!(
                    "          <elementProp name=\"HTTPsampler.Arguments\" elementType=\"Arguments\" guiclass=\"HTTPArgumentsPanel\" testclass=\"Arguments\" testname=\"用户定义的变量\" enabled=\"true\">\n            <collectionProp name=\"Arguments.arguments\">\n              <elementProp name=\"\" elementType=\"HTTPArgument\">\n                <boolProp name=\"HTTPArgument.always_encode\">false</boolProp>\n                <stringProp name=\"Argument.value\">{}</stringProp>\n                <stringProp name=\"Argument.metadata\">=</stringProp>\n                <boolProp name=\"HTTPArgument.use_equals\">true</boolProp>\n              </elementProp>\n            </collectionProp>\n          </elementProp>\n",
                    xml_escape(&api.body.raw)
                )
            }
        }
        "form" => {
            let fields: Vec<String> = api
                .body
                .form
                .iter()
                .filter(|f| f.enabled && !f.key.trim().is_empty())
                .map(|f| {
                    format!(
                        "              <elementProp name=\"{}\" elementType=\"HTTPArgument\">\n                <boolProp name=\"HTTPArgument.always_encode\">false</boolProp>\n                <stringProp name=\"Argument.value\">{}</stringProp>\n                <stringProp name=\"Argument.metadata\">=</stringProp>\n                <boolProp name=\"HTTPArgument.use_equals\">false</boolProp>\n              </elementProp>\n",
                        xml_escape(&f.key),
                        xml_escape(&f.value)
                    )
                })
                .collect();
            if fields.is_empty() {
                String::new()
            } else {
                format!(
                    "          <elementProp name=\"HTTPsampler.Arguments\" elementType=\"Arguments\" guiclass=\"HTTPArgumentsPanel\" testclass=\"Arguments\" testname=\"用户定义的变量\" enabled=\"true\">\n            <collectionProp name=\"Arguments.arguments\">\n{}            </collectionProp>\n          </elementProp>\n",
                    fields.join("")
                )
            }
        }
        "raw" => {
            if api.body.raw.trim().is_empty() {
                String::new()
            } else {
                format!(
                    "          <boolProp name=\"HTTPSampler.postBodyRaw\">true</boolProp>\n          <stringProp name=\"HTTPSampler.raw_body\">{}</stringProp>\n",
                    xml_escape(&api.body.raw)
                )
            }
        }
        _ => String::new(),
    };
    out.push_str(&args_xml);
    out.push_str(&format!(
        "          <stringProp name=\"HTTPSampler.domain\">{}</stringProp>\n",
        xml_escape(&domain)
    ));
    out.push_str("          <stringProp name=\"HTTPSampler.port\"></stringProp>\n");
    out.push_str("          <stringProp name=\"HTTPSampler.protocol\"></stringProp>\n");
    out.push_str(&format!(
        "          <stringProp name=\"HTTPSampler.path\">{}</stringProp>\n",
        xml_escape(&path)
    ));
    out.push_str(&format!(
        "          <stringProp name=\"HTTPSampler.method\">{}</stringProp>\n",
        xml_escape(&method)
    ));
    out.push_str("          <boolProp name=\"HTTPSampler.follow_redirects\">true</boolProp>\n");
    out.push_str("          <boolProp name=\"HTTPSampler.use_keepalive\">true</boolProp>\n");
    out.push_str("        </HTTPSamplerProxy>\n");
    out.push_str("        <hashTree/>\n");
    out
}

/// 路径 + 查询参数拼成 JMeter path
fn jmeter_path_with_query(api: &ApiFile) -> String {
    let mut path = api.path.clone();
    let enabled_query: Vec<&KeyValue> = api
        .query
        .iter()
        .filter(|q| q.enabled && !q.key.trim().is_empty())
        .collect();
    if !enabled_query.is_empty() {
        if !path.contains('?') {
            path.push('?');
        } else if !path.ends_with('?') && !path.ends_with('&') {
            path.push('&');
        }
        let pairs: Vec<String> = enabled_query
            .iter()
            .map(|q| format!("{}={}", q.key, q.value))
            .collect();
        path.push_str(&pairs.join("&"));
    }
    path
}

// ==================== apiDoc 导出 ====================

/// 导出 apiDoc 格式（api_project.json + api_data.json 两个文件内容）
pub fn to_apidoc(apis: &[(Vec<(String, bool)>, ApiFile)]) -> (Value, Value) {
    let mut root = PNode {
        name: "root".into(),
        apis: Vec::new(),
        children: BTreeMap::new(),
    };
    for (segs, api) in apis {
        let mut node = &mut root;
        for (s, _dep) in segs {
            node = node.children.entry(s.clone()).or_insert_with(|| PNode {
                name: s.clone(),
                apis: Vec::new(),
                children: BTreeMap::new(),
            });
        }
        node.apis.push(api);
    }
    // 顶层分组列表（保持顺序）
    let top_groups: Vec<&PNode> = root.children.values().collect();
    let root_name = top_groups.first().map(|n| n.name.as_str()).unwrap_or("API 文档");
    let mut groups: Vec<Value> = Vec::new();
    let mut group_order: Vec<String> = Vec::new();
    if !top_groups.is_empty() {
        for g in &top_groups {
            groups.push(json!({
                "name": g.name,
                "title": g.name,
                "description": "",
            }));
            group_order.push(g.name.clone());
        }
    }
    if !root.apis.is_empty() {
        groups.push(json!({
            "name": "未分组",
            "title": "未分组",
            "description": "",
        }));
        group_order.push("未分组".to_string());
    }
    // apis
    let mut apis_out: Vec<Value> = Vec::new();
    for g in &top_groups {
        for api in &g.apis {
            apis_out.push(api_to_apidoc(api, &g.name));
        }
    }
    for api in &root.apis {
        apis_out.push(api_to_apidoc(api, "未分组"));
    }
    let project = json!({
        "name": root_name,
        "version": "1.0.0",
        "description": "",
        "title": "API接口文档",
        "url": "/api",
        "sampleUrl": "",
        "header": { "title": "", "content": "" },
        "footer": { "title": "", "content": "" },
        "template": { "withCompare": true, "withGenerator": true, "withEditor": false },
        "order": group_order,
        "exclude": [],
        "language": "zh-cn"
    });
    let data = json!({
        "groups": groups,
        "defines": [],
        "apis": apis_out
    });
    (project, data)
}

/// 从 docParams 查找字段类型
fn apidoc_doc_type(api: &ApiFile, source: &str, key: &str) -> String {
    api.doc_params
        .iter()
        .find(|d| d.source == source && d.key == key)
        .map(|d| d.r#type.clone())
        .unwrap_or_else(|| "String".to_string())
}

/// 构造 {field, type, required, description} 字段对象
fn apidoc_field_obj(field: &str, ty: &str, required: bool, description: &str) -> Value {
    json!({
        "field": field,
        "type": ty,
        "required": required,
        "description": description
    })
}

fn apidoc_kv_fields(api: &ApiFile, source: &str, kv: &[KeyValue]) -> Vec<Value> {
    kv.iter()
        .filter(|k| k.enabled && !k.key.trim().is_empty())
        .map(|k| {
            apidoc_field_obj(
                k.key.trim(),
                &apidoc_doc_type(api, source, k.key.trim()),
                true,
                &k.description,
            )
        })
        .collect()
}

/// JSON 值 → 点分字段列表
fn apidoc_json_to_fields(prefix: &str, v: &Value, out: &mut Vec<Value>) {
    match v {
        Value::Object(m) => {
            for (k, val) in m {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                if val.is_object() {
                    apidoc_json_to_fields(&key, val, out);
                } else if val.is_array() {
                    let arr = val.as_array().unwrap();
                    let elem_ty = if arr.is_empty() {
                        "Object".to_string()
                    } else {
                        apidoc_type_str(&arr[0])
                    };
                    out.push(apidoc_field_obj(&key, &format!("{elem_ty}[]"), true, ""));
                    if let Some(first) = arr.first() {
                        if first.is_object() {
                            for (sk, sv) in first.as_object().unwrap() {
                                apidoc_json_to_fields(&format!("{key}[].{sk}"), sv, out);
                            }
                        }
                    }
                } else {
                    out.push(apidoc_field_obj(&key, &apidoc_type_str(val), true, ""));
                }
            }
        }
        _ => {
            let key = if prefix.is_empty() { "body" } else { prefix };
            out.push(apidoc_field_obj(key, &apidoc_type_str(v), true, ""));
        }
    }
}

fn apidoc_type_str(v: &Value) -> String {
    match v {
        Value::Number(n) => {
            if n.as_i64().is_some() {
                "Number".to_string()
            } else {
                "Float".to_string()
            }
        }
        Value::Bool(_) => "Boolean".to_string(),
        Value::Array(_) => "List".to_string(),
        Value::Object(_) => "Object".to_string(),
        _ => "String".to_string(),
    }
}

/// ApiFile → apiDoc api 对象
fn api_to_apidoc(api: &ApiFile, group_name: &str) -> Value {
    let mut parameter_fields = serde_json::Map::new();
    // body 字段
    let mut body_fields: Vec<Value> = Vec::new();
    match api.body.mode.as_str() {
        "json" => {
            if !api.body.raw.trim().is_empty() {
                if let Ok(v) = serde_json::from_str::<Value>(&api.body.raw) {
                    apidoc_json_to_fields("", &v, &mut body_fields);
                }
            }
        }
        "form" => {
            body_fields = apidoc_kv_fields(api, "body", &api.body.form);
        }
        _ => {
            if !api.body.raw.trim().is_empty() {
                body_fields.push(apidoc_field_obj("body", "String", true, ""));
            }
        }
    }
    if !body_fields.is_empty() {
        parameter_fields.insert("Parameter".to_string(), Value::Array(body_fields));
    }
    // query 字段
    let query_fields = apidoc_kv_fields(api, "query", &api.query);
    if !query_fields.is_empty() {
        parameter_fields.insert("Query".to_string(), Value::Array(query_fields));
    }
    // header 字段
    let header_fields = apidoc_kv_fields(api, "header", &api.headers);
    let header = if header_fields.is_empty() {
        Value::Null
    } else {
        json!({ "fields": { "Header": header_fields } })
    };
    // 响应
    let mut success_examples: Vec<Value> = Vec::new();
    let mut error_examples: Vec<Value> = Vec::new();
    for r in &api.responses {
        if r.body.trim().is_empty() {
            continue;
        }
        if r.status == 0 || r.status >= 400 {
            error_examples.push(json!({ "title": r.name, "content": r.body }));
        } else {
            success_examples.push(json!({ "title": r.name, "content": r.body }));
        }
    }
    let success = if success_examples.is_empty() {
        Value::Null
    } else {
        json!({ "examples": success_examples })
    };
    let error = if error_examples.is_empty() {
        Value::Null
    } else {
        json!({ "examples": error_examples })
    };
    // path {id} → :id
    let url = api.path.replace('{', ":").replace('}', "");
    json!({
        "group": group_name,
        "name": api.name,
        "title": api.name,
        "description": api.description,
        "method": api.method.to_uppercase(),
        "url": url,
        "parameter": { "fields": parameter_fields },
        "header": header,
        "success": success,
        "error": error,
        "successExamples": success_examples
    })
}
// ==================== 批量格式导出（10 种） ====================

struct ExtraGroup<'a> {
    name: String,
    apis: Vec<&'a ApiFile>,
}

/// 建立分组树（顶层分组 + 根级接口归入「未分组」）
fn extra_build_tree<'a>(apis: &'a [(Vec<(String, bool)>, ApiFile)]) -> (Vec<ExtraGroup<'a>>, String) {
    let mut root = PNode {
        name: "root".into(),
        apis: Vec::new(),
        children: BTreeMap::new(),
    };
    for (segs, api) in apis {
        let mut node = &mut root;
        for (s, _dep) in segs {
            node = node.children.entry(s.clone()).or_insert_with(|| PNode {
                name: s.clone(),
                apis: Vec::new(),
                children: BTreeMap::new(),
            });
        }
        node.apis.push(api);
    }
    let root_name = root
        .children
        .values()
        .next()
        .map(|n| n.name.clone())
        .unwrap_or_else(|| "API 文档".to_string());
    let mut groups: Vec<ExtraGroup> = root
        .children
        .values()
        .map(|n| ExtraGroup {
            name: n.name.clone(),
            apis: n.apis.clone(),
        })
        .collect();
    if !root.apis.is_empty() {
        groups.push(ExtraGroup {
            name: "未分组".to_string(),
            apis: root.apis.clone(),
        });
    }
    (groups, root_name)
}

fn extra_kv_enabled(kv: &KeyValue) -> bool {
    kv.enabled && !kv.key.trim().is_empty()
}

// ---------- apiDog ----------

pub fn to_apidog(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let (top_groups, root_name) = extra_build_tree(apis);
    let mut folders: Vec<Value> = Vec::new();
    let mut sort = 1i64;
    for g in &top_groups {
        let mut group_apis: Vec<Value> = Vec::new();
        for api in &g.apis {
            group_apis.push(apidog_api_out(api));
        }
        folders.push(json!({
            "name": g.name,
            "description": "",
            "sort": sort,
            "apis": group_apis,
        }));
        sort += 1;
    }
    json!({
        "version": "1.0",
        "projectMeta": { "name": root_name, "description": "", "maintainer": "", "createdAt": "" },
        "environments": [],
        "globalParams": [],
        "folders": folders,
    })
}

fn apidog_api_out(api: &ApiFile) -> Value {
    let mut headers: Vec<Value> = Vec::new();
    for h in api.headers.iter().filter(|h| extra_kv_enabled(h)) {
        headers.push(json!({ "key": h.key, "value": h.value, "description": h.description }));
    }
    let mut query_out: Vec<Value> = Vec::new();
    for q in api.query.iter().filter(|q| extra_kv_enabled(q)) {
        query_out.push(json!({ "key": q.key, "value": q.value, "description": q.description }));
    }
    let mut request_body = json!({ "mode": "none" });
    match api.body.mode.as_str() {
        "json" => {
            let example = serde_json::from_str::<Value>(&api.body.raw).unwrap_or(Value::Null);
            if !example.is_null() {
                request_body = json!({ "mode": "json", "example": example });
            }
        }
        "form" => {
            let mut formdata: Vec<Value> = Vec::new();
            for f in api.body.form.iter().filter(|f| extra_kv_enabled(f)) {
                formdata.push(json!({
                    "key": f.key, "value": f.value, "description": f.description,
                    "type": if f.is_file { "file" } else { "text" },
                }));
            }
            request_body = json!({ "mode": "formdata", "formdata": formdata });
        }
        _ => {
            if !api.body.raw.is_empty() {
                request_body = json!({ "mode": "raw", "raw": api.body.raw });
            }
        }
    }
    let mut responses: Vec<Value> = Vec::new();
    for r in &api.responses {
        if r.body.trim().is_empty() {
            continue;
        }
        let example = serde_json::from_str::<Value>(&r.body).unwrap_or(Value::Null);
        if !example.is_null() {
            responses.push(json!({
                "statusCode": r.status,
                "description": r.name,
                "example": example,
            }));
        }
    }
    json!({
        "name": api.name,
        "method": api.method.to_uppercase(),
        "path": api.path,
        "description": api.description,
        "status": "released",
        "auth": { "type": "none" },
        "request": {
            "headers": headers,
            "query": query_out,
            "body": request_body,
        },
        "responses": responses,
    })
}

// ---------- Bruno ----------

pub fn to_bruno(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let (top_groups, root_name) = extra_build_tree(apis);
    let mut folders: Vec<Value> = Vec::new();
    let mut seq = 1i64;
    for g in &top_groups {
        let mut requests: Vec<Value> = Vec::new();
        for api in &g.apis {
            requests.push(bruno_req_out(api, seq));
            seq += 1;
        }
        folders.push(json!({
            "info": { "name": g.name, "seq": seq },
            "scripts": {},
            "auth": { "mode": "none" },
            "requests": requests,
        }));
        seq += 1;
    }
    json!({
        "version": "1.0.0",
        "info": { "name": root_name, "description": "", "schema": "bruno-schema/1" },
        "settings": { "encodeUrl": true, "followRedirects": false, "maxRedirects": 5, "timeout": 0 },
        "scripts": { "flow": [], "filesystemAccess": "read", "preRequest": [], "postResponse": [] },
        "auth": { "mode": "none" },
        "environments": { "local": {} },
        "folders": folders,
    })
}

fn bruno_req_out(api: &ApiFile, seq: i64) -> Value {
    let mut headers = serde_json::Map::new();
    for h in api.headers.iter().filter(|h| extra_kv_enabled(h)) {
        headers.insert(h.key.clone(), Value::String(h.value.clone()));
    }
    let mut query_out: Vec<Value> = Vec::new();
    for q in api.query.iter().filter(|q| extra_kv_enabled(q)) {
        query_out.push(json!({ "key": q.key, "value": q.value }));
    }
    let body = match api.body.mode.as_str() {
        "form" => {
            let mut parts: Vec<String> = Vec::new();
            for f in api.body.form.iter().filter(|f| extra_kv_enabled(f)) {
                parts.push(format!("{}={}", f.key, f.value));
            }
            json!({ "type": "form-urlencoded", "data": parts.join("&") })
        }
        _ => {
            if api.body.raw.is_empty() {
                json!({ "type": "raw", "data": "" })
            } else {
                json!({ "type": "json", "data": api.body.raw })
            }
        }
    };
    json!({
        "info": { "name": api.name, "type": "http", "seq": seq },
        "http": {
            "method": api.method.to_uppercase(),
            "url": api.path,
            "headers": headers,
            "query": query_out,
            "body": body,
            "auth": { "mode": "none" },
        },
        "runtime": { "scripts": [] },
    })
}

// ---------- Apizza ----------

pub fn to_apizza(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let (top_groups, root_name) = extra_build_tree(apis);
    let mut folders: Vec<Value> = Vec::new();
    for g in &top_groups {
        let mut group_apis: Vec<Value> = Vec::new();
        for api in &g.apis {
            group_apis.push(apizza_api_out(api));
        }
        folders.push(json!({
            "folderName": g.name,
            "folderDesc": "",
            "children": [],
            "apis": group_apis,
        }));
    }
    json!({
        "version": "1.0.0",
        "projectName": root_name,
        "projectDesc": "",
        "createTime": "",
        "updateTime": "",
        "envs": [],
        "folders": folders,
    })
}

fn apizza_api_out(api: &ApiFile) -> Value {
    let mut headers: Vec<Value> = Vec::new();
    for h in api.headers.iter().filter(|h| extra_kv_enabled(h)) {
        headers.push(json!({ "key": h.key, "value": h.value }));
    }
    let mut query_params: Vec<Value> = Vec::new();
    for q in api.query.iter().filter(|q| extra_kv_enabled(q)) {
        query_params.push(json!({ "key": q.key, "value": q.value, "desc": q.description }));
    }
    let mut path_params: Vec<Value> = Vec::new();
    for p in api.params.iter().filter(|p| extra_kv_enabled(p)) {
        path_params.push(json!({ "key": p.key, "value": p.value, "desc": p.description }));
    }
    let (body_mode, body_raw, body_form, body_formdata) = match api.body.mode.as_str() {
        "form" => {
            let mut fd: Vec<Value> = Vec::new();
            for f in api.body.form.iter().filter(|f| extra_kv_enabled(f)) {
                fd.push(json!({
                    "key": f.key, "value": f.value, "type": if f.is_file { "file" } else { "text" }, "desc": f.description,
                }));
            }
            ("formdata".to_string(), String::new(), Vec::<Value>::new(), fd)
        }
        _ => {
            ("raw".to_string(), api.body.raw.clone(), Vec::<Value>::new(), Vec::<Value>::new())
        }
    };
    let mut responses: Vec<Value> = Vec::new();
    for r in &api.responses {
        if r.body.trim().is_empty() {
            continue;
        }
        responses.push(json!({
            "status": r.status,
            "name": r.name,
            "contentType": "application/json",
            "body": r.body,
        }));
    }
    json!({
        "apiName": api.name,
        "apiDesc": api.description,
        "method": api.method.to_uppercase(),
        "url": api.path,
        "headers": headers,
        "cookies": [],
        "queryParams": query_params,
        "pathParams": path_params,
        "bodyMode": body_mode,
        "bodyRaw": body_raw,
        "bodyForm": body_form,
        "bodyFormData": body_formdata,
        "responses": responses,
    })
}

// ---------- NEI ----------

/// json → 字段列表（点分 key，返回 [{name, type, required, description, example}]）
fn nei_json_to_params(prefix: &str, v: &Value, out: &mut Vec<Value>) {
    match v {
        Value::Object(m) => {
            for (k, val) in m {
                let key = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
                if val.is_object() {
                    out.push(json!({
                        "name": key, "type": "object", "required": true, "description": "", "example": serde_json::json!({}),
                    }));
                    nei_json_to_params(&key, val, out);
                } else if val.is_array() {
                    let arr = val.as_array().unwrap();
                    let elem = arr.first().cloned().unwrap_or(Value::Null);
                    let mut item = json!({ "type": "string" });
                    if elem.is_object() {
                        item = json!({ "type": "object" });
                    }
                    out.push(json!({
                        "name": key, "type": "array", "required": true, "description": "", "items": item, "example": val,
                    }));
                } else {
                    let ty = match val {
                        Value::Number(n) if n.as_i64().is_some() => "long",
                        Value::Number(_) => "double",
                        Value::Bool(_) => "boolean",
                        _ => "string",
                    };
                    out.push(json!({
                        "name": key, "type": ty, "required": true, "description": "", "example": val,
                    }));
                }
            }
        }
        _ => {}
    }
}

pub fn to_nei(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let (top_groups, root_name) = extra_build_tree(apis);
    let mut groups: Vec<Value> = Vec::new();
    let mut interfaces: Vec<Value> = Vec::new();
    let mut gid = 101i64;
    for g in &top_groups {
        let my_gid = gid;
        gid += 1;
        groups.push(json!({
            "id": my_gid,
            "name": g.name,
            "description": "",
            "parentId": 0,
        }));
        for api in &g.apis {
            interfaces.push(nei_api_out(api, my_gid));
        }
    }
    json!({
        "id": 1,
        "name": root_name,
        "description": "",
        "properties": { "baseUrl": "", "createTime": "" },
        "groups": groups,
        "datatypes": [],
        "interfaces": interfaces,
    })
}

fn nei_api_out(api: &ApiFile, group: i64) -> Value {
    let mut headers: Vec<Value> = Vec::new();
    for h in api.headers.iter().filter(|h| extra_kv_enabled(h)) {
        headers.push(json!({
            "name": h.key, "type": "string", "required": true,
            "description": h.description, "example": h.value,
        }));
    }
    let mut path_params: Vec<Value> = Vec::new();
    for p in api.params.iter().filter(|p| extra_kv_enabled(p)) {
        path_params.push(json!({
            "name": p.key, "type": "string", "required": true,
            "description": p.description, "example": p.value,
        }));
    }
    let mut query_params: Vec<Value> = Vec::new();
    for q in api.query.iter().filter(|q| extra_kv_enabled(q)) {
        query_params.push(json!({
            "name": q.key, "type": "string", "required": false,
            "description": q.description, "example": q.value,
        }));
    }
    let mut body_params: Vec<Value> = Vec::new();
    let body_type = if api.body.mode == "json" {
        if let Ok(v) = serde_json::from_str::<Value>(&api.body.raw) {
            nei_json_to_params("", &v, &mut body_params);
        }
        "json"
    } else if api.body.mode == "form" {
        for f in api.body.form.iter().filter(|f| extra_kv_enabled(f)) {
            body_params.push(json!({
                "name": f.key, "type": "string", "required": true,
                "description": f.description, "example": f.value,
            }));
        }
        "form"
    } else {
        "none"
    };
    let mut responses: Vec<Value> = Vec::new();
    for r in &api.responses {
        if r.body.trim().is_empty() {
            continue;
        }
        let example = serde_json::from_str::<Value>(&r.body).unwrap_or(Value::Null);
        responses.push(json!({
            "status": r.status,
            "description": r.name,
            "body": { "type": "json", "example": example },
        }));
    }
    json!({
        "id": 0,
        "name": api.name,
        "description": api.description,
        "group": group,
        "url": api.path,
        "method": api.method.to_uppercase(),
        "status": 1,
        "request": {
            "headers": headers,
            "pathParams": path_params,
            "queryParams": query_params,
            "body": { "type": body_type, "params": body_params },
        },
        "responses": responses,
    })
}

// ---------- DOClever ----------

pub fn to_doclever(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let (top_groups, _) = extra_build_tree(apis);
    let mut arr: Vec<Value> = Vec::new();
    let mut sort = 1i64;
    for g in &top_groups {
        arr.push(json!({
            "id": format!("folder_{sort}"),
            "name": g.name,
            "desc": "",
            "folder": true,
            "sort": sort,
            "children": [],
        }));
        for api in &g.apis {
            arr.push(doclever_api_out(api, sort));
            sort += 1;
        }
        sort += 1;
    }
    Value::Array(arr)
}

fn doclever_api_out(api: &ApiFile, sort: i64) -> Value {
    let mut headers: Vec<Value> = Vec::new();
    for h in api.headers.iter().filter(|h| extra_kv_enabled(h)) {
        headers.push(json!({
            "name": h.key, "value": h.value, "required": true, "desc": h.description,
        }));
    }
    let mut query_params: Vec<Value> = Vec::new();
    for q in api.query.iter().filter(|q| extra_kv_enabled(q)) {
        query_params.push(json!({ "name": q.key, "value": q.value, "desc": q.description }));
    }
    let body_info = match api.body.mode.as_str() {
        "form" => {
            let mut params: Vec<Value> = Vec::new();
            for f in api.body.form.iter().filter(|f| extra_kv_enabled(f)) {
                params.push(json!({
                    "name": f.key, "type": "String", "required": true,
                    "desc": f.description, "example": f.value, "range": [],
                }));
            }
            json!({ "bodyType": "form", "raw": "", "params": params })
        }
        _ => json!({
            "bodyType": "raw",
            "raw": api.body.raw,
            "params": [],
        }),
    };
    let mut responses: Vec<Value> = Vec::new();
    for r in &api.responses {
        if r.body.trim().is_empty() {
            continue;
        }
        responses.push(json!({
            "code": r.status,
            "name": r.name,
            "body": r.body,
        }));
    }
    json!({
        "id": format!("api_{}", uuid::Uuid::new_v4().simple()),
        "name": api.name,
        "desc": api.description,
        "path": api.path,
        "method": api.method.to_uppercase(),
        "status": 1,
        "sort": sort,
        "folder": false,
        "baseUrl": "",
        "inject": "",
        "headers": headers,
        "params": [],
        "queryParams": query_params,
        "bodyInfo": body_info,
        "responseParams": [],
        "mock": {},
        "children": [],
    })
}

// ---------- IO-Docs ----------

pub fn to_io_docs(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let (top_groups, root_name) = extra_build_tree(apis);
    let mut resources = serde_json::Map::new();
    for g in &top_groups {
        let mut methods = serde_json::Map::new();
        for api in &g.apis {
            let mkey = format!("{}_{}", api.method.to_lowercase(), api.path.replace('/', "_").replace('{', "").replace('}', ""));
            methods.insert(mkey, io_docs_api_out(api));
        }
        resources.insert(
            g.name.clone(),
            json!({ "description": "", "methods": methods }),
        );
    }
    json!({
        "name": root_name,
        "protocol": "https",
        "basePath": "/",
        "publicPath": [],
        "privatePath": [],
        "auth": { "oauth2": { "flows": [] } },
        "resources": resources,
    })
}

fn io_docs_api_out(api: &ApiFile) -> Value {
    let mut headers_out: Vec<Value> = Vec::new();
    for h in api.headers.iter().filter(|h| extra_kv_enabled(h)) {
        headers_out.push(json!({ "key": h.key, "value": h.value, "description": h.description }));
    }
    let mut parameters = serde_json::Map::new();
    let is_body = matches!(api.method.to_uppercase().as_str(), "POST" | "PUT" | "PATCH");
    if is_body && api.body.mode == "json" {
        if let Ok(v) = serde_json::from_str::<Value>(&api.body.raw) {
            if let Value::Object(m) = v {
                for (k, val) in m {
                    let (ty, default) = match &val {
                        Value::Number(n) if n.as_i64().is_some() => ("integer", Value::from(0)),
                        Value::Number(_) => ("number", Value::from(0)),
                        Value::Bool(_) => ("boolean", Value::Bool(false)),
                        Value::Array(_) => ("array", Value::Array(vec![])),
                        _ => ("string", Value::String(String::new())),
                    };
                    parameters.insert(k, json!({ "type": ty, "required": true, "default": default, "description": "" }));
                }
            }
        }
    } else {
        for q in api.query.iter().filter(|q| extra_kv_enabled(q)) {
            parameters.insert(
                q.key.clone(),
                json!({ "type": "string", "required": false, "default": q.value, "description": q.description }),
            );
        }
    }
    json!({
        "name": api.name,
        "description": api.description,
        "httpMethod": api.method.to_uppercase(),
        "path": api.path,
        "requiresOAuth": false,
        "headers": headers_out,
        "parameters": parameters,
    })
}

// ---------- EasyDoc ----------

pub fn to_easydoc(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let (top_groups, root_name) = extra_build_tree(apis);
    let mut catalog: Vec<Value> = Vec::new();
    let mut api_list: Vec<Value> = Vec::new();
    let mut cat_id = 501i64;
    for g in &top_groups {
        let my_id = cat_id;
        cat_id += 1;
        catalog.push(json!({
            "id": my_id,
            "parent_id": 0,
            "title": g.name,
            "sort": my_id - 500,
            "children": [],
        }));
        for api in &g.apis {
            api_list.push(easydoc_api_out(api, my_id));
        }
    }
    json!({
        "code": 0,
        "msg": "ok",
        "data": {
            "project_id": 1,
            "name": root_name,
            "description": "",
            "create_time": "",
            "update_time": "",
            "base_url": "",
            "catalog": catalog,
            "api_list": api_list,
        },
    })
}

fn easydoc_api_out(api: &ApiFile, cat_id: i64) -> Value {
    let mut headers: Vec<Value> = Vec::new();
    for h in api.headers.iter().filter(|h| extra_kv_enabled(h)) {
        headers.push(json!({
            "name": h.key, "value": h.value, "required": 1, "desc": h.description,
        }));
    }
    let mut req_params: Vec<Value> = Vec::new();
    for q in api.query.iter().filter(|q| extra_kv_enabled(q)) {
        req_params.push(json!({
            "name": q.key, "type": "string", "required": 1, "desc": q.description, "default": q.value,
        }));
    }
    let mut response_params: Vec<Value> = Vec::new();
    let (mut response_demo, mut error_demo) = (String::new(), String::new());
    for r in &api.responses {
        if r.body.trim().is_empty() {
            continue;
        }
        if r.status == 0 || r.status >= 400 {
            if error_demo.is_empty() {
                error_demo = r.body.clone();
            }
        } else if response_demo.is_empty() {
            response_demo = r.body.clone();
        }
    }
    // response_params 从第一个成功响应解析
    if !response_demo.is_empty() {
        if let Ok(v) = serde_json::from_str::<Value>(&response_demo) {
            let mut tmp: Vec<Value> = Vec::new();
            nei_json_to_params("", &v, &mut tmp);
            for t in tmp {
                response_params.push(json!({
                    "name": t["name"], "type": t["type"], "required": 1,
                    "desc": "", "example": t["example"],
                }));
            }
        }
    }
    let (req_body, req_form, body_type) = match api.body.mode.as_str() {
        "form" => {
            let mut form: Vec<Value> = Vec::new();
            for f in api.body.form.iter().filter(|f| extra_kv_enabled(f)) {
                form.push(json!({ "name": f.key, "value": f.value, "desc": f.description }));
            }
            (String::new(), form, "form".to_string())
        }
        _ => (api.body.raw.clone(), Vec::<Value>::new(), "raw".to_string()),
    };
    json!({
        "id": 0,
        "catalog_id": cat_id,
        "title": api.name,
        "desc": api.description,
        "method": api.method.to_uppercase(),
        "url": api.path,
        "request_type": "application/json",
        "response_type": "application/json",
        "mock_open": 0,
        "mock_url": "",
        "request_headers": headers,
        "request_params": req_params,
        "request_body": req_body,
        "request_body_type": body_type,
        "request_form": req_form,
        "response_params": response_params,
        "response_demo": response_demo,
        "error_demo": error_demo,
        "create_time": "",
        "update_time": "",
        "sort": 0,
    })
}

// ---------- DocWay ----------

/// json → docway 字段列表（点分 key + example）
fn docway_json_to_params(prefix: &str, v: &Value, out: &mut Vec<Value>) {
    match v {
        Value::Object(m) => {
            for (k, val) in m {
                let key = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
                if val.is_object() {
                    out.push(json!({
                        "name": key, "type": "object", "required": true, "description": "", "example": json!({}),
                    }));
                    docway_json_to_params(&key, val, out);
                } else if val.is_array() {
                    out.push(json!({
                        "name": key, "type": "array", "required": true, "description": "", "example": val,
                    }));
                } else {
                    let ty = match val {
                        Value::Number(n) if n.as_i64().is_some() => "int",
                        Value::Number(_) => "float",
                        Value::Bool(_) => "boolean",
                        _ => "string",
                    };
                    out.push(json!({
                        "name": key, "type": ty, "required": true, "description": "", "example": val,
                    }));
                }
            }
        }
        _ => {}
    }
}

pub fn to_docway(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let (top_groups, root_name) = extra_build_tree(apis);
    let mut docs: Vec<Value> = Vec::new();
    for g in &top_groups {
        let mut children: Vec<Value> = Vec::new();
        for api in &g.apis {
            children.push(docway_api_out(api));
        }
        docs.push(json!({ "name": g.name, "children": children }));
    }
    json!({ "name": root_name, "description": "", "docs": docs })
}

fn docway_api_out(api: &ApiFile) -> Value {
    let mut headers: Vec<Value> = Vec::new();
    for h in api.headers.iter().filter(|h| extra_kv_enabled(h)) {
        headers.push(json!({
            "name": h.key, "value": h.value, "required": true, "description": h.description,
        }));
    }
    let mut req_params: Vec<Value> = Vec::new();
    if api.body.mode == "json" {
        if let Ok(v) = serde_json::from_str::<Value>(&api.body.raw) {
            docway_json_to_params("", &v, &mut req_params);
        }
    }
    let mut resp_params: Vec<Value> = Vec::new();
    for r in &api.responses {
        if r.body.trim().is_empty() {
            continue;
        }
        if r.status == 0 || r.status >= 400 {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(&r.body) {
            let mut tmp: Vec<Value> = Vec::new();
            docway_json_to_params("", &v, &mut tmp);
            for t in tmp {
                resp_params.push(json!({
                    "name": t["name"], "type": t["type"], "required": true,
                    "description": "", "example": t["example"],
                }));
            }
        }
        break;
    }
    json!({
        "name": api.name,
        "method": api.method.to_uppercase(),
        "url": api.path,
        "description": api.description,
        "requestHeaders": headers,
        "requestParams": req_params,
        "responseParams": resp_params,
    })
}

// ---------- Hoppscotch ----------

pub fn to_hoppscotch(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let (top_groups, root_name) = extra_build_tree(apis);
    let mut folders: Vec<Value> = Vec::new();
    for g in &top_groups {
        let mut requests: Vec<Value> = Vec::new();
        for api in &g.apis {
            requests.push(hoppscotch_req_out(api));
        }
        folders.push(json!({
            "name": g.name,
            "description": "",
            "folders": [],
            "requests": requests,
        }));
    }
    json!({
        "v": "1.0",
        "name": root_name,
        "description": "",
        "auth": { "authType": "none", "authActive": false },
        "headers": [],
        "folders": folders,
        "requests": [],
    })
}

fn hoppscotch_req_out(api: &ApiFile) -> Value {
    let mut headers: Vec<Value> = Vec::new();
    for h in api.headers.iter().filter(|h| extra_kv_enabled(h)) {
        headers.push(json!({ "key": h.key, "value": h.value, "active": true }));
    }
    let mut params: Vec<Value> = Vec::new();
    for q in api.query.iter().filter(|q| extra_kv_enabled(q)) {
        params.push(json!({ "key": q.key, "value": q.value, "active": true }));
    }
    let body = match api.body.mode.as_str() {
        "form" => {
            let mut fd: Vec<Value> = Vec::new();
            for f in api.body.form.iter().filter(|f| extra_kv_enabled(f)) {
                fd.push(json!({
                    "key": f.key, "value": f.value, "active": true,
                    "type": if f.is_file { "file" } else { "text" },
                }));
            }
            json!({ "mode": "formdata", "formdata": fd })
        }
        _ => {
            if api.body.raw.is_empty() {
                json!({ "mode": "none" })
            } else {
                json!({ "mode": "raw", "raw": api.body.raw })
            }
        }
    };
    json!({
        "name": api.name,
        "method": api.method.to_uppercase(),
        "endpoint": format!("<<base_url>>{}", api.path),
        "params": params,
        "headers": headers,
        "body": body,
        "preRequestScript": "",
        "testScript": "",
        "auth": { "authActive": false, "authType": "none" },
    })
}

// ---------- MeterSphere ----------

pub fn to_metersphere(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let (top_groups, root_name) = extra_build_tree(apis);
    let mut node_tree: Vec<Value> = Vec::new();
    let mut data: Vec<Value> = Vec::new();
    let mut mod_id = 1i64;
    for g in &top_groups {
        let my_id = format!("mod-{mod_id}");
        mod_id += 1;
        node_tree.push(json!({
            "id": my_id,
            "name": g.name,
            "sort": mod_id - 1,
            "children": [],
        }));
        for api in &g.apis {
            data.push(metersphere_api_out(api, &my_id));
        }
    }
    json!({
        "projectName": root_name,
        "projectId": "project_1",
        "protocol": "http",
        "version": "1.0",
        "nodeTree": node_tree,
        "data": data,
        "cases": [],
        "mocks": [],
    })
}

fn metersphere_api_out(api: &ApiFile, module_id: &str) -> Value {
    let mut headers: Vec<Value> = Vec::new();
    for h in api.headers.iter().filter(|h| extra_kv_enabled(h)) {
        headers.push(json!({ "key": h.key, "value": h.value, "enable": true }));
    }
    let mut query: Vec<Value> = Vec::new();
    for q in api.query.iter().filter(|q| extra_kv_enabled(q)) {
        query.push(json!({ "key": q.key, "value": q.value, "enable": true }));
    }
    let body_type = if api.body.mode == "form" { "FORM_DATA" } else { "JSON" };
    let response = api
        .responses
        .iter()
        .find(|r| !r.body.trim().is_empty())
        .map(|r| json!({ "bodyType": "JSON", "raw": r.body }))
        .unwrap_or_else(|| json!({ "bodyType": "JSON", "raw": "" }));
    json!({
        "id": format!("api-{}", uuid::Uuid::new_v4().simple()),
        "name": api.name,
        "method": api.method.to_uppercase(),
        "path": api.path,
        "moduleId": module_id,
        "description": api.description,
        "status": "UNDONE",
        "request": {
            "headers": headers,
            "query": query,
            "body": { "bodyType": body_type, "raw": api.body.raw },
        },
        "response": response,
    })
}

// ---------- 统一导出入口 ----------

/// 返回 (文件内容, 默认文件名, 扩展名)

// ---------- RAP2 ----------

/// ApiFile → rap2 属性列表（scope request/response，parentId 嵌套）
fn rap2_props_from_value(
    prefix: &str,
    v: &Value,
    scope: &str,
    out: &mut Vec<Value>,
    parent_id: i64,
    counter: &mut i64,
) -> i64 {
    match v {
        Value::Object(m) => {
            let my_id = *counter;
            *counter += 1;
            out.push(json!({
                "id": my_id, "scope": scope, "pos": 1, "name": prefix,
                "type": "Object", "required": true, "value": "",
                "description": "", "parentId": parent_id, "priority": 1,
            }));
            for (k, val) in m {
                rap2_props_from_value(k, val, scope, out, my_id, counter);
            }
            my_id
        }
        Value::Array(arr) => {
            let my_id = *counter;
            *counter += 1;
            out.push(json!({
                "id": my_id, "scope": scope, "pos": 1, "name": prefix,
                "type": "Array", "required": true, "value": "",
                "description": "", "parentId": parent_id, "priority": 1,
            }));
            if let Some(first) = arr.first() {
                if first.is_object() {
                    for (k, val) in first.as_object().unwrap() {
                        rap2_props_from_value(k, val, scope, out, my_id, counter);
                    }
                }
            }
            my_id
        }
        Value::Number(n) => {
            let id = *counter;
            *counter += 1;
            out.push(json!({
                "id": id, "scope": scope, "pos": 1, "name": prefix,
                "type": if n.as_i64().is_some() { "Number" } else { "Float" },
                "required": true, "value": n, "description": "", "parentId": parent_id, "priority": 1,
            }));
            id
        }
        Value::Bool(b) => {
            let id = *counter;
            *counter += 1;
            out.push(json!({
                "id": id, "scope": scope, "pos": 1, "name": prefix,
                "type": "Boolean", "required": true, "value": b,
                "description": "", "parentId": parent_id, "priority": 1,
            }));
            id
        }
        _ => {
            let id = *counter;
            *counter += 1;
            out.push(json!({
                "id": id, "scope": scope, "pos": 1, "name": prefix,
                "type": "String", "required": true,
                "value": if v.is_null() { "" } else { v.as_str().unwrap_or("") },
                "description": "", "parentId": parent_id, "priority": 1,
            }));
            id
        }
    }
}

/// 生成接口的 properties（request：headers/query/params/body；response：第一个成功响应）
fn rap2_api_properties(api: &ApiFile) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    let mut counter: i64 = 1;
    for h in api.headers.iter().filter(|h| extra_kv_enabled(h)) {
        out.push(json!({
            "id": counter, "scope": "request", "pos": 1, "name": h.key,
            "type": "String", "required": true, "value": h.value,
            "description": h.description, "parentId": -1, "priority": out.len() as i64 + 1,
        }));
        counter += 1;
    }
    for q in api.query.iter().filter(|q| extra_kv_enabled(q)) {
        out.push(json!({
            "id": counter, "scope": "request", "pos": 2, "name": q.key,
            "type": "String", "required": true, "value": q.value,
            "description": q.description, "parentId": -1, "priority": out.len() as i64 + 1,
        }));
        counter += 1;
    }
    for p in api.params.iter().filter(|p| extra_kv_enabled(p)) {
        out.push(json!({
            "id": counter, "scope": "request", "pos": 3, "name": p.key,
            "type": "String", "required": true, "value": p.value,
            "description": p.description, "parentId": -1, "priority": out.len() as i64 + 1,
        }));
        counter += 1;
    }
    if api.body.mode == "json" {
        if let Ok(v) = serde_json::from_str::<Value>(&api.body.raw) {
            let ty = if v.is_object() { "Object" } else { "Array" };
            out.push(json!({
                "id": counter, "scope": "request", "pos": 4, "name": "body",
                "type": ty, "required": true, "value": "",
                "description": "", "parentId": -1, "priority": out.len() as i64 + 1,
            }));
            counter += 1;
            match &v {
                Value::Object(m) => {
                    for (k, val) in m {
                        rap2_props_from_value(k, val, "request", &mut out, counter - 1, &mut counter);
                    }
                }
                Value::Array(arr) => {
                    if let Some(first) = arr.first() {
                        if first.is_object() {
                            for (k, val) in first.as_object().unwrap() {
                                rap2_props_from_value(k, val, "request", &mut out, counter - 1, &mut counter);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    if let Some(r) = api.responses.iter().find(|r| !r.body.trim().is_empty()) {
        if let Ok(v) = serde_json::from_str::<Value>(&r.body) {
            match &v {
                Value::Object(m) => {
                    for (k, val) in m {
                        rap2_props_from_value(k, val, "response", &mut out, -1, &mut counter);
                    }
                }
                Value::Array(arr) => {
                    if let Some(first) = arr.first() {
                        if first.is_object() {
                            for (k, val) in first.as_object().unwrap() {
                                rap2_props_from_value(k, val, "response", &mut out, -1, &mut counter);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// 单接口 → rap2 interface 对象
fn rap2_interface_out(api: &ApiFile) -> Value {
    json!({
        "id": 0,
        "name": api.name,
        "url": api.path,
        "method": api.method.to_uppercase(),
        "status": "draft",
        "description": api.description,
        "priority": 0,
        "moduleId": -1,
        "repositoryId": -1,
        "creatorId": -1,
        "lockerId": -1,
        "createdAt": "",
        "updatedAt": "",
        "properties": rap2_api_properties(api),
    })
}

/// 项目格式：分组 → modules[]
pub fn to_rap2_project(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let (top_groups, root_name) = extra_build_tree(apis);
    let mut modules: Vec<Value> = Vec::new();
    let mut repo_id = 0i64;
    for g in &top_groups {
        repo_id += 1;
        let mut interfaces: Vec<Value> = Vec::new();
        for api in &g.apis {
            interfaces.push(rap2_interface_out(api));
        }
        modules.push(json!({
            "id": 9000 + repo_id,
            "name": g.name,
            "description": "",
            "priority": repo_id,
            "repositoryId": repo_id,
            "interfaces": interfaces,
        }));
    }
    json!({
        "data": {
            "id": 1,
            "name": root_name,
            "description": "",
            "logo": "",
            "token": "",
            "visibility": "public",
            "createdAt": "",
            "updatedAt": "",
            "modules": modules,
        }
    })
}

/// 单接口格式：data 直接是 interface
pub fn to_rap2_single(apis: &[(Vec<(String, bool)>, ApiFile)]) -> Value {
    let api = &apis[0].1;
    json!({ "data": rap2_interface_out(api) })
}


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
        "rap2-single" => (to_rap2_single(apis), "rap2-single", "json"),
        _ => return Err(format!("不支持的格式: {format}")),
    };
    let content = serde_json::to_string_pretty(&val).map_err(|e| format!("序列化失败: {e}"))?;
    Ok((content, fname.to_string(), ext.to_string()))
}
