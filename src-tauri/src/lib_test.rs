    use super::*;

    /// 极简 HTTP 服务器：对所有请求返回固定 JSON
    fn start_test_server() -> (std::net::SocketAddr, std::thread::JoinHandle<()>, std::sync::Arc<std::sync::atomic::AtomicBool>) {
        use std::io::ErrorKind;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use std::time::Duration;

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = stop.clone();
        let handle = std::thread::spawn(move || {
            loop {
                if stop2.load(Ordering::Relaxed) {
                    break;
                }
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let _ = std::thread::spawn(move || {
                            use std::io::{Read, Write};
                            let mut buf = [0u8; 4096];
                            let _ = stream.read(&mut buf);
                            let body = r#"{"hello":"world","n":42}"#;
                            let resp = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nX-Test: yes\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(),
                                body
                            );
                            let _ = stream.write_all(resp.as_bytes());
                        });
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });
        (addr, handle, stop)
    }

    #[tokio::test]
    async fn test_send_request_ok() {
        let (addr, handle, stop) = start_test_server();
        let req = HttpRequestData {
            method: "GET".into(),
            url: format!("http://{addr}/api/users/1001?page=1"),
            headers: vec![],
            body: None,
            body_file: None,
            form: None,
            timeout_ms: 5000,
        };
        let res = send_request(req).await.unwrap();
        assert!(res.ok);
        assert_eq!(res.status, 200);
        assert_eq!(res.status_text, "OK");
        assert!(res.body.contains("\"hello\":\"world\""));
        assert!(res.headers.iter().any(|(k, v)| k == "x-test" && v == "yes"));
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        handle.join().unwrap();
    }

    #[tokio::test]
    async fn test_send_request_multipart_file() {
        use axum::extract::Multipart;
        use axum::response::IntoResponse;

        async fn upload(mut mp: Multipart) -> impl IntoResponse {
            let mut text = String::new();
            let mut files = Vec::new();
            while let Some(field) = mp.next_field().await.unwrap() {
                let name = field.name().unwrap_or("").to_string();
                let data = field.bytes().await.unwrap();
                if name == "file" {
                    files.push(String::from_utf8_lossy(&data).to_string());
                } else {
                    text.push_str(&format!("{name}={}", String::from_utf8_lossy(&data)));
                }
            }
            format!("text:{text};files:{}", files.join(","))
        }

        let app = axum::Router::new().route("/upload", axum::routing::post(upload));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // 准备一个待上传的临时文件
        let file = std::env::temp_dir().join(format!("upload-test-{}.txt", std::process::id()));
        fs::write(&file, "hello-multipart").unwrap();

        let req = HttpRequestData {
            method: "POST".into(),
            url: format!("http://{addr}/upload"),
            headers: vec![],
            body: None,
            body_file: None,
            form: Some(vec![
                KeyValue {
                    key: "name".into(),
                    value: "张三".into(),
                    enabled: true,
                    is_file: false,
                    description: String::new(),
                },
                KeyValue {
                    key: "file".into(),
                    value: file.to_string_lossy().to_string(),
                    enabled: true,
                    is_file: true,
                    description: String::new(),
                },
            ]),
            timeout_ms: 5000,
        };
        let res = send_request(req).await.unwrap();
        server.abort();
        let _ = server.await;
        let _ = fs::remove_file(&file);

        assert!(res.ok, "multipart 请求应成功: {:?}", res.error);
        assert_eq!(res.status, 200);
        assert!(res.body.contains("name=张三"), "应包含文本字段: {}", res.body);
        assert!(res.body.contains("hello-multipart"), "应包含文件内容: {}", res.body);
    }

    #[tokio::test]
    async fn test_send_request_binary_file() {
        // 二进制模式：读取本地文件字节作为请求体发送
        async fn echo_body(body: axum::body::Bytes) -> impl axum::response::IntoResponse {
            format!("bytes:{}", String::from_utf8_lossy(&body))
        }

        let app = axum::Router::new().route("/echo", axum::routing::post(echo_body));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let file = std::env::temp_dir().join(format!("binary-test-{}.bin", std::process::id()));
        fs::write(&file, b"\x00\x01binary-body\xff").unwrap();

        let req = HttpRequestData {
            method: "POST".into(),
            url: format!("http://{addr}/echo"),
            headers: vec![],
            body: None,
            body_file: Some(file.to_string_lossy().to_string()),
            form: None,
            timeout_ms: 5000,
        };
        let res = send_request(req).await.unwrap();
        server.abort();
        let _ = server.await;
        let _ = fs::remove_file(&file);

        assert!(res.ok, "二进制请求应成功: {:?}", res.error);
        assert_eq!(res.status, 200);
        assert!(
            res.body.contains("binary-body"),
            "应发送文件字节: {}",
            res.body
        );
    }

    #[tokio::test]
    async fn test_send_request_bad_url() {
        // 未替换的 {{变量}} 会产生 reqwest builder error，应给出中文提示而不是裸的 builder error
        for url in ["http://{{host}}:8080/api", "127.0.0.1:8080/api"] {
            let req = HttpRequestData {
                method: "GET".into(),
                url: url.to_string(),
                headers: vec![],
                body: None,
                body_file: None,
                form: None,
                timeout_ms: 3000,
            };
            let res = send_request(req).await.unwrap();
            assert!(!res.ok, "url [{url}] 应失败");
            let err = res.error.unwrap_or_default();
            assert!(
                err.contains("URL 格式不正确") && !err.starts_with("builder"),
                "url [{url}] 错误信息不友好: {err}"
            );
        }
    }

    #[tokio::test]
    async fn test_send_request_connection_refused() {
        // 绑定一个端口后立刻释放，用于模拟连接失败
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let req = HttpRequestData {
            method: "GET".into(),
            url: format!("http://{addr}/"),
            headers: vec![],
            body: None,
            body_file: None,
            form: None,
            timeout_ms: 3000,
        };
        let res = send_request(req).await.unwrap();
        assert!(!res.ok);
        assert!(res.error.is_some());
    }

    #[test]
    fn test_read_env_map() {
        // 示例工作区：激活“开发环境”
        let root =
            Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../examples/demo-workspace"));
        let map = read_env_map(root);
        assert_eq!(
            map.get("baseUrl").map(|s| s.as_str()),
            Some("http://127.0.0.1:5050")
        );
        assert_eq!(map.get("token").map(|s| s.as_str()), Some("dev-token-123456"));
        // 不存在的环境 -> 空
        assert!(!map.contains_key("nope"));
    }

    #[test]
    fn test_import_postman() {
        let root = std::env::temp_dir().join(format!("apimgr-postman-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let coll = root.join("collection.json");
        fs::write(
            &coll,
            r#"{
                "info": { "name": "示例集合" },
                "variable": [
                    { "key": "baseUrl", "value": "https://api.example.com", "type": "string", "description": "接口基地址" },
                    { "key": "token", "value": "dev-token-123456", "type": "string" },
                    { "key": "timeout", "value": 30, "type": "number", "description": { "content": "超时秒数", "type": "text/plain" } }
                ],
                "item": [
                    {
                        "name": "获取用户",
                        "request": {
                            "method": "GET",
                            "url": {
                                "raw": "https://api.example.com/users/:id?page=1",
                                "query": [{ "key": "page", "value": "1", "disabled": false }]
                            },
                            "header": [{ "key": "Authorization", "value": "Bearer {{token}}", "disabled": false }]
                        }
                    },
                    {
                        "name": "订单",
                        "item": [
                            {
                                "name": "创建订单",
                                "request": {
                                    "method": "POST",
                                    "url": { "raw": "https://api.example.com/orders" },
                                    "body": { "mode": "raw", "raw": "{\"no\":\"1\"}" }
                                }
                            }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();
        let result = import_postman_file(&root, &coll).unwrap();
        assert!(result.folder.ends_with("示例集合"));
        // 集合变量 -> 环境变量集
        assert_eq!(result.env, "示例集合");
        assert_eq!(result.vars, 3);
        let env_store: EnvStore =
            serde_json::from_str(&fs::read_to_string(root.join("__envs.json")).unwrap()).unwrap();
        assert_eq!(env_store.active, "示例集合");
        assert_eq!(env_store.environments.len(), 1);
        let env = &env_store.environments[0];
        assert_eq!(env.name, "示例集合");
        assert_eq!(env.variables.len(), 3);
        let find = |k: &str| env.variables.iter().find(|v| v.key == k).unwrap();
        assert_eq!(find("baseUrl").value, "https://api.example.com");
        assert_eq!(find("baseUrl").description, "接口基地址");
        assert_eq!(find("token").value, "dev-token-123456");
        // 数字 value 转字符串、结构化 description 取 content
        assert_eq!(find("timeout").value, "30");
        assert_eq!(find("timeout").description, "超时秒数");
        // 顶层接口 + 子分组 + 子接口
        assert!(root.join("示例集合/获取用户.json").exists());
        assert!(root.join("示例集合/订单/创建订单.json").exists());
        // 校验内容：方法 / 路径变量 / query / header / body
        let api: ApiFile = serde_json::from_str(
            &fs::read_to_string(root.join("示例集合/获取用户.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(api.method, "GET");
        assert_eq!(api.path, "/users/{id}");
        assert_eq!(api.params.len(), 1);
        assert_eq!(api.params[0].key, "id");
        assert_eq!(api.query.len(), 1);
        assert_eq!(api.query[0].key, "page");
        assert_eq!(api.headers.len(), 1);
        assert_eq!(api.headers[0].key, "Authorization");
        let api2: ApiFile = serde_json::from_str(
            &fs::read_to_string(root.join("示例集合/订单/创建订单.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(api2.method, "POST");
        assert_eq!(api2.body.mode, "json");
        // 重复导入同一集合：变量按 key 合并，不产生重复集
        import_postman_file(&root, &coll).unwrap();
        let env_store2: EnvStore =
            serde_json::from_str(&fs::read_to_string(root.join("__envs.json")).unwrap()).unwrap();
        assert_eq!(env_store2.environments.len(), 1);
        assert_eq!(env_store2.environments[0].variables.len(), 3);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_env_store_roundtrip() {
        let dir = std::env::temp_dir().join(format!("env-test-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let store = EnvStore {
            active: "dev".into(),
            environments: vec![
                Environment {
                    name: "dev".into(),
                    variables: vec![EnvVariable {
                        key: "k".into(),
                        value: "v".into(),
                        default_value: "".into(),
                        description: "".into(),
                        enabled: true,
                    }],
                },
                Environment {
                    name: "prod".into(),
                    variables: vec![],
                },
            ],
        };
        write_pretty(&dir.join(ENV_FILE), &store).unwrap();
        let back = read_env_file(&dir);
        assert_eq!(back.active, "dev");
        assert_eq!(back.environments.len(), 2);
        assert_eq!(back.environments[0].variables[0].key, "k");
        assert_eq!(read_env_map(&dir).get("k").map(|s| s.as_str()), Some("v"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_env_default_value_fallback() {
        // 现有值为空时，自动使用默认值
        let dir = std::env::temp_dir().join(format!("env-default-test-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let store = EnvStore {
            active: "dev".into(),
            environments: vec![Environment {
                name: "dev".into(),
                variables: vec![
                    EnvVariable {
                        key: "empty_value".into(),
                        value: "".into(),
                        default_value: "fallback".into(),
                        description: "".into(),
                        enabled: true,
                    },
                    EnvVariable {
                        key: "has_value".into(),
                        value: "real".into(),
                        default_value: "fallback".into(),
                        description: "".into(),
                        enabled: true,
                    },
                ],
            }],
        };
        write_pretty(&dir.join(ENV_FILE), &store).unwrap();
        let map = read_env_map(&dir);
        assert_eq!(map.get("empty_value").map(|s| s.as_str()), Some("fallback"));
        assert_eq!(map.get("has_value").map(|s| s.as_str()), Some("real"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_next_version() {
        // 版本号递增：<name>.1.json / .2.json ...
        let dir = std::env::temp_dir().join(format!("version-test-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        for f in ["x.1.json", "x.3.json", "other.9.json"] {
            fs::write(dir.join(f), "{}").unwrap();
        }
        assert_eq!(next_version(&dir, "x"), 4);
        assert_eq!(next_version(&dir, "y"), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_version_saved_at_workspace_root() {
        // .version 必须创建在工作区根目录下，而不是接口文件所在的子目录
        let root = std::env::temp_dir().join(format!("version-root-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        // 接口文件位于根目录的子目录中
        let sub = root.join("some-folder");
        fs::create_dir_all(&sub).unwrap();

        let api = ApiFile {
            uuid: "11111111-2222-3333-4444-555555555555".into(),
            name: "创建用户".into(),
            method: "POST".into(),
            path: "/api/users".into(),
            url: "".into(),
            description: "".into(),
            headers: vec![],
            query: vec![],
            params: vec![],
            body: BodyData::default(),
            mock: MockConfig::default(),
            examples: vec![],
            responses: vec![],
            doc_params: vec![],
        deprecated: false,
        protocol: "http".into(),
        };
        let rel = save_api_version_at(&root, api).unwrap();
        assert!(rel.starts_with(".version/11111111-2222-3333-4444-555555555555/"));
        let ver_file = root
            .join(crate::VERSION_DATA_DIR)
            .join("11111111-2222-3333-4444-555555555555")
            .join("创建用户.1.json");
        assert!(ver_file.exists(), "版本文件应写入根目录 .version 下");
        assert!(
            !sub.join(crate::VERSION_DATA_DIR).exists(),
            "版本目录不应出现在接口所在子目录中"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("v0.1.5"), vec![0, 1, 5]);
        assert_eq!(parse_version("0.2.0"), vec![0, 2, 0]);
        assert_eq!(parse_version("V1.2.3-beta.4"), vec![1, 2, 3, 4]);
        assert_eq!(parse_version("9.9.9"), vec![9, 9, 9]);
        assert_eq!(parse_version(""), Vec::<u32>::new());
    }

    #[test]
    fn test_version_gt() {
        assert!(version_gt("0.2.0", "0.1.5"));
        assert!(version_gt("1.0.0", "0.9.9"));
        assert!(version_gt("0.1.10", "0.1.9"));
        assert!(version_gt("0.2", "0.1.9")); // 段数多
        assert!(!version_gt("0.1.5", "0.1.5"));
        assert!(!version_gt("0.1.4", "0.1.5"));
        assert!(!version_gt("0.1.9", "0.2.0"));
        assert!(!version_gt("", "0.1.5")); // 空版本不视为更新
    }

    /// 旧文件无 responses 字段时：返回成功取 mock 体、返回失败由 resp_fail 文档生成，docParams 重键到 resp:<id>
    #[test]
    fn ensure_responses_migrates_old_files() {
        let mut api = ApiFile {
            uuid: "u".into(),
            name: "测试".into(),
            method: "GET".into(),
            path: "/x".into(),
            url: String::new(),
            description: String::new(),
            headers: vec![],
            query: vec![],
            params: vec![],
            body: BodyData::default(),
            mock: MockConfig {
                enabled: false,
                status: 200,
                headers: vec![],
                delay: 0,
                body: r#"{"code":0}"#.into(),
            },
            examples: vec![],
            responses: vec![],
            doc_params: vec![DocParam {
                source: "resp_fail".into(),
                key: "message".into(),
                r#type: "String".into(),
                description: "错误描述".into(),
                item_type: String::new(),
                object_name: String::new(),
                children: vec![],
            }],
            deprecated: false,
            protocol: "http".into(),
        };
        ensure_responses(&mut api);
        assert_eq!(api.responses.len(), 2);
        assert_eq!(api.responses[0].name, "返回成功");
        assert_eq!(api.responses[0].status, 200);
        assert_eq!(api.responses[0].body, r#"{"code":0}"#);
        assert_eq!(api.responses[1].name, "返回失败");
        assert!(api.responses[1].body.contains("message"), "fail body: {}", api.responses[1].body);
        // docParams 已重键到新条目 id
        assert!(api.doc_params.iter().all(|d| d.source == format!("resp:{}", api.responses[1].id)));
    }

    /// 分组目录保存 .md/.html：export_api_markdown 的分支走 group_markdown_doc，目录不再是 read_api 目标
    #[test]
    fn group_markdown_doc_renders_group_dir() {
        let base = std::env::temp_dir().join(format!("apim-gmdoc-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let g = base.join("用户管理");
        fs::create_dir_all(&g).unwrap();
        let a = ApiFile {
            uuid: "u".into(),
            name: "接口A".into(),
            method: "GET".into(),
            path: "/a".into(),
            url: String::new(),
            description: String::new(),
            headers: vec![],
            query: vec![],
            params: vec![],
            body: BodyData::default(),
            mock: MockConfig {
                enabled: false,
                status: 200,
                headers: vec![],
                delay: 0,
                body: String::new(),
            },
            examples: vec![],
            responses: vec![],
            doc_params: vec![],
        deprecated: false,
        protocol: "http".into(),
        };
        fs::write(
            g.join("接口A.json"),
            serde_json::to_string(&a).unwrap(),
        )
        .unwrap();
        // 空分组直接报错（与导出逻辑一致）
        let empty = base.join("空分组");
        fs::create_dir_all(&empty).unwrap();
        assert!(group_markdown_doc(&base, &empty.to_string_lossy()).is_err());
        let (name, md) = group_markdown_doc(&base, &g.to_string_lossy()).expect("group doc");
        assert_eq!(name, "用户管理");
        assert!(md.contains("## 接口A"), "md: {md}");
        // 分组名即标题：不再重复输出 # 用户管理
        assert_eq!(md.matches("# 用户管理").count(), 1, "md: {md}");
        let _ = fs::remove_dir_all(&base);
    }

    /// 恢复到历史版本：先自动备份当前状态为新版本，再把版本内容写回主文件
    #[test]
    fn restore_api_version_backs_up_then_restores() {
        let base = std::env::temp_dir().join(format!("apim-restore-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("接口"));
        let uuid = "a1b2c3d4-1111-2222-3333-444455556666".to_string();
        let make = |name: &str, desc: &str| ApiFile {
            uuid: uuid.clone(),
            name: name.into(),
            method: "GET".into(),
            path: "/x".into(),
            url: String::new(),
            description: desc.into(),
            headers: vec![],
            query: vec![],
            params: vec![],
            body: BodyData::default(),
            mock: MockConfig {
                enabled: false,
                status: 200,
                headers: vec![],
                delay: 0,
                body: String::new(),
            },
            examples: vec![],
            responses: vec![],
            doc_params: vec![],
        deprecated: false,
        protocol: "http".into(),
        };
        let main = base.join("接口").join("接口A.json");
        save_api(main.to_string_lossy().to_string(), make("接口A", "v1 描述"))
            .unwrap();
        // 保存两个版本：v1（描述 v1 描述）与 v2（描述 v2 描述）
        save_api_version_at(&base, make("接口A", "v1 描述")).unwrap();
        let _v2 = save_api_version_at(&base, make("接口A", "v2 描述")).unwrap();
        // 主文件当前是 v2 描述
        let mut current = read_api(main.to_string_lossy().to_string()).unwrap();
        current.description = "v2 描述".into();
        save_api(main.to_string_lossy().to_string(), current).unwrap();
        // 列出版本：v2、v1（从大到小）
        let dir = base.join(crate::VERSION_DATA_DIR).join(&uuid);
        let files: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(files.len(), 2, "files: {files:?}");
        // 恢复到 v1
        let v1_path = dir.join("接口A.1.json");
        let restored_main = restore_api_version_at(&base, &v1_path.to_string_lossy(), &uuid);
        let main_str = restored_main.unwrap();
        assert_eq!(main_str, main.to_string_lossy().to_string());
        let restored = read_api(main_str).unwrap();
        assert_eq!(restored.description, "v1 描述");
        // 恢复前自动保存了当前（v2）为新版本 → 现在 3 个版本文件
        let files2: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(files2.len(), 3, "files: {files2:?}");
        let backup = read_api(dir.join("接口A.3.json").to_string_lossy().to_string()).unwrap();
        assert_eq!(backup.description, "v2 描述");
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn test_detect_vcs() {
        // 检测 .git / .svn；都没有则返回 None
        let root = std::env::temp_dir().join(format!("vcs-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        assert_eq!(detect_vcs(&root), None);
        fs::create_dir_all(root.join(".git")).unwrap();
        assert_eq!(detect_vcs(&root).as_deref(), Some("git"));
        fs::remove_dir_all(root.join(".git")).unwrap();
        fs::create_dir_all(root.join(".svn")).unwrap();
        assert_eq!(detect_vcs(&root).as_deref(), Some("svn"));
        // .git 优先于 .svn
        fs::create_dir_all(root.join(".git")).unwrap();
        assert_eq!(detect_vcs(&root).as_deref(), Some("git"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_import_openapi_file() {
        let root = std::env::temp_dir().join(format!("oas-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let spec = serde_json::json!({
            "openapi": "3.0.0",
            "info": { "title": "PetStore", "version": "1.0" },
            "servers": [ { "url": "https://api.example.com/v1" } ],
            "paths": {
                "/pets/{id}": {
                    "get": {
                        "tags": ["pets"],
                        "summary": "按 ID 获取宠物",
                        "parameters": [
                            { "name": "id", "in": "path", "required": true, "schema": { "type": "integer" } },
                            { "name": "verbose", "in": "query", "schema": { "type": "boolean", "default": true } }
                        ],
                        "responses": { "200": { "description": "ok" } }
                    }
                },
                "/pets": {
                    "post": {
                        "tags": ["pets"],
                        "summary": "新建宠物",
                        "requestBody": {
                            "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Pet" } } }
                        },
                        "responses": { "200": { "description": "ok" } }
                    }
                }
            },
            "components": {
                "schemas": {
                    "Pet": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "integer" },
                            "name": { "type": "string" },
                            "status": { "type": "string", "enum": ["available", "sold"] }
                        }
                    }
                }
            }
        });
        let spec_file = root.join("swagger.json");
        fs::write(&spec_file, serde_json::to_string(&spec).unwrap()).unwrap();

        let result = import_openapi_file(&root, &spec_file).unwrap();
        assert_eq!(result.count, 2);
        let folder = PathBuf::from(&result.folder);
        assert!(folder.join("__info.json").exists());

        // 按 tag 分组到 pets 子目录
        let pets = folder.join("pets");
        assert!(pets.exists());
        let get_api: ApiFile = serde_json::from_str(
            &fs::read_to_string(pets.join("GET _pets_{id}.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(get_api.method, "GET");
        assert_eq!(get_api.path, "/pets/{id}");
        assert_eq!(get_api.url, "https://api.example.com/v1/pets/{id}");
        assert_eq!(get_api.params.len(), 1);
        assert_eq!(get_api.query.len(), 1);
        assert_eq!(get_api.description, "按 ID 获取宠物");

        let post_api: ApiFile = serde_json::from_str(
            &fs::read_to_string(pets.join("POST _pets.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(post_api.body.mode, "json");
        assert!(post_api.body.raw.contains("\"name\""));

        // YAML 格式同样支持（.yaml / .yml）
        let yaml_content = serde_yaml::to_string(&spec).unwrap();
        let yaml_file = root.join("swagger.yaml");
        fs::write(&yaml_file, yaml_content).unwrap();
        let result2 = import_openapi_file(&root, &yaml_file).unwrap();
        assert_eq!(result2.count, 2);
        assert!(PathBuf::from(&result2.folder).join("pets").exists());

        let _ = fs::remove_dir_all(&root);
    }

    /// 使用 tests/data/apifox.json 真实文件验证 Apifox 项目导入
    #[test]
    fn test_import_apifox_file() {
        let root = std::env::temp_dir().join(format!("apimgr-apifox-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let file = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/data/apifox.json"
        ));
        let result = import_apifox_file(&root, &file).expect("apifox 导入失败");
        assert!(result.count > 0, "应至少导入 1 个接口，实际 {}", result.count);
        let folder = PathBuf::from(&result.folder);
        assert!(folder.join(INFO_FILE).exists());
        // 宠物商店示例：根集合下含「宠物」分组与 GET 接口
        let pets = folder.join("宠物");
        assert!(pets.exists(), "应生成「宠物」分组");
        // 直接读取「获取宠物」接口文件验证转换结果
        let api_file = pets.join("获取宠物.json");
        assert!(api_file.exists(), "应生成「获取宠物.json」");
        let api: ApiFile =
            serde_json::from_str(&fs::read_to_string(api_file).unwrap()).unwrap();
        assert_eq!(api.method, "GET");
        assert_eq!(api.path, "/pets/{id}");
        assert_eq!(api.params.len(), 1);
        assert_eq!(api.params[0].key, "id");
        // 同名分组合并：apiCollection 两个集合的「宠物」应合并，不生成「宠物 (2)」
        assert!(!folder.join("宠物 (2)").exists(), "不应生成重复分组「宠物 (2)」");
        assert!(
            folder.join("宠物").join("批量创建宠物.json").exists(),
            "第二个集合的接口应合并进「宠物」分组"
        );
        // webSocketCollection 的空分组占位（宠物/商店/用户 无接口）不应创建
        assert!(
            !folder.join("商店 (2)").exists(),
            "不应生成空分组「商店 (2)」"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// 使用 tests/data/apipost.json 真实文件验证 Apipost 项目导入
    #[test]
    fn test_import_apipost_file() {
        let root = std::env::temp_dir().join(format!("apimgr-apipost-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let file = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/data/apipost.json"
        ));
        let result = import_apipost_file(&root, &file).expect("apipost 导入失败");
        // 文件含 406 个 api + 15 个 graphql，folder 只建分组不计数
        assert_eq!(result.count, 421, "接口数应为 421，实际 {}", result.count);
        let folder = PathBuf::from(&result.folder);
        assert!(folder.join(INFO_FILE).exists());
        // Auth 分组（parent_id=0 的根分组）应存在
        assert!(folder.join("Auth").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_import_raml_file() {
        let root = std::env::temp_dir().join(format!("apimgr-raml-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let file = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/data/demo.raml"
        ));
        let result = import_raml_file(&root, &file).expect("raml 导入失败");
        // demo.raml 含 /users 的 get/post 两个接口
        assert_eq!(result.count, 2, "接口数应为 2，实际 {}", result.count);
        let folder = PathBuf::from(&result.folder);
        assert!(folder.join(INFO_FILE).exists());
        assert!(
            folder.join("GET _users.json").exists(),
            "GET _users.json 应存在"
        );
        let api: ApiFile = serde_json::from_str(
            &fs::read_to_string(folder.join("GET _users.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(api.method, "GET");
        assert_eq!(api.path, "/users");
        assert_eq!(api.url, "https://api.example.com/v1/users");
        // queryParameters page 应导入为查询参数
        assert_eq!(api.query.len(), 1);
        assert_eq!(api.query[0].key, "page");
        assert_eq!(api.query[0].value, "1");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_import_wadl_file() {
        let root = std::env::temp_dir().join(format!("apimgr-wadl-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let file = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/data/demo.wadl"
        ));
        let result = import_wadl_file(&root, &file).expect("wadl 导入失败");
        // demo.wadl 含 /users 的 GET/POST 两个接口
        assert_eq!(result.count, 2, "接口数应为 2，实际 {}", result.count);
        let folder = PathBuf::from(&result.folder);
        assert!(folder.join(INFO_FILE).exists());
        assert!(
            folder.join("GET _users.json").exists(),
            "GET _users.json 应存在"
        );
        let api: ApiFile = serde_json::from_str(
            &fs::read_to_string(folder.join("GET _users.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(api.method, "GET");
        assert_eq!(api.path, "/users");
        // query param page（style=query）应导入为查询参数
        assert_eq!(api.query.len(), 1);
        assert_eq!(api.query[0].key, "page");
        assert_eq!(api.query[0].value, "1");
        // 分组 INFO_FILE 应记录 base
        let info: InfoJson =
            serde_json::from_str(&fs::read_to_string(folder.join(INFO_FILE)).unwrap()).unwrap();
        assert_eq!(info.base_url.as_deref(), Some("https://api.example.com/v1"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_export_raml_wadl() {
        // 用导出的 RAML/WADL 再导回：round-trip 冒烟
        let root = std::env::temp_dir().join(format!("apimgr-rw-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        // 构造两个接口
        let make = |name: &str, method: &str, path: &str, is_ws: bool| ApiFile {
            uuid: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            method: method.into(),
            path: path.into(),
            url: format!("https://api.example.com{path}"),
            description: format!("{name} 描述"),
            headers: vec![KeyValue {
                key: "X-Token".into(),
                value: "abc".into(),
                enabled: true,
                is_file: false,
                description: "令牌".into(),
            }],
            query: vec![KeyValue {
                key: "page".into(),
                value: "1".into(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            params: vec![KeyValue {
                key: "id".into(),
                value: String::new(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            body: BodyData {
                mode: "json".into(),
                raw: "{\"a\":1}".into(),
                form: vec![],
                binary_path: String::new(),
            },
            mock: MockConfig::default(),
            examples: vec![],
            responses: vec![],
            doc_params: vec![],
            deprecated: false,
            protocol: if is_ws { "websocket".into() } else { "http".into() },
        };
        let apis = vec![
            (vec![], make("get", "GET", "/users", false)),
            (vec![], make("post", "POST", "/users", false)),
            (vec![], make("ws", "GET", "/ws/chat", true)),
        ];
        // RAML 导出：ws 应被过滤
        let raml = export::to_raml(&apis);
        assert!(raml.get("/users").is_some(), "RAML 应包含 /users");
        assert!(raml.get("/ws/chat").is_none(), "RAML 不应包含 WS 接口");
        assert_eq!(raml["/users"]["get"]["queryParameters"]["page"]["default"], "1");
        let yaml = serde_yaml::to_string(&raml).unwrap();
        assert!(yaml.contains("baseUri: https://api.example.com"));
        // WADL 导出
        let wadl = export::to_wadl(&apis);
        assert!(wadl.contains("<resource path=\"users\">"));
        assert!(wadl.contains("<method name=\"GET\">"));
        assert!(!wadl.contains("ws/chat"), "WADL 不应包含 WS 接口");
        // WADL 可再解析回接口
        let tmp = root.join("round.wadl");
        fs::write(&tmp, &wadl).unwrap();
        let re = import_wadl_file(&root, &tmp).expect("wadl round-trip 失败");
        assert_eq!(re.count, 2, "round-trip 接口数应为 2");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_import_har_file() {
        let root = std::env::temp_dir().join(format!("apimgr-har-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        // 内联迷你 HAR：两个 host、浏览器自动头应被过滤、json body、响应示例、urlencoded 表单
        let har = serde_json::json!({
            "log": {
                "version": "1.2",
                "creator": { "name": "t", "version": "1" },
                "pages": [{ "title": "示例站点" }],
                "entries": [
                    {
                        "request": {
                            "method": "GET",
                            "url": "https://api.example.com/users?page=2",
                            "headers": [
                                { "name": "Accept", "value": "*/*" },
                                { "name": "User-Agent", "value": "Mozilla/5.0" },
                                { "name": "X-Api-Key", "value": "secret123" },
                                { "name": "Cookie", "value": "sid=abc" }
                            ],
                            "queryString": [
                                { "name": "page", "value": "2" }
                            ],
                            "postData": null
                        },
                        "response": {
                            "status": 200,
                            "content": {
                                "mimeType": "application/json",
                                "text": "{\"list\":[]}"
                            }
                        }
                    },
                    {
                        "request": {
                            "method": "POST",
                            "url": "https://api.example.com/users",
                            "headers": [
                                { "name": "Content-Type", "value": "application/json" }
                            ],
                            "queryString": [],
                            "postData": {
                                "mimeType": "application/json",
                                "text": "{\"name\":\"张三\"}"
                            }
                        },
                        "response": {
                            "status": 201,
                            "content": {
                                "mimeType": "application/json",
                                "text": "{\"id\":1}"
                            }
                        }
                    },
                    {
                        "request": {
                            "method": "POST",
                            "url": "https://track.other.com/event",
                            "headers": [],
                            "queryString": [],
                            "postData": {
                                "mimeType": "application/x-www-form-urlencoded",
                                "text": "a=1&name=%E5%BC%A0%E4%B8%89"
                            }
                        },
                        "response": {
                            "status": 200,
                            "content": { "mimeType": "text/plain", "text": "ok" }
                        }
                    }
                ]
            }
        });
        let file = root.join("sample.har");
        fs::write(&file, serde_json::to_string_pretty(&har).unwrap()).unwrap();
        let result = import_har_file(&root, &file).expect("har 导入失败");
        assert_eq!(result.count, 3, "接口数应为 3，实际 {}", result.count);
        let folder = PathBuf::from(&result.folder);
        assert!(folder.join(INFO_FILE).exists());
        // 按 host 分小组
        let api_host = folder.join("api.example.com");
        let track_host = folder.join("track.other.com");
        assert!(api_host.is_dir(), "api.example.com 分组应存在");
        assert!(track_host.is_dir(), "track.other.com 分组应存在");
        // GET /users 的接口：query 参数、浏览器头被过滤、X-Api-Key 保留
        let get_file = api_host.join("GET _users.json");
        assert!(get_file.exists(), "GET _users.json 应存在");
        let api: ApiFile =
            serde_json::from_str(&fs::read_to_string(get_file).unwrap()).unwrap();
        assert_eq!(api.method, "GET");
        assert_eq!(api.path, "/users");
        assert_eq!(api.query.len(), 1);
        assert_eq!(api.query[0].key, "page");
        assert_eq!(api.query[0].value, "2");
        assert_eq!(api.headers.len(), 1, "浏览器自动头应被过滤");
        assert_eq!(api.headers[0].key, "x-api-key");
        assert_eq!(api.headers[0].value, "secret123");
        // 响应示例已存储
        assert_eq!(api.responses.len(), 1);
        assert_eq!(api.responses[0].status, 200);
        assert!(api.responses[0].body.contains("list"));
        // POST json body
        let post_file = api_host.join("POST _users.json");
        let post: ApiFile =
            serde_json::from_str(&fs::read_to_string(post_file).unwrap()).unwrap();
        assert_eq!(post.body.mode, "json");
        assert!(post.body.raw.contains("张三"));
        assert_eq!(post.responses[0].status, 201);
        // urlencoded 表单 → form 列表 + 解码
        let ev_file = track_host.join("POST _event.json");
        let ev: ApiFile = serde_json::from_str(&fs::read_to_string(ev_file).unwrap()).unwrap();
        assert_eq!(ev.body.mode, "form");
        assert_eq!(ev.body.form.len(), 2);
        assert_eq!(ev.body.form[0].key, "a");
        assert_eq!(ev.body.form[0].value, "1");
        assert_eq!(ev.body.form[1].value, "张三");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_import_yapi_swagger() {
        // YApi 的 swagger 数据导出（tests/data/yapi.json）应走 openapi 导入
        let root = std::env::temp_dir().join(format!("apimgr-yapi-s-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let file = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/data/yapi.json"
        ));
        let result = import_yapi_file(&root, &file).expect("yapi(swagger) 导入失败");
        // paths: /user/{uid} get、/user/add post、/order/list get
        assert_eq!(result.count, 3, "接口数应为 3，实际 {}", result.count);
        let folder = PathBuf::from(&result.folder);
        assert!(folder.join(INFO_FILE).exists());
        // 按 tag 分组的「用户模块」应存在，且含 GET _user_{uid}.json
        let um = folder.join("用户模块");
        assert!(um.is_dir(), "用户模块分组应存在");
        assert!(um.join("GET _user_{uid}.json").exists());
        let api: ApiFile = serde_json::from_str(
            &fs::read_to_string(um.join("GET _user_{uid}.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(api.method, "GET");
        assert_eq!(api.path, "/user/{uid}");
        // path 参数 uid + query 参数 withExtra
        assert!(api.params.iter().any(|p| p.key == "uid"));
        assert!(api.query.iter().any(|q| q.key == "withExtra"));
        // Authorization header
        assert!(api.headers.iter().any(|h| h.key.eq_ignore_ascii_case("Authorization")));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_import_yapi_native() {
        // YApi 原生导出树：分组/接口/表单/json body/WS
        let root = std::env::temp_dir().join(format!("apimgr-yapi-n-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let doc = serde_json::json!([
            {
                "name": "用户模块",
                "desc": "",
                "children": [
                    {
                        "name": "获取用户",
                        "api": {
                            "method": "GET",
                            "path": "/user/:uid",
                            "title": "获取用户",
                            "desc": "根据ID查询",
                            "req_query": [
                                { "name": "withExtra", "value": "", "desc": "是否扩展", "example": "true" }
                            ],
                            "req_headers": [
                                { "name": "X-Token", "value": "abc", "desc": "令牌" }
                            ],
                            "req_body_type": null,
                            "req_body_other": "",
                            "req_body_form": [],
                            "res_body_type": "json",
                            "res_body": "{\"code\":0}",
                            "protocol": "http"
                        }
                    },
                    {
                        "name": "新增用户",
                        "api": {
                            "method": "POST",
                            "path": "/user/add",
                            "title": "新增用户",
                            "desc": "",
                            "req_body_type": "json",
                            "req_body_other": "{\"name\":\"张三\"}",
                            "res_body_type": null,
                            "res_body": "",
                            "protocol": "http"
                        }
                    },
                    {
                        "name": "上传头像",
                        "api": {
                            "method": "POST",
                            "path": "/user/avatar",
                            "title": "上传头像",
                            "desc": "",
                            "req_body_type": "form",
                            "req_body_form": [
                                { "name": "file", "type": "file", "desc": "图片" },
                                { "name": "tag", "value": "avatar", "type": "text" }
                            ],
                            "res_body_type": null,
                            "res_body": "",
                            "protocol": "http"
                        }
                    },
                    {
                        "name": "消息推送",
                        "api": {
                            "method": "GET",
                            "path": "ws://example.com/chat",
                            "title": "消息推送",
                            "desc": "",
                            "req_body_type": null,
                            "req_body_other": "",
                            "res_body_type": null,
                            "res_body": "",
                            "protocol": "ws"
                        }
                    },
                    {
                        "name": "空分组",
                        "desc": "",
                        "children": []
                    }
                ]
            }
        ]);
        let file = root.join("yapi-native.json");
        fs::write(&file, serde_json::to_string_pretty(&doc).unwrap()).unwrap();
        let result = import_yapi_file(&root, &file).expect("yapi 原生导入失败");
        assert_eq!(result.count, 4, "接口数应为 4，实际 {}", result.count);
        let folder = PathBuf::from(&result.folder);
        let um = folder.join("用户模块");
        assert!(um.is_dir(), "用户模块分组应存在");
        assert!(!um.join("空分组").exists(), "空分组不应创建");
        // 获取用户：query/header/路径参数 :uid → {uid}
        let get: ApiFile = serde_json::from_str(
            &fs::read_to_string(um.join("获取用户.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(get.method, "GET");
        assert_eq!(get.path, "/user/{uid}");
        assert!(get.params.iter().any(|p| p.key == "uid"));
        assert!(get.query.iter().any(|q| q.key == "withExtra" && q.value == "true"));
        assert!(get.headers.iter().any(|h| h.key == "X-Token"));
        assert_eq!(get.responses[0].body, "{\"code\":0}");
        // 新增用户：json body 格式化
        let add: ApiFile = serde_json::from_str(
            &fs::read_to_string(um.join("新增用户.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(add.body.mode, "json");
        assert!(add.body.raw.contains("张三"));
        // 上传头像：form 文件字段
        let up: ApiFile = serde_json::from_str(
            &fs::read_to_string(um.join("上传头像.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(up.body.mode, "form");
        assert!(up.body.form.iter().any(|f| f.key == "file" && f.is_file));
        assert!(up.body.form.iter().any(|f| f.key == "tag" && f.value == "avatar"));
        // WS 接口
        let ws: ApiFile = serde_json::from_str(
            &fs::read_to_string(um.join("消息推送.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(ws.protocol, "websocket");
        assert!(ws.path.starts_with("ws://"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_export_yapi_roundtrip() {
        // 导出 YApi 原生格式后能再导入回来
        let root = std::env::temp_dir().join(format!("apimgr-yapi-e-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let make = |name: &str, method: &str, path: &str| ApiFile {
            uuid: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            method: method.into(),
            path: path.into(),
            url: path.into(),
            description: format!("{name} 描述"),
            headers: vec![KeyValue {
                key: "X-Token".into(),
                value: "abc".into(),
                enabled: true,
                is_file: false,
                description: "令牌".into(),
            }],
            query: vec![KeyValue {
                key: "page".into(),
                value: "1".into(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            params: vec![KeyValue {
                key: "id".into(),
                value: String::new(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            body: BodyData {
                mode: "json".into(),
                raw: "{\"a\":1}".into(),
                form: vec![],
                binary_path: String::new(),
            },
            mock: MockConfig::default(),
            examples: vec![],
            responses: vec![ResponseItem {
                id: "r1".into(),
                name: "HTTP 200".into(),
                status: 200,
                content_type: "application/json".into(),
                body: "{\"ok\":true}".into(),
            }],
            doc_params: vec![],
            deprecated: false,
            protocol: "http".into(),
        };
        let apis: Vec<(Vec<(String, bool)>, ApiFile)> = vec![
            (vec![("用户模块".to_string(), true)], make("获取用户", "GET", "/user/{id}")),
            (vec![], make("订单列表", "GET", "/order/list")),
        ];
let v = export::to_yapi(&apis);
        let arr = v.as_array().expect("yapi 导出应为数组");
        // 根级「订单列表」+ 分组「用户模块」
        assert_eq!(arr.len(), 2);
        let um = arr
            .iter()
            .find(|n| n.get("name").and_then(|x| x.as_str()) == Some("用户模块"))
            .expect("用户模块分组");
        let api_item = um["children"][0].clone();
        assert_eq!(api_item["api"]["method"], "GET");
        assert_eq!(api_item["api"]["path"], "/user/:id", "路径参数应为 :id 语法");
        assert_eq!(api_item["api"]["req_query"][0]["name"], "page");
        assert_eq!(api_item["api"]["res_body"], "{\"ok\":true}");
        // round-trip：导出 → 再导入
        let tmp = root.join("round.json");
        fs::write(&tmp, serde_json::to_string_pretty(&v).unwrap()).unwrap();
        let re = import_yapi_file(&root, &tmp).expect("yapi round-trip 失败");
        assert_eq!(re.count, 2, "round-trip 接口数应为 2");
        let folder = PathBuf::from(&re.folder);
        assert!(folder.join("用户模块").join("获取用户.json").exists());
        assert!(folder.join("订单列表.json").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_import_eolink_file() {
        let root = std::env::temp_dir().join(format!("apimgr-eolink-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let file = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/data/eolink.json"
        ));
        let result = import_eolink_file(&root, &file).expect("eolink 导入失败");
        assert_eq!(result.count, 1, "接口数应为 1，实际 {}", result.count);
        let folder = PathBuf::from(&result.folder);
        assert_eq!(folder.file_name().unwrap().to_string_lossy(), "订单管理服务");
        // 顶层组「订单模块」→ 子组「订单操作」→ 创建订单.json
        let om = folder.join("订单模块");
        assert!(om.is_dir(), "订单模块分组应存在");
        let op = om.join("订单操作");
        assert!(op.is_dir(), "订单操作分组应存在");
        let api: ApiFile = serde_json::from_str(
            &fs::read_to_string(op.join("创建订单.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(api.method, "POST");
        assert_eq!(api.path, "/order/{orderType}/create");
        // 路径参数 orderType（requestRestList 补了 example）
        assert!(api.params.iter().any(|p| p.key == "orderType" && p.value == "normal"));
        // 查询参数 channel
        assert!(api.query.iter().any(|q| q.key == "channel" && q.value == "app"));
        // 请求头 Authorization
        assert!(api.headers.iter().any(|h| h.key == "Authorization"));
        // json body 嵌套结构
        assert_eq!(api.body.mode, "json");
        assert!(api.body.raw.contains("userId"));
        assert!(api.body.raw.contains("receiverName"));
        // 描述合并 apiDesc + apiNote
        assert!(api.description.contains("批量下单"));
        assert!(api.description.contains("鉴权token"));
        // 2 个响应示例（200/400）
        assert_eq!(api.responses.len(), 2);
        assert!(api.responses.iter().any(|r| r.status == 200 && r.body.contains("orderId")));
        assert!(api.responses.iter().any(|r| r.status == 400));
        // INFO_FILE base_url 来自环境 host
        let info: InfoJson =
            serde_json::from_str(&fs::read_to_string(folder.join(INFO_FILE)).unwrap()).unwrap();
        assert!(info.base_url.as_deref().unwrap_or("").contains("api.local"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_import_insomnia_file() {
        let root = std::env::temp_dir().join(format!("apimgr-ins-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let file = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/data/Insomnia.yml"
        ));
        let result = import_insomnia_file(&root, &file).expect("insomnia 导入失败");
        assert_eq!(result.count, 1, "接口数应为 1，实际 {}", result.count);
        let folder = PathBuf::from(&result.folder);
        assert_eq!(folder.file_name().unwrap().to_string_lossy(), "Project API");
        let um = folder.join("用户模块");
        assert!(um.is_dir(), "用户模块分组应存在");
        let api: ApiFile = serde_json::from_str(
            &fs::read_to_string(um.join("创建用户 POST.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(api.method, "POST");
        // {{baseUrl}} 由集合级 environment 替换
        assert_eq!(api.path, "/user");
        assert_eq!(api.body.mode, "json");
        assert!(api.body.raw.contains("test"));
        // bearer token → Authorization 头
        assert!(api.headers.iter().any(|h| {
            h.key.eq_ignore_ascii_case("authorization") && h.value.contains("demo-token")
        }));
        // INFO_FILE base_url
        let info: InfoJson =
            serde_json::from_str(&fs::read_to_string(folder.join(INFO_FILE)).unwrap()).unwrap();
        assert_eq!(info.base_url.as_deref(), Some("https://api.example.com"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_export_eolink_insomnia_roundtrip() {
        let root = std::env::temp_dir().join(format!("apimgr-ei-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let make = |name: &str, method: &str, path: &str| ApiFile {
            uuid: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            method: method.into(),
            path: path.into(),
            url: path.into(),
            description: format!("{name} 描述"),
            headers: vec![KeyValue {
                key: "Authorization".into(),
                value: "Bearer tok123".into(),
                enabled: true,
                is_file: false,
                description: "鉴权".into(),
            }],
            query: vec![KeyValue {
                key: "page".into(),
                value: "1".into(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            params: vec![KeyValue {
                key: "orderType".into(),
                value: "normal".into(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            body: BodyData {
                mode: "json".into(),
                raw: "{\"userId\":1,\"addr\":{\"city\":\"赣州\"}}".into(),
                form: vec![],
                binary_path: String::new(),
            },
            mock: MockConfig::default(),
            examples: vec![],
            responses: vec![ResponseItem {
                id: "r1".into(),
                name: "HTTP 200".into(),
                status: 200,
                content_type: "application/json".into(),
                body: "{\"code\":0}".into(),
            }],
            doc_params: vec![],
            deprecated: false,
            protocol: "http".into(),
        };
        let apis: Vec<(Vec<(String, bool)>, ApiFile)> = vec![(
            vec![("订单模块".to_string(), true), ("订单操作".to_string(), true)],
            make("创建订单", "POST", "/order/{orderType}/create"),
        )];
        // Eolink 导出 → 再导入
        let ev = export::to_eolink(&apis);
        assert_eq!(ev["apiGroupList"][0]["groupName"], "订单模块");
        let eapi = &ev["apiGroupList"][0]["childGroupList"][0]["apiList"][0];
        assert_eq!(eapi["apiMethod"], "POST");
        assert_eq!(eapi["apiUri"], "/order/{orderType}/create");
        assert_eq!(eapi["requestInfo"]["requestRestList"][0]["key"], "orderType");
        assert_eq!(eapi["requestInfo"]["requestQueryList"][0]["key"], "page");
        // serde_json Map 键按字母序，用 find 断言
        let bl = eapi["requestInfo"]["requestBodyJsonList"]
            .as_array()
            .unwrap();
        assert!(bl.iter().any(|x| x["key"] == "addr"));
        let addr = bl.iter().find(|x| x["key"] == "addr").unwrap();
        assert_eq!(addr["children"][0]["key"], "city");
        assert!(bl.iter().any(|x| x["key"] == "userId"));
        assert_eq!(eapi["responseInfoList"][0]["responseCode"], 200);
        let etmp = root.join("eolink-out.json");
        fs::write(&etmp, serde_json::to_string_pretty(&ev).unwrap()).unwrap();
        let re = import_eolink_file(&root, &etmp).expect("eolink round-trip 失败");
        assert_eq!(re.count, 1, "eolink round-trip 接口数应为 1");
        // Insomnia 导出 → 再导入
        let iv = export::to_insomnia(&apis);
        assert_eq!(iv["type"], "collection.insomnia.rest/5.0");
        assert_eq!(iv["children"][0]["name"], "订单模块");
        let req = &iv["children"][0]["children"][0]["children"][0];
        assert_eq!(req["method"], "POST");
        assert!(req["url"].as_str().unwrap().contains("baseUrl"));
        assert_eq!(req["authentication"]["type"], "bearer");
        assert_eq!(req["authentication"]["token"], "tok123");
        assert_eq!(req["body"]["mimeType"], "application/json");
        let itmp = root.join("insomnia-out.yml");
        fs::write(&itmp, serde_yaml::to_string(&iv).unwrap()).unwrap();
        let ri = import_insomnia_file(&root, &itmp).expect("insomnia round-trip 失败");
        assert_eq!(ri.count, 1, "insomnia round-trip 接口数应为 1");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_import_jmeter_file() {
        let root = std::env::temp_dir().join(format!("apimgr-jmx-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let file = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/data/jmeter.jmx"
        ));
        let result = import_jmeter_file(&root, &file).expect("jmeter 导入失败");
        assert_eq!(result.count, 5, "接口数应为 5，实际 {}", result.count);
        let folder = PathBuf::from(&result.folder);
        assert_eq!(
            folder.file_name().unwrap().to_string_lossy(),
            "综合业务接口测试计划"
        );
        // ThreadGroup「业务线程组」→ 分组目录
        let tg = folder.join("业务线程组");
        assert!(tg.is_dir(), "业务线程组分组应存在");
        // 登录接口：POST + json body + 变量替换
        let login: ApiFile = serde_json::from_str(
            &fs::read_to_string(tg.join("1-登录获取token.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(login.method, "POST");
        assert_eq!(login.path, "/api/login");
        assert_eq!(login.body.mode, "json");
        assert!(login.body.raw.contains("username"));
        // HeaderManager 的 Content-Type 应用到接口
        assert!(login.headers.iter().any(|h| {
            h.key.eq_ignore_ascii_case("content-type") && h.value.contains("application/json")
        }));
        // GET 接口：path 中 query 拆出
        let info: ApiFile = serde_json::from_str(
            &fs::read_to_string(tg.join("2-获取用户信息.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(info.method, "GET");
        assert_eq!(info.path, "/api/user/info");
        assert!(info.query.iter().any(|q| q.key == "token"));
        // DELETE 接口
        let del: ApiFile = serde_json::from_str(
            &fs::read_to_string(tg.join("5-删除订单.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(del.method, "DELETE");
        assert_eq!(del.path, "/api/order/del");
        // INFO_FILE base_url 来自 host 变量
        let info_f: InfoJson =
            serde_json::from_str(&fs::read_to_string(folder.join(INFO_FILE)).unwrap()).unwrap();
        assert_eq!(info_f.base_url.as_deref(), Some("http://127.0.0.1:8080"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_export_jmeter_roundtrip() {
        let root = std::env::temp_dir().join(format!("apimgr-jmx-e-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let make = |name: &str, method: &str, path: &str| ApiFile {
            uuid: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            method: method.into(),
            path: path.into(),
            url: path.into(),
            description: String::new(),
            headers: vec![KeyValue {
                key: "X-Token".into(),
                value: "abc".into(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            query: vec![KeyValue {
                key: "page".into(),
                value: "2".into(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            params: vec![],
            body: BodyData {
                mode: "json".into(),
                raw: "{\"name\":\"测试\"}".into(),
                form: vec![],
                binary_path: String::new(),
            },
            mock: MockConfig::default(),
            examples: vec![],
            responses: vec![],
            doc_params: vec![],
            deprecated: false,
            protocol: "http".into(),
        };
        let apis: Vec<(Vec<(String, bool)>, ApiFile)> = vec![
            (vec![("用户模块".to_string(), true)], make("创建用户", "POST", "/user")),
            (vec![], make("订单列表", "GET", "/order/list")),
        ];
        let xml = export::to_jmeter(&apis);
        assert!(xml.contains("<jmeterTestPlan"));
        assert!(xml.contains("testname=\"用户模块\""));
        assert!(xml.contains("testname=\"API Manager\""));
        assert!(xml.contains("HTTPSampler.path\">/user"));
        assert!(xml.contains("HTTPSampler.method\">POST"));
        // query 拼进 path
        assert!(xml.contains("/order/list?page=2"));
        // HeaderManager 保留 X-Token
        assert!(xml.contains("Header.name\">X-Token"));
        // round-trip：导出 → 再导入
        let tmp = root.join("round.jmx");
        fs::write(&tmp, &xml).unwrap();
        let re = import_jmeter_file(&root, &tmp).expect("jmeter round-trip 失败");
        assert_eq!(re.count, 2, "jmeter round-trip 接口数应为 2");
        let folder = PathBuf::from(&re.folder);
        let created: ApiFile = serde_json::from_str(
            &fs::read_to_string(folder.join("用户模块").join("创建用户.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(created.body.mode, "json");
        assert!(created.body.raw.contains("测试"));
        assert!(created.headers.iter().any(|h| h.key == "X-Token"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_import_apidoc_files() {
        let root = std::env::temp_dir().join(format!("apimgr-apidoc-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let base = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/data"));
        let result = import_apidoc_files(&root, &base.join("api_project.json"), &base.join("api_data.json"))
            .expect("apidoc 导入失败");
        assert_eq!(result.count, 7, "接口数应为 7，实际 {}", result.count);
        let folder = PathBuf::from(&result.folder);
        assert_eq!(folder.file_name().unwrap().to_string_lossy(), "后端API接口文档");
        // INFO base_url = sampleUrl
        let info_f: InfoJson =
            serde_json::from_str(&fs::read_to_string(folder.join(INFO_FILE)).unwrap()).unwrap();
        assert_eq!(info_f.base_url.as_deref(), Some("http://127.0.0.1:8080/api"));
        // 分组：用户模块 / 订单模块
        let user_dir = folder.join("用户模块");
        assert!(user_dir.is_dir(), "用户模块分组应存在");
        // 登录接口：POST json body + 嵌套字段展开
        let login: ApiFile = serde_json::from_str(
            &fs::read_to_string(user_dir.join("用户登录.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(login.method, "POST");
        assert_eq!(login.path, "/api/user/login");
        assert_eq!(login.body.mode, "json");
        assert!(login.body.raw.contains("username"));
        assert!(login.body.raw.contains("password"));
        // 响应：successExamples → status 200，error.examples → 返回失败
        assert!(login.responses.iter().any(|r| r.status == 200 && r.name == "登录成功"));
        assert!(login.responses.iter().any(|r| r.status == 0 && r.name == "登录失败"));
        // docParams：body 字段 + resp_success 字段
        assert!(login.doc_params.iter().any(|d| d.source == "body" && d.key == "username"));
        assert!(login.doc_params.iter().any(|d| d.source == "resp_success" && d.key == "data.token"));
        // header 字段 → 请求头
        let info: ApiFile = serde_json::from_str(
            &fs::read_to_string(user_dir.join("获取当前登录用户信息.json")).unwrap(),
        )
        .unwrap();
        assert!(info.headers.iter().any(|h| h.key == "Authorization"));
        // 路径参数 :orderId → {orderId}
        let detail: ApiFile = serde_json::from_str(
            &fs::read_to_string(folder.join("订单模块").join("获取订单详情.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(detail.path, "/api/order/{orderId}");
        assert!(detail.params.iter().any(|p| p.key == "orderId"));
        // 创建订单：数组字段 goodsList 展开
        let create: ApiFile = serde_json::from_str(
            &fs::read_to_string(folder.join("订单模块").join("创建订单.json")).unwrap(),
        )
        .unwrap();
        assert!(create.body.raw.contains("goodsList"));
        assert!(create.body.raw.contains("goodsId"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_export_apidoc_roundtrip() {
        let root = std::env::temp_dir().join(format!("apimgr-apidoc-e-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let make = |name: &str, method: &str, path: &str| ApiFile {
            uuid: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            method: method.into(),
            path: path.into(),
            url: path.into(),
            description: String::new(),
            headers: vec![KeyValue {
                key: "Authorization".into(),
                value: String::new(),
                enabled: true,
                is_file: false,
                description: "Bearer token".into(),
            }],
            query: vec![KeyValue {
                key: "page".into(),
                value: "1".into(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            params: vec![KeyValue {
                key: "orderId".into(),
                value: String::new(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            body: BodyData {
                mode: "json".into(),
                raw: "{\"username\":\"zhangsan\",\"info\":{\"age\":18},\"tags\":[\"a\"]}".into(),
                form: vec![],
                binary_path: String::new(),
            },
            mock: MockConfig::default(),
            examples: vec![],
            responses: vec![ResponseItem {
                id: uuid::Uuid::new_v4().to_string(),
                name: "成功".into(),
                status: 200,
                content_type: "application/json".into(),
                body: "{\"code\":0}".into(),
            }],
            doc_params: vec![],
            deprecated: false,
            protocol: "http".into(),
        };
        let apis: Vec<(Vec<(String, bool)>, ApiFile)> = vec![
            (vec![("订单模块".to_string(), true)], make("订单详情", "GET", "/api/order/{orderId}")),
        ];
        let (proj, data) = export::to_apidoc(&apis);
        assert_eq!(proj["name"].as_str(), Some("订单模块"));
        let groups = data["groups"].as_array().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0]["name"].as_str(), Some("订单模块"));
        let apis_out = data["apis"].as_array().unwrap();
        assert_eq!(apis_out.len(), 1);
        let a = &apis_out[0];
        assert_eq!(a["group"].as_str(), Some("订单模块"));
        assert_eq!(a["url"].as_str(), Some("/api/order/:orderId"));
        // header 字段
        assert_eq!(
            a["header"]["fields"]["Header"][0]["field"].as_str(),
            Some("Authorization")
        );
        // query → Query 字段
        assert_eq!(a["parameter"]["fields"]["Query"][0]["field"].as_str(), Some("page"));
        // body 嵌套展开：username / info.age / tags[]
        let pf = a["parameter"]["fields"]["Parameter"].as_array().unwrap();
        let fields: Vec<&str> = pf.iter().map(|f| f["field"].as_str().unwrap()).collect();
        assert!(fields.contains(&"username"));
        assert!(fields.contains(&"info.age"));
        assert!(fields.contains(&"tags"));
        // successExamples
        assert_eq!(a["successExamples"][0]["content"].as_str(), Some("{\"code\":0}"));
        // round-trip：导出 → 写文件 → 再导入
        let dir = root.join("out");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("api_project.json"), serde_json::to_string_pretty(&proj).unwrap()).unwrap();
        fs::write(dir.join("api_data.json"), serde_json::to_string_pretty(&data).unwrap()).unwrap();
        let re = import_apidoc_files(&root, &dir.join("api_project.json"), &dir.join("api_data.json"))
            .expect("apidoc round-trip 失败");
        assert_eq!(re.count, 1, "apidoc round-trip 接口数应为 1");
        let folder = PathBuf::from(&re.folder);
        let created: ApiFile = serde_json::from_str(
            &fs::read_to_string(folder.join("订单模块").join("订单详情.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(created.path, "/api/order/{orderId}");
        assert_eq!(created.body.mode, "json");
        assert!(created.body.raw.contains("username"));
        assert!(created.body.raw.contains("info"));
        assert!(created.headers.iter().any(|h| h.key == "Authorization"));
        assert!(created.query.iter().any(|q| q.key == "page"));
        assert!(created.params.iter().any(|p| p.key == "orderId"));
        assert!(created.responses.iter().any(|r| r.status == 200));
        let _ = fs::remove_dir_all(&root);
    }

        #[test]
    fn test_history_roundtrip() {
        // 保存 -> 分页列表 -> 详情 -> 按天统计 全链路
        let root = std::env::temp_dir().join(format!("history-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let input = HistoryInput {
            method: "GET".into(),
            url: "http://127.0.0.1:8080/api/users".into(),
            api_uuid: "abc-123".into(),
            api_name: "用户列表".into(),
            req_headers: vec![("Content-Type".into(), "application/json".into())],
            req_body: Some("{\"a\":1}".into()),
            ok: true,
            status: 200,
            status_text: "OK".into(),
            resp_headers: vec![("X-Test".into(), "yes".into())],
            resp_body: "{\"hello\":\"world\"}".into(),
            time_ms: 12,
            size: 100,
            error: None,
        };
        let id = save_history_to(&root, input).unwrap();
        assert!(root.join(HISTORY_DIR).exists());

        // 列表分页
        let page = history_records_from(&root, 0, 100).unwrap();
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].id, id);
        assert_eq!(page[0].status, 200);
        assert_eq!(page[0].api_uuid, "abc-123");
        // offset 越界返回空
        assert!(history_records_from(&root, 5, 100).unwrap().is_empty());

        // 详情
        let detail = history_detail_from(&root, &id).unwrap();
        assert_eq!(detail.req_headers[0].0, "Content-Type");
        assert_eq!(detail.req_body.as_deref(), Some("{\"a\":1}"));
        assert_eq!(detail.resp_body, "{\"hello\":\"world\"}");
        // 不存在的 id
        assert!(history_detail_from(&root, "nope").is_err());

        // 按天统计
        let days = history_days_from(&root).unwrap();
        assert_eq!(days.len(), 1);
        assert_eq!(days[0].count, 1);

        // 清空
        fs::remove_dir_all(root.join(HISTORY_DIR)).unwrap();
        assert!(history_records_from(&root, 0, 100).unwrap().is_empty());
        assert!(history_days_from(&root).unwrap().is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_example_roundtrip() {
        // 保存（同名覆盖）-> 列表 -> 读取 -> 删除 全链路
        let root = std::env::temp_dir().join(format!("example-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let make = |name: &str, url: &str| ExampleFile {
            name: name.into(),
            time: 1700000000,
            method: "GET".into(),
            url: url.into(),
            req_headers: vec![("Accept".into(), "*/*".into())],
            req_path: vec![("id".into(), "42".into())],
            req_query: vec![("page".into(), "1".into())],
            req_body: None,
            status: 200,
            status_text: "OK".into(),
            resp_headers: vec![("X-Test".into(), "yes".into())],
            resp_body: "{\"ok\":true}".into(),
            time_ms: 8,
            size: 64,
            error: None,
        };

        // 名称哈希稳定：同名两次保存得到相同文件名（覆盖）
        let f1 = save_example_to(&root, "uuid-1", "登录成功", make("登录成功", "http://a/b")).unwrap();
        let f2 = save_example_to(&root, "uuid-1", "登录成功", make("登录成功", "http://a/b?x=2")).unwrap();
        assert_eq!(f1, f2);
        // 不同名 -> 不同文件
        let f3 = save_example_to(&root, "uuid-1", "查询列表", make("查询列表", "http://a/c")).unwrap();
        assert_ne!(f1, f3);
        // 不同接口 -> 不同目录
        let f4 = save_example_to(&root, "uuid-2", "登录成功", make("登录成功", "http://a/b")).unwrap();
        assert_eq!(f1, f4);

        let list = list_examples_from(&root, "uuid-1").unwrap();
        assert_eq!(list.len(), 2);
        // 最新在前
        assert_eq!(list[0].name, "查询列表");
        assert!(list.iter().all(|s| s.file.ends_with(".json")));

        // 读取详情
        let detail = read_example_file(&root, "uuid-1", &f3).unwrap();
        assert_eq!(detail.url, "http://a/c");
        assert_eq!(detail.resp_body, "{\"ok\":true}");
        assert_eq!(detail.req_path[0], ("id".to_string(), "42".to_string()));
        assert_eq!(detail.req_query[0], ("page".to_string(), "1".to_string()));

        // 空 uuid / 空名称报错
        assert!(save_example_to(&root, "", "x", make("x", "")).is_err());
        assert!(save_example_to(&root, "uuid-1", "   ", make("x", "")).is_err());

        // 防目录穿越
        assert!(example_path(&root, "uuid-1", "../evil.json").is_err());

        // 删除后列表为空
        fs::remove_file(root.join(EXAMPLES_DIR).join("uuid-1").join(&f3)).unwrap();
        assert_eq!(list_examples_from(&root, "uuid-1").unwrap().len(), 1);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_move_entry() {
        // 移动接口文件与目录到目标目录；重名时自动加序号；禁止移入自身子目录
        let root = std::env::temp_dir().join(format!("move-test-{}", std::process::id()));
        let a = root.join("a");
        let b = root.join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        fs::create_dir_all(&a.join("sub")).unwrap();
        fs::write(a.join("api.json"), "{}").unwrap();
        fs::write(a.join("sub").join("deep.json"), "{}").unwrap();

        // 接口移入 b
        let new_path = move_entry_inner(
            &root,
            &a.join("api.json").to_string_lossy(),
            &b.to_string_lossy(),
        )
        .unwrap();
        assert_eq!(new_path, b.join("api.json").to_string_lossy());
        assert!(b.join("api.json").exists());

        // 目录 sub 移入 b（含内部文件）
        let new_path = move_entry_inner(&root, &a.join("sub").to_string_lossy(), &b.to_string_lossy())
            .unwrap();
        assert!(b.join("sub").join("deep.json").exists());
        assert_eq!(new_path, b.join("sub").to_string_lossy());

        // 目录不能移入自身子目录
        let err = move_entry_inner(&root, &b.to_string_lossy(), &b.join("sub").to_string_lossy())
            .unwrap_err();
        assert!(err.contains("子目录"));

        let _ = fs::remove_dir_all(&root);
    }

    /// 复制接口：uuid 重新生成、名称追加「 副本」、同目录重名自动加序号
    #[test]
    fn copy_api_regenerates_uuid() {
        let root = std::env::temp_dir().join(format!("apim-copy-api-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let src = root.join("创建用户.json");
        let api = ApiFile {
            uuid: "old-uuid".into(),
            name: "创建用户".into(),
            method: "POST".into(),
            path: "/api/users".into(),
            url: String::new(),
            description: String::new(),
            headers: vec![],
            query: vec![],
            params: vec![],
            body: BodyData::default(),
            mock: MockConfig::default(),
            examples: vec![],
            responses: vec![],
            doc_params: vec![],
        deprecated: false,
        protocol: "http".into(),
        };
        write_pretty(&src, &api).unwrap();

        let dst = root.join("创建用户 副本.json");
        copy_api_file(&src, &dst).unwrap();
        let copied: ApiFile = serde_json::from_str(&fs::read_to_string(&dst).unwrap()).unwrap();
        assert_ne!(copied.uuid, "old-uuid");
        assert_eq!(copied.name, "创建用户 副本");
        assert_eq!(copied.method, "POST");
        assert_eq!(copied.path, "/api/users");
        let _ = fs::remove_dir_all(&root);
    }

    /// 复制分组：递归复制整棵树，每个接口 uuid 重新生成，分组 __info.json 名称追加「 副本」
    #[test]
    fn copy_dir_regenerates_all_uuids() {
        let root = std::env::temp_dir().join(format!("apim-copy-dir-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let g = root.join("用户管理");
        let sub = g.join("子分组");
        fs::create_dir_all(&sub).unwrap();
        fs::write(
            g.join(INFO_FILE),
            r#"{"name":"用户管理","description":""}"#,
        )
        .unwrap();
        let mk = |p: &std::path::Path, uuid: &str, name: &str| {
            let api = ApiFile {
                uuid: uuid.into(),
                name: name.into(),
                method: "GET".into(),
                path: "/x".into(),
                url: String::new(),
                description: String::new(),
                headers: vec![],
                query: vec![],
                params: vec![],
                body: BodyData::default(),
                mock: MockConfig::default(),
                examples: vec![],
                responses: vec![],
                doc_params: vec![],
        deprecated: false,
        protocol: "http".into(),
            };
            write_pretty(p, &api).unwrap();
        };
        mk(&g.join("接口A.json"), "uuid-a", "接口A");
        mk(&sub.join("接口B.json"), "uuid-b", "接口B");
        // 点目录不应被复制（.examples 与旧 uuid 绑定）
        fs::create_dir_all(g.join(crate::EXAMPLES_DATA_DIR)).unwrap();
        fs::write(g.join(crate::EXAMPLES_DATA_DIR).join("x.json"), "{}").unwrap();

        let dst = root.join("用户管理 副本");
        copy_dir_with_new_uuids(&g, &dst).unwrap();

        let a: ApiFile = serde_json::from_str(&fs::read_to_string(dst.join("接口A.json")).unwrap()).unwrap();
        assert_ne!(a.uuid, "uuid-a");
        assert_eq!(a.name, "接口A 副本");
        let b: ApiFile = serde_json::from_str(&fs::read_to_string(dst.join("子分组").join("接口B.json")).unwrap()).unwrap();
        assert_ne!(b.uuid, "uuid-b");
        assert!(!dst.join(crate::EXAMPLES_DATA_DIR).exists());
        let info: Value = serde_json::from_str(&fs::read_to_string(dst.join(INFO_FILE)).unwrap()).unwrap();
        assert_eq!(info["name"], "用户管理 副本");
        let _ = fs::remove_dir_all(&root);
    }



    #[test]
    fn test_import_extra_formats() {
        let root = std::env::temp_dir().join(format!("apimgr-extra-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let base = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/data"));
        // (格式, 文件名, 期望接口数, 关键断言闭包)
        let cases: Vec<(&str, &str, usize)> = vec![
            ("apidog", "demo.apidog.json", 2),
            ("bruno", "bruno.json", 3),
            ("apizza", "apizza.json", 4),
            ("nei", "nei.json", 2),
            ("doclever", "DOClever.json", 2),
            ("io-docs", "io-docs.json", 8),
            ("easydoc", "easydoc.json", 3),
            ("docway", "docway.mjson", 3),
            ("hoppscotch", "Hoppscotch.json", 6),
            ("metersphere", "MeterSphere.json", 2),
        ];
        for (format, fname, expected) in cases {
            let sub = root.join(format);
            fs::create_dir_all(&sub).unwrap();
            let result = import_extra_files(&sub, &base.join(fname), format)
                .unwrap_or_else(|e| panic!("{format} 导入失败: {e}"));
            assert_eq!(result.count, expected, "{format} 接口数应为 {expected}，实际 {}", result.count);
            let folder = PathBuf::from(&result.folder);
            assert!(folder.join(INFO_FILE).is_file(), "{format} INFO 文件应存在");
            // 至少有一个接口文件
            let mut found = 0usize;
            fn walk_count(dir: &Path, found: &mut usize) {
                if let Ok(rd) = fs::read_dir(dir) {
                    for e in rd.flatten() {
                        let p = e.path();
                        if p.is_dir() {
                            walk_count(&p, found);
                        } else if p.extension().map(|x| x == "json").unwrap_or(false) && p.file_name().map(|n| n != INFO_FILE).unwrap_or(false) {
                            *found += 1;
                        }
                    }
                }
            }
            walk_count(&folder, &mut found);
            assert_eq!(found, expected, "{format} 磁盘接口文件数应为 {expected}");
            // 抽查读取第一个接口文件可解析
            if let Some(first) = std::fs::read_dir(&folder).unwrap().flatten().find(|e| {
                let p = e.path();
                p.is_file() && p.extension().map(|x| x == "json").unwrap_or(false)
                    && p.file_name().map(|n| n != INFO_FILE).unwrap_or(false)
            }) {
                let _: ApiFile = serde_json::from_str(&fs::read_to_string(first.path()).unwrap())
                    .unwrap_or_else(|er| panic!("{format} 接口文件解析失败: {er}"));
            }
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_export_extra_roundtrip() {
        let root = std::env::temp_dir().join(format!("apimgr-extra-e-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let mk = |name: &str, method: &str, path: &str, body_mode: &str| ApiFile {
            uuid: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            method: method.into(),
            path: path.into(),
            url: path.into(),
            description: "测试接口".into(),
            headers: vec![KeyValue {
                key: "Authorization".into(),
                value: "Bearer token".into(),
                enabled: true,
                is_file: false,
                description: "鉴权".into(),
            }],
            query: vec![KeyValue {
                key: "page".into(),
                value: "1".into(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            params: vec![KeyValue {
                key: "orderId".into(),
                value: String::new(),
                enabled: true,
                is_file: false,
                description: String::new(),
            }],
            body: if body_mode == "json" {
                BodyData {
                    mode: "json".into(),
                    raw: "{\"username\":\"zhangsan\",\"age\":18}".into(),
                    form: vec![],
                    binary_path: String::new(),
                }
            } else {
                BodyData {
                    mode: "form".into(),
                    raw: String::new(),
                    form: vec![KeyValue {
                        key: "file".into(),
                        value: "a.txt".into(),
                        enabled: true,
                        is_file: true,
                        description: String::new(),
                    }],
                    binary_path: String::new(),
                }
            },
            mock: MockConfig::default(),
            examples: vec![],
            responses: vec![ResponseItem {
                id: uuid::Uuid::new_v4().to_string(),
                name: "成功".into(),
                status: 200,
                content_type: "application/json".into(),
                body: "{\"code\":0}".into(),
            }],
            doc_params: vec![],
            deprecated: false,
            protocol: "http".into(),
        };
        let apis: Vec<(Vec<(String, bool)>, ApiFile)> = vec![
            (vec![("用户模块".to_string(), true)], mk("用户登录", "POST", "/api/user/login", "json")),
            (vec![("订单模块".to_string(), true)], mk("上传文件", "POST", "/api/order/upload", "form")),
        ];
        let formats = [
            "apidog", "bruno", "apizza", "nei", "doclever", "io-docs", "easydoc", "docway", "hoppscotch", "metersphere",
        ];
        for format in formats {
            let (content, fname, ext) = export::export_extra(&apis, format)
                .unwrap_or_else(|e| panic!("{format} 导出失败: {e}"));
            assert!(!content.is_empty(), "{format} 导出内容不应为空");
            let out_dir = root.join(format);
            fs::create_dir_all(&out_dir).unwrap();
            let out_file = out_dir.join(format!("{fname}.{ext}"));
            fs::write(&out_file, &content).unwrap();
            let re = import_extra_files(&root, &out_file, format)
                .unwrap_or_else(|e| panic!("{format} round-trip 导入失败: {e}"));
            assert_eq!(re.count, 2, "{format} round-trip 接口数应为 2，实际 {}", re.count);
            let folder = PathBuf::from(&re.folder);
            let mut created: Vec<ApiFile> = Vec::new();
            fn walk_read(dir: &Path, out: &mut Vec<ApiFile>) {
                if let Ok(rd) = fs::read_dir(dir) {
                    for e in rd.flatten() {
                        let p = e.path();
                        if p.is_dir() {
                            walk_read(&p, out);
                        } else if p.extension().map(|x| x == "json").unwrap_or(false) && p.file_name().map(|n| n != INFO_FILE).unwrap_or(false) {
                            if let Ok(v) = serde_json::from_str::<ApiFile>(&fs::read_to_string(&p).unwrap()) {
                                out.push(v);
                            }
                        }
                    }
                }
            }
            walk_read(&folder, &mut created);
            assert_eq!(created.len(), 2, "{format} round-trip 磁盘接口数应为 2");
            let login = created.iter().find(|a| a.path.contains("/api/user/login")).expect(&format!("{format} 应含登录接口"));
            assert_eq!(login.method, "POST", "{format} 登录接口 method");
            assert!(login.headers.iter().any(|h| h.key == "Authorization"), "{format} 登录接口 header 保留");
            if format != "io-docs" && format != "docway" {
                // io-docs/docway 无 query/body 区分，参数全部归入 body
                assert!(login.query.iter().any(|q| q.key == "page"), "{format} 登录接口 query 保留");
            }
            let upload = created.iter().find(|a| a.path.contains("/api/order/upload")).expect(&format!("{format} 应含上传接口"));
            if format != "io-docs" && format != "docway" && format != "metersphere" {
                assert_eq!(upload.body.mode, "form", "{format} 上传接口 body mode");
                assert!(!upload.body.form.is_empty(), "{format} 上传接口 form 字段保留");
            }
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_import_rap2() {
        let root = std::env::temp_dir().join(format!("apimgr-rap2-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let base = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/data"));
        // 项目格式
        let r = import_rap2_files(&root, &base.join("rap2-project.json")).expect("rap2 项目导入失败");
        assert_eq!(r.count, 6, "项目格式接口数应为 6，实际 {}", r.count);
        let folder = PathBuf::from(&r.folder);
        // 三个分组目录
        for mod_name in ["用户管理", "商品管理", "订单管理"] {
            assert!(folder.join(sanitize_filename(mod_name)).is_dir(), "缺少分组 {mod_name}");
        }
        // 用户管理分组下：获取用户列表 GET /api/user/list，响应含 code/msg/data
        let ulist = folder.join(sanitize_filename("用户管理")).join(sanitize_filename("获取用户列表.json"));
        let api: ApiFile = serde_json::from_str(&fs::read_to_string(&ulist).unwrap()).unwrap();
        assert_eq!(api.method, "GET");
        assert_eq!(api.path, "/api/user/list");
        assert!(api.headers.iter().any(|h| h.key == "Authorization"), "Authorization 应识别为 header");
        assert!(api.query.iter().any(|q| q.key == "page"), "page 应为 query");
        assert!(api.responses.iter().any(|r| r.body.contains("\"code\"") && r.body.contains("\"data\"")), "响应示例应含 code/data");
        // DELETE 接口 path 参数
        let del = folder.join(sanitize_filename("用户管理")).join(sanitize_filename("删除用户.json"));
        let api: ApiFile = serde_json::from_str(&fs::read_to_string(&del).unwrap()).unwrap();
        assert_eq!(api.method, "DELETE");
        assert!(api.path.contains("{userId}"), "删除用户 path 应保留 {{userId}}，实际 {}", api.path);
        assert!(api.params.iter().any(|p| p.key == "userId"), "userId 应为 path 参数");
        // 订单管理 POST /api/order body json
        let ord = folder.join(sanitize_filename("订单管理")).join(sanitize_filename("创建订单.json"));
        let api: ApiFile = serde_json::from_str(&fs::read_to_string(&ord).unwrap()).unwrap();
        assert_eq!(api.method, "POST");
        assert_eq!(api.path, "/api/order");
        assert_eq!(api.body.mode, "json", "订单接口应有 json body");
        assert!(api.body.raw.contains("receiverInfo") && api.body.raw.contains("goodsItems"), "body 应含嵌套 receiverInfo/goodsItems");
        // 单接口格式
        let r2 = import_rap2_files(&root, &base.join("rap2-single.json")).expect("rap2 单接口导入失败");
        assert_eq!(r2.count, 1, "单接口格式接口数应为 1");
        let folder2 = PathBuf::from(&r2.folder);
        let single = fs::read_dir(&folder2).unwrap().flatten()
            .find(|e| e.path().extension().map(|x| x == "json").unwrap_or(false) && e.file_name() != INFO_FILE)
            .unwrap();
        let api: ApiFile = serde_json::from_str(&fs::read_to_string(single.path()).unwrap()).unwrap();
        assert_eq!(api.method, "PUT");
        assert!(api.path.contains("{orderId}"), "单接口 path 应含 {{orderId}}，实际 {}", api.path);
        assert!(api.params.iter().any(|p| p.key == "orderId"), "orderId 应为 path 参数");
        assert_eq!(api.body.mode, "json", "单接口应有 json body（receiver/goodsList）");
        assert!(api.body.raw.contains("receiverName"), "body 应含 receiverName 嵌套");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_export_rap2_roundtrip() {
        let root = std::env::temp_dir().join(format!("apimgr-rap2-e-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let mk = |name: &str, method: &str, path: &str| ApiFile {
            uuid: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            method: method.into(),
            path: path.into(),
            url: path.into(),
            description: "测试".into(),
            headers: vec![KeyValue { key: "Authorization".into(), value: "Bearer x".into(), enabled: true, is_file: false, description: String::new() }],
            query: vec![KeyValue { key: "page".into(), value: "1".into(), enabled: true, is_file: false, description: String::new() }],
            params: vec![KeyValue { key: "orderId".into(), value: String::new(), enabled: true, is_file: false, description: String::new() }],
            body: BodyData {
                mode: "json".into(),
                raw: "{\"username\":\"zhangsan\",\"info\":{\"age\":18}}".into(),
                form: vec![],
                binary_path: String::new(),
            },
            mock: MockConfig::default(),
            examples: vec![],
            responses: vec![ResponseItem {
                id: uuid::Uuid::new_v4().to_string(),
                name: "成功".into(),
                status: 200,
                content_type: "application/json".into(),
                body: "{\"code\":0,\"data\":{\"total\":5}}".into(),
            }],
            doc_params: vec![],
            deprecated: false,
            protocol: "http".into(),
        };
        let apis: Vec<(Vec<(String, bool)>, ApiFile)> = vec![
            (vec![("用户模块".to_string(), true)], mk("登录", "POST", "/api/login")),
            (vec![("订单模块".to_string(), true)], mk("删除订单", "DELETE", "/api/order/{orderId}")),
        ];
        // 项目格式闭环
        let proj = export::to_rap2_project(&apis);
        let file = root.join("rap2-project.json");
        fs::write(&file, serde_json::to_string_pretty(&proj).unwrap()).unwrap();
        let re = import_rap2_files(&root, &file).expect("rap2 项目 round-trip 导入失败");
        assert_eq!(re.count, 2);
        let folder = PathBuf::from(&re.folder);
        let mut apis2: Vec<ApiFile> = Vec::new();
        for dir in fs::read_dir(&folder).unwrap().flatten() {
            if dir.path().is_dir() {
                for f in fs::read_dir(dir.path()).unwrap().flatten() {
                    if f.path().extension().map(|x| x == "json").unwrap_or(false) && f.file_name() != INFO_FILE {
                        apis2.push(serde_json::from_str(&fs::read_to_string(f.path()).unwrap()).unwrap());
                    }
                }
            }
        }
        assert_eq!(apis2.len(), 2);
        let login = apis2.iter().find(|a| a.path == "/api/login").unwrap();
        assert_eq!(login.method, "POST");
        assert!(login.headers.iter().any(|h| h.key == "Authorization"), "round-trip header 保留");
        assert!(login.query.iter().any(|q| q.key == "page"), "round-trip query 保留");
        assert!(login.body.raw.contains("info"), "round-trip 嵌套 body 保留");
        assert!(login.responses.iter().any(|r| r.body.contains("total")), "round-trip 响应保留");
        let del = apis2.iter().find(|a| a.path.contains("/api/order/")).unwrap();
        assert!(del.params.iter().any(|p| p.key == "orderId"), "round-trip path 参数保留");
        let _ = fs::remove_dir_all(&root);
    }

    /// demo 生成的对象示例链路：save_objects_impl 写出 .object/ 目录（分组 + 对象），
    /// save_custom_mock_impl 写出 .mock/zodiac.js 占位符（create_demo 的后半段）
    #[test]
    fn test_demo_creates_objects_and_mock() {
        let root = std::env::temp_dir().join(format!("apim-demo-obj-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let now = chrono::Local::now().timestamp();
        let prop = |key: &str, kind: &str, item_kind: &str, description: &str, mock: &str| ObjectProp {
            key: key.into(),
            kind: kind.into(),
            item_kind: item_kind.into(),
            ref_hash: String::new(),
            description: description.into(),
            mock: mock.into(),
        };
        let obj_def = |name: &str, object_name: &str, group: &str, description: &str, properties: Vec<ObjectProp>| ObjectDef {
            uuid: uuid::Uuid::new_v4().to_string(),
            hash: String::new(),
            name: name.into(),
            object_name: object_name.into(),
            package_name: String::new(),
            group: group.into(),
            deprecated: false,
            description: description.into(),
            properties,
            created_at: now,
            updated_at: now,
        };
        let store = ObjectStore {
            groups: vec![
                ObjectGroup { id: "用户管理".into(), name: "用户管理".into(), deprecated: false },
                ObjectGroup { id: "订单管理".into(), name: "订单管理".into(), deprecated: false },
            ],
            objects: vec![
                obj_def("用户", "User", "用户管理", "系统用户信息", vec![
                    prop("id", "Integer", "Integer", "主键", ""),
                    prop("name", "String", "String", "用户名", "@cname"),
                    prop("zodiac", "String", "String", "星座", "@zodiac"),
                ]),
                obj_def("订单", "Order", "订单管理", "用户订单", vec![
                    prop("id", "Integer", "Integer", "订单ID", ""),
                    prop("no", "String", "String", "订单编号", "SO2024"),
                    prop("amount", "Float", "Float", "订单金额", "99.5"),
                ]),
            ],
        };
        crate::objects::save_objects_impl(&root, &store).unwrap();
        // 分组信息 + 分组目录 + 对象文件（含中文分组路径）
        assert!(root.join(".api-manager/object/__info_obj.json").exists(), "分组信息文件存在");
        assert!(root.join(".api-manager/object/用户管理/用户.obj.json").exists(), "用户对象文件存在");
        assert!(root.join(".api-manager/object/订单管理/订单.obj.json").exists(), "订单对象文件存在");
        // 对象文件内容：hash 被重算、zodiac mock 示例保留
        let obj: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(root.join(".api-manager/object/用户管理/用户.obj.json")).unwrap()).unwrap();
        assert!(!obj["hash"].as_str().unwrap().is_empty(), "hash 重算非空");
        let props = obj["properties"].as_array().unwrap();
        assert!(props.iter().any(|p| p["key"] == "zodiac" && p["mock"] == "@zodiac"), "zodiac 字段保留");
        // 星座占位符
        crate::mock::save_custom_mock_impl(
            &root,
            &crate::mock::CustomMock {
                name: "zodiac".into(),
                enabled: true,
                desc: "十二星座之一".into(),
                code: "(ctx) => ctx.pick([\"白羊座\",\"金牛座\"])".into(),
            },
            None,
        )
        .unwrap();
        assert!(root.join(".api-manager/mock/zodiac.js").exists(), "星座占位符文件存在");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_migrate_legacy_data_dirs() {
        let root = std::env::temp_dir().join(format!("apimgr-migrate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        // 模拟存量项目：旧目录散落在根目录
        let legacy: [&str; 6] = [".version", ".object", ".mock", ".history", ".gen_log", ".examples"];
        for d in legacy {
            let dir = root.join(d);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("keep.txt"), "x").unwrap();
        }
        crate::migrate_legacy_data_dirs(&root).unwrap();
        // 旧目录应被移走，新目录 .api-manager/ 下内容完整
        for d in legacy {
            assert!(!root.join(d).exists(), "旧目录 {} 应被迁移", d);
        }
        for (d, sub) in [
            (".version", "version"),
            (".object", "object"),
            (".mock", "mock"),
            (".history", "history"),
            (".gen_log", "gen_log"),
            (".examples", "examples"),
        ] {
            let f = root.join(".api-manager").join(sub).join("keep.txt");
            assert!(f.exists(), ".api-manager/{sub}/keep.txt 应存在");
        }
        // 幂等：.api-manager 已存在时再次调用不报错、不重复迁移
        crate::migrate_legacy_data_dirs(&root).unwrap();
        assert!(root.join(".api-manager/version/keep.txt").exists());
        let _ = std::fs::remove_dir_all(&root);
    }
