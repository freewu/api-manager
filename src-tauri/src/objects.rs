// ==================== 对象管理（数据结构 / JSON 导入 / 唯一标识 / 引用统计） ====================
// 对象存储在 <工作区>/.object/ 目录，目录即分组：
//   .object/__info_obj.json               : 分组信息（ObjectGroup 列表，目录代表分组）
//   .object/<对象名称>.obj.json            : 未分组对象
//   .object/<分组路径>/<对象名称>.obj.json  : 分组对象（多级分组 = 嵌套目录）
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
    /// 已废弃标记（展示用，不影响功能）
    pub deprecated: bool,
}

impl Default for ObjectGroup {
    fn default() -> Self {
        Self { id: String::new(), name: String::new(), deprecated: false }
    }
}

/// 对象属性类型


#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ObjectProp {
    pub key: String,
    /// String / Integer / Float / Boolean / Datetime / Date / Time / List / Object / Any
    pub kind: String,
    /// List 的元素类型（String / Integer / Float / Boolean / Datetime / Date / Time / Object / Any）
    pub item_kind: String,
    /// Object / List(Object) 引用的对象 hash（空表示未引用）
    pub ref_hash: String,
    pub description: String,
    /// mock 值（示例数据，不参与结构 hash）
    pub mock: String,
}

