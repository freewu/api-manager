//! 接口文档 Markdown：导出（render）、HTML 预览（md_to_html）、导入回读（parse）。
//! 导出与导入格式自洽，保证「查看 Markdown → 保存 → 再导入」能完整还原接口。

use crate::{ApiFile, BodyData, DocParam, KeyValue, MockConfig};
use serde::Serialize;
use serde_json::Value;
use std::fmt::Write as _;

// ==================== 导出 ====================

/// 把接口渲染为 Markdown 文档（新格式：# 分组名 → ## 接口名 → > 方法 URL → header/请求参数/响应参数）
/// group_deprecated：所在分组（或其祖先分组）已废弃时传 true，接口继承「已废弃」标注
pub fn render(api: &ApiFile, group: &str, group_deprecated: bool) -> String {
    // 旧文件没有 responses 时按 mock 体 / resp_fail 文档补全，保证响应部分不丢
    let mut ensured = api.clone();
    let legacy = ensured.responses.is_empty()
        && (!ensured.mock.body.trim().is_empty()
            || ensured
                .doc_params
                .iter()
                .any(|d| d.source == "resp_success" || d.source == "resp_fail"));
    if legacy {
        crate::ensure_responses(&mut ensured);
    }
    let api = &ensured;
    let mut s = String::new();
    let g = group.trim();
    if !g.is_empty() {
        let _ = writeln!(s, "# {g}\n");
    }

    let name = api.name.trim();
    let name = if name.is_empty() { "未命名接口" } else { name };
    // 接口自身废弃，或其所在分组废弃 → 名称加标注，便于文档中识别
    let dep_mark = if api.deprecated || group_deprecated { "（已废弃）" } else { "" };
    let _ = writeln!(s, "## {name}{dep_mark}\n");

    // > Method url（url 为空时回退到 path，保证导出文档不丢 URL）
    let url = api.url.trim();
    let url = if url.is_empty() { api.path.trim() } else { url };
    // WebSocket 没有 method，直接标注为 WebSocket，避免在文档里显示虚假的 HTTP 方法
    let bline = if api.protocol == "websocket" {
        if url.is_empty() {
            "> WebSocket".to_string()
        } else {
            format!("> WebSocket {url}")
        }
    } else {
        let method = api.method.trim();
        let method = if method.is_empty() { "GET" } else { method };
        if url.is_empty() {
            format!("> {method}")
        } else {
            format!("> {method} {url}")
        }
    };
    let _ = writeln!(s, "{bline}\n");

    // 描述（普通段落，保证导出→导入可还原）
    let desc = api.description.trim();
    if !desc.is_empty() {
        for l in desc.lines() {
            let _ = writeln!(s, "{}", l.trim());
        }
        let _ = writeln!(s);
    }

    // ## header：Key: Value 行
    let headers: Vec<&KeyValue> = api.headers.iter().filter(|r| !r.key.trim().is_empty()).collect();
    if !headers.is_empty() {
        let _ = writeln!(s, "## header\n");
        for r in &headers {
            let _ = writeln!(s, "{}: {}", r.key.trim(), r.value.trim());
        }
        let _ = writeln!(s);
    }

    // ## 请求参数：### path / ### query / ### body（无内容不展示）
    let mut req = String::new();
    let path_rows: Vec<&KeyValue> = api.params.iter().filter(|r| !r.key.trim().is_empty()).collect();
    if !path_rows.is_empty() {
        let _ = writeln!(req, "### path\n");
        req.push_str(&doc_table(api, "path", &path_rows));
        req.push('\n');
    }
    let query_rows: Vec<&KeyValue> = api.query.iter().filter(|r| !r.key.trim().is_empty()).collect();
    if !query_rows.is_empty() {
        let _ = writeln!(req, "### query\n");
        req.push_str(&doc_table(api, "query", &query_rows));
        req.push('\n');
    }
    render_body(api, &mut req);
    if !req.is_empty() {
        let _ = writeln!(s, "## 请求参数\n");
        s.push_str(&req);
    }

    // ## 响应参数：每个「响应」页签条目一节（名称 + HTTP 状态码）+ 请求示例
    let mut resp = String::new();
    for r in &api.responses {
        if r.name.trim().is_empty() && r.body.trim().is_empty() {
            continue;
        }
        let mut rows: Vec<(String, String, String)> = Vec::new();
        let body = r.body.trim();
        if let Ok(v) = serde_json::from_str::<Value>(body) {
            walk_json(v, "", api, &format!("resp:{}", r.id), &mut rows);
        } else {
            let src = format!("resp:{}", r.id);
            let docs: Vec<&DocParam> = api.doc_params.iter().filter(|d| d.source == src).collect();
            for d in docs {
                flatten_doc(d, "", &mut rows);
            }
        }
        let title = if r.status > 0 {
            format!("{}（HTTP {}）", r.name, r.status)
        } else {
            r.name.clone()
        };
        let _ = writeln!(resp, "### {}\n", title);
        if !rows.is_empty() {
            resp.push_str(&three_col_table(&rows));
            resp.push('\n');
        }
        if !body.is_empty() {
            let _ = writeln!(resp, "```json\n{}\n```\n", pretty_json(body));
        }
    }
    let _ = writeln!(resp, "### 请求示例\n");
    let _ = writeln!(resp, "```bash\n{}\n```\n", curl_example(api));
    if !resp.is_empty() {
        let _ = writeln!(s, "## 响应参数\n");
        s.push_str(&resp);
    }

    s
}

/// path / query / body-form 请求参数表：字段 | 类型 | 描述（类型/说明来自 docParams 覆盖）
fn doc_table(api: &ApiFile, source: &str, rows: &[&KeyValue]) -> String {
    let data: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            let d = doc_at(api, source, &[r.key.trim()]);
            vec![
                r.key.trim().to_string(),
                d.as_ref().map(|d| d.r#type.trim().to_string()).unwrap_or_default(),
                d.as_ref().map(|d| d.description.trim().to_string()).unwrap_or_default(),
            ]
        })
        .collect();
    md_table(&["字段", "类型", "描述"], &data)
}

/// 三列表格（字段 | 类型 | 描述）→ Markdown
fn three_col_table(rows: &[(String, String, String)]) -> String {
    let data: Vec<Vec<String>> = rows
        .iter()
        .map(|(k, t, d)| vec![k.clone(), t.clone(), d.clone()])
        .collect();
    md_table(&["字段", "类型", "描述"], &data)
}

