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
