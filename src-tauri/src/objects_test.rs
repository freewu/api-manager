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
            ObjectProp { key: "b".into(), kind: "String".into(), ..Default::default() },
            ObjectProp { key: "a".into(), kind: "Integer".into(), ..Default::default() },
        ];
        let h1 = object_hash(&p1);
        assert_eq!(h1.len(), 12);
        // 顺序无关
        p1.reverse();
        assert_eq!(object_hash(&p1), h1);
        // 不同结构不同 hash
        let p2 = vec![
            ObjectProp { key: "a".into(), kind: "Integer".into(), ..Default::default() },
            ObjectProp { key: "b".into(), kind: "Boolean".into(), ..Default::default() },
        ];
        assert_ne!(object_hash(&p2), h1);
        // list + 引用参与 hash
        let p3 = vec![
            ObjectProp { key: "a".into(), kind: "List".into(), item_kind: "Object".into(), ref_hash: "x".into(), ..Default::default() },
        ];
        let p4 = vec![
            ObjectProp { key: "a".into(), kind: "List".into(), item_kind: "Object".into(), ref_hash: "y".into(), ..Default::default() },
        ];
        assert_ne!(object_hash(&p3), object_hash(&p4));
    }

    #[test]
    fn test_import_json_nested_and_reuse() {
        let root = tmpdir("import");
        let json = r#"{"name":"alice","age":18,"addr":{"city":"bj","zip":"100000"},"tags":["a","b"],"orders":[{"id":1}]}"#;
        let res = import_json_object_impl(&root, "User", "g1", json).unwrap();
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
        assert_eq!(addr_prop.kind, "Object");
        assert!(!addr_prop.ref_hash.is_empty());
        let orders_prop = user.properties.iter().find(|p| p.key == "orders").unwrap();
        assert_eq!(orders_prop.kind, "List");
        assert_eq!(orders_prop.item_kind, "Object");

        // 第二次导入相同结构：全部复用
        let res2 = import_json_object_impl(&root, "User2", "g2", json).unwrap();
        assert_eq!(res2.created.len(), 0, "相同结构应复用");
        assert_eq!(res2.reused.len(), 3);
        assert_eq!(res2.objects.len(), 0, "复用时不重建对象");
        // top_hash 指向已存在的顶层对象（User 或 User2，结构相同 hash 相同）
        let store = list_objects_impl(&root).unwrap();
        assert!(store.objects.iter().any(|o| o.hash == res2.top_hash), "top_hash 应在 store 中");
    }

    #[test]
    fn test_import_json_invalid() {
        let root = tmpdir("invalid");
        let r = import_json_object_impl(&root, "X", "", "{bad");
        assert!(r.is_err());
        let r2 = import_json_object_impl(&root, "X", "", "[1,2,3]");
        assert!(r2.is_err(), "顶层数组应报错");
    }

    #[test]
    fn test_map_sql_type_date_time() {
        assert_eq!(map_sql_type("DATETIME"), "Datetime");
        assert_eq!(map_sql_type("TIMESTAMP"), "Datetime");
        assert_eq!(map_sql_type("TIMESTAMP(6)"), "Datetime");
        assert_eq!(map_sql_type("DATE"), "Date");
        assert_eq!(map_sql_type("TIME"), "Time");
        assert_eq!(map_sql_type("VARCHAR(50)"), "String");
        assert_eq!(map_sql_type("INT"), "Integer");
        assert_eq!(map_sql_type("FLOAT"), "Float");
        assert_eq!(map_sql_type("DOUBLE"), "Float");
        assert_eq!(map_sql_type("DECIMAL(10,2)"), "Float");
    }

    #[test]
    fn test_parse_create_tables_basic() {
        let ddl = r#"
CREATE TABLE users (
  id BIGINT PRIMARY KEY,
  name VARCHAR(50) NOT NULL COMMENT '用户名称',
  age INT,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
) COMMENT='用户信息表';
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
        assert_eq!(tables[0].1, "用户信息表", "表级 COMMENT 应提取");
        assert_eq!(tables[1].0, "orders", "IF NOT EXISTS 与 schema 前缀应处理");
        assert_eq!(tables[1].1, "", "无表级 COMMENT 时为空");
        let cols = split_columns(&tables[0].2);
        assert_eq!(cols.len(), 4);
        let (name, kind, _desc) = parse_column(&cols[0]).unwrap();
        assert_eq!(name, "id");
        assert_eq!(kind, "Integer");
        let (name, kind, desc) = parse_column(&cols[1]).unwrap();
        assert_eq!(name, "name");
        assert_eq!(kind, "String");
        assert_eq!(desc, "用户名称", "COMMENT 应提取为描述");
        let (_, kind, _) = parse_column(&cols[2]).unwrap();
        assert_eq!(kind, "Integer");
        // 表级约束应被忽略
        let constraint_cols: Vec<_> = split_columns(&tables[1].2);
        let parsed: Vec<_> = constraint_cols.iter().filter_map(|c| parse_column(c)).collect();
        assert_eq!(parsed.len(), 3, "PRIMARY KEY(...) 约束行应被跳过");
    }

    #[test]
    fn test_import_ddl_creates_and_reuses() {
        let root = tmpdir("ddl");
        let ddl = "CREATE TABLE t_user (id INT NOT NULL, name VARCHAR(50));";
        let res = import_ddl_impl(&root, "db", ddl).unwrap();
        assert_eq!(res.created.len(), 1);
        assert_eq!(res.objects[0].name, "t_user");
        assert_eq!(res.objects[0].group, "db");
        assert_eq!(res.objects[0].properties.len(), 2);
        // 相同结构再次导入 → 复用
        let res2 = import_ddl_impl(&root, "db", ddl).unwrap();
        assert_eq!(res2.created.len(), 0);
        assert_eq!(res2.reused.len(), 1);
        // 多表
        let ddl2 = "CREATE TABLE a (x INT);\nCREATE TABLE b (y VARCHAR(10) NOT NULL);";
        let res3 = import_ddl_impl(&root, "", ddl2).unwrap();
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
        let res = import_ddl_impl(&root, "", ddl).unwrap();
        assert_eq!(res.created.len(), 1);
        let o = &res.objects[0];
        assert_eq!(o.name, "my_users", "反引号表名应清洗");
        assert_eq!(o.properties.len(), 2, "CONSTRAINT 行与 -- 注释应忽略");
        let first = o.properties.iter().find(|p| p.key == "first name").unwrap();
        assert_eq!(first.description, "名字");
        let bio = o.properties.iter().find(|p| p.key == "bio").unwrap();
        assert_eq!(bio.kind, "String");
    }

    #[test]
    fn test_save_recomputes_hash() {
        let root = tmpdir("save");
        let mut store = ObjectStore::default();
        let o = ObjectDef {
            hash: "stale".into(),
            name: "A".into(),
            properties: vec![ObjectProp { key: "x".into(), kind: "String".into(), ..Default::default() }],
            ..Default::default()
        };
        store.objects.push(o.clone());
        save_objects_impl(&root, &store).unwrap();
        let loaded = list_objects_impl(&root).unwrap();
        assert_eq!(loaded.objects[0].hash, object_hash(&o.properties));
        assert_ne!(loaded.objects[0].hash, "stale");
    }

    #[test]
    fn test_group_deprecated_persist() {
        let root = tmpdir("group-dep");
        let mut store = ObjectStore::default();
        store.groups.push(ObjectGroup { id: "用户管理".into(), name: "用户管理".into(), deprecated: true });
        store.groups.push(ObjectGroup { id: "订单/明细".into(), name: "明细".into(), deprecated: false });
        save_objects_impl(&root, &store).unwrap();
        let loaded = list_objects_impl(&root).unwrap();
        let user = loaded.groups.iter().find(|g| g.id == "用户管理").unwrap();
        assert!(user.deprecated, "已废弃标记应持久化到 __info_obj.json 并回读");
        let detail = loaded.groups.iter().find(|g| g.id == "订单/明细").unwrap();
        assert!(!detail.deprecated);
    }

    #[test]
    fn test_dir_storage_layout() {
        let root = tmpdir("layout");
        let mut store = ObjectStore::default();
        store.groups.push(ObjectGroup { id: "用户管理".into(), name: "用户管理".into(), deprecated: false });
        store.groups.push(ObjectGroup { id: "订单/明细".into(), name: "明细".into(), deprecated: false });
        store.objects.push(ObjectDef {
            uuid: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            hash: String::new(),
            name: "User".into(),
            group: "用户管理".into(),
            deprecated: false,
            object_name: String::new(),
            package_name: String::new(),
            description: "用户".into(),
            properties: vec![ObjectProp { key: "id".into(), kind: "Integer".into(), ..Default::default() }],
            created_at: 1,
            updated_at: 2,
        });
        store.objects.push(ObjectDef {
            uuid: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            hash: String::new(),
            name: "OrderItem".into(),
            group: "订单/明细".into(),
            deprecated: false,
            object_name: String::new(),
            package_name: String::new(),
            description: String::new(),
            properties: vec![],
            created_at: 1,
            updated_at: 2,
        });
        store.objects.push(ObjectDef {
            uuid: "cccccccccccccccccccccccccccccccc".into(),
            hash: String::new(),
            name: "Plain".into(),
            group: String::new(),
            deprecated: false,
            object_name: String::new(),
            package_name: String::new(),
            description: String::new(),
            properties: vec![ObjectProp { key: "name".into(), kind: "String".into(), ..Default::default() }],
            created_at: 1,
            updated_at: 2,
        });
        save_objects_impl(&root, &store).unwrap();

        let base = root.join(".object");
        assert!(base.join("__info_obj.json").exists(), "应有分组信息文件");
        assert!(base.join("用户管理").join("User.obj.json").exists(), "分组目录下的对象文件");
        assert!(base.join("订单").join("明细").join("OrderItem.obj.json").exists(), "多级分组 = 嵌套目录");
        assert!(base.join("Plain.obj.json").exists(), "未分组对象在根目录");

        let loaded = list_objects_impl(&root).unwrap();
        // 目录即分组：用户管理 + 订单（父级）+ 订单/明细（子级）
        assert_eq!(loaded.groups.len(), 3);
        assert_eq!(loaded.objects.len(), 3);
        let user = loaded.objects.iter().find(|o| o.name == "User").unwrap();
        assert_eq!(user.group, "用户管理");
        assert_eq!(user.hash.len(), 12, "保存时应重算 hash");
        assert_eq!(user.description, "用户");
        let item = loaded.objects.iter().find(|o| o.name == "OrderItem").unwrap();
        assert_eq!(item.group, "订单/明细");

        // 全量重建：删除一个对象后再保存，旧文件不残留
        store.objects.retain(|o| o.name != "User");
        save_objects_impl(&root, &store).unwrap();
        assert!(!base.join("用户管理").join("User.obj.json").exists(), "删除对象后旧文件应清除");
    }

    #[test]
    fn test_object_versions() {
        let root = tmpdir("versions");
        let uuid = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let mut obj = ObjectDef {
            uuid: uuid.into(),
            hash: "h1".into(),
            name: "User".into(),
            group: "用户管理".into(),
            deprecated: false,
            object_name: String::new(),
            package_name: String::new(),
            description: "v1".into(),
            properties: vec![ObjectProp { key: "id".into(), kind: "number".into(), ..Default::default() }],
            created_at: 1,
            updated_at: 2,
        };
        save_object_version(&root, uuid, &obj).unwrap();
        obj.description = "v2".into();
        obj.hash = "h2".into();
        save_object_version(&root, uuid, &obj).unwrap();

        let list = list_object_versions(&root, uuid).unwrap();
        assert_eq!(list.len(), 2, "应有 2 个版本");
        assert_eq!(list[0].version, 1);
        assert_eq!(list[1].version, 2, "版本号应递增");
        assert_eq!(list[0].description, "v1");
        assert_eq!(list[1].prop_count, 1);
        assert!(list[1].saved_at > 0);

        let v1 = read_object_version(&root, uuid, 1).unwrap();
        assert_eq!(v1.description, "v1");
        assert_eq!(v1.hash, "h1", "历史版本内容应保持原样");
        let v2 = read_object_version(&root, uuid, 2).unwrap();
        assert_eq!(v2.hash, "h2");

        // 非法 uuid 返回空列表
        assert!(list_object_versions(&root, "../").unwrap().is_empty());
        // 无效 uuid 保存报错
        assert!(save_object_version(&root, "bad/../uuid", &obj).is_err());
    }

    #[test]
    fn test_migrate_old_lowercase_kinds() {
        let root = tmpdir("migrate");
        // 模拟旧版本数据：小写类型 + 引用关系（refHash 基于旧类型 hash）
        let dir = root.join(".object");
        std::fs::create_dir_all(&dir).unwrap();
        let old_child = serde_json::json!({
            "uuid": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "hash": "child-old-hash",
            "name": "Child",
            "group": "",
            "deprecated": false,
            "objectName": "Child",
            "packageName": "",
            "description": "",
            "properties": [{"key": "id", "kind": "number", "itemKind": "number", "refHash": "", "description": "", "mock": ""}],
            "createdAt": 1,
            "updatedAt": 2
        });
        std::fs::write(dir.join("Child.obj.json"), serde_json::to_string(&old_child).unwrap()).unwrap();
        let old_parent = serde_json::json!({
            "uuid": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "hash": "parent-old-hash",
            "name": "Parent",
            "group": "",
            "deprecated": false,
            "objectName": "Parent",
            "packageName": "",
            "description": "",
            "properties": [
                {"key": "createdAt", "kind": "datetime", "itemKind": "string", "refHash": "", "description": "", "mock": ""},
                {"key": "child", "kind": "object", "itemKind": "string", "refHash": "child-old-hash", "description": "", "mock": ""}
            ],
            "createdAt": 1,
            "updatedAt": 2
        });
        std::fs::write(dir.join("Parent.obj.json"), serde_json::to_string(&old_parent).unwrap()).unwrap();

        let store = list_objects_impl(&root).unwrap();
        assert_eq!(store.objects.len(), 2);
        let parent = store.objects.iter().find(|o| o.name == "Parent").unwrap();
        let child = store.objects.iter().find(|o| o.name == "Child").unwrap();
        // 类型归一化为 PascalCase
        assert_eq!(parent.properties[0].kind, "Datetime");
        assert_eq!(parent.properties[1].kind, "Object");
        assert_eq!(child.properties[0].kind, "Integer");
        // hash 重算，且引用迁移到新 hash
        assert_ne!(parent.hash, "parent-old-hash");
        assert_eq!(parent.properties[1].ref_hash, child.hash, "refHash 应从旧 hash 迁移到新 hash");
        assert_eq!(child.hash.len(), 12);
    }

    #[test]
    fn test_duplicate_structure_objects_rejected() {
        let root = tmpdir("dup-hash");
        let mut store = ObjectStore::default();
        // 两个空属性对象（结构 hash 相同）→ 视为重复对象，拒绝保存
        for u in ["dddddddddddddddddddddddddddddddd", "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"] {
            store.objects.push(ObjectDef {
                uuid: u.into(),
                hash: "tmp".into(),
                name: format!("对象{u}").into(),
                group: String::new(),
                deprecated: false,
                object_name: String::new(),
                package_name: String::new(),
                description: String::new(),
                properties: vec![],
                created_at: 1,
                updated_at: 2,
            });
        }
        let err = save_objects_impl(&root, &store).unwrap_err();
        assert!(err.contains("重复对象"), "应提示重复对象：{err}");

        // 调整其中一个的结构后可以保存
        store.objects[1].properties = vec![ObjectProp {
            key: "name".into(),
            kind: "String".into(),
            ..Default::default()
        }];
        save_objects_impl(&root, &store).unwrap();
        let loaded = list_objects_impl(&root).unwrap();
        assert_eq!(loaded.objects.len(), 2);
        assert_ne!(loaded.objects[0].hash, loaded.objects[1].hash);

        // 按 uuid 删除一个，另一个保留
        store.objects.retain(|o| o.uuid != "dddddddddddddddddddddddddddddddd");
        save_objects_impl(&root, &store).unwrap();
        let loaded = list_objects_impl(&root).unwrap();
        assert_eq!(loaded.objects.len(), 1);
        assert_eq!(loaded.objects[0].name, "对象eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
    }