/// Body：form 输出 KV 表 + 示例 JSON；json 输出推导表 + 原 JSON；raw 输出代码块
fn render_body(api: &ApiFile, s: &mut String) {
    let mut body = String::new();
    match api.body.mode.as_str() {
        "form" => {
            let rows: Vec<&KeyValue> = api.body.form.iter().filter(|r| !r.key.trim().is_empty()).collect();
            if rows.is_empty() {
                return;
            }
            let _ = writeln!(body, "### body\n");
            body.push_str(&doc_table(api, "body", &rows));
            body.push('\n');
            let _ = writeln!(body, "```json\n{}\n```\n", form_sample_json(api, &rows));
        }
        "json" => {
            if api.body.raw.trim().is_empty() {
                return;
            }
            let _ = writeln!(body, "### body\n");
            let mut rows: Vec<(String, String, String)> = Vec::new();
            if let Ok(v) = serde_json::from_str::<Value>(&api.body.raw) {
                walk_json(v, "", api, "body", &mut rows);
            }
            if !rows.is_empty() {
                body.push_str(&three_col_table(&rows));
                body.push('\n');
            }
            let _ = writeln!(body, "```json\n{}\n```\n", pretty_json(&api.body.raw));
        }
        "raw" => {
            if api.body.raw.trim().is_empty() {
                return;
            }
            let _ = writeln!(body, "### body\n");
            let _ = writeln!(body, "```\n{}\n```\n", api.body.raw.trim());
        }
        _ => return,
    }
    s.push_str(&body);
}

/// 格式化 JSON 字符串（解析失败原样返回）
fn pretty_json(raw: &str) -> String {
    match serde_json::from_str::<Value>(raw) {
        Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| raw.trim().to_string()),
        Err(_) => raw.trim().to_string(),
    }
}

/// form 表单 → 示例 JSON（值优先取 docParams 说明，否则取表单值）
fn form_sample_json(api: &ApiFile, rows: &[&KeyValue]) -> String {
    let mut root = serde_json::Map::new();
    for r in rows {
        let d = doc_at(api, "body", &[r.key.trim()]);
        let desc = d.map(|d| d.description.trim().to_string()).unwrap_or_default();
        let val = if !desc.is_empty() { desc } else { r.value.trim().to_string() };
        root.insert(r.key.trim().to_string(), Value::String(val));
    }
    serde_json::to_string_pretty(&Value::Object(root)).unwrap_or_default()
}

/// 由响应字段（点分路径 + 类型 + 说明）生成示例 JSON（按类型给样例值）
pub(crate) fn sample_json_from_rows(rows: &[(String, String, String)]) -> String {
    let mut root = serde_json::Map::new();
    for (k, t, d) in rows {
        let parts: Vec<&str> = k.split('.').map(|p| p.trim()).filter(|p| !p.is_empty()).collect();
        if parts.is_empty() {
            continue;
        }
        let mut cur = &mut root;
        for (i, p) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                cur.insert(p.to_string(), sample_val(t, d));
            } else {
                cur = cur
                    .entry(p.to_string())
                    .or_insert_with(|| Value::Object(serde_json::Map::new()))
                    .as_object_mut()
                    .expect("刚插入的对象容器");
            }
        }
    }
    serde_json::to_string_pretty(&Value::Object(root)).unwrap_or_default()
}

/// 按类型生成样例值（字符串取说明，无说明取空串）
fn sample_val(ty: &str, desc: &str) -> Value {
    match ty.to_lowercase().as_str() {
        "integer" | "int" => serde_json::json!(1),
        "float" | "double" | "number" => serde_json::json!(0.0),
        "boolean" | "bool" => serde_json::json!(true),
        "list" | "array" => serde_json::json!([]),
        "object" => serde_json::json!({}),
        _ => Value::String(if desc.is_empty() { String::new() } else { desc.to_string() }),
    }
}

/// 生成 curl 请求示例
fn curl_example(api: &ApiFile) -> String {
    let method = api.method.trim();
    let method = if method.is_empty() { "GET" } else { method };
    // url 为空时回退到 path，避免 curl 示例缺 URL
    let url = api.url.trim();
    let url = if url.is_empty() { api.path.trim() } else { url };
    let mut parts = vec![format!("curl -X {method} {url}")];
    for h in &api.headers {
        let k = h.key.trim();
        if !k.is_empty() {
            parts.push(format!("-H \"{k}: {}\"", h.value.trim()));
        }
    }
    let mut body_str = String::new();
    match api.body.mode.as_str() {
        "json" | "raw" => body_str = api.body.raw.trim().to_string(),
        "form" => {
            let rows: Vec<&KeyValue> = api.body.form.iter().filter(|r| !r.key.trim().is_empty()).collect();
            let mut root = serde_json::Map::new();
            for r in &rows {
                root.insert(r.key.trim().to_string(), Value::String(r.value.trim().to_string()));
            }
            body_str = serde_json::to_string(&Value::Object(root)).unwrap_or_default();
        }
        _ => {}
    }
    if !body_str.is_empty() && method != "GET" && method != "HEAD" {
        parts.push(format!("-d '{}'", body_str));
    }
    parts.join(" ")
}

/// 按 source + key 路径查找 docParams 条目（key 为空时不匹配）
fn doc_at<'a>(api: &'a ApiFile, source: &str, keys: &[&str]) -> Option<&'a DocParam> {
    let mut arr = &api.doc_params;
    let mut cur: Option<&DocParam> = None;
    for k in keys {
        cur = arr.iter().find(|d| d.source == source && d.key == *k);
        cur?;
        arr = &cur.as_ref().unwrap().children;
    }
    cur
}

