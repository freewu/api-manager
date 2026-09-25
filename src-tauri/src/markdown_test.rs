    use super::*;

    fn sample_api() -> ApiFile {
        ApiFile {
        pack: vec![],
        unpack: vec![],
        net: None,
        mq: None,
            uuid: "u1".into(),
            name: "创建用户".into(),
            method: "POST".into(),
            path: "/api/users".into(),
            url: "http://example.com/api/users".into(),
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
                binary_path: String::new(),
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
            prescript: String::new(),
            examples: vec![],
            responses: vec![],
            doc_params: vec![DocParam {
                source: "resp_fail".into(),
                key: "code".into(),
                r#type: "Integer".into(),
                description: "错误码".into(),
                item_type: String::new(),
                object_name: String::new(),
                children: vec![],
            }],
            deprecated: false,
            protocol: "http".into(),
        }
    }

    /// 响应页签条目 → 文档：每个条目一节（名称 + HTTP 状态码），字段由示例体推导
    #[test]
    fn render_uses_response_tab_entries() {
        let mut api = sample_api();
        api.responses = vec![
            crate::ResponseItem {
                id: "r1".into(),
                name: "返回成功".into(),
                status: 200,
                content_type: "application/json".into(),
                body: r#"{"code":0,"data":{"name":"张三"}}"#.into(),
            },
            crate::ResponseItem {
                id: "r2".into(),
                name: "参数校验失败".into(),
                status: 422,
                content_type: "application/json".into(),
                body: r#"{"code":422,"message":"name 不能为空"}"#.into(),
            },
        ];
        let md = render(&api, "", false);
        assert!(md.contains("### 返回成功（HTTP 200）\n"), "md: {md}");
        assert!(md.contains("### 参数校验失败（HTTP 422）\n"), "md: {md}");
        assert!(md.contains("| data.name | String |"), "md: {md}");
        // 自定义错误返回名导入回读
        let parsed = parse(&md).expect("parse ok");
        let a = &parsed.apis[0];
        assert_eq!(a.responses.len(), 2);
        assert_eq!(a.responses[1].name, "参数校验失败");
        assert_eq!(a.responses[1].status, 422);
        assert!(a.responses[1].body.contains("name 不能为空"));
    }

    /// HTML 导出：按 nav 参数生成悬浮导航栏（off / left / right），导航脚本引用分组与接口标题
    #[test]
    fn wrap_html_nav() {
        let md = "# 用户管理\n\n## 创建用户\n\n> GET /api/users\n\n## 请求参数\n\n## 响应参数\n\n# 订单管理\n\n## 查询订单\n\n> GET /api/orders\n";
        let right = wrap_html("接口文档", md, "right");
        assert!(right.contains("<body class=\"nav-right\">"), "right 位置");
        assert!(right.contains("id=\"doc-nav\""), "包含导航容器");
        assert!(right.contains("id=\"doc-nav-toggle\""), "包含窄屏切换按钮");
        assert!(right.contains("article h1, article h2"), "导航脚本扫描标题");
        assert!(right.contains("'header'"), "跳过 header 小节");

        let left = wrap_html("接口文档", md, "left");
        assert!(left.contains("<body class=\"nav-left\">"));
        let off = wrap_html("接口文档", md, "off");
        assert!(off.contains("<body class=\"nav-off\">"));
        let unknown = wrap_html("接口文档", md, "xyz");
        assert!(unknown.contains("<body class=\"nav-right\">"), "非法值回退右侧");
        // 标题转义不被破坏
        let t = wrap_html("A & B <C>", md, "off");
        assert!(t.contains("<title>A &amp; B &lt;C&gt;</title>"));
    }

    #[test]
    fn roundtrip() {
        let api = sample_api();
        let md = render(&api, "用户管理", false);
        // 新格式结构
        assert!(md.starts_with("# 用户管理\n"), "分组标题");
        assert!(md.contains("## 创建用户\n"), "接口标题");
        assert!(md.contains("> POST http://example.com/api/users"), "方法+URL");
        assert!(md.contains("## header\n"), "header 小节");
        assert!(md.contains("Content-Type: application/json"));
        assert!(md.contains("## 请求参数\n"));
        assert!(md.contains("### path\n"));
        assert!(md.contains("### query\n"));
        assert!(md.contains("### body\n"));
        assert!(md.contains("## 响应参数\n"));
        assert!(md.contains("### 返回成功（HTTP 200）\n"));
        assert!(md.contains("### 返回失败（HTTP 400）\n"));
        assert!(md.contains("### 请求示例\n"));
        assert!(md.contains("curl -X POST http://example.com/api/users"));
        assert!(md.contains("-H \"Content-Type: application/json\""));

        let parsed = parse(&md).expect("parse ok");
        assert_eq!(parsed.group, "用户管理");
        assert_eq!(parsed.apis.len(), 1);
        let a = &parsed.apis[0];
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
        assert!(a.mock.body.contains("code"));
    }

    #[test]
    fn render_falls_back_to_path_when_url_empty() {
        // 回归：url 为空但 path 有值时，导出 Markdown 必须带上 URL（否则文档只有方法没地址）
        let mut api = sample_api();
        api.url = String::new();
        api.path = "/api/users".into();
        let md = render(&api, "用户管理", false);
        assert!(md.contains("> POST /api/users"), "url 为空时回退 path");
        assert!(md.contains("curl -X POST /api/users"), "curl 示例同样回退 path");

        // 回读自洽：`> POST /api/users` 能还原 path
        let parsed = parse(&md).expect("parse ok");
        let a = &parsed.apis[0];
        assert_eq!(a.method, "POST");
        assert_eq!(a.path, "/api/users");
    }

    #[test]
    fn multi_api_and_empty_group() {
        // 无分组 + 多接口：H1 缺失时 group 为空
        let md = "## 接口一\n\n> GET /a\n\n## 接口二\n\n> POST /b\n";
        let parsed = parse(md).expect("parse ok");
        assert_eq!(parsed.group, "");
        assert_eq!(parsed.apis.len(), 2);
        assert_eq!(parsed.apis[0].name, "接口一");
        assert_eq!(parsed.apis[0].path, "/a");
        assert_eq!(parsed.apis[1].name, "接口二");
        assert_eq!(parsed.apis[1].method, "POST");
    }

    #[test]
    fn parse_old_format() {
        // 兼容旧格式（# 接口名 + ## 基本信息 + 描述引用行）
        let md = "# 旧接口\n\n> 描述内容\n\n## 基本信息\n\n- 方法: PUT\n- 路径: /old\n";
        let parsed = parse(md).expect("parse ok");
        assert_eq!(parsed.apis.len(), 1);
        let a = &parsed.apis[0];
        assert_eq!(a.name, "旧接口");
        assert_eq!(a.method, "PUT");
        assert_eq!(a.path, "/old");
        assert_eq!(a.description, "描述内容");
    }

    #[test]
    fn render_expands_object_children() {
        // 回归：类型为 Object 的字段必须展开下级字段（否则「值/子字段不显示」）
        let mut api = crate::ApiFile {
        pack: vec![],
        unpack: vec![],
        net: None,
        mq: None,
            uuid: "u1".into(),
            name: "创建用户".into(),
            method: "POST".into(),
            path: "/api/users".into(),
            url: "http://example.com/api/users".into(),
            description: String::new(),
            headers: vec![],
            query: vec![],
            params: vec![],
            body: crate::BodyData {
                mode: "json".into(),
                raw: String::new(),
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
        };

        // 注册 types 模拟 openapi_tag 子字段
        api.mock.body = r#"{"data":{"name":"张三","id":1},"code":0}"#.into();
        let md = render(&api, "", false);
        assert!(md.contains("| data | Object |"), "md: {md}");
        assert!(md.contains("| data.name | String |"), "md: {md}");
        assert!(md.contains("| data.id | Integer |"), "md: {md}");
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
        // 新格式响应表格（字段|类型|描述，点分路径）→ docParams 树
        let md = "# 测试\n\n## 接口A\n\n> GET http://x/api\n\n## 响应参数\n\n### 成功响应\n\n| 字段 | 类型 | 描述 |\n| --- | --- | --- |\n| code | Integer | 状态码 |\n| data.name | String | 姓名 |\n";
        let parsed = parse(md).expect("parse ok");
        assert_eq!(parsed.group, "测试");
        let a = &parsed.apis[0];
        assert_eq!(a.name, "接口A");
        let success: Vec<&DocParam> = a.doc_params.iter().filter(|d| d.source == "resp_success").collect();
        assert_eq!(success.len(), 2);
        let data = success.iter().find(|d| d.key == "data").unwrap();
        assert_eq!(data.children.len(), 1);
        assert_eq!(data.children[0].key, "name");
        assert_eq!(data.children[0].description, "姓名");
    }

    #[test]
    fn render_net_packet_roundtrip() {
        // TCP / UDP：文档用「报文结构」（封包 / 解包）代替 HTTP 参数，且导出→导入自洽
        let api = ApiFile {
            pack: vec![
                PacketField {
                    key: "magic".into(),
                    kind: "fixed".into(),
                    bytes: 4,
                    value: "AM01".into(),
                    description: "魔数".into(),
                    len_from: None,
                },
                PacketField {
                    key: "len".into(),
                    kind: "var".into(),
                    bytes: 1,
                    value: String::new(),
                    description: "长度".into(),
                    len_from: None,
                },
                PacketField {
                    key: "payload".into(),
                    kind: "varlen".into(),
                    bytes: 0,
                    value: "hello".into(),
                    description: "内容".into(),
                    len_from: Some(1),
                },
            ],
            unpack: vec![PacketField {
                key: "cmd".into(),
                kind: "var".into(),
                bytes: 1,
                value: String::new(),
                description: "命令字".into(),
                len_from: None,
            }],
            net: Some(NetConfig {
                host: "127.0.0.1".into(),
                port: 9100,
                timeout_ms: 5000,
            }),
            mq: None,
            uuid: "n1".into(),
            name: "TCP 回显".into(),
            method: String::new(),
            path: "/".into(),
            url: String::new(),
            description: "TCP 回显测试".into(),
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
            protocol: "tcp".into(),
        };
        let md = render(&api, "TCP", false);
        assert!(md.contains("> TCP 127.0.0.1:9100"), "{md}");
        assert!(md.contains("## 报文结构"), "{md}");
        assert!(md.contains("### 封包"), "{md}");
        assert!(md.contains("### 解包"), "{md}");
        assert!(md.contains("- 超时: 5000 ms"), "{md}");
        assert!(md.contains("| magic | 固定值 | 4 | AM01 | 魔数 |"), "{md}");
        assert!(md.contains("| payload | 不定长变量 | 取自 2 len | hello | 内容 |"), "{md}");
        // 不再输出 HTTP 专用小节
        assert!(!md.contains("## header"), "{md}");
        assert!(!md.contains("## 请求参数"), "{md}");
        assert!(!md.contains("## 响应参数"), "{md}");

        // 回读自洽
        let parsed = parse(&md).expect("parse ok");
        assert_eq!(parsed.group, "TCP");
        let a = &parsed.apis[0];
        assert_eq!(a.protocol, "tcp");
        assert_eq!(a.method, "");
        assert!(a.headers.is_empty() && a.params.is_empty() && a.responses.is_empty());
        let net = a.net.clone().expect("net 配置还原");
        assert_eq!(net.host, "127.0.0.1");
        assert_eq!(net.port, 9100);
        assert_eq!(net.timeout_ms, 5000);
        assert_eq!(a.pack.len(), 3);
        assert_eq!(a.pack[0].bytes, 4);
        assert_eq!(a.pack[1].kind, "var");
        assert_eq!(a.pack[2].kind, "varlen");
        assert_eq!(a.pack[2].len_from, Some(1));
        assert_eq!(a.unpack.len(), 1);
        assert_eq!(a.unpack[0].key, "cmd");
    }

    #[test]
    fn render_mq_roundtrip() {
        // MQ：文档用「MQ 配置」（连接信息 + 生产消息）代替 HTTP 参数，且导出→导入自洽
        let mut api = sample_api();
        api.protocol = "mq".into();
        api.method = String::new();
        api.path = "/".into();
        api.url = String::new();
        api.headers.clear();
        api.query.clear();
        api.params.clear();
        api.responses.clear();
        api.body = BodyData {
            mode: "json".into(),
            raw: "{\"orderId\":1}".into(),
            form: vec![],
            binary_path: String::new(),
        };
        api.mq = Some(MqConfig {
            kind: "rocketmq".into(),
            host: "10.0.0.9".into(),
            port: 9876,
            topic: "order.created".into(),
            group: "order-service".into(),
            offset: "earliest".into(),
            max_messages: 5,
            timeout_ms: 8000,
        });
        let md = render(&api, "MQ", false);
        assert!(md.contains("> MQ RocketMQ 10.0.0.9:9876"), "{md}");
        assert!(md.contains("## MQ 配置"), "{md}");
        assert!(md.contains("- 类型: RocketMQ"), "{md}");
        assert!(md.contains("- Topic: order.created"), "{md}");
        assert!(md.contains("- 消费组: order-service"), "{md}");
        assert!(md.contains("- 起始位置: earliest"), "{md}");
        assert!(md.contains("- 拉取条数: 5"), "{md}");
        assert!(md.contains("- 超时: 8000 ms"), "{md}");
        assert!(md.contains("### 生产消息"), "{md}");
        assert!(md.contains("{\"orderId\":1}"), "{md}");
        // 不再输出 HTTP 专用小节
        assert!(!md.contains("## header"), "{md}");
        assert!(!md.contains("## 请求参数"), "{md}");
        assert!(!md.contains("## 响应参数"), "{md}");

        // 回读自洽
        let parsed = parse(&md).expect("parse ok");
        let a = &parsed.apis[0];
        assert_eq!(a.protocol, "mq");
        assert_eq!(a.method, "");
        assert!(a.headers.is_empty() && a.params.is_empty() && a.responses.is_empty());
        let mq = a.mq.clone().expect("mq 配置还原");
        assert_eq!(mq.kind, "rocketmq");
        assert_eq!(mq.host, "10.0.0.9");
        assert_eq!(mq.port, 9876);
        assert_eq!(mq.topic, "order.created");
        assert_eq!(mq.group, "order-service");
        assert_eq!(mq.offset, "earliest");
        assert_eq!(mq.max_messages, 5);
        assert_eq!(mq.timeout_ms, 8000);
        assert_eq!(a.body.raw, "{\"orderId\":1}");
        assert_eq!(a.body.mode, "json");
    }

    #[test]
    fn render_mq_without_custom_port() {
        // 地址省略端口时回读按 MQ 类型补默认端口
        let mut api = sample_api();
        api.protocol = "mq".into();
        api.method = String::new();
        api.path = "/".into();
        api.url = String::new();
        api.headers.clear();
        api.query.clear();
        api.params.clear();
        api.responses.clear();
        api.body = BodyData {
            mode: "raw".into(),
            raw: "hello".into(),
            form: vec![],
            binary_path: String::new(),
        };
        api.mq = Some(MqConfig {
            kind: "rabbitmq".into(),
            host: "127.0.0.1".into(),
            port: 0,
            topic: String::new(),
            group: String::new(),
            offset: "latest".into(),
            max_messages: 1,
            timeout_ms: 3000,
        });
        let md = render(&api, "", false);
        assert!(md.contains("> MQ RabbitMQ 127.0.0.1"), "{md}");
        let parsed = parse(&md).expect("parse ok");
        let mq = parsed.apis[0].mq.clone().expect("mq 配置还原");
        assert_eq!(mq.kind, "rabbitmq");
        assert_eq!(mq.port, 5672);
    }

    #[test]
    fn render_udp_without_pack() {
        // UDP 未定义封包字段：只输出连接信息，不产生 HTTP 小节
        let mut api = sample_api();
        api.protocol = "udp".into();
        api.method = String::new();
        api.path = "/".into();
        api.url = String::new();
        api.headers.clear();
        api.query.clear();
        api.params.clear();
        api.body = BodyData::default();
        api.responses.clear();
        api.net = Some(NetConfig {
            host: "192.168.1.10".into(),
            port: 9101,
            timeout_ms: 3000,
        });
        let md = render(&api, "", false);
        assert!(md.contains("> UDP 192.168.1.10:9101"), "{md}");
        assert!(md.contains("- 目标地址: 192.168.1.10:9101"), "{md}");
        assert!(!md.contains("### 封包"), "{md}");
        let parsed = parse(&md).expect("parse ok");
        assert_eq!(parsed.apis[0].protocol, "udp");
        assert_eq!(parsed.apis[0].net.as_ref().unwrap().port, 9101);
    }
