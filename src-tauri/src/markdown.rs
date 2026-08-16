//! 接口文档 Markdown：导出（render）、HTML 预览（md_to_html）、导入回读（parse）。
//! 导出与导入格式自洽，保证「查看 Markdown → 保存 → 再导入」能完整还原接口。

use crate::{ApiFile, BodyData, DocParam, KeyValue, MockConfig};
use serde_json::Value;
use std::fmt::Write as _;

// ==================== 导出 ====================

/// 把接口渲染为 Markdown 文档（一个接口 = 一个文档）
pub fn render(api: &ApiFile) -> String {
    let mut s = String::new();
    let name = api.name.trim();
    let name = if name.is_empty() { "未命名接口" } else { name };
    let _ = writeln!(s, "# {name}\n");

    let desc = api.description.trim();
    if !desc.is_empty() {
        for l in desc.lines() {
            let _ = writeln!(s, "> {}", l.trim());
        }
        let _ = writeln!(s);
    }

    let _ = writeln!(s, "## 基本信息\n");
    let _ = writeln!(s, "- 方法: {}", api.method.trim());
    let _ = writeln!(s, "- 路径: {}", api.path.trim());
    if !api.url.trim().is_empty() {
        let _ = writeln!(s, "- URL: {}", api.url.trim());
    }
    let _ = writeln!(s);

    // 请求 Header / Query / Path：扁平 KV 表（带类型/说明，来自 docParams 覆盖）
    let req_sections: [(&str, &str, &[KeyValue]); 3] = [
        ("请求 Header", "header", &api.headers),
        ("Query", "query", &api.query),
        ("Path", "path", &api.params),
    ];
    for (title, source, rows) in req_sections {
        let rows: Vec<&KeyValue> = rows.iter().filter(|r| !r.key.trim().is_empty()).collect();
        if rows.is_empty() {
            continue;
        }
        let _ = writeln!(s, "## {title}\n");
        let data: Vec<Vec<String>> = rows
            .iter()
            .map(|r| {
                let d = doc_at(api, source, &[r.key.trim()]);
                vec![
                    r.key.trim().to_string(),
                    r.value.trim().to_string(),
                    d.as_ref().map(|d| d.r#type.trim().to_string()).unwrap_or_default(),
                    d.as_ref().map(|d| d.description.trim().to_string()).unwrap_or_default(),
                ]
            })
            .collect();
        s.push_str(&md_table(&["参数名", "值", "类型", "说明"], &data));
        s.push('\n');
    }

    // Body
    render_body(api, &mut s);

    // 响应：请求成功（来自 Mock 响应体 JSON 推导）+ 请求失败（手动条目）
    let mut resp = String::new();
    let mut success_rows: Vec<(String, String, String)> = Vec::new(); // 点分路径, 类型, 说明
    if let Ok(v) = serde_json::from_str::<Value>(&api.mock.body) {
        walk_json(v, "", api, "resp_success", &mut success_rows);
    }
    if !success_rows.is_empty() {
        let _ = writeln!(resp, "### 请求成功\n");
        let data: Vec<Vec<String>> = success_rows
            .iter()
            .map(|(k, t, d)| vec![k.clone(), t.clone(), d.clone()])
            .collect();
        resp.push_str(&md_table(&["字段名", "类型", "说明"], &data));
        resp.push('\n');
    }
    let fail_docs: Vec<&DocParam> = api.doc_params.iter().filter(|d| d.source == "resp_fail").collect();
    if !fail_docs.is_empty() {
        let _ = writeln!(resp, "### 请求失败\n");
        let mut flat: Vec<(String, String, String)> = Vec::new();
        for d in fail_docs {
            flatten_doc(d, "", &mut flat);
        }
        let data: Vec<Vec<String>> = flat
            .iter()
            .map(|(k, t, desc)| vec![k.clone(), t.clone(), desc.clone()])
            .collect();
        resp.push_str(&md_table(&["字段名", "类型", "说明"], &data));
        resp.push('\n');
    }
    if !resp.is_empty() {
        let _ = writeln!(s, "## 响应\n");
        s.push_str(&resp);
    }

    // Mock
    if api.mock.enabled {
        let _ = writeln!(s, "## Mock\n");
        let _ = writeln!(s, "- 状态码: {}", api.mock.status);
        let _ = writeln!(s, "- 启用: 是\n");
        let mock_headers: Vec<&KeyValue> = api
            .mock
            .headers
            .iter()
            .filter(|r| !r.key.trim().is_empty())
            .collect();
        if !mock_headers.is_empty() {
            let _ = writeln!(s, "### Mock 请求头\n");
            let data: Vec<Vec<String>> = mock_headers
                .iter()
                .map(|r| vec![r.key.trim().to_string(), r.value.trim().to_string(), r.description.trim().to_string()])
                .collect();
            s.push_str(&md_table(&["参数名", "值", "说明"], &data));
            s.push('\n');
        }
        if !api.mock.body.trim().is_empty() {
            let _ = writeln!(s, "### Mock 响应体\n");
            let _ = writeln!(s, "```json\n{}\n```\n", api.mock.body.trim());
        }
    }

    s
}

/// Body：form 模式输出 KV 表；json 模式输出代码块；raw 模式输出代码块
fn render_body(api: &ApiFile, s: &mut String) {
    let mut body = String::new();
    match api.body.mode.as_str() {
        "form" => {
            let rows: Vec<&KeyValue> = api.body.form.iter().filter(|r| !r.key.trim().is_empty()).collect();
            if rows.is_empty() {
                return;
            }
            let _ = writeln!(body, "## Body\n");
            let data: Vec<Vec<String>> = rows
                .iter()
                .map(|r| {
                    let d = doc_at(api, "body", &[r.key.trim()]);
                    vec![
                        r.key.trim().to_string(),
                        r.value.trim().to_string(),
                        d.as_ref().map(|d| d.r#type.trim().to_string()).unwrap_or_default(),
                        d.as_ref().map(|d| d.description.trim().to_string()).unwrap_or_default(),
                    ]
                })
                .collect();
            body.push_str(&md_table(&["参数名", "值", "类型", "说明"], &data));
            body.push('\n');
        }
        "json" => {
            if api.body.raw.trim().is_empty() {
                return;
            }
            let _ = writeln!(body, "## Body\n");
            let _ = writeln!(body, "```json\n{}\n```\n", api.body.raw.trim());
        }
        "raw" => {
            if api.body.raw.trim().is_empty() {
                return;
            }
            let _ = writeln!(body, "## Body\n");
            let _ = writeln!(body, "```\n{}\n```\n", api.body.raw.trim());
        }
        _ => return,
    }
    s.push_str(&body);
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
fn flatten_doc(d: &DocParam, prefix: &str, out: &mut Vec<(String, String, String)>) {
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
fn walk_json(v: Value, prefix: &str, api: &ApiFile, source: &str, out: &mut Vec<(String, String, String)>) {
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
        Value::Object(_) => ("Object".into(), None),
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

// ==================== 导入 ====================

/// 从 Markdown 文档解析接口（支持一个文件多个 # 标题 = 多个接口）
pub fn parse(md: &str) -> Result<Vec<ApiFile>, String> {
    let blocks = split_h1(md);
    if blocks.is_empty() {
        return Err("未找到接口标题（# 接口名称）".into());
    }
    let mut apis = Vec::new();
    for b in blocks {
        if let Some(api) = parse_one(&b)? {
            apis.push(api);
        }
    }
    Ok(apis)
}

/// 按一级标题（# ）切分文档块
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

/// 解析单个接口块
fn parse_one(block: &str) -> Result<Option<ApiFile>, String> {
    let mut name = String::new();
    let mut desc_lines: Vec<String> = Vec::new();
    let mut section = String::new(); // 当前 ## 小节
    let mut subsection = String::new(); // 当前 ### 小节
    let mut sections: Vec<(String, String, String)> = Vec::new(); // (## , ### , 内容)
    let mut content = String::new();

    for raw in block.lines() {
        let line = raw.trim_end();
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
        if line.trim_start().starts_with("### ") {
            if !content.trim().is_empty() {
                sections.push((section.clone(), subsection.clone(), std::mem::take(&mut content)));
            }
            subsection = line.trim_start().trim_start_matches('#').trim().to_string();
            continue;
        }
        if line.trim_start().starts_with('>') {
            let t = line.trim_start().trim_start_matches('>').trim();
            if !t.is_empty() {
                desc_lines.push(t.to_string());
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
        method: "GET".to_string(),
        path: String::new(),
        url: String::new(),
        description: desc_lines.join("\n"),
        headers: vec![],
        query: vec![],
        params: vec![],
        body: BodyData::default(),
        mock: MockConfig::default(),
        examples: vec![],
        doc_params: vec![],
    };

    for (sec, sub, text) in &sections {
        let sec = sec.as_str();
        match sec {
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

    Ok(Some(api))
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
    let desc_col = header.iter().position(|h| h.contains("说明"));
    let mut out = Vec::new();
    for row in raw.iter().skip(1) {
        let key = row.first().cloned().unwrap_or_default();
        let col1 = row.get(1).cloned().unwrap_or_default();
        let ty = type_col.and_then(|c| row.get(c)).cloned().unwrap_or_default();
        let desc = desc_col.and_then(|c| row.get(c)).cloned().unwrap_or_default();
        out.push(TableRow { key, col1, ty, desc });
    }
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_api() -> ApiFile {
        ApiFile {
            uuid: "u1".into(),
            name: "创建用户".into(),
            method: "POST".into(),
            path: "/api/users".into(),
            url: String::new(),
            description: "创建新用户".into(),
            headers: vec![KeyValue {
                key: "Content-Type".into(),
                value: "application/json".into(),
                enabled: true,
                description: "内容类型".into(),
                is_file: false,
            }],
            query: vec![KeyValue {
                key: "verbose".into(),
                value: "1".into(),
                enabled: true,
                description: "详细输出".into(),
                is_file: false,
            }],
            params: vec![KeyValue {
                key: "id".into(),
                value: "".into(),
                enabled: true,
                description: "用户 ID".into(),
                is_file: false,
            }],
            body: BodyData {
                mode: "json".into(),
                raw: "{\"name\":\"张三\",\"age\":18,\"tags\":[\"a\",\"b\"],\"address\":{\"city\":\"北京\"}}".into(),
                form: vec![],
            },
            mock: MockConfig {
                enabled: true,
                status: 201,
                headers: vec![KeyValue {
                    key: "X-Req-Id".into(),
                    value: "abc".into(),
                    enabled: true,
                    description: "请求 ID".into(),
                    is_file: false,
                }],
                delay: 0,
                body: "{\"code\":0,\"data\":{\"name\":\"张三\"}}".into(),
            },
            examples: vec![],
            doc_params: vec![],
        }
    }

    #[test]
    fn roundtrip() {
        let api = sample_api();
        let md = render(&api);
        let parsed = parse(&md).expect("parse ok");
        assert_eq!(parsed.len(), 1);
        let a = &parsed[0];
        assert_eq!(a.name, "创建用户");
        assert_eq!(a.method, "POST");
        assert_eq!(a.path, "/api/users");
        assert_eq!(a.description, "创建新用户");
        assert_eq!(a.headers.len(), 1);
        assert_eq!(a.headers[0].key, "Content-Type");
        assert_eq!(a.headers[0].value, "application/json");
        assert_eq!(a.query.len(), 1);
        assert_eq!(a.params.len(), 1);
        assert_eq!(a.body.mode, "json");
        assert!(a.body.raw.contains("张三"));
        assert!(a.mock.enabled);
        assert_eq!(a.mock.status, 201);
        assert_eq!(a.mock.headers.len(), 1);
        assert!(a.mock.body.contains("code"));
    }

    #[test]
    fn md_html_basic() {
        let html = md_to_html("# 标题\n\n> 说明\n\n- a\n- b\n\n| 参数名 | 值 |\n| --- | --- |\n| x | 1 |\n\n```json\n{\"a\":1}\n```\n");
        assert!(html.contains("<h1>标题</h1>"));
        assert!(html.contains("<blockquote>说明</blockquote>"));
        assert!(html.contains("<ul>"));
        assert!(html.contains("<table>"));
        assert!(html.contains("<pre"));
    }

    #[test]
    fn md_html_no_hang_on_subheadings() {
        // 回归：渲染结果含 ## / ### 子标题与单行表格，必须能正常退出（曾因段落分支不前进导致死循环卡死应用）
        let html = md_to_html(
            "# 创建用户\n\n## 基本信息\n\n- 方法: POST\n\n## 响应\n\n### 请求成功\n\n| 字段名 | 类型 | 说明 |\n| --- | --- | --- |\n| code | Integer | 状态码 |\n| 孤立 | 行 | 表 |\n\n## Mock\n",
        );
        assert!(html.contains("<h2>基本信息</h2>"));
        assert!(html.contains("<h3>请求成功</h3>"));
        assert!(html.contains("<table>"));
    }

    #[test]
    fn doc_table_roundtrip() {
        // 响应表格（字段名|类型|说明，点分路径）→ docParams 树
        let md = "# 测试\n\n## 响应\n\n### 请求成功\n\n| 字段名 | 类型 | 说明 |\n| --- | --- | --- |\n| code | Integer | 状态码 |\n| data.name | String | 姓名 |\n";
        let parsed = parse(md).expect("parse ok");
        let a = &parsed[0];
        let success: Vec<&DocParam> = a.doc_params.iter().filter(|d| d.source == "resp_success").collect();
        assert_eq!(success.len(), 2);
        let data = success.iter().find(|d| d.key == "data").unwrap();
        assert_eq!(data.children.len(), 1);
        assert_eq!(data.children[0].key, "name");
        assert_eq!(data.children[0].description, "姓名");
    }
}