/// 递归展开 docParams 树为「点分路径」行
pub(crate) fn flatten_doc(d: &DocParam, prefix: &str, out: &mut Vec<(String, String, String)>) {
    let key = if prefix.is_empty() {
        d.key.clone()
    } else {
        format!("{prefix}.{}", d.key)
    };
    out.push((key.clone(), d.r#type.clone(), d.description.clone()));
    for c in &d.children {
        flatten_doc(c, &key, out);
    }
}

/// 递归把 JSON 值展开为「点分路径」行，类型按值推导，并合并 docParams 覆盖
pub(crate) fn walk_json(v: Value, prefix: &str, api: &ApiFile, source: &str, out: &mut Vec<(String, String, String)>) {
    match v {
        Value::Object(map) => {
            let mut entries: Vec<(String, Value)> = map.into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            for (k, val) in entries {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                let (guess, children) = guess_val(&val);
                let key_list: Vec<&str> = key.split('.').collect();
                let d = doc_at(api, source, &key_list);
                let t = d
                    .map(|d| d.r#type.trim().to_string())
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| guess.to_string());
                let desc = d.map(|d| d.description.trim().to_string()).unwrap_or_default();
                out.push((key.clone(), t, desc));
                if let Some(cs) = children {
                    walk_json(cs, &key, api, source, out);
                }
            }
        }
        Value::Array(items) => {
            let key = if prefix.is_empty() { "items".to_string() } else { format!("{prefix}.items") };
            let first = items.first().cloned();
            let guess = "List";
            let key_list: Vec<&str> = key.split('.').collect();
            let d = doc_at(api, source, &key_list);
            let t = d
                .map(|d| d.r#type.trim().to_string())
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| guess.to_string());
            let desc = d.map(|d| d.description.trim().to_string()).unwrap_or_default();
            out.push((key.clone(), t, desc));
            if let Some(f) = first {
                if f.is_object() {
                    walk_json(f, &key, api, source, out);
                }
            }
        }
        _ => {
            let key = prefix.to_string();
            let guess = guess_val(&v).0;
            let key_list: Vec<&str> = key.split('.').collect();
            let d = doc_at(api, source, &key_list);
            let t = d
                .map(|d| d.r#type.trim().to_string())
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| guess.to_string());
            let desc = d.map(|d| d.description.trim().to_string()).unwrap_or_default();
            out.push((key, t, desc));
        }
    }
}

/// 按值推导字段类型（与前端一致）
fn guess_val(v: &Value) -> (String, Option<Value>) {
    match v {
        Value::Null => ("String".into(), None),
        Value::Bool(_) => ("Boolean".into(), None),
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                ("Integer".into(), None)
            } else {
                ("Float".into(), None)
            }
        }
        Value::Array(items) => {
            let first = items.first().cloned();
            let children = if let Some(f) = first {
                if f.is_object() {
                    Some(f)
                } else {
                    None
                }
            } else {
                None
            };
            ("List".into(), children)
        }
        Value::Object(map) => {
            // Object 类型展开下级字段（否则表格中只显示 Object 一行，值/子字段不显示）
            ("Object".into(), Some(Value::Object(map.clone())))
        }
        Value::String(_) => ("String".into(), None),
    }
}

/// 生成 Markdown 表格（表头 + 数据行）
fn md_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut s = String::new();
    s.push_str("| ");
    s.push_str(&headers.iter().map(|h| esc_cell(h)).collect::<Vec<_>>().join(" | "));
    s.push_str(" |\n");
    s.push_str("| ");
    s.push_str(&headers.iter().map(|_| "---").collect::<Vec<_>>().join(" | "));
    s.push_str(" |\n");
    for row in rows {
        let mut cells: Vec<String> = headers
            .iter()
            .enumerate()
            .map(|(i, _)| row.get(i).cloned().unwrap_or_default())
            .map(|c| esc_cell(&c))
            .collect();
        // 补齐不足的表头列数
        while cells.len() < headers.len() {
            cells.push(String::new());
        }
        s.push_str("| ");
        s.push_str(&cells.join(" | "));
        s.push_str(" |\n");
    }
    s
}

fn esc_cell(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

// ==================== Markdown → HTML ====================

/// 简易 Markdown → HTML（支持标题、引用、列表、表格、代码块、行内代码/加粗、段落）
pub fn md_to_html(md: &str) -> String {
    let mut out = String::new();
    let lines: Vec<&str> = md.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        // 代码块
        if line.trim_start().starts_with("```") {
            let lang = line.trim_start_matches('`').trim();
            let mut buf = String::new();
            i += 1;
            while i < lines.len() && !lines[i].trim_start().starts_with("```") {
                buf.push_str(lines[i]);
                buf.push('\n');
                i += 1;
            }
            i += 1; // 跳过结束 ``` 
            let cls = if lang.is_empty() { "" } else { &format!(" class=\"lang-{lang}\"") };
            let _ = write!(out, "<pre{cls}><code>{}</code></pre>\n", escape_html(buf.trim_end()));
            continue;
        }
        // 标题：支持 # ~ ####（注意去掉旧的 `!starts_with("##")` 限制，否则 ## 行会落入段落分支导致死循环）
        if line.trim_start().starts_with('#') {
            let trimmed = line.trim_start();
            let mut level = 0;
            for ch in trimmed.chars() {
                if ch == '#' {
                    level += 1;
                } else {
                    break;
                }
            }
            if (1..=6).contains(&level) {
                let text = trimmed[level..].trim();
                if !text.is_empty() {
                    let _ = writeln!(out, "<h{level}>{}</h{level}>", inline(text));
                }
                i += 1;
                continue;
            }
        }
        // 引用：连续 > 行
        if line.trim_start().starts_with('>') {
            let mut buf = String::new();
            while i < lines.len() && lines[i].trim_start().starts_with('>') {
                let t = lines[i].trim_start().trim_start_matches('>').trim();
                buf.push_str(t);
                buf.push('\n');
                i += 1;
            }
            let _ = writeln!(out, "<blockquote>{}</blockquote>", inline(buf.trim_end()));
            continue;
        }
        // 表格：当前行以 | 开头且下一行是分隔行
        if line.trim_start().starts_with('|') && i + 1 < lines.len() && is_table_sep(lines[i + 1]) {
            let mut rows: Vec<Vec<String>> = Vec::new();
            while i < lines.len() && lines[i].trim_start().starts_with('|') {
                if !is_table_sep(lines[i]) {
                    rows.push(split_cells(lines[i]));
                }
                i += 1;
            }
            if !rows.is_empty() {
                let header = rows.remove(0);
                let mut t = String::from("<table><thead><tr>");
                for h in &header {
                    let _ = write!(t, "<th>{}</th>", inline(h));
                }
                t.push_str("</tr></thead><tbody>");
                for row in rows {
                    t.push_str("<tr>");
                    for (idx, cell) in row.iter().enumerate() {
                        let tag = if idx == 0 { "th" } else { "td" };
                        let _ = write!(t, "<{tag}>{}</{tag}>", inline(cell));
                    }
                    t.push_str("</tr>");
                }
                t.push_str("</tbody></table>\n");
                out.push_str(&t);
            }
            continue;
        }
        // 无序列表：连续 - 行
        if let Some(item) = strip_prefix(line.trim_start(), "- ") {
            let mut items: Vec<String> = Vec::new();
            items.push(inline(item));
            i += 1;
            while i < lines.len() {
                if let Some(it) = strip_prefix(lines[i].trim_start(), "- ") {
                    items.push(inline(it));
                    i += 1;
                } else {
                    break;
                }
            }
            let _ = write!(out, "<ul>{}</ul>\n", items.iter().map(|x| format!("<li>{x}</li>")).collect::<String>());
            continue;
        }
        // 分隔线
        if line.trim() == "---" {
            out.push_str("<hr>\n");
            i += 1;
            continue;
        }
        // 段落：收集连续非空且非块起始的行（必须保证至少前进一行，避免死循环）
        if !line.trim().is_empty() {
            let mut buf = String::new();
            let mut advanced = false;
            while i < lines.len()
                && !lines[i].trim().is_empty()
                && !is_block_start(lines[i])
            {
                buf.push_str(lines[i].trim());
                buf.push(' ');
                i += 1;
                advanced = true;
            }
            if advanced {
                let _ = writeln!(out, "<p>{}</p>", inline(buf.trim_end()));
                continue;
            }
        }
        i += 1;
    }
    out
}

