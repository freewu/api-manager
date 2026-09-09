//! 演示案例：create_demo 命令在空工作区生成示例分组 + 接口 + 环境变量。

use super::*;

/// 在空工作区中生成演示案例（示例分组 + 接口 + 环境变量）
#[tauri::command]
pub(crate) fn create_demo(
    state: State<'_, WorkspaceState>,
    types: Option<Vec<String>>,
) -> Result<(), String> {
    let root = workspace_root(&state)?;
    // 勾选类型（http / websocket / socketio / graphql / webdav / object），未传时默认全部生成
    let has = |kind: &str| types.as_ref().map_or(true, |list| list.iter().any(|s| s == kind));
    // 不判断工作区是否为空：演示案例直接生成（同名文件会被覆盖）
    let api_file = |name: &str, method: &str, path: &str, description: &str| {
        serde_json::json!({
            "uuid": uuid::Uuid::new_v4().to_string(),
            "name": name,
            "method": method,
            "path": path,
            "url": "",
            "description": description,
            "headers": [],
            "query": [],
            "params": [],
            "body": { "mode": "none", "raw": "", "form": [] },
            "mock": { "enabled": false, "status": 200, "headers": [], "delay": 0, "body": "" },
            "examples": []
        })
    };
    let write = |dir: &str, file: &str, value: &serde_json::Value| -> Result<(), String> {
        let dir_path = if dir.is_empty() {
            root.clone()
        } else {
            root.join(dir)
        };
        fs::create_dir_all(&dir_path).map_err(|e| format!("创建目录失败: {e}"))?;
        write_pretty(&dir_path.join(file), value)
    };

    // docParams 快捷构造：位置 + 字段名 + 类型 + 说明（children 可嵌套下级字段）
    let d = |source: &str, key: &str, ty: &str, desc: &str, children: Vec<serde_json::Value>| -> serde_json::Value {
        serde_json::json!({
            "source": source, "key": key, "type": ty, "description": desc,
            "itemType": "", "objectName": key, "children": children
        })
    };

    // 根信息 + 环境变量
    write("", INFO_FILE, &serde_json::json!({
        "name": "演示 API 集合",
        "description": "这是一个示例工作区，展示了 API Manager 的目录组织方式",
        "baseUrl": "{{baseUrl}}",
        "mockPort": 5050
    }))?;
    write("", ENV_FILE, &serde_json::json!({
        "active": "开发环境",
        "environments": [
            {
                "name": "开发环境",
                "variables": [
                    { "key": "baseUrl", "value": "http://127.0.0.1:5050", "defaultValue": "https://api.example.com", "description": "接口服务地址", "enabled": true },
                    { "key": "token", "value": "dev-token-123456", "defaultValue": "demo-token", "description": "鉴权令牌", "enabled": true }
                ]
            },
            {
                "name": "生产环境",
                "variables": [
                    { "key": "baseUrl", "value": "https://api.example.com", "defaultValue": "https://api.example.com", "description": "接口服务地址", "enabled": true },
                    { "key": "token", "value": "prod-token-abcdef", "defaultValue": "demo-token", "description": "鉴权令牌", "enabled": true }
                ]
            }
        ]
    }))?;

    if has("http") {
    // 用户管理分组
    write("用户管理", INFO_FILE, &serde_json::json!({ "name": "用户管理", "description": "用户相关接口" }))?;
    let mut create_user = api_file("创建用户", "POST", "/api/users", "创建一个新用户");
    create_user["headers"] = serde_json::json!([{ "key": "Content-Type", "value": "application/json", "enabled": true, "description": "" }]);
    create_user["body"] = serde_json::json!({ "mode": "json", "raw": "{\n  \"name\": \"张三\",\n  \"email\": \"zhangsan@example.com\",\n  \"role\": \"user\"\n}", "form": [] });
    create_user["mock"] = serde_json::json!({ "enabled": true, "status": 201, "headers": [], "delay": 0, "body": "{\n  \"code\": 0,\n  \"data\": {\n    \"id\": 1001,\n    \"name\": \"张三\",\n    \"email\": \"zhangsan@example.com\"\n  },\n  \"message\": \"创建成功\"\n}" });
    create_user["docParams"] = serde_json::json!([
        d("body", "name", "String", "用户名", vec![]),
        d("body", "email", "String", "邮箱地址", vec![]),
        d("body", "role", "String", "用户角色（user / admin / vip）", vec![]),
        d("resp_success", "code", "Integer", "状态码，0 表示成功", vec![]),
        d("resp_success", "data", "Object", "创建成功的用户数据", vec![
            d("resp_success", "id", "Integer", "用户ID", vec![]),
            d("resp_success", "name", "String", "用户名", vec![]),
            d("resp_success", "email", "String", "邮箱地址", vec![]),
        ]),
        d("resp_success", "message", "String", "提示信息", vec![]),
        d("resp_fail", "code", "Integer", "错误码，非 0 表示失败", vec![]),
        d("resp_fail", "message", "String", "错误描述", vec![]),
        d("resp_fail", "errors", "Object", "字段校验错误明细", vec![
            d("resp_fail", "field", "String", "出错的字段名", vec![]),
            d("resp_fail", "reason", "String", "出错原因", vec![]),
        ]),
    ]);
    write("用户管理", "创建用户.json", &create_user)?;

    let mut get_user = api_file("获取用户信息", "GET", "/api/users/{id}", "查询单个用户信息");
    get_user["params"] = serde_json::json!([{ "key": "id", "value": "1", "enabled": true, "description": "用户ID" }]);
    get_user["mock"] = serde_json::json!({ "enabled": true, "status": 200, "headers": [], "delay": 0, "body": "{\n  \"code\": 0,\n  \"data\": {\n    \"id\": 1,\n    \"name\": \"张三\",\n    \"email\": \"zhangsan@example.com\"\n  },\n  \"message\": \"成功\"\n}" });
    get_user["docParams"] = serde_json::json!([
        d("path", "id", "Integer", "用户ID", vec![]),
        d("resp_success", "code", "Integer", "状态码，0 表示成功", vec![]),
        d("resp_success", "data", "Object", "用户信息", vec![
            d("resp_success", "id", "Integer", "用户ID", vec![]),
            d("resp_success", "name", "String", "用户名", vec![]),
            d("resp_success", "email", "String", "邮箱地址", vec![]),
        ]),
        d("resp_success", "message", "String", "提示信息", vec![]),
        d("resp_fail", "code", "Integer", "错误码（404 表示用户不存在）", vec![]),
        d("resp_fail", "message", "String", "错误描述", vec![]),
    ]);
    write("用户管理", "获取用户信息.json", &get_user)?;

    let mut del_user = api_file("删除用户", "DELETE", "/api/users/{id}", "删除指定用户");
    del_user["params"] = serde_json::json!([{ "key": "id", "value": "1", "enabled": true, "description": "用户ID" }]);
    del_user["mock"] = serde_json::json!({ "enabled": true, "status": 200, "headers": [], "delay": 0, "body": "{\n  \"code\": 0,\n  \"message\": \"删除成功\"\n}" });
    del_user["docParams"] = serde_json::json!([
        d("path", "id", "Integer", "用户ID", vec![]),
        d("resp_success", "code", "Integer", "状态码，0 表示成功", vec![]),
        d("resp_success", "message", "String", "提示信息", vec![]),
        d("resp_fail", "code", "Integer", "错误码（404 表示用户不存在）", vec![]),
        d("resp_fail", "message", "String", "错误描述", vec![]),
    ]);
    write("用户管理", "删除用户.json", &del_user)?;

    let mut update_user = api_file("更新用户", "PUT", "/api/users/{id}", "全量更新用户信息");
    update_user["params"] = serde_json::json!([{ "key": "id", "value": "1", "enabled": true, "description": "用户ID" }]);
    update_user["headers"] = serde_json::json!([{ "key": "Content-Type", "value": "application/json", "enabled": true, "description": "" }]);
    update_user["body"] = serde_json::json!({ "mode": "json", "raw": "{\n  \"name\": \"张三\",\n  \"email\": \"zhangsan@example.com\",\n  \"role\": \"admin\"\n}", "form": [] });
    update_user["mock"] = serde_json::json!({ "enabled": true, "status": 200, "headers": [], "delay": 0, "body": "{\n  \"code\": 0,\n  \"data\": {\n    \"id\": 1,\n    \"name\": \"张三\",\n    \"email\": \"zhangsan@example.com\",\n    \"role\": \"admin\"\n  },\n  \"message\": \"更新成功\"\n}" });
    update_user["docParams"] = serde_json::json!([
        d("path", "id", "Integer", "用户ID", vec![]),
        d("body", "name", "String", "用户名", vec![]),
        d("body", "email", "String", "邮箱地址", vec![]),
        d("body", "role", "String", "用户角色（user / admin / vip）", vec![]),
        d("resp_success", "code", "Integer", "状态码，0 表示成功", vec![]),
        d("resp_success", "data", "Object", "更新后的用户数据", vec![
            d("resp_success", "id", "Integer", "用户ID", vec![]),
            d("resp_success", "name", "String", "用户名", vec![]),
            d("resp_success", "email", "String", "邮箱地址", vec![]),
            d("resp_success", "role", "String", "用户角色", vec![]),
        ]),
        d("resp_success", "message", "String", "提示信息", vec![]),
        d("resp_fail", "code", "Integer", "错误码（404 表示用户不存在）", vec![]),
        d("resp_fail", "message", "String", "错误描述", vec![]),
    ]);
    write("用户管理", "更新用户.json", &update_user)?;

    let mut patch_user = api_file("部分更新用户", "PATCH", "/api/users/{id}", "仅更新用户的指定字段");
    patch_user["params"] = serde_json::json!([{ "key": "id", "value": "1", "enabled": true, "description": "用户ID" }]);
    patch_user["headers"] = serde_json::json!([{ "key": "Content-Type", "value": "application/json", "enabled": true, "description": "" }]);
    patch_user["body"] = serde_json::json!({ "mode": "json", "raw": "{\n  \"role\": \"vip\"\n}", "form": [] });
    patch_user["mock"] = serde_json::json!({ "enabled": true, "status": 200, "headers": [], "delay": 0, "body": "{\n  \"code\": 0,\n  \"data\": {\n    \"id\": 1,\n    \"role\": \"vip\"\n  },\n  \"message\": \"更新成功\"\n}" });
    patch_user["docParams"] = serde_json::json!([
        d("path", "id", "Integer", "用户ID", vec![]),
        d("body", "role", "String", "要更新的字段（仅传需要修改的字段）", vec![]),
        d("resp_success", "code", "Integer", "状态码，0 表示成功", vec![]),
        d("resp_success", "data", "Object", "更新后的用户数据（仅包含更新的字段）", vec![
            d("resp_success", "id", "Integer", "用户ID", vec![]),
            d("resp_success", "role", "String", "更新后的角色", vec![]),
        ]),
        d("resp_success", "message", "String", "提示信息", vec![]),
        d("resp_fail", "code", "Integer", "错误码", vec![]),
        d("resp_fail", "message", "String", "错误描述", vec![]),
    ]);
    write("用户管理", "部分更新用户.json", &patch_user)?;

    // 订单管理分组
    write("订单管理", INFO_FILE, &serde_json::json!({ "name": "订单管理", "description": "订单相关接口" }))?;
    let mut list_orders = api_file("获取订单列表", "GET", "/api/orders", "分页查询订单列表");
    list_orders["query"] = serde_json::json!([
        { "key": "page", "value": "1", "enabled": true, "description": "页码" },
        { "key": "pageSize", "value": "10", "enabled": true, "description": "每页数量" }
    ]);
    list_orders["mock"] = serde_json::json!({ "enabled": true, "status": 200, "headers": [], "delay": 0, "body": "{\n  \"code\": 0,\n  \"data\": {\n    \"list\": [\n      { \"id\": 1001, \"no\": \"SO20240101001\", \"amount\": 99.5 },\n      { \"id\": 1002, \"no\": \"SO20240101002\", \"amount\": 199.0 }\n    ],\n    \"total\": 2\n  },\n  \"message\": \"成功\"\n}" });
    list_orders["docParams"] = serde_json::json!([
        d("query", "page", "Integer", "页码，从 1 开始", vec![]),
        d("query", "pageSize", "Integer", "每页数量，最大 100", vec![]),
        d("resp_success", "code", "Integer", "状态码，0 表示成功", vec![]),
        d("resp_success", "data", "Object", "分页数据", vec![
            d("resp_success", "list", "List", "订单列表", vec![
                d("resp_success", "items", "Object", "订单信息", vec![
                    d("resp_success", "id", "Integer", "订单ID", vec![]),
                    d("resp_success", "no", "String", "订单编号", vec![]),
                    d("resp_success", "amount", "Float", "订单金额", vec![]),
                ]),
            ]),
            d("resp_success", "total", "Integer", "总记录数", vec![]),
        ]),
        d("resp_success", "message", "String", "提示信息", vec![]),
        d("resp_fail", "code", "Integer", "错误码", vec![]),
        d("resp_fail", "message", "String", "错误描述", vec![]),
    ]);
    write("订单管理", "获取订单列表.json", &list_orders)?;

    let mut head_order = api_file("检查订单状态", "HEAD", "/api/orders/{id}", "仅获取响应头，不返回响应体");
    head_order["params"] = serde_json::json!([{ "key": "id", "value": "1001", "enabled": true, "description": "订单ID" }]);
    head_order["mock"] = serde_json::json!({ "enabled": true, "status": 200, "headers": [{ "key": "X-Order-Status", "value": "paid", "enabled": true }], "delay": 0, "body": "" });
    write("订单管理", "检查订单状态.json", &head_order)?;

    let mut options_orders = api_file("订单接口预检", "OPTIONS", "/api/orders", "跨域预检请求（CORS）");
    options_orders["mock"] = serde_json::json!({ "enabled": true, "status": 204, "headers": [{ "key": "Access-Control-Allow-Methods", "value": "GET,POST,PUT,PATCH,DELETE,HEAD,OPTIONS", "enabled": true }], "delay": 0, "body": "" });
    write("订单管理", "订单接口预检.json", &options_orders)?;

    }

    if has("websocket") {
    // WebSocket 分组（与 tests/websocket-server.py 一一对应）：仅保留一个回显示例，
    // 服务器会回传该连接获取到的 query / header 参数供核对
    write("WebSocket", INFO_FILE, &serde_json::json!({ "name": "WebSocket", "description": "WebSocket 接口示例（与 tests/websocket-server.py 一一对应）" }))?;

    let ws_desc = "WebSocket 回显演示接口，配合测试服务 tests/websocket-server.py 使用。\n\n【启动测试服务】\n1. 安装依赖：pip install websockets\n2. 启动服务：python tests/websocket-server.py\n   - 默认监听 ws://127.0.0.1:8765\n   - 自定义端口：python tests/websocket-server.py 9999\n\n【接口说明】\n- 连接地址：ws://127.0.0.1:8765/echo\n- 连接时携带 Query 参数：token={{token}}（开发环境下值为 dev-token-123456）\n- 浏览器 WebSocket API 无法自定义请求头：Header 页签中配置的值不会发送，服务器回传的 header 为连接时的标准请求头（host、user-agent 等）\n\n【测试步骤】\n1. 点击「发送」建立连接，连接成功后会先收到一条欢迎消息（type: welcome，含本次连接的 query / header）\n2. 在消息输入框输入任意内容并发送\n3. 服务器回传消息内容及本次连接收到的 query / header，例如：\n{\"type\":\"message\",\"query\":{\"token\":\"dev-token-123456\"},\"header\":{\"host\":\"127.0.0.1:8765\",\"user-agent\":\"<客户端 User-Agent>\"},\"message\":\"hello\"}";
    let mut ws_echo = api_file("WebSocket 回显", "GET", "/echo", ws_desc);
    ws_echo["protocol"] = serde_json::json!("websocket");
    ws_echo["url"] = serde_json::json!("ws://127.0.0.1:8765/echo?token={{token}}");
    ws_echo["query"] = serde_json::json!([{ "key": "token", "value": "{{token}}", "enabled": true, "description": "鉴权令牌" }]);
    ws_echo["body"] = serde_json::json!({ "mode": "raw", "raw": "hello, this is a websocket echo message", "form": [], "binaryPath": "" });
    ws_echo["responses"] = serde_json::json!([
        { "id": format!("ws-echo-{}", uuid::Uuid::new_v4()), "name": "回显成功", "status": 0, "content_type": "application/json", "body": "{\n  \"type\": \"message\",\n  \"query\": {\"token\": \"dev-token-123456\"},\n  \"header\": {\"host\": \"127.0.0.1:8765\", \"user-agent\": \"<客户端 User-Agent>\"},\n  \"message\": \"hello, this is a websocket echo message\"\n}" }
    ]);
    write("WebSocket", "WebSocket 回显.json", &ws_echo)?;

    }

    if has("graphql") {
    // GraphQL 分组（与 tests/graphql-server.py 一一对应）：仅支持 POST + JSON body，不支持 Mock
    write("GraphQL", INFO_FILE, &serde_json::json!({ "name": "GraphQL", "description": "GraphQL 接口示例（与 tests/graphql-server.py 一一对应）" }))?;

    let gql_desc = "GraphQL 接口演示，配合测试服务 tests/graphql-server.py 使用。\n\n【启动测试服务】\n1. 无需安装第三方依赖（纯 Python 标准库）\n2. 启动服务：python tests/graphql-server.py\n   - 默认监听 http://127.0.0.1:8080/graphql\n   - 自定义端口：python tests/graphql-server.py 9999\n\n【接口说明】\n- GraphQL 接口固定使用 POST 方法，Body 仅支持 JSON 格式\n- 不支持 Mock（GraphQL 无法按路径生成路由）\n- 请求体结构：{ \"query\": \"...\", \"variables\": {} }\n\n【测试步骤】\n1. 点击「发送」执行下方 query / mutation 语句\n2. 服务端返回对应数据（data 字段）或错误信息（errors 字段）";

    let mut gql_query_user = api_file("查询用户", "POST", "/graphql", gql_desc);
    gql_query_user["protocol"] = serde_json::json!("graphql");
    gql_query_user["url"] = serde_json::json!("http://127.0.0.1:8080/graphql");
    gql_query_user["headers"] = serde_json::json!([{ "key": "Content-Type", "value": "application/json", "enabled": true, "description": "" }]);
    gql_query_user["body"] = serde_json::json!({ "mode": "json", "raw": "{\n  \"query\": \"query { user(id: 1) { id name email role } }\"\n}", "form": [], "binaryPath": "" });
    gql_query_user["responses"] = serde_json::json!([
        { "id": format!("gql-user-{}", uuid::Uuid::new_v4()), "name": "返回成功", "status": 200, "content_type": "application/json", "body": "{\n  \"data\": {\n    \"user\": {\n      \"id\": 1,\n      \"name\": \"张三\",\n      \"email\": \"zhangsan@example.com\",\n      \"role\": \"user\"\n    }\n  }\n}" }
    ]);
    gql_query_user["docParams"] = serde_json::json!([
        d("body", "query", "String", "GraphQL 查询语句（query / mutation）", vec![]),
        d("body", "variables", "Object", "查询变量（可选）", vec![]),
        d("resp_success", "data", "Object", "查询结果数据", vec![
            d("resp_success", "user", "Object", "用户信息", vec![
                d("resp_success", "id", "Integer", "用户ID", vec![]),
                d("resp_success", "name", "String", "用户名", vec![]),
                d("resp_success", "email", "String", "邮箱地址", vec![]),
                d("resp_success", "role", "String", "用户角色", vec![]),
            ]),
        ]),
        d("resp_fail", "errors", "List", "GraphQL 错误列表（如用户不存在）", vec![]),
    ]);
    write("GraphQL", "查询用户.json", &gql_query_user)?;

    let mut gql_list_users = api_file("用户列表", "POST", "/graphql", "查询全部用户（GraphQL query）");
    gql_list_users["protocol"] = serde_json::json!("graphql");
    gql_list_users["url"] = serde_json::json!("http://127.0.0.1:8080/graphql");
    gql_list_users["headers"] = serde_json::json!([{ "key": "Content-Type", "value": "application/json", "enabled": true, "description": "" }]);
    gql_list_users["body"] = serde_json::json!({ "mode": "json", "raw": "{\n  \"query\": \"query { users { id name email role } }\"\n}", "form": [], "binaryPath": "" });
    gql_list_users["responses"] = serde_json::json!([
        { "id": format!("gql-users-{}", uuid::Uuid::new_v4()), "name": "返回成功", "status": 200, "content_type": "application/json", "body": "{\n  \"data\": {\n    \"users\": [\n      { \"id\": 1, \"name\": \"张三\", \"email\": \"zhangsan@example.com\", \"role\": \"user\" },\n      { \"id\": 2, \"name\": \"李四\", \"email\": \"lisi@example.com\", \"role\": \"admin\" }\n    ]\n  }\n}" }
    ]);
    write("GraphQL", "用户列表.json", &gql_list_users)?;

    let mut gql_create_user = api_file("创建用户", "POST", "/graphql", "通过 mutation 创建用户");
    gql_create_user["protocol"] = serde_json::json!("graphql");
    gql_create_user["url"] = serde_json::json!("http://127.0.0.1:8080/graphql");
    gql_create_user["headers"] = serde_json::json!([{ "key": "Content-Type", "value": "application/json", "enabled": true, "description": "" }]);
    gql_create_user["body"] = serde_json::json!({ "mode": "json", "raw": "{\n  \"query\": \"mutation { createUser(name: \\\"王五\\\", email: \\\"wangwu@example.com\\\") { id name email } }\"\n}", "form": [], "binaryPath": "" });
    gql_create_user["responses"] = serde_json::json!([
        { "id": format!("gql-create-{}", uuid::Uuid::new_v4()), "name": "返回成功", "status": 200, "content_type": "application/json", "body": "{\n  \"data\": {\n    \"createUser\": {\n      \"id\": 3,\n      \"name\": \"王五\",\n      \"email\": \"wangwu@example.com\"\n    }\n  }\n}" }
    ]);
    write("GraphQL", "创建用户.json", &gql_create_user)?;

    let mut gql_order = api_file("查询订单", "POST", "/graphql", "查询订单详情（含嵌套字段）");
    gql_order["protocol"] = serde_json::json!("graphql");
    gql_order["url"] = serde_json::json!("http://127.0.0.1:8080/graphql");
    gql_order["headers"] = serde_json::json!([{ "key": "Content-Type", "value": "application/json", "enabled": true, "description": "" }]);
    gql_order["body"] = serde_json::json!({ "mode": "json", "raw": "{\n  \"query\": \"query { order(id: 1001) { id no amount items { name price } } }\"\n}", "form": [], "binaryPath": "" });
    gql_order["responses"] = serde_json::json!([
        { "id": format!("gql-order-{}", uuid::Uuid::new_v4()), "name": "返回成功", "status": 200, "content_type": "application/json", "body": "{\n  \"data\": {\n    \"order\": {\n      \"id\": 1001,\n      \"no\": \"SO20240101001\",\n      \"amount\": 99.5,\n      \"items\": [\n        { \"name\": \"鼠标\", \"price\": 49.5 },\n        { \"name\": \"键盘\", \"price\": 50.0 }\n      ]\n    }\n  }\n}" }
    ]);
    write("GraphQL", "查询订单.json", &gql_order)?;

    }

    if has("socketio") {
    // Socket.IO 分组（与 tests/socketio-server.py 一一对应）：实时消息交互，展示与 WebSocket 一致
    write("Socket.IO", INFO_FILE, &serde_json::json!({ "name": "Socket.IO", "description": "Socket.IO 接口示例（与 tests/socketio-server.py 一一对应）" }))?;
    let sio_desc = "Socket.IO 实时消息接口演示，配合测试服务 tests/socketio-server.py 使用。\n\n【启动测试服务】\n1. 安装依赖：pip install python-socketio simple-websocket\n2. 启动服务：python tests/socketio-server.py\n   - 默认监听 http://127.0.0.1:8090\n   - 自定义端口：python tests/socketio-server.py 9999\n\n【接口说明】\n- Socket.IO 连接地址为 http://127.0.0.1:8090（不提供 ws/wss 切换，由库内部协商传输方式）\n- 消息事件名固定为 message：发送的消息会原样回显，并附带本次连接的 query 参数\n- 浏览器端不可自定义请求头，Header 页签中的配置不会发送\n\n【测试步骤】\n1. 点击「发送」建立连接，连接成功后会先收到一条欢迎消息（type: welcome）\n2. 在消息输入框输入任意内容并发送\n3. 服务器回传消息内容及本次连接的 query 参数，例如：\n{\"type\":\"message\",\"query\":{\"token\":\"dev-token-123456\"},\"message\":\"hello\"}";
    let mut sio_chat = api_file("实时聊天", "GET", "/", sio_desc);
    sio_chat["protocol"] = serde_json::json!("socketio");
    sio_chat["url"] = serde_json::json!("http://127.0.0.1:8090");
    sio_chat["body"] = serde_json::json!({ "mode": "text", "raw": "hello socket.io", "form": [], "binaryPath": "" });
    sio_chat["responses"] = serde_json::json!([]);
    write("Socket.IO", "实时聊天.json", &sio_chat)?;

    let mut sio_broadcast = api_file("广播通知", "GET", "/", "向所有已连接客户端广播一条消息（Socket.IO broadcast 事件）。\n\n【测试步骤】\n1. 先启动 tests/socketio-server.py（默认 http://127.0.0.1:8090）\n2. 点击「发送」建立连接并收到欢迎消息\n3. 发送消息：{\"cmd\":\"broadcast\",\"msg\":\"hello everyone\"}\n4. 所有连接的客户端都会收到这条广播（type: broadcast）");
    sio_broadcast["protocol"] = serde_json::json!("socketio");
    sio_broadcast["url"] = serde_json::json!("http://127.0.0.1:8090");
    sio_broadcast["body"] = serde_json::json!({ "mode": "json", "raw": "{\n  \"cmd\": \"broadcast\",\n  \"msg\": \"hello everyone\"\n}", "form": [], "binaryPath": "" });
    sio_broadcast["responses"] = serde_json::json!([]);
    write("Socket.IO", "广播通知.json", &sio_broadcast)?;

    }

    // WebDAV 分组（与 tests/webdav-server.py 一一对应）：编辑体验与 HTTP 一致，
    // 仅方法下拉框额外提供 WebDAV 协议（PROPFIND / PROPPATCH / MKCOL / COPY / MOVE / LOCK / UNLOCK / REPORT），不支持 Mock
    if has("webdav") {
        write("WebDAV", INFO_FILE, &serde_json::json!({ "name": "WebDAV", "description": "WebDAV 接口示例（与 tests/webdav-server.py 一一对应，无 Mock）" }))?;

        let dav_desc = "WebDAV 文件夹操作演示，配合测试服务 tests/webdav-server.py 使用。\n\n【启动测试服务】\n1. 无需安装第三方依赖（纯 Python 标准库）\n2. 启动服务：python tests/webdav-server.py\n   - 默认监听 http://127.0.0.1:8081\n   - 自定义端口：python tests/webdav-server.py 9999\n\n【接口说明】\n- WebDAV 接口与 HTTP 编辑体验一致，方法下拉框额外提供 PROPFIND / PROPPATCH / MKCOL / COPY / MOVE / LOCK / UNLOCK / REPORT\n- 不支持 Mock\n\n【测试步骤】\n1. 点击「发送」执行下方方法\n2. 服务端返回对应响应（PROPFIND 返回 multistatus XML 等）";
        let mut dav_propfind = api_file("目录属性 PROPFIND", "PROPFIND", "/dav", dav_desc);
        dav_propfind["protocol"] = serde_json::json!("webdav");
        dav_propfind["url"] = serde_json::json!("http://127.0.0.1:8081");
        dav_propfind["headers"] = serde_json::json!([
            { "key": "Depth", "value": "1", "enabled": true, "description": "1 = 仅当前集合及其直接子资源" },
            { "key": "Content-Type", "value": "application/xml; charset=utf-8", "enabled": true, "description": "" },
        ]);
        dav_propfind["body"] = serde_json::json!({ "mode": "xml", "raw": "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<d:propfind xmlns:d=\"DAV:\">\n  <d:prop>\n    <d:displayname/>\n    <d:resourcetype/>\n    <d:getcontentlength/>\n    <d:getlastmodified/>\n  </d:prop>\n</d:propfind>", "form": [] });
        dav_propfind["responses"] = serde_json::json!([
            { "id": format!("dav-propfind-{}", uuid::Uuid::new_v4()), "name": "属性列表", "status": 207, "content_type": "application/xml", "body": "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<d:multistatus xmlns:d=\"DAV:\">\n  <d:response>\n    <d:href>/dav/</d:href>\n    <d:propstat>\n      <d:prop>\n        <d:displayname>dav</d:displayname>\n        <d:resourcetype><d:collection/></d:resourcetype>\n      </d:prop>\n      <d:status>HTTP/1.1 200 OK</d:status>\n    </d:propstat>\n  </d:response>\n</d:multistatus>" }
        ]);
        write("WebDAV", "目录属性 PROPFIND.json", &dav_propfind)?;

        let mut dav_mkcol = api_file("创建集合 MKCOL", "MKCOL", "/dav/demo-dir", "在服务器上创建一个新集合（目录）。\n\n【测试步骤】\n1. 启动 tests/webdav-server.py（默认 http://127.0.0.1:8081）\n2. 点击「发送」，服务器返回 201 Created，随后可用 PROPFIND 看到 demo-dir");
        dav_mkcol["protocol"] = serde_json::json!("webdav");
        dav_mkcol["url"] = serde_json::json!("http://127.0.0.1:8081");
        dav_mkcol["responses"] = serde_json::json!([
            { "id": format!("dav-mkcol-{}", uuid::Uuid::new_v4()), "name": "创建成功", "status": 201, "content_type": "text/plain", "body": "Created" }
        ]);
        write("WebDAV", "创建集合 MKCOL.json", &dav_mkcol)?;

        let mut dav_put = api_file("上传文件 PUT", "PUT", "/dav/hello.txt", "将文本内容作为文件上传到集合 /dav/hello.txt。\n\n【测试步骤】\n1. 启动 tests/webdav-server.py（默认 http://127.0.0.1:8081）\n2. 点击「发送」，服务器返回 201 Created，随后可用 GET 下载验证");
        dav_put["protocol"] = serde_json::json!("webdav");
        dav_put["url"] = serde_json::json!("http://127.0.0.1:8081");
        dav_put["headers"] = serde_json::json!([{ "key": "Content-Type", "value": "text/plain", "enabled": true, "description": "" }]);
        dav_put["body"] = serde_json::json!({ "mode": "raw", "raw": "hello webdav\n", "form": [] });
        dav_put["responses"] = serde_json::json!([
            { "id": format!("dav-put-{}", uuid::Uuid::new_v4()), "name": "上传成功", "status": 201, "content_type": "text/plain", "body": "Created" }
        ]);
        write("WebDAV", "上传文件 PUT.json", &dav_put)?;

        let mut dav_copy = api_file("复制资源 COPY", "COPY", "/dav/hello.txt", "将 /dav/hello.txt 复制到 /dav/hello-copy.txt（通过 Destination 请求头指定目标）。\n\n【测试步骤】\n1. 先上传 hello.txt（PUT 示例）\n2. 点击「发送」，服务器返回 201 Created，/dav 下出现 hello-copy.txt");
        dav_copy["protocol"] = serde_json::json!("webdav");
        dav_copy["url"] = serde_json::json!("http://127.0.0.1:8081");
        dav_copy["headers"] = serde_json::json!([
            { "key": "Destination", "value": "http://127.0.0.1:8081/dav/hello-copy.txt", "enabled": true, "description": "复制目标地址" },
            { "key": "Overwrite", "value": "T", "enabled": true, "description": "T = 允许覆盖" },
        ]);
        dav_copy["responses"] = serde_json::json!([
            { "id": format!("dav-copy-{}", uuid::Uuid::new_v4()), "name": "复制成功", "status": 201, "content_type": "text/plain", "body": "Created" }
        ]);
        write("WebDAV", "复制资源 COPY.json", &dav_copy)?;

        let mut dav_proppatch = api_file("修改属性 PROPPATCH", "PROPPATCH", "/dav/hello.txt", "修改文件的扩展属性（dead properties）。\n\n【测试步骤】\n1. 先上传 hello.txt（PUT 示例）\n2. 点击「发送」，body 通过 <set> 写入 z:category = code-review\n3. 服务器返回 207 multistatus，属性设置成功");
        dav_proppatch["protocol"] = serde_json::json!("webdav");
        dav_proppatch["url"] = serde_json::json!("http://127.0.0.1:8081");
        dav_proppatch["headers"] = serde_json::json!([{ "key": "Content-Type", "value": "application/xml; charset=utf-8", "enabled": true, "description": "" }]);
        dav_proppatch["body"] = serde_json::json!({ "mode": "xml", "raw": "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<d:propertyupdate xmlns:d=\"DAV:\" xmlns:z=\"urn:z\">\n  <d:set>\n    <d:prop>\n      <z:category>code-review</z:category>\n    </d:prop>\n  </d:set>\n</d:propertyupdate>", "form": [] });
        dav_proppatch["responses"] = serde_json::json!([
            { "id": format!("dav-proppatch-{}", uuid::Uuid::new_v4()), "name": "属性已设置", "status": 207, "content_type": "application/xml", "body": "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<d:multistatus xmlns:d=\"DAV:\">\n  <d:response>\n    <d:href>/dav/hello.txt</d:href>\n    <d:propstat>\n      <d:prop><d:category/></d:prop>\n      <d:status>HTTP/1.1 200 OK</d:status>\n    </d:propstat>\n  </d:response>\n</d:multistatus>" }
        ]);
        write("WebDAV", "修改属性 PROPPATCH.json", &dav_proppatch)?;

        let mut dav_move = api_file("移动资源 MOVE", "MOVE", "/dav/hello.txt", "将 /dav/hello.txt 移动到 /dav/hello-moved.txt（Destination 请求头指定新位置，Overwrite 决定是否允许覆盖）。\n\n【测试步骤】\n1. 先上传 hello.txt（PUT 示例）\n2. 点击「发送」，服务器返回 204 No Content，原路径不再存在");
        dav_move["protocol"] = serde_json::json!("webdav");
        dav_move["url"] = serde_json::json!("http://127.0.0.1:8081");
        dav_move["headers"] = serde_json::json!([
            { "key": "Destination", "value": "http://127.0.0.1:8081/dav/hello-moved.txt", "enabled": true, "description": "移动目标地址" },
            { "key": "Overwrite", "value": "T", "enabled": true, "description": "T = 允许覆盖" },
        ]);
        dav_move["responses"] = serde_json::json!([
            { "id": format!("dav-move-{}", uuid::Uuid::new_v4()), "name": "移动成功", "status": 204, "content_type": "text/plain", "body": "" }
        ]);
        write("WebDAV", "移动资源 MOVE.json", &dav_move)?;

        let mut dav_lock = api_file("加锁 LOCK", "LOCK", "/dav/hello.txt", "对资源加排它写锁，防止其他客户端同时修改。\n\n【测试步骤】\n1. 先上传 hello.txt（PUT 示例）\n2. 点击「发送」，服务器返回 200 及 lockdiscovery 内容\n3. 请把响应头 Lock-Token（形如 <opaquelocktoken:xxxx>）复制到下面的 UNLOCK 示例中使用");
        dav_lock["protocol"] = serde_json::json!("webdav");
        dav_lock["url"] = serde_json::json!("http://127.0.0.1:8081");
        dav_lock["headers"] = serde_json::json!([{ "key": "Content-Type", "value": "application/xml; charset=utf-8", "enabled": true, "description": "" }]);
        dav_lock["body"] = serde_json::json!({ "mode": "xml", "raw": "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<d:lockinfo xmlns:d=\"DAV:\">\n  <d:lockscope><d:exclusive/></d:lockscope>\n  <d:locktype><d:write/></d:locktype>\n  <d:owner><d:href>api-manager-demo</d:href></d:owner>\n</d:lockinfo>", "form": [] });
        dav_lock["responses"] = serde_json::json!([
            { "id": format!("dav-lock-{}", uuid::Uuid::new_v4()), "name": "已加锁", "status": 200, "content_type": "application/xml", "body": "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<d:prop xmlns:d=\"DAV:\"><d:lockdiscovery>\n  <d:activelock>\n    <d:locktype><d:write/></d:locktype>\n    <d:lockscope><d:exclusive/></d:lockscope>\n    <d:depth>infinity</d:depth>\n    <d:owner><d:href>api-manager-demo</d:href></d:owner>\n    <d:timeout>Second-3600</d:timeout>\n    <d:locktoken><d:href>opaquelocktoken:demo-lock-1</d:href></d:locktoken>\n  </d:activelock>\n</d:lockdiscovery></d:prop>" }
        ]);
        write("WebDAV", "加锁 LOCK.json", &dav_lock)?;

        let mut dav_unlock = api_file("解锁 UNLOCK", "UNLOCK", "/dav/hello.txt", "释放资源的写锁。\n\n【测试步骤】\n1. 先用 LOCK 示例加锁\n2. 把响应头返回的 Lock-Token（含 <> 尖括号）填到下方请求头的 Lock-Token 值中\n3. 点击「发送」，服务器返回 204 No Content");
        dav_unlock["protocol"] = serde_json::json!("webdav");
        dav_unlock["url"] = serde_json::json!("http://127.0.0.1:8081");
        dav_unlock["headers"] = serde_json::json!([{ "key": "Lock-Token", "value": "<opaquelocktoken:demo-lock-1>", "enabled": true, "description": "LOCK 响应返回的令牌，示例值仅占位" }]);
        dav_unlock["responses"] = serde_json::json!([
            { "id": format!("dav-unlock-{}", uuid::Uuid::new_v4()), "name": "已解锁", "status": 204, "content_type": "text/plain", "body": "" }
        ]);
        write("WebDAV", "解锁 UNLOCK.json", &dav_unlock)?;

        let mut dav_report = api_file("查询属性 REPORT", "REPORT", "/dav/hello.txt", "按 RFC 3253 的扩展属性查询；本测试服务简化为与 PROPFIND（Depth 0）等价。\n\n【测试步骤】\n1. 先上传 hello.txt（PUT 示例）\n2. 点击「发送」，返回 207 multistatus 属性列表");
        dav_report["protocol"] = serde_json::json!("webdav");
        dav_report["url"] = serde_json::json!("http://127.0.0.1:8081");
        dav_report["headers"] = serde_json::json!([{ "key": "Content-Type", "value": "application/xml; charset=utf-8", "enabled": true, "description": "" }]);
        dav_report["body"] = serde_json::json!({ "mode": "xml", "raw": "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<d:expand-property xmlns:d=\"DAV:\">\n  <d:property name=\"displayname\"/>\n  <d:property name=\"resourcetype\"/>\n  <d:property name=\"getcontentlength\"/>\n</d:expand-property>", "form": [] });
        dav_report["responses"] = serde_json::json!([
            { "id": format!("dav-report-{}", uuid::Uuid::new_v4()), "name": "属性列表", "status": 207, "content_type": "application/xml", "body": "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<d:multistatus xmlns:d=\"DAV:\">\n  <d:response>\n    <d:href>/dav/hello.txt</d:href>\n    <d:propstat>\n      <d:prop>\n        <d:displayname>hello.txt</d:displayname>\n        <d:resourcetype/>\n        <d:getcontentlength>12</d:getcontentlength>\n      </d:prop>\n      <d:status>HTTP/1.1 200 OK</d:status>\n    </d:propstat>\n  </d:response>\n</d:multistatus>" }
        ]);
        write("WebDAV", "查询属性 REPORT.json", &dav_report)?;
    }

    if has("object") {
    // 对象示例：工作区 .object/ 下生成「用户管理 / 订单管理」分组与几个对象，
    // 与上面的接口演示呼应（属性含 mock 示例值，可配合数据生成体验）
    let now = chrono::Local::now().timestamp();
    let prop = |key: &str, kind: &str, item_kind: &str, description: &str, mock: &str| {
        crate::objects::ObjectProp {
            key: key.into(),
            kind: kind.into(),
            item_kind: item_kind.into(),
            ref_hash: String::new(),
            description: description.into(),
            mock: mock.into(),
        }
    };
    let obj_def = |name: &str, object_name: &str, group: &str, description: &str, properties: Vec<crate::objects::ObjectProp>| {
        crate::objects::ObjectDef {
            uuid: uuid::Uuid::new_v4().to_string(),
            hash: String::new(), // save_objects 会重算
            name: name.into(),
            object_name: object_name.into(),
            package_name: String::new(),
            group: group.into(),
            deprecated: false,
            description: description.into(),
            properties,
            created_at: now,
            updated_at: now,
            order: None,
        }
    };
    let demo_store = crate::objects::ObjectStore {
        groups: vec![
            crate::objects::ObjectGroup { id: "用户管理".into(), name: "用户管理".into(), deprecated: false, order: None },
            crate::objects::ObjectGroup { id: "订单管理".into(), name: "订单管理".into(), deprecated: false, order: None },
        ],
        objects: vec![
            obj_def("用户", "User", "用户管理", "系统用户信息", vec![
                prop("id", "Integer", "Integer", "主键", ""),
                prop("name", "String", "String", "用户名", "@cname"),
                prop("email", "String", "String", "邮箱地址", "@email"),
                prop("role", "String", "String", "用户角色（user / admin / vip）", "user"),
                prop("zodiac", "String", "String", "星座", "@zodiac"),
                prop("createdAt", "Datetime", "String", "创建时间", "@datetime"),
            ]),
            obj_def("订单", "Order", "订单管理", "用户订单", vec![
                prop("id", "Integer", "Integer", "订单ID", ""),
                prop("no", "String", "String", "订单编号", "SO2024"),
                prop("amount", "Float", "Float", "订单金额（元）", "99.5"),
                prop("status", "String", "String", "订单状态（pending/paid/shipped/done）", "paid"),
                prop("userId", "Integer", "Integer", "下单用户ID", "1001"),
                prop("createdAt", "Datetime", "String", "下单时间", "@datetime"),
            ]),
            obj_def("订单明细", "OrderItem", "订单管理", "订单包含的商品明细", vec![
                prop("id", "Integer", "Integer", "明细ID", ""),
                prop("productName", "String", "String", "商品名称", "@ctitle(6)"),
                prop("price", "Float", "Float", "单价（元）", "19.9"),
                prop("quantity", "Integer", "Integer", "数量", "2"),
            ]),
        ],
    };
    crate::objects::save_objects_impl(&root, &demo_store)?;

    // 创建星座占位符 @zodiac（自定义 mock 占位符示例，可在接口/对象 mock 数据中使用）
    // 重复生成演示案例时占位符已存在，忽略该错误（其余演示数据仍正常生成）
    let _ = crate::mock::save_custom_mock_impl(
        &root,
        &crate::mock::CustomMock {
            name: "zodiac".into(),
            enabled: true,
            desc: "十二星座之一".into(),
            code: "(ctx) => ctx.pick([\"白羊座\",\"金牛座\",\"双子座\",\"巨蟹座\",\"狮子座\",\"处女座\",\"天秤座\",\"天蝎座\",\"射手座\",\"摩羯座\",\"水瓶座\",\"双鱼座\"])".into(),
        },
        None,
    );

    }

    Ok(())
}
