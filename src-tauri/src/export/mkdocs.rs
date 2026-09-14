//! 由 export.rs 拆分：MkDocs 文档站点
#![allow(unused_imports)]
use crate::{sanitize_filename, ApiFile};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// 生成 MkDocs 站点目录：返回 (相对路径, 内容) 列表。
/// 结构：mkdocs.yml（站点配置 + nav） / docs/index.md（首页） / docs/<分组>/<接口>.md
pub fn mkdocs_files(
    site_name: &str,
    apis: &[(Vec<(String, bool)>, ApiFile)],
) -> Vec<(PathBuf, String)> {
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    // docs 根 index.md 已被首页占用，先占位避免顶层接口重名
    let mut used: Vec<PathBuf> = vec![PathBuf::from("index.md")];

    // 构建分组树（分组目录名去空白/非法字符；显示名保留原样）
    let mut tree = MkNode {
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
            dep_inherit = dep_inherit || *dep;
            let display = if dep_inherit {
                format!("{}（已废弃）", s.trim())
            } else {
                s.trim().to_string()
            };
            let name = slug_group(s);
            cur = cur.children.entry(name.clone()).or_insert_with(|| MkNode {
                name,
                display,
                deprecated: dep_inherit,
                apis: Vec::new(),
                children: BTreeMap::new(),
            });
        }
        cur.apis.push(api);
    }

    // 接口页面：docs/<分组路径>/<接口名>.md
    write_pages(&tree, Path::new(""), &mut files, &mut used);

    // 站点名：工作区名（缺省时用「接口文档」）
    let title = {
        let t = site_name.trim();
        if t.is_empty() {
            "接口文档"
        } else {
            t
        }
    };

    // docs/index.md：首页（站点名 + 目录）
    let mut index = format!("# {title}\n\n");
    index.push_str(&index_bullets(&tree, Path::new(""), 0));
    files.push((Path::new("docs").join("index.md"), index));

    // mkdocs.yml：站点配置（nav 使用 docs 目录内相对路径）
    let mut yml = String::from("# MkDocs 配置：由 API Manager 导出生成\n");
    yml.push_str("# 本地预览：pip install mkdocs && mkdocs serve\n");
    yml.push_str(&format!("site_name: {}\n", yaml_quote(title)));
    yml.push_str("nav:\n");
    yml.push_str(&format!("  - {}: index.md\n", yaml_quote("首页")));
    nav_yaml(&tree, Path::new(""), 1, &mut yml);
    files.push((PathBuf::from("mkdocs.yml"), yml));

    files
}

struct MkNode<'a> {
    /// 目录名（已去空白、去非法字符）
    name: String,
    /// 显示名（保留原样，用于 nav 与首页目录）
    display: String,
    /// 分组自身或其祖先分组是否已废弃（接口继承此标注）
    deprecated: bool,
    apis: Vec<&'a ApiFile>,
    children: BTreeMap<String, MkNode<'a>>,
}

/// 分组目录名：去掉全部空白字符（空格/制表/全角空格），其余非法字符替换为 _
fn slug_group(name: &str) -> String {
    sanitize_filename(name)
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// 接口文件名（不含扩展名）：空名回退为「未命名接口」
fn api_base(api: &ApiFile) -> String {
    let n = api.name.trim();
    if n.is_empty() {
        "未命名接口".to_string()
    } else {
        sanitize_filename(n)
    }
}

/// 递归写出接口页面（同名自动加序号）
fn write_pages(
    n: &MkNode,
    dir: &Path,
    files: &mut Vec<(PathBuf, String)>,
    used: &mut Vec<PathBuf>,
) {
    for api in &n.apis {
        let base = api_base(api);
        let mut rel = dir.join(format!("{base}.md"));
        let mut i = 2;
        while used.iter().any(|u| u == &rel) {
            rel = dir.join(format!("{base}({i}).md"));
            i += 1;
        }
        used.push(rel.clone());
        files.push((
            Path::new("docs").join(&rel),
            crate::markdown::render(api, &n.display, n.deprecated),
        ));
    }
    for (_, c) in &n.children {
        write_pages(c, &dir.join(&c.name), files, used);
    }
}

/// 首页目录：嵌套列表（链接相对 docs 根）
fn index_bullets(n: &MkNode, dir: &Path, depth: usize) -> String {
    let mut out = String::new();
    let indent = "  ".repeat(depth);
    for api in &n.apis {
        let rel = rel_md(dir, api);
        out.push_str(&format!("{indent}- [{}]({})\n", api.name.trim(), rel));
    }
    for (_, c) in &n.children {
        out.push_str(&format!("{indent}- {}\n", c.display));
        out.push_str(&index_bullets(c, &dir.join(&c.name), depth + 1));
    }
    out
}

/// mkdocs.yml 的 nav 片段（分组递归嵌套）
fn nav_yaml(n: &MkNode, dir: &Path, depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    for api in &n.apis {
        let rel = rel_md(dir, api);
        out.push_str(&format!(
            "{indent}- {}: {}\n",
            yaml_quote(api.name.trim()),
            yaml_quote(&rel)
        ));
    }
    for (_, c) in &n.children {
        out.push_str(&format!("{indent}- {}:\n", yaml_quote(&c.display)));
        nav_yaml(c, &dir.join(&c.name), depth + 1, out);
    }
}

/// docs 目录内的相对链接（Windows 分隔符转 /）
fn rel_md(dir: &Path, api: &ApiFile) -> String {
    dir.join(format!("{}.md", api_base(api)))
        .to_string_lossy()
        .replace('\\', "/")
}

/// YAML 双引号字符串转义
fn yaml_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}
