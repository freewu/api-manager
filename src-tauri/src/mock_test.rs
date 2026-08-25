    use super::*;

    #[test]
    fn test_parse_route_segments() {
        let mut api = ApiFile {
            uuid: "test-uuid".into(),
            name: "t".into(),
            method: "GET".into(),
            path: "/api/users/{id}".into(),
            url: String::new(),
            description: String::new(),
            headers: vec![],
            query: vec![],
            params: vec![],
            body: crate::BodyData::default(),
            mock: crate::MockConfig {
                enabled: true,
                status: 200,
                headers: vec![],
                delay: 5,
                body: "{\"id\": \"{{path.id}}\"}".into(),
            },
            examples: vec![],
            responses: vec![],
            doc_params: vec![],
            deprecated: false,
            protocol: "http".into(),
        };
        let route = parse_route(&api).unwrap();
        assert_eq!(route.segments.len(), 3);
        match &route.segments[0] {
            Segment::Literal(l) => assert_eq!(l, "api"),
            _ => panic!(),
        }
        match &route.segments[2] {
            Segment::Param(p) => assert_eq!(p, "id"),
            _ => panic!(),
        }
        assert_eq!(route.method, "GET");
        assert_eq!(route.delay_ms, 5);
        assert!(route.body.contains("{{path.id}}"));

        // 未启用 mock 则返回 None
        api.mock.enabled = false;
        assert!(parse_route(&api).is_none());

        // 空方法 → ANY
        api.mock.enabled = true;
        api.method = "".into();
        assert_eq!(parse_route(&api).unwrap().method, "ANY");

        // :id 语法
        api.method = "POST".into();
        api.path = "/v1/orders/:orderId".into();
        let r2 = parse_route(&api).unwrap();
        match &r2.segments[2] {
            Segment::Param(p) => assert_eq!(p, "orderId"),
            _ => panic!(),
        }
    }

    #[test]
    fn test_scan_workspace() {
        // 扫描示例工作区，应能找到 4 条启用了 mock 的路由
        let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../examples/demo-workspace"));
        let routes = scan_workspace(root);
        assert_eq!(routes.len(), 4, "期望 4 条 mock 路由");
        let methods: Vec<&str> = routes.iter().map(|r| r.method.as_str()).collect();
        assert!(methods.contains(&"GET"));
        assert!(methods.contains(&"POST"));
        assert!(methods.contains(&"DELETE"));
    }

    #[test]
    fn test_custom_mock_crud() {
        let d = std::env::temp_dir().join(format!("apim-custom-mock-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();

        // 空目录 → 空列表
        assert!(list_custom_mocks_impl(&d).is_empty());

        // 保存两个占位符
        save_custom_mock_impl(
            &d,
            &CustomMock { name: "cusId".into(), enabled: true, desc: "自定义ID".into(), code: "(ctx) => 'CUS-' + ctx.randInt(1, 9)".into() },
            None,
        )
        .unwrap();
        save_custom_mock_impl(
            &d,
            &CustomMock { name: "cusTime".into(), enabled: false, desc: "自定义时间".into(), code: "(ctx) => 'T'".into() },
            None,
        )
        .unwrap();
        let list = list_custom_mocks_impl(&d);
        assert_eq!(list.len(), 2);
        let cus_id = list.iter().find(|m| m.name == "cusId").unwrap();
        assert!(cus_id.enabled);
        assert_eq!(cus_id.desc, "自定义ID");
        assert!(cus_id.code.contains("randInt"));
        let cus_time = list.iter().find(|m| m.name == "cusTime").unwrap();
        assert!(!cus_time.enabled);

        // 重命名：old_name 指向旧名，旧文件被删除
        save_custom_mock_impl(
            &d,
            &CustomMock { name: "cusUid".into(), enabled: true, desc: "改名".into(), code: "(ctx) => 'U'".into() },
            Some("cusId"),
        )
        .unwrap();
        let list = list_custom_mocks_impl(&d);
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|m| m.name == "cusUid"));
        assert!(!list.iter().any(|m| m.name == "cusId"));

        // 与内置 mock.js 冲突
        let r = save_custom_mock_impl(&d, &CustomMock { name: "cname".into(), enabled: true, desc: "".into(), code: "x".into() }, None);
        assert!(r.is_err());
        // 重复名称
        let r2 = save_custom_mock_impl(&d, &CustomMock { name: "cusUid".into(), enabled: true, desc: "".into(), code: "y".into() }, None);
        assert!(r2.is_err());
        // 非法名称
        let r3 = save_custom_mock_impl(&d, &CustomMock { name: "1bad".into(), enabled: true, desc: "".into(), code: "y".into() }, None);
        assert!(r3.is_err());
        // 空代码
        let r4 = save_custom_mock_impl(&d, &CustomMock { name: "ok".into(), enabled: true, desc: "".into(), code: " ".into() }, None);
        assert!(r4.is_err());

        // 删除
        delete_custom_mock_impl(&d, "cusTime").unwrap();
        let list = list_custom_mocks_impl(&d);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "cusUid");

        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn test_apply_env_vars() {
        let mut envs = HashMap::new();
        envs.insert("token".to_string(), "T-123".to_string());
        envs.insert("baseUrl".to_string(), "http://x".to_string());
        envs.insert("path".to_string(), "应被保留".to_string()); // 系统变量不替换
        let out = apply_env_vars(
            "{\"token\": \"{{token}}\", \"base\": \"{{baseUrl}}\", \"p\": \"{{path}}\", \"pd\": \"{{path.id}}\"}",
            &envs,
        );
        assert!(out.contains("\"T-123\""));
        assert!(out.contains("\"http://x\""));
        assert!(out.contains("{{path}}")); // 保留
        assert!(out.contains("{{path.id}}")); // 保留
    }