/// 是否为 Markdown 块起始行（标题/引用/表格/列表/代码块/分隔线）
fn is_block_start(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with('#')
        || t.starts_with('>')
        || t.starts_with('|')
        || t.starts_with('-')
        || t.starts_with("```")
        || t.trim() == "---"
}

fn is_table_sep(line: &str) -> bool {
    let cells = split_cells(line);
    !cells.is_empty() && cells.iter().all(|c| !c.trim().is_empty() && c.trim().chars().all(|ch| ch == '-' || ch == ':' || ch == ' '))
}

fn split_cells(line: &str) -> Vec<String> {
    let t = line.trim().trim_start_matches('|').trim_end_matches('|');
    t.split('|').map(|c| c.trim().to_string()).collect()
}

fn strip_prefix<'a>(s: &'a str, p: &str) -> Option<&'a str> {
    s.strip_prefix(p)
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// 行内格式：反引号代码 + **加粗**
fn inline(s: &str) -> String {
    let esc = escape_html(s);
    let mut out = String::new();
    let mut in_code = false;
    for seg in esc.split('`') {
        if in_code {
            out.push_str("<code>");
            out.push_str(seg);
            out.push_str("</code>");
        } else {
            out.push_str(&bold(seg));
        }
        in_code = !in_code;
    }
    out
}

fn bold(s: &str) -> String {
    let mut out = String::new();
    let mut b = false;
    for seg in s.split("**") {
        if b {
            out.push_str("<strong>");
            out.push_str(seg);
            out.push_str("</strong>");
        } else {
            out.push_str(seg);
        }
        b = !b;
    }
    out
}

