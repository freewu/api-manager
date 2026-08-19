//! 导出：Postman Collection v2.1 / OpenAPI 3.0 / Docsify 文档目录。
//! 收集选中路径（接口或分组）下的全部接口，按格式生成内容。

use crate::{read_api, read_info_file, sanitize_filename, ApiFile, ENV_FILE, INFO_FILE};
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
    for p in paths {
        let abs = Path::new(p);
        if abs.is_dir() {
            let mut segs = Vec::new();
            walk_dir(abs, &mut segs, &mut out)?;
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