impl Default for ObjectProp {
    fn default() -> Self {
        Self {
            key: String::new(),
            kind: "String".into(),
            item_kind: "String".into(),
            ref_hash: String::new(),
            description: String::new(),
            mock: String::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ObjectDef {
    /// 稳定标识（不随属性变化，用于版本管理目录 .object_version/<uuid>/）
    #[serde(default)]
    pub uuid: String,
    /// 唯一标识：属性按 key 排序拼接后的 SHA-256 前 12 位
    pub hash: String,
    pub name: String,
    /// 代码生成类名（可空；不设置则不生成代码）
    #[serde(rename = "object_name", default)]
    pub object_name: String,
    /// Java 包名（可空；生成 Java 代码时输出 package 语句）
    #[serde(rename = "package_name", default)]
    pub package_name: String,
    /// 所属分组 id（空串为未分组）
    pub group: String,
    /// 已废弃标记（展示用，不影响功能）
    pub deprecated: bool,
    pub description: String,
    pub properties: Vec<ObjectProp>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Default for ObjectDef {
    fn default() -> Self {
        Self {
            uuid: String::new(),
            hash: String::new(),
            name: String::new(),
            object_name: String::new(),
            package_name: String::new(),
            group: String::new(),
            deprecated: false,
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
            if p.kind == "List" {
                s.push(':');
                s.push_str(&p.item_kind);
            }
            if (p.kind == "Object" || (p.kind == "List" && p.item_kind == "Object"))
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
pub fn list_objects_impl(root: &Path) -> Result<ObjectStore, String> {
    let dir = root.join(crate::OBJECT_DATA_DIR);
    if !dir.exists() {
        return Ok(ObjectStore::default());
    }
    let mut groups: Vec<ObjectGroup> = Vec::new();
    let mut objects: Vec<ObjectDef> = Vec::new();

    // 分组信息（__info_obj.json，含已废弃标记）：目录重建时合并，旧数据无此文件则回退 false
    let mut info_deprecated: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    if let Ok(text) = std::fs::read_to_string(dir.join("__info_obj.json")) {
        if let Ok(list) = serde_json::from_str::<Vec<ObjectGroup>>(&text) {
            for g in list {
                info_deprecated.insert(g.id, g.deprecated);
            }
        }
    }

    // 递归扫描：目录 = 分组（多级嵌套），*.obj.json = 对象
    fn scan(
        dir: &Path,
        group_id: &str,
        groups: &mut Vec<ObjectGroup>,
        objects: &mut Vec<ObjectDef>,
        info_deprecated: &std::collections::HashMap<String, bool>,
    ) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            let fname = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                let id = if group_id.is_empty() {
                    fname.clone()
                } else {
                    format!("{group_id}/{fname}")
                };
                groups.push(ObjectGroup {
                    id: id.clone(),
                    name: fname,
                    deprecated: info_deprecated.get(&id).copied().unwrap_or(false),
                });
                scan(&path, &id, groups, objects, info_deprecated);
            } else if fname == "__info_obj.json" {
                continue;
            } else if let Some(stem) = fname.strip_suffix(".obj.json") {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    if let Ok(mut o) = serde_json::from_str::<ObjectDef>(&text) {
                        if o.name.trim().is_empty() {
                            o.name = stem.to_string();
                        }
                        if !group_id.is_empty() && o.group.is_empty() {
                            o.group = group_id.to_string();
                        }
                        objects.push(o);
                    }
                }
            }
        }
    }
    scan(&dir, "", &mut groups, &mut objects, &info_deprecated);
    let mut store = ObjectStore { groups, objects };
    migrate_object_kinds(&mut store);
    Ok(store)
}

/// 旧版本对象数据兼容：属性类型归一化（旧小写类型 → 新 PascalCase 类型，与接口文档 tab 一致）。
/// 类型变化导致结构 hash 变化，因此同时重算对象 hash，并把 refHash 引用从旧 hash 迁移到新 hash
/// （与 save_objects 的冲突后缀规则一致，保证同结构对象 hash 唯一）。
fn migrate_object_kinds(store: &mut ObjectStore) {
    for o in &mut store.objects {
        for p in &mut o.properties {
            p.kind = normalize_kind(&p.kind);
            p.item_kind = normalize_kind(&p.item_kind);
        }
    }
    let mut map: HashMap<String, String> = HashMap::new();
    let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();
    for o in &mut store.objects {
        let base = object_hash(&o.properties);
        let mut h = base.clone();
        let mut n = 2;
        while used.contains(&h) {
            h = format!("{base}-{n}");
            n += 1;
        }
        used.insert(h.clone());
        if o.hash != h {
            map.insert(o.hash.clone(), h.clone());
            o.hash = h;
        }
    }
    for o in &mut store.objects {
        for p in &mut o.properties {
            if let Some(h) = map.get(&p.ref_hash) {
                p.ref_hash = h.clone();
            }
        }
    }
}

/// 属性类型归一化：旧小写类型 → 新 PascalCase 类型；新类型或未知值原样返回。
/// 旧 "number" 无法区分整数/浮点，统一归为 Integer（用户可手动改为 Float）。
fn normalize_kind(k: &str) -> String {
    match k {
        "string" => "String".into(),
        "number" => "Integer".into(),
        "boolean" => "Boolean".into(),
        "datetime" => "Datetime".into(),
        "date" => "Date".into(),
        "time" => "Time".into(),
        "list" => "List".into(),
        "object" => "Object".into(),
        "any" => "Any".into(),
        other => other.to_string(),
    }
}

/// 保存对象存储（全量重建 data/.object 目录）。
/// 目录即分组：分组信息写入 __info_obj.json，每个对象一个 <名称>.obj.json 文件。
/// 保存时重新计算每个对象的 hash（属性变化后保持一致），
/// 并修复失效引用（refHash 指向不存在的对象时尝试按名称匹配，否则清空）。
pub fn save_objects_impl(root: &Path, store: &ObjectStore) -> Result<String, String> {
    let dir = root.join(crate::OBJECT_DATA_DIR);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("清理对象目录失败: {e}"))?;
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建对象目录失败: {e}"))?;

    // 分组信息（目录代表分组，空分组也建目录）
    let mut groups: Vec<ObjectGroup> = store.groups.clone();
    groups.sort_by(|a, b| a.id.cmp(&b.id));
    groups.dedup_by(|a, b| a.id == b.id);
    let info_text = serde_json::to_string_pretty(&groups).map_err(|e| format!("序列化分组失败: {e}"))?;
    std::fs::write(dir.join("__info_obj.json"), info_text)
        .map_err(|e| format!("写入分组信息失败: {e}"))?;
    for g in &groups {
        std::fs::create_dir_all(dir.join(&g.id)).map_err(|e| format!("创建分组目录失败: {e}"))?;
    }

    // 重复对象检测：相同结构（所有 key 组成的 hash 相同）且不同 uuid 的对象视为重复，不做保存
    {
        let mut seen: HashMap<String, &str> = HashMap::new();
        for o in &store.objects {
            let h = object_hash(&o.properties);
            if let Some(prev) = seen.get(&h) {
                // 同一 uuid 重复出现视为同一条目，放行
                if *prev == o.uuid && !o.uuid.trim().is_empty() {
                    continue;
                }
                return Err(format!(
                    "存在结构相同的重复对象（hash 相同），已取消保存：请删除或调整其中一个对象"
                ));
            }
            seen.insert(h, &o.uuid);
        }
    }

    // 重算 hash（结构签名）+ 修复失效引用（uuid 为空时生成稳定标识，兼容旧数据）
    // hash 仅保证"相同结构复用"，不保证对象唯一：store 内 hash 冲突时追加 -2/-3 后缀，
    // 避免删除/定位按 hash 误伤其他对象（对象唯一性以 uuid 为准）
    let mut objects: Vec<ObjectDef> = Vec::new();
    let mut used_hashes: std::collections::HashSet<String> = std::collections::HashSet::new();
    for mut o in store.objects.clone() {
        let base = object_hash(&o.properties);
        let mut h = base.clone();
        let mut n = 2;
        while used_hashes.contains(&h) {
            h = format!("{base}-{n}");
            n += 1;
        }
        used_hashes.insert(h.clone());
        o.hash = h;
        if o.uuid.trim().is_empty() {
            o.uuid = uuid::Uuid::new_v4().to_string();
        }
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

    // 对象文件：<分组目录>/<对象名称>.obj.json（同名冲突加 -2/-3 后缀，内容 name 不变）
    let mut written: std::collections::HashSet<String> = std::collections::HashSet::new();
    for o in &objects {
        let safe = o.name.replace(['/', '\\'], "_");
        let mut fname = format!("{safe}.obj.json");
        let mut n = 2;
        while written.contains(&fname) {
            fname = format!("{safe}-{n}.obj.json");
            n += 1;
        }
        written.insert(fname.clone());
        let obj_dir = if o.group.is_empty() { dir.clone() } else { dir.join(&o.group) };
        std::fs::create_dir_all(&obj_dir).map_err(|e| format!("创建分组目录失败: {e}"))?;
        let text = serde_json::to_string_pretty(o).map_err(|e| format!("序列化对象失败: {e}"))?;
        std::fs::write(obj_dir.join(&fname), text).map_err(|e| format!("写入对象文件失败: {e}"))?;
    }
    Ok(dir.to_string_lossy().to_string())
}

/// 从 JSON 文本生成对象定义（嵌套 object 提取为独立对象，hash 相同则复用）。
/// 返回创建的对象列表（顶层对象排最后，嵌套对象在前，便于前端合并）。
pub fn import_json_object_impl(
    root: &Path,
    name: &str,
    group: &str,
    json_text: &str,
) -> Result<ObjectImportResult, String> {
    let mut store = list_objects_impl(root)?;
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
                    ..ObjectProp::default()
                };
                match val {
                    serde_json::Value::String(_) => {
                        p.kind = "String".into();
                    }
                    serde_json::Value::Number(n) => {
                        p.kind = if n.as_i64().is_some() || n.as_u64().is_some() {
                            "Integer".into()
                        } else {
                            "Float".into()
                        };
                    }
                    serde_json::Value::Bool(_) => p.kind = "Boolean".into(),
                    serde_json::Value::Null => p.kind = "Any".into(),
                    serde_json::Value::Array(arr) => {
                        let item_kind = if arr.is_empty() {
                            "Any".to_string()
                        } else {
                            // 取第一个非空元素推断类型
                            let mut it = "Any".to_string();
                            for el in arr {
                                match el {
                                    serde_json::Value::String(_) => it = "String".into(),
                                    serde_json::Value::Number(n) => {
                                        it = if n.as_i64().is_some() || n.as_u64().is_some() {
                                            "Integer".into()
                                        } else {
                                            "Float".into()
                                        };
                                    }
                                    serde_json::Value::Bool(_) => it = "Boolean".into(),
                                    serde_json::Value::Object(_) => {
                                        it = "Object".into();
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                            it
                        };
                        p.kind = "List".into();
                        p.item_kind = item_kind.clone();
                        if item_kind == "Object" {
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
                        p.kind = "Object".into();
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
            uuid: uuid::Uuid::new_v4().to_string(),
            hash: hash.clone(),
            name: suggested_name.to_string(),
            object_name: suggested_name.to_string(),
            package_name: String::new(),
            group: group.to_string(),
            deprecated: false,
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
        save_objects_impl(root, &store)?;
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
pub fn import_ddl_impl(root: &Path, group: &str, ddl: &str) -> Result<ObjectImportResult, String> {
    let mut store = list_objects_impl(root)?;
    let mut created: Vec<String> = Vec::new();
    let mut reused: Vec<String> = Vec::new();
    let mut generated: Vec<ObjectDef> = Vec::new();
    let group = group.trim().to_string();

    let tables = parse_create_tables(ddl);
    if tables.is_empty() {
        return Err("未识别到 CREATE TABLE 语句".into());
    }
    let mut top_hash = String::new();
    for (table_name, table_comment, body) in tables {
        // 对象文件名优先使用表 COMMENT；为空才用表名
        let file_name = if table_comment.trim().is_empty() {
            table_name.clone()
        } else {
            table_comment.trim().to_string()
        };
        let mut props: Vec<ObjectProp> = Vec::new();
        for col in split_columns(&body) {
            if let Some((key, kind, desc)) = parse_column(&col) {
                props.push(ObjectProp {
                    key,
                    kind,
                    description: desc,
                    mock: String::new(),
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
            uuid: uuid::Uuid::new_v4().to_string(),
            hash: hash.clone(),
            name: file_name.clone(),
            object_name: table_name.clone(),
            package_name: String::new(),
            group: group.clone(),
            deprecated: false,
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
        save_objects_impl(root, &store)?;
    }
    Ok(ObjectImportResult {
        objects: generated,
        created,
        reused,
        top_hash,
    })
}

/// 提取 DDL 中所有 CREATE TABLE 语句 → (表名, 表体内容)
fn parse_create_tables(ddl: &str) -> Vec<(String, String, String)> {
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
        // 表级 COMMENT：右括号后到分号前的表选项文本中提取（如 ENGINE=... COMMENT='用户表'）
        let mut tail = String::new();
        let tail_start = body_start + end_rel;
        for c in ddl[tail_start..].chars() {
            if c == ';' || c == '(' {
                break;
            }
            tail.push(c);
        }
        let table_comment = extract_comment(&tail);
        out.push((clean_table_name(raw_name), table_comment, body));
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
fn parse_column(col: &str) -> Option<(String, String, String)> {
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
    let desc = extract_comment(&attrs);
    Some((name, map_sql_type(&type_tok).to_string(), desc))
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
        let mut rest_trim = rest.trim_start();
        // 兼容表选项写法 COMMENT='xxx'（无空格）与列级写法 COMMENT 'xxx'
        if let Some(after_eq) = rest_trim.strip_prefix('=') {
            rest_trim = after_eq.trim_start();
        }
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

/// SQL 类型 → 对象属性类型（PascalCase，与接口文档 tab 风格一致）
fn map_sql_type(t: &str) -> &'static str {
    let t = t.to_ascii_uppercase();
    if t.starts_with("INT")
        || t.starts_with("SERIAL")
        || t.starts_with("BIGINT")
        || t.starts_with("SMALLINT")
        || t.starts_with("TINYINT")
        || t.starts_with("MEDIUMINT")
        || t == "YEAR"
    {
        "Integer"
    } else if t.starts_with("FLOAT")
        || t.starts_with("DOUBLE")
        || t.starts_with("DEC")
        || t.starts_with("NUM")
        || t.starts_with("REAL")
        || t.starts_with("MONEY")
    {
        "Float"
    } else if t.starts_with("BOOL") || t == "BIT" {
        "Boolean"
    } else if t.starts_with("DATETIME") || t.starts_with("TIMESTAMP") {
        "Datetime"
    } else if t.starts_with("DATE") {
        "Date"
    } else if t.starts_with("TIME") {
        "Time"
    } else if t.starts_with("JSON") {
        "Any"
    } else {
        // VARCHAR / CHAR / TEXT / BLOB / CLOB / BINARY / ENUM / UUID 等
        "String"
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
/// 遍历工作区接口 json（排除 .history/.examples/.object/.versions/__info.json 等隐藏目录），
/// 通过 docParams 的 objectName（Object 类型）匹配对象名。
pub fn object_usage_impl(root: &Path, store: &ObjectStore) -> Result<Vec<ObjectUsageItem>, String> {
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
                    continue; // 跳过 .history/.examples/.object/.versions 等隐藏目录
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

// ==================== 对象版本管理（.object_version/<uuid>/<版本号>.json） ====================

/// 对象版本信息（.object_version/<uuid>/<n>.json）
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectVersionInfo {
    pub version: u32,
    /// 保存时间（Unix 秒）
    pub saved_at: u64,
    pub name: String,
    pub description: String,
    pub prop_count: usize,
    pub hash: String,
}

/// 保存对象版本快照：写入 <工作区>/.object_version/<uuid>/<版本号>.json（版本号递增）
pub fn save_object_version(root: &Path, uuid: &str, snapshot: &ObjectDef) -> Result<String, String> {
    let uuid = uuid.trim().to_string();
    if !crate::valid_uuid(&uuid) {
        return Err("无效的对象 uuid".into());
    }
    let dir = root.join(crate::OBJECT_VERSION_DATA_DIR).join(&uuid);
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建对象版本目录失败: {e}"))?;
    let mut max: u32 = 0;
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for entry in rd.flatten() {
            let fname = entry.file_name().to_string_lossy().to_string();
            if let Some(n) = fname.strip_suffix(".json") {
                if let Ok(n) = n.parse::<u32>() {
                    max = max.max(n);
                }
            }
        }
    }
    let version = max + 1;
    let target = dir.join(format!("{version}.json"));
    let text = serde_json::to_string_pretty(snapshot).map_err(|e| format!("序列化对象版本失败: {e}"))?;
    std::fs::write(&target, text).map_err(|e| format!("写入对象版本失败: {e}"))?;
    Ok(format!(".api-manager/object_version/{uuid}/{version}.json"))
}

/// 对象版本列表（按版本号升序）
pub fn list_object_versions(root: &Path, uuid: &str) -> Result<Vec<ObjectVersionInfo>, String> {
    let uuid = uuid.trim().to_string();
    if !crate::valid_uuid(&uuid) {
        return Ok(vec![]);
    }
    let dir = root.join(crate::OBJECT_VERSION_DATA_DIR).join(&uuid);
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut list: Vec<ObjectVersionInfo> = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("读取对象版本目录失败: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let fname = entry.file_name().to_string_lossy().to_string();
        let Some(n) = fname.strip_suffix(".json") else { continue };
        let Ok(version) = n.parse::<u32>() else { continue };
        let saved_at = entry
            .metadata()
            .and_then(|m| m.modified())
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0))
            .unwrap_or(0);
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(o) = serde_json::from_str::<ObjectDef>(&text) {
                list.push(ObjectVersionInfo {
                    version,
                    saved_at,
                    name: o.name,
                    description: o.description,
                    prop_count: o.properties.len(),
                    hash: o.hash,
                });
            }
        }
    }
    list.sort_by_key(|v| v.version);
    Ok(list)
}

/// 读取指定版本的对象快照
pub fn read_object_version(root: &Path, uuid: &str, version: u32) -> Result<ObjectDef, String> {
    let uuid = uuid.trim().to_string();
    if !crate::valid_uuid(&uuid) {
        return Err("无效的对象 uuid".into());
    }
    let path = root.join(crate::OBJECT_VERSION_DATA_DIR).join(&uuid).join(format!("{version}.json"));
    let text = std::fs::read_to_string(&path).map_err(|e| format!("读取对象版本失败: {e}"))?;
    let mut o: ObjectDef =
        serde_json::from_str(&text).map_err(|e| format!("解析对象版本失败: {e}"))?;
    // 旧版本快照中的小写类型归一化为新 PascalCase 类型（仅影响展示，不改磁盘）
    for p in &mut o.properties {
        p.kind = normalize_kind(&p.kind);
        p.item_kind = normalize_kind(&p.item_kind);
    }
    Ok(o)
}


use crate::{workspace_root, WorkspaceState};
use tauri::State;


/// 列出对象存储（分组 + 对象定义）
#[tauri::command]
pub(crate) fn list_objects(
    state: State<'_, WorkspaceState>,
) -> Result<ObjectStore, String> {
    let root = workspace_root(&state)?;
    list_objects_impl(&root)
}

/// 保存对象存储（整体覆盖写）
#[tauri::command]
pub(crate) fn save_objects(
    state: State<'_, WorkspaceState>,
    store: ObjectStore,
) -> Result<String, String> {
    let root = workspace_root(&state)?;
    save_objects_impl(&root, &store)
}

/// 数据生成结果
#[derive(serde::Serialize)]
pub(crate) struct GenDataResult {
    file: String,
    dir: String,
    count: usize,
    elapsed_ms: u64,
}

/// 数据生成提交的属性配置（写入日志）
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct GenPropItem {
    key: String,
    kind: String,
    mock: String,
    enabled: bool,
    #[serde(default)]
    desc: Option<String>,
}

/// 单条生成记录（.gen_log/<时间戳>_<object-uuid>.json）
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct GenLogItem {
    file: String,
    time: i64,
    time_str: String,
    object_uuid: String,
    object_name: String,
    dir: String,
    format: String,
    table: String,
    count: usize,
    elapsed_ms: u64,
    props: Vec<GenPropItem>,
}

/// 写入生成的数据文件，并在工作区 .gen_log/<时间戳>_<object-uuid>.json 保存一条生成记录
/// （含提交的数据与耗时）。
#[tauri::command]
pub(crate) fn gen_data(
    state: State<'_, WorkspaceState>,
    dir: String,
    file_name: String,
    content: String,
    format: String,
    table: String,
    count: usize,
    elapsed_ms: u64,
    object_uuid: String,
    object_name: String,
    props: Vec<GenPropItem>,
) -> Result<GenDataResult, String> {
    let dir_path = std::path::Path::new(&dir);
    if !dir_path.is_dir() {
        return Err(format!("导出目录不存在: {dir}"));
    }
    let path = dir_path.join(&file_name);
    std::fs::write(&path, content).map_err(|e| format!("写入文件失败: {e}"))?;

    // 生成记录：工作区根 .gen_log/<时间戳>_<object-uuid>.json（每条记录一个文件）
    let root = workspace_root(&state)?;
    let log_dir = root.join(crate::GEN_LOG_DATA_DIR);
    std::fs::create_dir_all(&log_dir).map_err(|e| format!("创建 .gen_log 失败: {e}"))?;
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let log_path = log_dir.join(format!("{ts}_{object_uuid}.json"));
    let record = GenLogItem {
        file: file_name.clone(),
        time: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0),
        time_str: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        object_uuid,
        object_name,
        dir: dir.clone(),
        format,
        table,
        count,
        elapsed_ms,
        props,
    };
    let text = serde_json::to_string_pretty(&record).map_err(|e| format!("序列化生成记录失败: {e}"))?;
    std::fs::write(&log_path, text).map_err(|e| format!("写入生成记录失败: {e}"))?;

    Ok(GenDataResult { file: file_name, dir, count, elapsed_ms })
}

/// 读取 .gen_log 下全部生成记录（按时间倒序）。
#[tauri::command]
pub(crate) fn list_gen_logs(
    state: State<'_, WorkspaceState>,
) -> Result<Vec<GenLogItem>, String> {
    let root = workspace_root(&state)?;
    let log_dir = root.join(crate::GEN_LOG_DATA_DIR);
    if !log_dir.is_dir() {
        return Ok(vec![]);
    }
    let mut items: Vec<GenLogItem> = vec![];
    let read = std::fs::read_dir(&log_dir).map_err(|e| format!("读取 .gen_log 失败: {e}"))?;
    for entry in read.flatten() {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Ok(t) = std::fs::read_to_string(&p) {
                if let Ok(v) = serde_json::from_str::<GenLogItem>(&t) {
                    items.push(v);
                }
            }
        }
    }
    items.sort_by(|a, b| b.time.cmp(&a.time));
    Ok(items)
}

/// 从 JSON 文本生成对象（嵌套 object 提取为独立对象，hash 相同则复用已有对象）
#[tauri::command]
pub(crate) fn import_json_object(
    state: State<'_, WorkspaceState>,
    name: String,
    group: String,
    json: String,
) -> Result<ObjectImportResult, String> {
    let root = workspace_root(&state)?;
    import_json_object_impl(&root, &name, &group, &json)
}

/// 从 SQL CREATE TABLE 建表语句生成对象（每个表一个对象）
#[tauri::command]
pub(crate) fn import_ddl(
    state: State<'_, WorkspaceState>,
    group: String,
    ddl: String,
) -> Result<ObjectImportResult, String> {
    let root = workspace_root(&state)?;
    import_ddl_impl(&root, &group, &ddl)
}

/// 对象被接口文档引用的统计（接口数量 + 引用接口列表）
#[tauri::command]
pub(crate) fn object_usage(
    state: State<'_, WorkspaceState>,
    store: ObjectStore,
) -> Result<Vec<ObjectUsageItem>, String> {
    let root = workspace_root(&state)?;
    object_usage_impl(&root, &store)
}

#[cfg(test)]
#[path = "objects_test.rs"]
mod tests;
