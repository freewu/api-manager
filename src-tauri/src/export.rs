//! 导出：Postman Collection v2.1 / OpenAPI 3.0 / Docsify 文档目录。
//! 收集选中路径（接口或分组）下的全部接口，按格式生成内容。

use crate::{read_api, read_info_file, sanitize_filename, ApiFile, ENV_FILE, INFO_FILE};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// 收集选中路径下的全部接口。
/// 返回 (分组路径段, ApiFile)：分组路径段为各层分组的显示名称（不含工作区根）。
pub fn collect_apis(root: &Path, paths: &[String]) -> Result<Vec<(Vec<String>, ApiFile)>, String> {
    let mut out: Vec<(Vec<String>, ApiFile)> = Vec::new();
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
fn walk_dir(
    dir: &Path,
    segs: &mut Vec<String>,
    out: &mut Vec<(Vec<String>, ApiFile)>,
) -> Result<(), String> {
    let info = read_info_file(dir);
    let dir_name = dir
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    segs.push(info.name.clone().unwrap_or(dir_name));
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
pub fn to_postman(apis: &[(Vec<String>, ApiFile)]) -> Value {
    let mut root = PNode {
        name: "API Manager".to_string(),
        apis: Vec::new(),
        children: BTreeMap::new(),
    };
    for (segs, api) in apis {
        let mut cur = &mut root;
        for s in segs {
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
pub fn to_openapi(title: &str, apis: &[(Vec<String>, ApiFile)]) -> Value {
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

fn openapi_operation(segs: &[String], api: &ApiFile) -> Value {
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
    let mut op = json!({
        "summary": api.name,
        "description": api.description,
        "parameters": params,
        "responses": {
            "200": { "description": format!("Mock 响应（状态码 {}）", api.mock.status) }
        }
    });
    if !segs.is_empty() {
        op["tags"] = json!([segs.join("/")]);
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

/// 生成 Docsify 文档目录：返回 (相对路径, 内容) 列表，
/// 含 _sidebar.md、根 README.md（首页）与 index.html（开启 _sidebar 支持）
pub fn docsify_files(apis: &[(Vec<String>, ApiFile)]) -> Vec<(PathBuf, String)> {
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    let mut used: Vec<PathBuf> = Vec::new();

    // 根 README.md 是 Docsify 首页，先占位避免顶层接口重名
    used.push(PathBuf::from("README.md"));

    // 接口 .md 文件：<分组路径>/<接口名>.md（重名自动加序号）
    let mut tree: SideNode = SideNode {
        name: String::new(),
        apis: Vec::new(),
        children: BTreeMap::new(),
    };
    for (segs, api) in apis {
        let mut cur = &mut tree;
        for s in segs {
            let name = sanitize_filename(s);
            cur = cur.children.entry(name.clone()).or_insert_with(|| SideNode {
                name,
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
    name: String,
    apis: Vec<&'a ApiFile>,
    children: BTreeMap<String, SideNode<'a>>,
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
        files.push((rel, crate::markdown::render(api, &n.name)));
    }
    // 子分组
    for (_, c) in &n.children {
        let sub = dir.join(&c.name);
        // 分组 README.md：标题 + 子项链接
        let mut readme = format!("# {}\n\n", c.name);
        for api in &c.apis {
            let base = if api.name.trim().is_empty() {
                "未命名接口".to_string()
            } else {
                sanitize_filename(api.name.trim())
            };
            readme.push_str(&format!("- [{}]({}.md)\n", api.name, base));
        }
        for (name, _) in &c.children {
            readme.push_str(&format!("- [{}]({}/)\n", name, name));
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
            rel.to_string_lossy().replace('\\', "/")
        ));
    }
    // 子分组
    for (_, c) in &n.children {
        let sub = dir.join(&c.name);
        out.push_str(&format!(
            "{indent}- [{}]({}/)\n",
            c.name,
            sub.to_string_lossy().replace('\\', "/")
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
            },
            mock: crate::MockConfig::default(),
            examples: vec![],
            doc_params: vec![],
        }
    }

    #[test]
    fn postman_shape() {
        let apis = vec![(vec!["用户管理".to_string()], sample())];
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
        let apis = vec![(vec!["用户管理".to_string()], sample())];
        let v = to_openapi("测试", &apis);
        assert_eq!(v["openapi"], "3.0.1");
        assert_eq!(v["paths"]["/api/users"]["post"]["summary"], "创建用户");
        assert_eq!(v["paths"]["/api/users"]["post"]["tags"][0], "用户管理");
        assert!(v["paths"]["/api/users"]["post"]["requestBody"].is_object());
    }

    #[test]
    fn docsify_files_ok() {
        let apis = vec![
            (vec!["用户管理".to_string()], sample()),
            (vec!["用户管理".to_string()], sample()), // 同名接口 → 加序号
        ];
        let files = docsify_files(&apis);
        let names: Vec<String> = files
            .iter()
            .map(|(p, _)| p.to_string_lossy().replace('\\', "/"))
            .collect();
        assert!(names.contains(&"用户管理/创建用户.md".to_string()));
        assert!(names.contains(&"用户管理/创建用户(2).md".to_string()));
        assert!(names.contains(&"用户管理/README.md".to_string()));
        assert!(names.contains(&"_sidebar.md".to_string()));
        assert!(names.contains(&"README.md".to_string()));
        assert!(names.contains(&"index.html".to_string()));
        let sidebar = files.iter().find(|(p, _)| p.to_string_lossy() == "_sidebar.md").unwrap().1.clone();
        assert!(sidebar.contains("[创建用户]"));
        let index = files.iter().find(|(p, _)| p.to_string_lossy() == "index.html").unwrap().1.clone();
        assert!(index.contains("loadSidebar: true"));
        let readme = files.iter().find(|(p, _)| p.to_string_lossy() == "README.md").unwrap().1.clone();
        assert!(readme.contains("[创建用户]"));
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
        assert!(apis.iter().all(|(s, _)| s == &vec!["用户管理".to_string()]));
        let _ = fs::remove_dir_all(&base);
    }

    /// 同路径同方法的不同接口（如重名文件）在 OpenAPI 中不应互相覆盖，追加序号保留全部
    #[test]
    fn openapi_keeps_duplicate_path_method() {
        let mut a2 = sample();
        a2.name = "创建用户(2)".into();
        let apis = vec![
            (vec!["用户管理".to_string()], sample()),
            (vec!["用户管理".to_string()], a2),
        ];
        let v = to_openapi("测试", &apis);
        assert_eq!(v["paths"]["/api/users"]["post"]["summary"], "创建用户");
        assert_eq!(v["paths"]["/api/users (2)"]["post"]["summary"], "创建用户(2)");
    }
}
