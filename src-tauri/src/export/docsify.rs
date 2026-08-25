//! 由 export.rs 拆分：Docsify 文档目录
#[allow(unused_imports)]
use crate::{read_api, read_info_file, sanitize_filename, ApiFile, BodyData, KeyValue, MockConfig, ENV_FILE, INFO_FILE};
#[allow(unused_imports)]
use serde_json::{json, Map, Value};
#[allow(unused_imports)]
use std::collections::BTreeMap;
#[allow(unused_imports)]
use std::fs;
#[allow(unused_imports)]
use std::path::{Path, PathBuf};

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


// ==================== Apifox 导出 ====================