/// 生成独立的 HTML 文档：由 Markdown 渲染而来，含内置样式（单文件可直接双击打开）。
/// nav：HTML 文档悬浮导航栏位置（off 关闭 / left 左侧 / right 右侧，默认右侧），
/// 导航根据文档中的分组（# 标题）与接口（## 接口名）自动生成。
pub fn wrap_html(title: &str, md: &str, nav: &str) -> String {
    let html = md_to_html(md);
    let title = escape_html(title);
    let nav = match nav {
        "off" => "off",
        "left" => "left",
        _ => "right",
    };
    format!(
        r##"<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
body{{font-family:-apple-system,'Segoe UI','Microsoft YaHei',sans-serif;max-width:860px;margin:32px auto;padding:0 20px;color:#24292f;line-height:1.6}}
h1,h2,h3{{border-bottom:1px solid #e5e7eb;padding-bottom:6px}}
table{{border-collapse:collapse;width:100%;margin:8px 0}}
th,td{{border:1px solid #d0d7de;padding:6px 10px;font-size:13px;text-align:left}}
th{{background:#f6f8fa}}
pre{{background:#f6f8fa;border:1px solid #d0d7de;border-radius:6px;padding:10px;overflow:auto}}
code{{font-family:Consolas,Menlo,monospace;font-size:12.5px}}
blockquote{{border-left:4px solid #d0d7de;margin:8px 0;padding:2px 12px;color:#57606a}}
ul{{padding-left:22px}}
/* ===== 悬浮导航栏 ===== */
#doc-nav{{position:fixed;top:0;bottom:0;width:210px;overflow-y:auto;box-sizing:border-box;padding:16px 10px;font-size:13px;background:#fbfbfc;z-index:10}}
body.nav-right #doc-nav{{right:0;border-left:1px solid #e5e7eb}}
body.nav-left #doc-nav{{left:0;border-right:1px solid #e5e7eb}}
body.nav-off #doc-nav{{display:none}}
#doc-nav .nav-title{{font-weight:600;font-size:12px;color:#57606a;text-transform:uppercase;letter-spacing:.05em;padding:0 8px 8px;border-bottom:1px solid #e5e7eb;margin-bottom:8px}}
.nav-groups{{list-style:none;margin:0;padding:0}}
.nav-group-link{{display:block;padding:5px 8px;color:#24292f;font-weight:600;text-decoration:none;border-radius:4px}}
.nav-group-link:hover{{background:#f0f2f5}}
.nav-group-link.active{{color:#0969da}}
.nav-apis{{list-style:none;margin:0 0 6px;padding:0 0 0 12px;border-left:1px solid #e5e7eb}}
.nav-api-link{{display:block;padding:3px 8px;color:#57606a;text-decoration:none;border-radius:4px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}}
.nav-api-link:hover{{color:#0969da;background:#f0f2f5}}
.nav-api-link.active{{color:#0969da;background:#ddf4ff;font-weight:600}}
/* 窄屏：导航改为抽屉，右上/左上角悬浮按钮切换 */
#doc-nav-toggle{{position:fixed;top:12px;right:12px;z-index:11;display:none;border:1px solid #d0d7de;background:#fff;color:#24292f;border-radius:6px;padding:5px 10px;font-size:13px;cursor:pointer}}
body.nav-left #doc-nav-toggle{{right:auto;left:12px}}
@media (max-width:1279px){{
  #doc-nav-toggle{{display:block}}
  #doc-nav{{top:46px;bottom:0;background:#fff;box-shadow:-2px 0 12px rgba(0,0,0,.1);transform:translateX(100%);transition:transform .2s ease}}
  body.nav-left #doc-nav{{transform:translateX(-100%);box-shadow:2px 0 12px rgba(0,0,0,.1)}}
  #doc-nav.open{{transform:translateX(0)}}
}}
</style>
</head>
<body class="nav-{nav}">
<button id="doc-nav-toggle" type="button" aria-label="目录">☰ 目录</button>
<aside id="doc-nav"><div class="nav-title">目录</div></aside>
<article>{html}</article>
<script>
(function () {{
  // 收集分组（h1）与接口（h2）标题，跳过接口内部小节（header / 请求参数 / 响应参数）
  var SKIP = {{ 'header': 1, '请求参数': 1, '响应参数': 1 }};
  var nav = document.getElementById('doc-nav');
  if (!nav) return;
  var groups = [];
  var current = null;
  var n = 0;
  var heads = document.querySelectorAll('article h1, article h2');
  for (var i = 0; i < heads.length; i++) {{
    var h = heads[i];
    var txt = (h.textContent || '').trim();
    if (!h.id) h.id = 'sec-' + (++n);
    if (h.tagName === 'H1') {{
      current = {{ title: txt, el: h, apis: [] }};
      groups.push(current);
    }} else {{
      if (SKIP[txt]) continue;
      if (!current) {{ current = {{ title: '', el: null, apis: [] }}; groups.push(current); }}
      current.apis.push({{ title: txt, el: h }});
    }}
  }}
  if (!groups.length) return;
  var ul = document.createElement('ul');
  ul.className = 'nav-groups';
  groups.forEach(function (g) {{
    var li = document.createElement('li');
    if (g.el) {{
      var a = document.createElement('a');
      a.href = '#' + g.el.id;
      a.className = 'nav-group-link';
      a.textContent = g.title;
      li.appendChild(a);
    }}
    if (g.apis.length) {{
      var sub = document.createElement('ul');
      sub.className = 'nav-apis';
      g.apis.forEach(function (api) {{
        var li2 = document.createElement('li');
        var a2 = document.createElement('a');
        a2.href = '#' + api.el.id;
        a2.className = 'nav-api-link';
        a2.textContent = api.title;
        li2.appendChild(a2);
        sub.appendChild(li2);
      }});
      li.appendChild(sub);
    }}
    ul.appendChild(li);
  }});
  nav.appendChild(ul);
  // 滚动监听：高亮当前所在章节
  var links = Array.prototype.slice.call(nav.querySelectorAll('a'));
  function onScroll() {{
    var cur = null;
    for (var i = 0; i < links.length; i++) {{
      var t = document.getElementById(links[i].getAttribute('href').slice(1));
      if (t && t.getBoundingClientRect().top <= 110) cur = links[i];
    }}
    for (var j = 0; j < links.length; j++) links[j].classList.remove('active');
    if (cur) cur.classList.add('active');
  }}
  window.addEventListener('scroll', onScroll, {{ passive: true }});
  onScroll();
  // 窄屏抽屉：悬浮按钮切换显隐
  var toggle = document.getElementById('doc-nav-toggle');
  if (toggle) toggle.addEventListener('click', function () {{ nav.classList.toggle('open'); }});
}})();
</script>
</body>
</html>
"##
    )
}

// ==================== 导入 ====================

/// 解析结果：分组名 + 接口列表（新格式：# 分组名 → 多个 ## 接口名）
pub struct ParsedMarkdown {
    pub group: String,
    pub apis: Vec<ApiFile>,
}

/// 从 Markdown 文档解析接口（新格式：# 分组名 → 多个 ## 接口名；兼容旧格式 # 接口名）
pub fn parse(md: &str) -> Result<ParsedMarkdown, String> {
    let mut group = String::new();
    let mut apis: Vec<ApiFile> = Vec::new();
    for block in split_h1(md) {
        if group.is_empty() {
            group = first_h1(&block);
        }
        // 旧格式判定：首个 ## 标题是已知小节名（基本信息/请求 Header/…），或整块无 ##
        let first_h2 = block.lines().find(|l| l.trim_start().starts_with("## "));
        let old_format = match first_h2 {
            Some(l) => {
                let t = l.trim_start().trim_start_matches('#').trim();
                is_section_name(t)
            }
            None => true,
        };
        if old_format {
            if let Some(api) = parse_one(&block, true)? {
                apis.push(api);
            }
        } else {
            for b in split_apis(&block) {
                // 跳过仅含 # 分组名的前导块
                if !b.lines().any(|l| l.trim_start().starts_with("## ")) {
                    continue;
                }
                if let Some(api) = parse_one(&b, false)? {
                    apis.push(api);
                }
            }
        }
    }
    if apis.is_empty() {
        return Err("未找到接口标题（## 接口名称）".into());
    }
    Ok(ParsedMarkdown { group, apis })
}

/// 是否为已知的 Markdown 小节名（旧格式 ## 标题）
fn is_section_name(t: &str) -> bool {
    matches!(
        t,
        "基本信息" | "请求 Header" | "Query" | "Path" | "Body" | "响应" | "Mock" | "header" | "请求参数" | "响应参数"
    )
}

/// 取块内第一个一级标题（# 分组名）
fn first_h1(block: &str) -> String {
    for line in block.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("# ") {
            return rest.trim().to_string();
        }
        if line.trim_start().starts_with("##") {
            break;
        }
    }
    String::new()
}

/// 按一级标题（# ）切分文档块（新格式：# 分组名）
fn split_h1(md: &str) -> Vec<String> {
    let mut blocks: Vec<String> = Vec::new();
    let mut cur = String::new();
    for line in md.lines() {
        if line.starts_with("# ") && !cur.trim().is_empty() {
            blocks.push(std::mem::take(&mut cur));
        }
        cur.push_str(line);
        cur.push('\n');
    }
    if !cur.trim().is_empty() {
        blocks.push(cur);
    }
    blocks
}

/// 按二级标题（## 接口名）切分接口块（已知小节名如 header/请求参数/响应参数 不切分）
fn split_apis(block: &str) -> Vec<String> {
    let mut blocks: Vec<String> = Vec::new();
    let mut cur = String::new();
    for line in block.lines() {
        if line.starts_with("## ") {
            let t = line.trim_start().trim_start_matches('#').trim();
            if !is_section_name(t) && !cur.trim().is_empty() {
                blocks.push(std::mem::take(&mut cur));
            }
        }
        cur.push_str(line);
        cur.push('\n');
    }
    if !cur.trim().is_empty() {
        blocks.push(cur);
    }
    blocks
}

/// 解析单个接口块（新格式 ## 接口名 起；旧格式 # 接口名 起）
fn parse_one(block: &str, old_format: bool) -> Result<Option<ApiFile>, String> {
    let mut name = String::new();
    let mut method: Option<String> = None;
    let mut protocol = "http".to_string();
    let mut url = String::new();
    let mut desc_lines: Vec<String> = Vec::new();
    let mut section = String::new();
    let mut subsection = String::new();
    let mut sections: Vec<(String, String, String)> = Vec::new();
    let mut content = String::new();

    for raw in block.lines() {
        let line = raw.trim_end();
        if line.trim_start().starts_with("### ") {
            if !content.trim().is_empty() {
                sections.push((section.clone(), subsection.clone(), std::mem::take(&mut content)));
            }
            subsection = line.trim_start().trim_start_matches('#').trim().to_string();
            continue;
        }
        if old_format {
            // 旧格式：# 接口名 为接口名；## 小节为分区标题
            if let Some(rest) = line.strip_prefix("# ") {
                name = rest.trim().to_string();
                continue;
            }
            if line.trim_start().starts_with("## ") {
                if !content.trim().is_empty() {
                    sections.push((section.clone(), subsection.clone(), std::mem::take(&mut content)));
                }
                section = line.trim_start().trim_start_matches('#').trim().to_string();
                subsection.clear();
                continue;
            }
        } else {
            // 新格式：## 接口名（已知小节名除外，如 header/请求参数/响应参数）
            if line.trim_start().starts_with("## ") {
                let t = line.trim_start().trim_start_matches('#').trim().to_string();
                if !content.trim().is_empty() {
                    sections.push((section.clone(), subsection.clone(), std::mem::take(&mut content)));
                }
                if is_section_name(&t) {
                    section = t;
                    subsection.clear();
                } else {
                    name = t;
                    section.clear();
                    subsection.clear();
                }
                continue;
            }
            if line.trim_start().starts_with("# ") {
                continue; // 忽略 # 分组名（本块头部）
            }
        }
        if line.trim_start().starts_with('>') {
            let t = line.trim_start().trim_start_matches('>').trim();
            if !t.is_empty() {
                // WebSocket 标注：> WebSocket url（还原 protocol，method 置空）
                let ws_rest = ["WebSocket", "websocket", "WEBSOCKET"]
                    .iter()
                    .find_map(|p| t.strip_prefix(p))
                    .map(|r| r.trim().to_string());
                if let Some(ws_rest) = ws_rest {
                    protocol = "websocket".to_string();
                    method = Some(String::new());
                    url = ws_rest;
                } else if let Some((m, u)) = parse_method_url(t) {
                    method = Some(m);
                    url = u;
                } else {
                    desc_lines.push(t.to_string());
                }
            }
            continue;
        }
        content.push_str(line);
        content.push('\n');
    }
    if !content.trim().is_empty() {
        sections.push((section.clone(), subsection.clone(), content));
    }

    if name.is_empty() {
        return Ok(None);
    }

    let mut api = ApiFile {
        uuid: String::new(),
        name: name.clone(),
        method: method.clone().unwrap_or_else(|| "GET".to_string()),
        path: String::new(),
        url: url.clone(),
        description: desc_lines.join("\n"),
        headers: vec![],
        query: vec![],
        params: vec![],
        body: BodyData::default(),
        mock: MockConfig::default(),
        prescript: String::new(),
        examples: vec![],
        responses: vec![],
        doc_params: vec![],
        deprecated: false,
        protocol,
        order: None,
    };

    for (sec, sub, text) in &sections {
        let sec = sec.as_str();
        if sec.is_empty() {
            // 描述段落（> Method url 之后的普通段落）
            for line in text.lines() {
                let t = line.trim();
                if !t.is_empty() {
                    desc_lines.push(t.to_string());
                }
            }
            continue;
        }
        match sec {
            "header" => {
                for line in text.lines() {
                    let t = line.trim();
                    if t.is_empty() {
                        continue;
                    }
                    if let Some((k, v)) = t.split_once(':') {
                        api.headers.push(KeyValue {
                            key: k.trim().to_string(),
                            value: v.trim().to_string(),
                            enabled: true,
                            description: String::new(),
                            is_file: false,
                        });
                    }
                }
            }
            "基本信息" => {
                for line in text.lines() {
                    if let Some(v) = line.strip_prefix("- 方法:") {
                        let v = v.trim();
                        if !v.is_empty() {
                            api.method = v.to_string();
                        }
                    } else if let Some(v) = line.strip_prefix("- 路径:") {
                        api.path = v.trim().to_string();
                    } else if let Some(v) = line.strip_prefix("- URL:") {
                        api.url = v.trim().to_string();
                    }
                }
            }
            "请求参数" => match sub.as_str() {
                "path" => parse_kv_section(&mut api, "path", text),
                "query" => parse_kv_section(&mut api, "query", text),
                "body" => parse_kv_section(&mut api, "body", text),
                _ => {}
            },
            "响应参数" => match sub.as_str() {
                "成功响应" => {
                    parse_doc_section(&mut api, "resp_success", text);
                    // 成功响应 JSON 代码块 → Mock 响应体
                    if let Some(fence) = first_fence(text) {
                        if let Ok(v) = serde_json::from_str::<Value>(&fence) {
                            api.mock.body = serde_json::to_string(&v).unwrap_or(fence);
                        }
                    }
                }
                "失败响应" => parse_doc_section(&mut api, "resp_fail", text),
                "请求示例" => {}
                other => {
                    // 新版：响应页签条目（### 返回成功（HTTP 200））
                    let mut entry = crate::ResponseItem::default();
                    entry.id = uuid::Uuid::new_v4().to_string();
                    let (name, status) = split_status_name(other);
                    entry.name = name;
                    entry.status = status;
                    if let Some(fence) = first_fence(text) {
                        entry.body = match serde_json::from_str::<Value>(&fence) {
                            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or(fence),
                            Err(_) => fence,
                        };
                        // 返回成功同步到 Mock 响应体（保持导入 → Mock 可用）
                        if entry.status == 200 || entry.name.contains("返回成功") {
                            api.mock.body = entry.body.clone();
                        }
                    }
                    parse_doc_section(&mut api, &format!("resp:{}", entry.id), text);
                    api.responses.push(entry);
                }
            },
            "请求 Header" | "Query" | "Path" | "Body" => {
                let source = match sec {
                    "请求 Header" => "header",
                    "Query" => "query",
                    "Path" => "path",
                    _ => "body",
                };
                parse_kv_section(&mut api, source, text);
            }
            "响应" => match sub.as_str() {
                "请求成功" => parse_doc_section(&mut api, "resp_success", text),
                "请求失败" => parse_doc_section(&mut api, "resp_fail", text),
                _ => {}
            },
            "Mock" => match sub.as_str() {
                "" => {
                    for line in text.lines() {
                        if let Some(v) = line.strip_prefix("- 状态码:") {
                            api.mock.status = v.trim().parse().unwrap_or(200);
                        } else if let Some(v) = line.strip_prefix("- 启用:") {
                            api.mock.enabled = v.trim().contains('是');
                        }
                    }
                }
                "Mock 请求头" => {
                    for r in parse_table_rows(text) {
                        if r.key.is_empty() {
                            continue;
                        }
                        api.mock.headers.push(KeyValue {
                            key: r.key,
                            value: r.col1,
                            enabled: true,
                            description: r.desc,
                            is_file: false,
                        });
                    }
                }
                "Mock 响应体" => {
                    if let Some(fence) = first_fence(text) {
                        api.mock.body = fence.trim().to_string();
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    // 描述段落（> Method url 之后的普通段落）在 sections 循环中累积到 desc_lines，回写字段
    if !desc_lines.is_empty() {
        api.description = desc_lines.join("\n");
    }

    // 从 URL 推导 path（新格式没有单独路径行：去掉 scheme://host 即为路径）
    if api.path.is_empty() && !api.url.trim().is_empty() {
        api.path = derive_path(&api.url);
    }

    Ok(Some(api))
}

/// 从完整 URL 推导路径（http://host/api/x → /api/x；无协议时原样）
fn derive_path(url: &str) -> String {
    let u = url.trim();
    if let Some(idx) = u.find("://") {
        let after = &u[idx + 3..];
        if let Some(slash) = after.find('/') {
            let p = &after[slash..];
            if !p.is_empty() {
                return p.to_string();
            }
        }
        return "/".to_string();
    }
    u.to_string()
}

/// 解析引用行：`METHOD url`（已知方法名）→ (方法, URL)，否则 None（当作描述）
fn parse_method_url(t: &str) -> Option<(String, String)> {
    let mut it = t.split_whitespace();
    let first = it.next()?;
    let upper = first.to_uppercase();
    const METHODS: [&str; 7] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
    if METHODS.contains(&upper.as_str()) {
        let rest: Vec<&str> = it.collect();
        return Some((upper, rest.join(" ")));
    }
    None
}

/// 解析 Header / Query / Path / Body（form）小节：表格 → KeyValue + docParams
fn parse_kv_section(api: &mut ApiFile, source: &str, text: &str) {
    // 优先 JSON/代码块 → Body 的 json/raw 模式
    if source == "body" {
        if let Some(fence) = first_fence(text) {
            let raw = fence.trim().to_string();
            if serde_json::from_str::<Value>(&raw).is_ok() {
                api.body.mode = "json".to_string();
            } else {
                api.body.mode = "raw".to_string();
            }
            api.body.raw = raw;
            return;
        }
    }
    let rows = parse_table_rows(text);
    if rows.is_empty() {
        return;
    }
    if source == "body" {
        api.body.mode = "form".to_string();
    }
    let target: &mut Vec<KeyValue> = match source {
        "header" => &mut api.headers,
        "query" => &mut api.query,
        "path" => &mut api.params,
        _ => &mut api.body.form,
    };
    for r in rows {
        if r.key.is_empty() {
            continue;
        }
        target.push(KeyValue {
            key: r.key.clone(),
            value: r.col1.clone(),
            enabled: true,
            description: r.desc.clone(),
            is_file: false,
        });
        // 类型或说明非空时写入 docParams（source + key 关联）
        if !r.ty.is_empty() || !r.desc.is_empty() {
            upsert_doc(&mut api.doc_params, source, &[r.key.as_str()], &r.ty, &r.desc);
        }
    }
}

/// 解析 响应 小节：表格 → docParams（点分路径 → 树状 children）
fn parse_doc_section(api: &mut ApiFile, source: &str, text: &str) {
    let rows = parse_table_rows(text);
    for r in rows {
        if r.key.is_empty() {
            continue;
        }
        let ty = if r.ty.is_empty() { r.col1.clone() } else { r.ty.clone() };
        let parts: Vec<&str> = r.key.split('.').map(|p| p.trim()).filter(|p| !p.is_empty()).collect();
        if parts.is_empty() {
            continue;
        }
        upsert_doc(&mut api.doc_params, source, &parts, &ty, &r.desc);
    }
}

/// 在 docParams 中按 source + key 路径创建/更新条目（最后一个 key 记录类型与说明）
fn upsert_doc(list: &mut Vec<DocParam>, source: &str, keys: &[&str], ty: &str, desc: &str) {
    let mut arr = list;
    for (i, k) in keys.iter().enumerate() {
        let idx = match arr.iter().position(|d| d.source == source && d.key == *k) {
            Some(idx) => idx,
            None => {
                arr.push(DocParam {
                    source: source.to_string(),
                    key: k.to_string(),
                    r#type: String::new(),
                    description: String::new(),
                    item_type: String::new(),
                    object_name: String::new(),
                    children: vec![],
                });
                arr.len() - 1
            }
        };
        if i == keys.len() - 1 {
            if !ty.is_empty() {
                arr[idx].r#type = ty.to_string();
            }
            if !desc.is_empty() {
                arr[idx].description = desc.to_string();
            }
            return;
        }
        arr = &mut arr[idx].children;
    }
}

/// 解析 Markdown 表格：识别表头列含义（参数名/字段名、值/类型、说明）
struct TableRow {
    key: String,
    col1: String,
    ty: String,
    desc: String,
}

fn parse_table_rows(text: &str) -> Vec<TableRow> {
    let mut raw: Vec<Vec<String>> = Vec::new();
    for line in text.lines() {
        let t = line.trim();
        if !t.starts_with('|') || is_table_sep(t) {
            continue;
        }
        raw.push(split_cells(t));
    }
    if raw.is_empty() {
        return Vec::new();
    }
    let header = &raw[0];
    let type_col = header.iter().position(|h| h.contains("类型"));
    let desc_col = header.iter().position(|h| h.contains("说明") || h.contains("描述"));
    // 「值」列存在才取值（新格式 字段|类型|描述 无值列，col1 留空）
    let value_col = header.iter().position(|h| h.contains("值"));
    let mut out = Vec::new();
    for row in raw.iter().skip(1) {
        let key = row.first().cloned().unwrap_or_default();
        let col1 = value_col.and_then(|c| row.get(c)).cloned().unwrap_or_default();
        let ty = type_col.and_then(|c| row.get(c)).cloned().unwrap_or_default();
        let desc = desc_col.and_then(|c| row.get(c)).cloned().unwrap_or_default();
        out.push(TableRow { key, col1, ty, desc });
    }
    out
}

/// 从「名称（HTTP 200）」中拆出名称与状态码（兼容无状态码的名称）
fn split_status_name(raw: &str) -> (String, u16) {
    let t = raw.trim();
    if let Some(idx) = t.rfind('（') {
        let tail = &t[idx + '（'.len_utf8()..];
        if let Some(end) = tail.find('）') {
            if let Some(code) = tail[..end].strip_prefix("HTTP ") {
                if let Ok(n) = code.trim().parse::<u16>() {
                    return (t[..idx].trim().to_string(), n);
                }
            }
        }
    }
    (t.to_string(), 0)
}

/// 取第一个 ``` 围栏代码块内容
fn first_fence(text: &str) -> Option<String> {
    let mut in_fence = false;
    let mut buf = String::new();
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            if in_fence {
                return Some(buf);
            }
            in_fence = true;
            buf.clear();
            continue;
        }
        if in_fence {
            buf.push_str(line);
            buf.push('\n');
        }
    }
    None
}

// ==================== 测试 ====================


use crate::{export, read_api, read_info_file, sanitize_filename, unique_path, workspace_root, WorkspaceState};
use std::fs;
use std::path::Path;
use tauri::{AppHandle, State};

/// 接口 Markdown 文档（供前端预览）
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownDoc {
    name: String,
    md: String,
    html: String,
}

/// 从接口文件路径推导分组名（父目录名；接口直接在工作区根目录下时为空）
fn group_of(path: &str, root: &str) -> String {
    let parent = Path::new(path).parent().unwrap_or(Path::new(""));
    let norm = |p: &str| p.trim_end_matches(['/', '\\']).trim_end_matches('/').to_string();
    if norm(&parent.to_string_lossy()) == norm(root) {
        return String::new();
    }
    parent
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// 将任意 Markdown 文本渲染为 HTML 片段（用于接口描述预览）
#[tauri::command]
pub(crate) fn render_markdown(text: String) -> String {
    md_to_html(&text)
}

/// 渲染接口的 Markdown 文档（含 HTML 预览版）
#[tauri::command]
pub(crate) fn render_api_markdown(state: State<'_, WorkspaceState>, path: String) -> Result<MarkdownDoc, String> {
    let root = workspace_root(&state)?;
    let group = group_of(&path, &root.to_string_lossy());
    let api = read_api(path)?;
    let md = render(&api, &group, false);
    let html = md_to_html(&md);
    Ok(MarkdownDoc { name: api.name, md, html })
}

/// 生成分组（含其下全部子分组/接口）的单个 Markdown：返回 (分组名, markdown)
pub(crate) fn group_markdown_doc(root: &Path, path: &str) -> Result<(String, String), String> {
    let apis = export::collect_apis(root, &[path.to_string()])?;
    if apis.is_empty() {
        return Err("所选内容中没有接口".into());
    }
    let name = read_info_file(Path::new(path))
        .name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| {
            Path::new(path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        });
    let md = export::markdown_single_file(&name, &apis);
    Ok((name, md))
}

/// 渲染分组（含其下全部子分组/接口）为单个 Markdown 文档
#[tauri::command]
pub(crate) fn render_group_markdown(state: State<'_, WorkspaceState>, path: String) -> Result<MarkdownDoc, String> {
    let root = workspace_root(&state)?;
    let (name, md) = group_markdown_doc(&root, &path)?;
    let html = md_to_html(&md);
    Ok(MarkdownDoc { name, md, html })
}

/// 导出接口 Markdown / HTML：弹出目录选择框，写入 <接口名>.md 或 <接口名>.html
#[tauri::command]
pub(crate) fn export_api_markdown(
    app: AppHandle,
    state: State<'_, WorkspaceState>,
    path: String,
    format: String,
    nav: String,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let root = workspace_root(&state)?;
    // 分组（目录）走分组 Markdown；接口文件走单接口 Markdown
    let (name, md) = if Path::new(&path).is_dir() {
        group_markdown_doc(&root, &path)?
    } else {
        let group = group_of(&path, &root.to_string_lossy());
        let api = read_api(path)?;
        (api.name.clone(), render(&api, &group, false))
    };
    let fmt = if format.eq_ignore_ascii_case("html") { "html" } else { "md" };
    let picked = app
        .dialog()
        .file()
        .set_title("选择保存目录")
        .blocking_pick_folder();
    let Some(dir) = picked else {
        return Ok(None);
    };
    let dir = dir.into_path().map_err(|e| e.to_string())?;
    let base = sanitize_filename(&name);
    let base = if base.trim().is_empty() {
        "未命名文档".to_string()
    } else {
        base
    };
    let target = unique_path(&dir, &base, &format!(".{fmt}"));
    let content = if fmt == "html" {
        wrap_html(&name, &md, &nav)
    } else {
        md
    };
    fs::write(&target, content).map_err(|e| format!("写入失败: {e}"))?;
    Ok(Some(target.to_string_lossy().to_string()))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownImportResult {
    pub folder: String,
    pub count: usize,
    /// 导入统计（http / ws / graphql / socketio）
    #[serde(default)]
    pub http: usize,
    #[serde(default)]
    pub ws: usize,
    #[serde(default)]
    pub graphql: usize,
    #[serde(default)]
    pub socketio: usize,
}

#[cfg(test)]
#[path = "markdown_test.rs"]
mod tests;
