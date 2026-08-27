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
            prescript: String::new(),
            examples: vec![],
            responses: vec![],
            doc_params: vec![],
            deprecated: false,
            protocol: "http".into(),
        order: None,
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

    /// 勾选分组时导出弹窗会同时提交外层分组与嵌套分组路径，嵌套分组不应被重复收集
    #[test]
    fn collect_apis_dedupes_nested_dirs() {
        let base = std::env::temp_dir().join(format!("apim-dedupe2-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let g = base.join("用户管理");
        let sub = g.join("子分组");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join(INFO_FILE), r#"{"name":"子分组"}"#).unwrap();
        let mut a = sample();
        a.name = "接口A".into();
        fs::write(sub.join("接口A.json"), serde_json::to_string(&a).unwrap()).unwrap();
        // 外层分组 + 嵌套分组同时被选中（导出弹窗的实际行为）
        let paths = vec![
            g.to_string_lossy().to_string(),
            sub.to_string_lossy().to_string(),
        ];
        let apis = collect_apis(&base, &paths).expect("collect");
        // 嵌套分组已随外层收集 → 恰好 1 个，不重复
        assert_eq!(apis.len(), 1);
        assert_eq!(
            apis[0].0,
            vec![
                ("用户管理".to_string(), false),
                ("子分组".to_string(), false)
            ]
        );
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
