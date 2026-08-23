// ==================== 对象管理（数据结构 / JSON 导入 / 唯一标识 / 引用统计） ====================
// 对象存储在 <工作区>/.objects/objects.json：
//   groups  : 对象分组（可多级？目前为平铺分组，name 支持 "父级/子级" 命名实现多级）
//   objects : 对象定义（属性、引用、统计）
//
// 唯一标识（hash）：对象所有属性按 key 字母排序，拼接 "key:kind[:itemKind][:refHash]" 后
// 做 SHA-256 取前 12 位。相同结构（含引用）的对象 hash 相同，创建时直接复用已有对象。

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ObjectGroup {
    pub id: String,
    pub name: String,
}

impl Default for ObjectGroup {
    fn default() -> Self {
        Self { id: String::new(), name: String::new() }
    }
}

/// 对象属性类型


#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ObjectProp {
    pub key: String,
    /// string / number / boolean / object / list / any
    pub kind: String,
    /// list 的元素类型（string / number / boolean / object / any）
    pub item_kind: String,
    /// object / list(object) 引用的对象 hash（空表示未引用）
    pub ref_hash: String,
    pub description: String,
    pub required: bool,
}

impl Default for ObjectProp {
    fn default() -> Self {
        Self {
            key: String::new(),
            kind: "string".into(),
            item_kind: "string".into(),
            ref_hash: String::new(),
            description: String::new(),
            required: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ObjectDef {
    /// 唯一标识：属性按 key 排序拼接后的 SHA-256 前 12 位
    pub hash: String,
    pub name: String,
    /// 所属分组 id（空串为未分组）
    pub group: String,
    pub description: String,
    pub properties: Vec<ObjectProp>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Default for ObjectDef {
    fn default() -> Self {
        Self {
            hash: String::new(),
            name: String::new(),
            group: String::new(),
            description: String::new(),
            properties: vec![],
            created_at: 0,
            updated_at: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ObjectStore {
    pub groups: Vec<ObjectGroup>,
    pub objects: Vec<ObjectDef>,
}

impl Default for ObjectStore {
    fn default() -> Self {
        Self { groups: vec![], objects: vec![] }
    }
}

/// 对象被接口文档引用的统计（供对象列表「接口数量」与跳转）
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectUsageApi {
    pub name: String,
    pub method: String,
    pub path: String,
    pub protocol: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectUsageItem {
    pub hash: String,
    pub api_count: usize,
    pub apis: Vec<ObjectUsageApi>,
}

/// JSON 导入结果：新建对象与复用对象（嵌套 object 提取为独立对象，hash 相同则复用）
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectImportResult {
    pub objects: Vec<ObjectDef>,
    pub created: Vec<String>,
    pub reused: Vec<String>,
    /// 顶层对象 hash（复用场景下指向已有对象）
    pub top_hash: String,
}

fn now_ts() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// 计算对象 hash：属性按 key 字母排序，拼接 "key:kind[:itemKind][:refHash]" 后 SHA-256 前 12 位
pub fn object_hash(props: &[ObjectProp]) -> String {
    let mut parts: Vec<String> = props
        .iter()
        .map(|p| {
            let mut s = format!("{}:{}", p.key.trim(), p.kind);
            if p.kind == "list" {
                s.push(':');
                s.push_str(&p.item_kind);
            }
            if (p.kind == "object" || (p.kind == "list" && p.item_kind == "object"))
                && !p.ref_hash.is_empty()
            {
                s.push(':');
                s.push_str(&p.ref_hash);
            }
            s
        })
        .collect();
    parts.sort();
    let joined = parts.join(",");
    let mut hasher = Sha256::new();
    hasher.update(joined.as_bytes());
    let digest = hasher.finalize();
    let hex: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
    hex[..12].to_string()
}

/// 在 store 中按 hash 查找对象
pub fn find_object<'a>(store: &'a ObjectStore, hash: &str) -> Option<&'a ObjectDef> {
    store.objects.iter().find(|o| o.hash == hash)
}

/// 按名字查找对象（文档引用通过名字关联）
#[allow(dead_code)]
pub fn find_object_by_name<'a>(store: &'a ObjectStore, name: &str) -> Option<&'a ObjectDef> {
    let n = name.trim();
    if n.is_empty() {
        return None;
    }
    store.objects.iter().find(|o| o.name == n)
}

/// 列出对象存储（无文件时返回空）
pub fn list_objects(root: &Path) -> Result<ObjectStore, String> {
    let file = root.join(".objects").join("objects.json");
    if !file.exists() {
        return Ok(ObjectStore::default());
    }
    let text = std::fs::read_to_string(&file).map_err(|e| format!("读取对象文件失败: {e}"))?;
    let store: ObjectStore = serde_json::from_str(&text).map_err(|e| format!("解析对象文件失败: {e}"))?;
    Ok(store)
}

/// 保存对象存储（整体覆盖写）。
/// 保存时重新计算每个对象的 hash（属性变化后保持一致），
/// 并修复失效引用（refHash 指向不存在的对象时尝试按名称匹配，否则清空）。
pub fn save_objects(root: &Path, store: &ObjectStore) -> Result<String, String> {
    let dir = root.join(".objects");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建对象目录失败: {e}"))?;
    let file = dir.join("objects.json");

    let mut objects: Vec<ObjectDef> = Vec::new();
    for mut o in store.objects.clone() {
        o.hash = object_hash(&o.properties);
        objects.push(o);
    }
    let name_map: HashMap<String, String> = objects
        .iter()
        .map(|o| (o.name.clone(), o.hash.clone()))
        .collect();
    let hashes: std::collections::HashSet<String> =
        objects.iter().map(|o| o.hash.clone()).collect();
    for o in &mut objects {
        for p in &mut o.properties {
            if !p.ref_hash.is_empty() && !hashes.contains(&p.ref_hash) {
                if let Some(h) = name_map.get(&p.ref_hash) {
                    p.ref_hash = h.clone();
                } else {
                    p.ref_hash.clear();
                }
            }
        }
    }
    let store = ObjectStore {
        groups: store.groups.clone(),
        objects,
    };
    let text = serde_json::to_string_pretty(&store).map_err(|e| format!("序列化对象失败: {e}"))?;
    std::fs::write(&file, text).map_err(|e| format!("写入对象文件失败: {e}"))?;
    Ok(file.to_string_lossy().to_string())
}

/// 从 JSON 文本生成对象定义（嵌套 object 提取为独立对象，hash 相同则复用）。
/// 返回创建的对象列表（顶层对象排最后，嵌套对象在前，便于前端合并）。
pub fn import_json_object(
    root: &Path,
    name: &str,
    group: &str,
    json_text: &str,
) -> Result<ObjectImportResult, String> {
    let mut store = list_objects(root)?;
    let mut created: Vec<String> = Vec::new();
    let mut reused: Vec<String> = Vec::new();
    let mut generated: Vec<ObjectDef> = Vec::new();

    let value: serde_json::Value =
        serde_json::from_str(json_text).map_err(|e| format!("JSON 解析失败: {e}"))?;

    let name = name.trim().to_string();
    let group = group.trim().to_string();
    if name.is_empty() {
        return Err("对象名称不能为空".into());
    }

    // 递归生成：返回该 JSON 值对应的对象 hash（object 才生成对象）
    fn build(
        store: &mut ObjectStore,
        generated: &mut Vec<ObjectDef>,
        created: &mut Vec<String>,
        reused: &mut Vec<String>,
        group: &str,
        v: &serde_json::Value,
        suggested_name: &str,
    ) -> Result<String, String> {
        // 对象：顶层与嵌套 object
        let mut props: Vec<ObjectProp> = Vec::new();
        if let serde_json::Value::Object(map) = v {
            let mut entries: Vec<(&String, &serde_json::Value)> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            for (k, val) in entries {
                let mut p = ObjectProp {
                    key: k.clone(),
                    required: true,
                    ..ObjectProp::default()
                };
                match val {
                    serde_json::Value::String(_) => {
                        p.kind = "string".into();
                    }
                    serde_json::Value::Number(n) => {
                        p.kind = if n.as_i64().is_some() || n.as_u64().is_some() {
                            "number".into()
                        } else {
                            "number".into()
                        };
                    }
                    serde_json::Value::Bool(_) => p.kind = "boolean".into(),
                    serde_json::Value::Null => p.kind = "any".into(),
                    serde_json::Value::Array(arr) => {
                        let item_kind = if arr.is_empty() {
                            "any".to_string()
                        } else {
                            // 取第一个非空元素推断类型
                            let mut it = "any".to_string();
                            for el in arr {
                                match el {
                                    serde_json::Value::String(_) => it = "string".into(),
                                    serde_json::Value::Number(_) => it = "number".into(),
                                    serde_json::Value::Bool(_) => it = "boolean".into(),
                                    serde_json::Value::Object(_) => {
                                        it = "object".into();
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                            it
                        };
                        p.kind = "list".into();
                        p.item_kind = item_kind.clone();
                        if item_kind == "object" {
                            // 提取数组元素对象为独立对象
                            if let Some(first) = arr.iter().find(|e| e.is_object()) {
                                let child_name = format!("{}{}", suggested_name, capitalize(k));
                                let h = build(
                                    store,
                                    generated,
                                    created,
                                    reused,
                                    group,
                                    first,
                                    &child_name,
                                )?;
                                p.ref_hash = h;
                            }
                        }
                    }
                    serde_json::Value::Object(_) => {
                        p.kind = "object".into();
                        let child_name = format!("{}{}", suggested_name, capitalize(k));
                        let h = build(store, generated, created, reused, group, val, &child_name)?;
                        p.ref_hash = h;
                    }
                }
                props.push(p);
            }
        } else {
            return Err("JSON 顶层必须是对象".into());
        }

        let hash = object_hash(&props);
        // hash 相同：直接复用已有对象
        if let Some(_existing) = find_object(store, &hash) {
            if !reused.contains(&hash) {
                reused.push(hash.clone());
            }
            return Ok(hash);
        }
        // 已在本轮生成列表中（嵌套结构相同）
        if let Some(_gen) = generated.iter().find(|g| g.hash == hash) {
            if !reused.contains(&hash) {
                reused.push(hash.clone());
            }
            return Ok(hash);
        }
        let now = now_ts();
        let def = ObjectDef {
            hash: hash.clone(),
            name: suggested_name.to_string(),
            group: group.to_string(),
            description: String::new(),
            properties: props,
            created_at: now,
            updated_at: now,
        };
        generated.push(def);
        created.push(hash.clone());
        Ok(hash)
    }

    let top_hash = build(&mut store, &mut generated, &mut created, &mut reused, &group, &value, &name)?;
    // 顶层对象优先使用用户输入的名字
    if let Some(top) = generated.iter_mut().find(|g| g.hash == top_hash) {
        top.name = name.clone();
        if !group.is_empty() {
            top.group = group.clone();
        }
    }

    // 合并新建对象并持久化（保证后续导入能按 hash 复用已有对象）
    if !generated.is_empty() {
        store.objects.extend(generated.clone());
        save_objects(root, &store)?;
    }

    Ok(ObjectImportResult {
        objects: generated,
        created,
        reused,
        top_hash,
    })
}

/// 通过 SQL CREATE TABLE 建表语句生成对象（每个表一个对象）。
/// 类型映射见 map_sql_type；列名 → key，NOT NULL/PRIMARY KEY → 必填，COMMENT → 描述；
/// 表级约束（PRIMARY KEY(...)/FOREIGN KEY/UNIQUE KEY/CONSTRAINT/CHECK/INDEX）自动忽略。
pub fn import_ddl(root: &Path, group: &str, ddl: &str) -> Result<ObjectImportResult, String> {
    let mut store = list_objects(root)?;
    let mut created: Vec<String> = Vec::new();
    let mut reused: Vec<String> = Vec::new();
    let mut generated: Vec<ObjectDef> = Vec::new();
    let group = group.trim().to_string();

    let tables = parse_create_tables(ddl);
    if tables.is_empty() {
        return Err("未识别到 CREATE TABLE 语句".into());
    }
    let mut top_hash = String::new();
    for (table_name, body) in tables {
        let mut props: Vec<ObjectProp> = Vec::new();
        for col in split_columns(&body) {
            if let Some((key, kind, required, desc)) = parse_column(&col) {
                props.push(ObjectProp {
                    key,
                    kind,
                    required,
                    description: desc,
                    ..ObjectProp::default()
                });
            }
        }
        let hash = object_hash(&props);
        // hash 相同：复用已有对象（含本轮已生成）
        if find_object(&store, &hash).is_some() || generated.iter().any(|g| g.hash == hash) {
            if !reused.contains(&hash) {
                reused.push(hash.clone());
            }
            if top_hash.is_empty() {
                top_hash = hash.clone();
            }
            continue;
        }
        let now = now_ts();
        let def = ObjectDef {
            hash: hash.clone(),
            name: table_name.clone(),
            group: group.clone(),
            description: String::new(),
            properties: props,
            created_at: now,
            updated_at: now,
        };
        if top_hash.is_empty() {
            top_hash = hash.clone();
        }
        generated.push(def);
        created.push(hash.clone());
    }
    if !generated.is_empty() {
        store.objects.extend(generated.clone());
        save_objects(root, &store)?;
    }
    Ok(ObjectImportResult {
        objects: generated,
        created,
        reused,
        top_hash,
    })
}

/// 提取 DDL 中所有 CREATE TABLE 语句 → (表名, 表体内容)
fn parse_create_tables(ddl: &str) -> Vec<(String, String)> {
    let bytes = ddl.as_bytes();
    let lower = ddl.to_ascii_lowercase();
    let n = bytes.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        let Some(rel) = lower[i..].find("create table") else { break };
        let ct = i + rel;
        let mut p = ct + "create table".len();
        let seg = &lower[p..];
        if seg.trim_start().starts_with("if not exists") {
            p += seg.find("if not exists").unwrap() + "if not exists".len();
        }
        while p < n && bytes[p].is_ascii_whitespace() {
            p += 1;
        }
        let name_start = p;
        while p < n && !bytes[p].is_ascii_whitespace() && bytes[p] != b'(' {
            p += 1;
        }
        let raw_name = ddl[name_start..p].trim();
        if raw_name.is_empty() {
            i = p;
            continue;
        }
        // 跳到 '('（遇到 ';' 说明无表体，跳过该段）
        while p < n && bytes[p] != b'(' {
            if bytes[p] == b';' {
                break;
            }
            p += 1;
        }
        if p >= n || bytes[p] != b'(' {
            i = p;
            continue;
        }
        // 括号匹配收集表体（忽略注释与字符串字面量；char_indices 保持字节偏移，避免中文被逐字节拆分）
        let body_start = p + 1;
        let mut it = ddl[body_start..].char_indices().peekable();
        let mut depth = 1usize;
        let mut in_str = false;
        let mut line_comment = false;
        let mut block_comment = false;
        let mut body = String::new();
        let mut end_rel = 0usize;
        while depth > 0 {
            let Some((rel, c)) = it.next() else { break };
            end_rel = rel + c.len_utf8();
            if line_comment {
                if c == '\n' {
                    line_comment = false;
                }
                body.push(' ');
                continue;
            }
            if block_comment {
                if c == '*' && it.peek().map(|(_, x)| *x) == Some('/') {
                    block_comment = false;
                    it.next();
                    body.push(' ');
                    body.push(' ');
                } else {
                    body.push(' ');
                }
                continue;
            }
            if in_str {
                if c == '\'' {
                    if it.peek().map(|(_, x)| *x) == Some('\'') {
                        body.push('\'');
                        body.push('\'');
                        it.next();
                    } else {
                        in_str = false;
                        body.push('\'');
                    }
                } else {
                    body.push(c);
                }
                continue;
            }
            match c {
                '\'' => {
                    in_str = true;
                    body.push('\'');
                }
                '-' if it.peek().map(|(_, x)| *x) == Some('-') => {
                    line_comment = true;
                    it.next();
                    body.push(' ');
                    body.push(' ');
                }
                '/' if it.peek().map(|(_, x)| *x) == Some('*') => {
                    block_comment = true;
                    it.next();
                    body.push(' ');
                    body.push(' ');
                }
                '(' => {
                    depth += 1;
                    body.push('(');
                }
                ')' => {
                    depth -= 1;
                    if depth > 0 {
                        body.push(')');
                    }
                }
                _ => body.push(c),
            }
        }
        out.push((clean_table_name(raw_name), body));
        i = body_start + end_rel;
    }
    out
}

/// 表名清洗：schema.table / "quoted" / `backtick` / [bracket] → 取末段并去引号
fn clean_table_name(raw: &str) -> String {
    let mut last = raw.rsplit('.').next().unwrap_or(raw);
    for ch in ['"', '`', '[', ']'] {
        last = last.trim_matches(ch);
    }
    last.to_string()
}

/// 按顶层逗号分割列定义（忽略括号内与字符串内的逗号；按字符迭代避免中文被拆字节）
fn split_columns(body: &str) -> Vec<String> {
    let mut cols = Vec::new();
    let mut cur = String::new();
    let mut depth = 0usize;
    let mut in_str = false;
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        if in_str {
            if c == '\'' {
                if chars.peek() == Some(&'\'') {
                    cur.push('\'');
                    cur.push('\'');
                    chars.next();
                } else {
                    in_str = false;
                    cur.push('\'');
                }
            } else {
                cur.push(c);
            }
            continue;
        }
        match c {
            '\'' => {
                in_str = true;
                cur.push('\'');
            }
            '(' => {
                depth += 1;
                cur.push('(');
            }
            ')' => {
                if depth > 0 {
                    depth -= 1;
                }
                cur.push(')');
            }
            ',' if depth == 0 => {
                cols.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    if !cur.trim().is_empty() {
        cols.push(cur.trim().to_string());
    }
    cols
}

/// 解析单列定义 → (列名, 类型映射, 必填, 描述)；表级约束返回 None
fn parse_column(col: &str) -> Option<(String, String, bool, String)> {
    let t = col.trim();
    if t.is_empty() {
        return None;
    }
    let upper = t.to_ascii_uppercase();
    for kw in [
        "PRIMARY KEY",
        "UNIQUE KEY",
        "UNIQUE INDEX",
        "FOREIGN KEY",
        "CONSTRAINT",
        "CHECK",
        "KEY",
        "INDEX",
        "FULLTEXT",
    ] {
        if upper.starts_with(kw) {
            return None;
        }
    }
    let (name, rest) = split_name_rest(t)?;
    let name = name
        .trim_matches('"')
        .trim_matches('`')
        .trim_matches('[')
        .trim_matches(']')
        .to_string();
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    // 类型 token（可能带 (…) 参数，如 VARCHAR(50) / DECIMAL(10,2)）
    let b = rest.as_bytes();
    let mut p = 0;
    while p < b.len() && !b[p].is_ascii_whitespace() && b[p] != b'(' {
        p += 1;
    }
    let type_tok: String = rest[..p]
        .chars()
        .take_while(|c| c.is_ascii_alphabetic() || *c == '_')
        .collect();
    let mut q = p;
    if q < b.len() && b[q] == b'(' {
        let mut depth = 0usize;
        while q < b.len() {
            match b[q] {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        q += 1;
                        break;
                    }
                }
                _ => {}
            }
            q += 1;
        }
    }
    let attrs = rest[q..].to_string();
    let upper_attrs = attrs.to_ascii_uppercase();
    let required = upper_attrs.contains("NOT NULL") || upper_attrs.contains("PRIMARY KEY");
    let desc = extract_comment(&attrs);
    Some((name, map_sql_type(&type_tok).to_string(), required, desc))
}

/// 拆分列名与其余部分（支持 "quoted name" / `name` / [name]）
fn split_name_rest(t: &str) -> Option<(&str, &str)> {
    let t = t.trim_start();
    if t.is_empty() {
        return None;
    }
    let first = t.as_bytes()[0];
    let close = match first {
        b'"' => Some(b'"'),
        b'`' => Some(b'`'),
        b'[' => Some(b']'),
        _ => None,
    };
    if let Some(close) = close {
        let b = t.as_bytes();
        let mut i = 1;
        while i < b.len() && b[i] != close {
            i += 1;
        }
        if i >= b.len() {
            return None;
        }
        Some((&t[..=i], t[i + 1..].trim_start()))
    } else {
        match t.find(char::is_whitespace) {
            Some(i) => Some((&t[..i], t[i..].trim_start())),
            None => Some((t, "")),
        }
    }
}

/// 提取列定义中的 COMMENT '...' 作为描述
fn extract_comment(attrs: &str) -> String {
    let lower = attrs.to_ascii_lowercase();
    let mut idx = 0;
    while let Some(rel) = lower[idx..].find("comment") {
        let pos = idx + rel;
        let rest = &attrs[pos + "comment".len()..];
        let rest_trim = rest.trim_start();
        if let Some(inner) = rest_trim.strip_prefix('\'') {
            let mut out = String::new();
            let mut chars = inner.chars().peekable();
            while let Some(c) = chars.next() {
                if c == '\'' {
                    if chars.peek() == Some(&'\'') {
                        out.push('\'');
                        chars.next();
                    } else {
                        break;
                    }
                } else {
                    out.push(c);
                }
            }
            return out;
        }
        idx = pos + 1;
    }
    String::new()
}

/// SQL 类型 → 对象属性类型
fn map_sql_type(t: &str) -> &'static str {
    let t = t.to_ascii_uppercase();
    if t.starts_with("INT")
        || t.starts_with("SERIAL")
        || t.starts_with("BIGINT")
        || t.starts_with("SMALLINT")
        || t.starts_with("TINYINT")
        || t.starts_with("MEDIUMINT")
        || t.starts_with("DEC")
        || t.starts_with("NUM")
        || t.starts_with("FLOAT")
        || t.starts_with("DOUBLE")
        || t.starts_with("REAL")
        || t.starts_with("MONEY")
        || t == "YEAR"
    {
        "number"
    } else if t.starts_with("BOOL") || t == "BIT" {
        "boolean"
    } else if t.starts_with("DATE") || t.starts_with("TIME") || t.starts_with("TIMESTAMP") {
        "string"
    } else if t.starts_with("JSON") {
        "any"
    } else {
        // VARCHAR / CHAR / TEXT / BLOB / CLOB / BINARY / ENUM / UUID 等
        "string"
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// 统计每个对象被接口文档引用的数量与接口列表。
/// 遍历工作区接口 json（排除 .history/.examples/.objects/.versions/__info.json），
/// 通过 docParams 的 objectName（Object 类型）匹配对象名。
pub fn object_usage(root: &Path, store: &ObjectStore) -> Result<Vec<ObjectUsageItem>, String> {
    let mut by_name: HashMap<String, Vec<ObjectUsageApi>> = HashMap::new();

    // 递归收集 docParams 中的 objectName 引用
    fn collect_docs(docs: &[crate::DocParam], api: &ObjectUsageApi, by: &mut HashMap<String, Vec<ObjectUsageApi>>) {
        for d in docs {
            if !d.object_name.trim().is_empty() {
                by.entry(d.object_name.trim().to_string()).or_default().push(api.clone());
            }
            collect_docs(&d.children, api, by);
        }
    }

    fn walk_dir(dir: &Path, by: &mut HashMap<String, Vec<ObjectUsageApi>>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(".") && name != ".git" {
                    continue; // 跳过 .history / .examples / .objects / .versions 等隐藏目录
                }
                walk_dir(&path, by);
                continue;
            }
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                let fname = entry.file_name().to_string_lossy().to_string();
                if fname == "__info.json" {
                    continue;
                }
                if let Ok(text) = std::fs::read_to_string(&path) {
                    if let Ok(api) = serde_json::from_str::<crate::ApiFile>(&text) {
                        let usage = ObjectUsageApi {
                            name: api.name.clone(),
                            method: api.method.clone(),
                            path: api.path.clone(),
                            protocol: api.protocol.clone(),
                        };
                        collect_docs(&api.doc_params, &usage, by);
                    }
                }
            }
        }
    }
    walk_dir(root, &mut by_name);

    let mut items: Vec<ObjectUsageItem> = Vec::new();
    for obj in &store.objects {
        let apis = by_name.remove(&obj.name).unwrap_or_default();
        items.push(ObjectUsageItem {
            hash: obj.hash.clone(),
            api_count: apis.len(),
            apis,
        });
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("apim-objects-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn test_object_hash_sorted_keys() {
        let mut p1 = vec![
            ObjectProp { key: "b".into(), kind: "string".into(), ..Default::default() },
            ObjectProp { key: "a".into(), kind: "number".into(), ..Default::default() },
        ];
        let h1 = object_hash(&p1);
        assert_eq!(h1.len(), 12);
        // 顺序无关
        p1.reverse();
        assert_eq!(object_hash(&p1), h1);
        // 不同结构不同 hash
        let p2 = vec![
            ObjectProp { key: "a".into(), kind: "number".into(), ..Default::default() },
            ObjectProp { key: "b".into(), kind: "boolean".into(), ..Default::default() },
        ];
        assert_ne!(object_hash(&p2), h1);
        // list + 引用参与 hash
        let p3 = vec![
            ObjectProp { key: "a".into(), kind: "list".into(), item_kind: "object".into(), ref_hash: "x".into(), ..Default::default() },
        ];
        let p4 = vec![
            ObjectProp { key: "a".into(), kind: "list".into(), item_kind: "object".into(), ref_hash: "y".into(), ..Default::default() },
        ];
        assert_ne!(object_hash(&p3), object_hash(&p4));
    }

    #[test]
    fn test_import_json_nested_and_reuse() {
        let root = tmpdir("import");
        let json = r#"{"name":"alice","age":18,"addr":{"city":"bj","zip":"100000"},"tags":["a","b"],"orders":[{"id":1}]}"#;
        let res = import_json_object(&root, "User", "g1", json).unwrap();
        // 顶层 User + 嵌套 Addr + 嵌套 OrdersItem
        assert_eq!(res.objects.len(), 3, "应有 User/Addr/OrdersItem 三个对象");
        assert_eq!(res.created.len(), 3);
        let names: Vec<&str> = res.objects.iter().map(|o| o.name.as_str()).collect();
        assert!(names.contains(&"User"));
        assert!(names.contains(&"UserAddr"));
        assert!(names.contains(&"UserOrders"));
        // 顶层 User 归组 g1
        let user = res.objects.iter().find(|o| o.name == "User").unwrap();
        assert_eq!(user.group, "g1");
        // 引用关系
        let addr_prop = user.properties.iter().find(|p| p.key == "addr").unwrap();
        assert_eq!(addr_prop.kind, "object");
        assert!(!addr_prop.ref_hash.is_empty());
        let orders_prop = user.properties.iter().find(|p| p.key == "orders").unwrap();
        assert_eq!(orders_prop.kind, "list");
        assert_eq!(orders_prop.item_kind, "object");

        // 第二次导入相同结构：全部复用
        let res2 = import_json_object(&root, "User2", "g2", json).unwrap();
        assert_eq!(res2.created.len(), 0, "相同结构应复用");
        assert_eq!(res2.reused.len(), 3);
        assert_eq!(res2.objects.len(), 0, "复用时不重建对象");
        // top_hash 指向已存在的顶层对象（User 或 User2，结构相同 hash 相同）
        let store = list_objects(&root).unwrap();
        assert!(store.objects.iter().any(|o| o.hash == res2.top_hash), "top_hash 应在 store 中");
    }

    #[test]
    fn test_import_json_invalid() {
        let root = tmpdir("invalid");
        let r = import_json_object(&root, "X", "", "{bad");
        assert!(r.is_err());
        let r2 = import_json_object(&root, "X", "", "[1,2,3]");
        assert!(r2.is_err(), "顶层数组应报错");
    }

    #[test]
    fn test_parse_create_tables_basic() {
        let ddl = r#"
CREATE TABLE users (
  id BIGINT PRIMARY KEY,
  name VARCHAR(50) NOT NULL COMMENT '用户名称',
  age INT,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS public.orders (
  order_id INT NOT NULL,
  amount DECIMAL(10,2),
  note TEXT,
  PRIMARY KEY (order_id)
);
"#;
        let tables = parse_create_tables(ddl);
        assert_eq!(tables.len(), 2);
        assert_eq!(tables[0].0, "users");
        assert_eq!(tables[1].0, "orders", "IF NOT EXISTS 与 schema 前缀应处理");
        let cols = split_columns(&tables[0].1);
        assert_eq!(cols.len(), 4);
        let (name, kind, required, desc) = parse_column(&cols[0]).unwrap();
        assert_eq!(name, "id");
        assert_eq!(kind, "number");
        assert!(required, "PRIMARY KEY 应必填");
        let (name, kind, required, desc) = parse_column(&cols[1]).unwrap();
        assert_eq!(name, "name");
        assert_eq!(kind, "string");
        assert!(required);
        assert_eq!(desc, "用户名称", "COMMENT 应提取为描述");
        let (_, kind, required, _) = parse_column(&cols[2]).unwrap();
        assert_eq!(kind, "number");
        assert!(!required);
        // 表级约束应被忽略
        let constraint_cols: Vec<_> = split_columns(&tables[1].1);
        let parsed: Vec<_> = constraint_cols.iter().filter_map(|c| parse_column(c)).collect();
        assert_eq!(parsed.len(), 3, "PRIMARY KEY(...) 约束行应被跳过");
    }

    #[test]
    fn test_import_ddl_creates_and_reuses() {
        let root = tmpdir("ddl");
        let ddl = "CREATE TABLE t_user (id INT NOT NULL, name VARCHAR(50));";
        let res = import_ddl(&root, "db", ddl).unwrap();
        assert_eq!(res.created.len(), 1);
        assert_eq!(res.objects[0].name, "t_user");
        assert_eq!(res.objects[0].group, "db");
        assert_eq!(res.objects[0].properties.len(), 2);
        // 相同结构再次导入 → 复用
        let res2 = import_ddl(&root, "db", ddl).unwrap();
        assert_eq!(res2.created.len(), 0);
        assert_eq!(res2.reused.len(), 1);
        // 多表
        let ddl2 = "CREATE TABLE a (x INT);\nCREATE TABLE b (y VARCHAR(10) NOT NULL);";
        let res3 = import_ddl(&root, "", ddl2).unwrap();
        assert_eq!(res3.created.len(), 2);
    }

    #[test]
    fn test_import_ddl_quoted_and_comments() {
        let root = tmpdir("ddl2");
        let ddl = r#"
-- 用户表
CREATE TABLE `my_users` (
  `first name` VARCHAR(30) NOT NULL COMMENT '名字',
  -- 备注字段
  bio TEXT,
  CONSTRAINT pk PRIMARY KEY (`first name`)
);
"#;
        let res = import_ddl(&root, "", ddl).unwrap();
        assert_eq!(res.created.len(), 1);
        let o = &res.objects[0];
        assert_eq!(o.name, "my_users", "反引号表名应清洗");
        assert_eq!(o.properties.len(), 2, "CONSTRAINT 行与 -- 注释应忽略");
        let first = o.properties.iter().find(|p| p.key == "first name").unwrap();
        assert!(first.required);
        assert_eq!(first.description, "名字");
        let bio = o.properties.iter().find(|p| p.key == "bio").unwrap();
        assert_eq!(bio.kind, "string");
        assert!(!bio.required);
    }

    #[test]
    fn test_save_recomputes_hash() {
        let root = tmpdir("save");
        let mut store = ObjectStore::default();
        let mut o = ObjectDef {
            hash: "stale".into(),
            name: "A".into(),
            properties: vec![ObjectProp { key: "x".into(), kind: "string".into(), ..Default::default() }],
            ..Default::default()
        };
        store.objects.push(o.clone());
        save_objects(&root, &store).unwrap();
        let loaded = list_objects(&root).unwrap();
        assert_eq!(loaded.objects[0].hash, object_hash(&o.properties));
        assert_ne!(loaded.objects[0].hash, "stale");
    }
}
