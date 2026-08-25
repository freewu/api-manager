use crate::{ApiFile, MockRunState, MockStatus, INFO_FILE};
use axum::{
    body::Body,
    extract::State as AxState,
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tower_http::cors::CorsLayer;
use percent_encoding::percent_decode_str;

// ==================== 路由表 ====================

#[derive(Clone, Debug)]
pub enum Segment {
    Literal(String),
    Param(String),
}

#[derive(Clone, Debug)]
pub struct MockRoute {
    pub method: String, // "GET" / "POST" / ... / "ANY"
    pub segments: Vec<Segment>,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub delay_ms: u64,
    pub body: String,
}

#[derive(Clone)]
pub struct MockServerState {
    pub routes: Arc<RwLock<Vec<MockRoute>>>,
    /// 全局环境变量（来自工作区 __envs.json 的激活环境）
    pub envs: Arc<RwLock<HashMap<String, String>>>,
}

fn parse_route(api: &ApiFile) -> Option<MockRoute> {
    if !api.mock.enabled {
        return None;
    }
    // GraphQL 接口暂不支持 Mock（无法按路径生成路由）
    if api.protocol == "graphql" || api.protocol == "socketio" {
        return None;
    }
    let path = if api.path.trim().is_empty() {
        "/"
    } else {
        api.path.trim()
    };
    // 去掉 query 部分
    let path = path.split('?').next().unwrap_or(path);
    let segments = path
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|seg| {
            let seg = seg.trim();
            if seg.starts_with('{') && seg.ends_with('}') {
                Segment::Param(seg[1..seg.len() - 1].to_string())
            } else if let Some(stripped) = seg.strip_prefix(':') {
                Segment::Param(stripped.to_string())
            } else {
                Segment::Literal(seg.to_string())
            }
        })
        .collect::<Vec<_>>();

    let headers = api
        .mock
        .headers
        .iter()
        .filter(|h| !h.key.trim().is_empty())
        .map(|h| (h.key.trim().to_string(), h.value.clone()))
        .collect();

    Some(MockRoute {
        method: if api.method.trim().is_empty() {
            "ANY".to_string()
        } else {
            api.method.trim().to_uppercase()
        },
        segments,
        status: api.mock.status.max(100).min(599),
        headers,
        delay_ms: api.mock.delay,
        body: api.mock.body.clone(),
    })
}

/// 扫描工作区，收集所有启用了 mock 的接口
pub fn scan_workspace(root: &Path) -> Vec<MockRoute> {
    let mut routes = Vec::new();
    scan_dir(root, &mut routes);
    routes
}

fn scan_dir(dir: &Path, routes: &mut Vec<MockRoute>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                scan_dir(&path, routes);
            } else if path.extension().map(|e| e == "json").unwrap_or(false)
                && name != INFO_FILE
            {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(api) = serde_json::from_str::<ApiFile>(&content) {
                        if let Some(route) = parse_route(&api) {
                            routes.push(route);
                        }
                    }
                }
            }
        }
    }
}

// ==================== 自定义 Mock 占位符 ====================

/// 自定义 mock 占位符：文件保存在工作目录 .mock/<name>.js（name 不含 @）
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomMock {
    /// 占位符标识（不含 @），使用时写作 @name
    pub name: String,
    /// 是否启用（未启用则不展示、不参与生成）
    pub enabled: bool,
    /// 说明文字
    pub desc: String,
    /// JS 代码：(ctx) => 返回值，ctx 提供 randInt/pick/random 等工具
    pub code: String,
}

/// mock.js 内置占位符名（自定义占位符不允许与这些冲突）
const BUILTIN_MOCK_NAMES: &[&str] = &[
    "cname", "name", "first", "last", "email", "phone", "id", "guid", "integer", "float",
    "natural", "boolean", "date", "time", "datetime", "now", "url", "domain", "ip",
    "protocol", "city", "province", "county", "zip", "word", "title", "sentence",
    "paragraph", "color", "image", "avatar", "string", "character",
];

fn mock_dir(root: &Path) -> PathBuf {
    root.join(".mock")
}

/// 校验占位符标识：非空、字母/数字/下划线、不以数字开头
pub fn valid_mock_name(name: &str) -> Result<String, String> {
    let n = name.trim().trim_start_matches('@');
    if n.is_empty() {
        return Err("占位符标识不能为空".to_string());
    }
    let mut chars = n.chars();
    let first = chars.next().unwrap();
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err("占位符标识需以字母或下划线开头".to_string());
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("占位符标识仅允许字母/数字/下划线".to_string());
    }
    if BUILTIN_MOCK_NAMES.contains(&n) {
        return Err(format!("@{n} 与 mock.js 内置占位符冲突，请换一个标识"));
    }
    Ok(n.to_string())
}

/// 列出 .mock 目录下所有自定义占位符（按标识排序）
pub fn list_custom_mocks_impl(root: &Path) -> Vec<CustomMock> {
    let dir = mock_dir(root);
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();
            if !file_name.ends_with(".js") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(&path) {
                if let Some(m) = parse_custom_mock(&content, &file_name) {
                    out.push(m);
                }
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// 解析单个占位符文件：文件头 /** */ 注释存元数据，注释之后为 JS 代码
fn parse_custom_mock(content: &str, file_name: &str) -> Option<CustomMock> {
    let mut enabled = true;
    let mut desc = String::new();
    let trimmed = content.trim_start();
    let code_start = if let Some(rest) = trimmed.strip_prefix("/**") {
        if let Some(end) = rest.find("*/") {
            let meta = &rest[..end];
            for line in meta.lines() {
                let line = line.trim().trim_start_matches('*').trim();
                if let Some(v) = line.strip_prefix("@enabled") {
                    enabled = v.trim().eq_ignore_ascii_case("true") || v.trim() == "1";
                } else if let Some(v) = line.strip_prefix("@desc") {
                    desc = v.trim().to_string();
                }
            }
            &rest[end + 2..]
        } else {
            rest
        }
    } else {
        trimmed
    };
    let name = file_name.trim_end_matches(".js").to_string();
    if name.is_empty() {
        return None;
    }
    Some(CustomMock {
        name,
        enabled,
        desc,
        code: code_start.trim().to_string(),
    })
}

/// 保存自定义占位符：写入 .mock/<name>.js；old_name 与 name 不同时视为重命名，删除旧文件
pub fn save_custom_mock_impl(root: &Path, input: &CustomMock, old_name: Option<&str>) -> Result<(), String> {
    let name = valid_mock_name(&input.name)?;
    if input.code.trim().is_empty() {
        return Err("占位符代码不能为空".to_string());
    }
    let dir = mock_dir(root);
    fs::create_dir_all(&dir).map_err(|e| format!("创建 .mock 目录失败: {e}"))?;
    let path = dir.join(format!("{name}.js"));
    if path.exists() && old_name.map(|o| o != name).unwrap_or(true) {
        return Err(format!("占位符 @{name} 已存在").to_string());
    }
    if let Some(old) = old_name {
        if old != name {
            let _ = fs::remove_file(dir.join(format!("{old}.js")));
        }
    }
    let content = format!(
        "/**\n * @name {name}\n * @enabled {}\n * @desc {}\n */\n{}\n",
        if input.enabled { "true" } else { "false" },
        input.desc.trim(),
        input.code.trim()
    );
    fs::write(&path, content).map_err(|e| format!("写入占位符文件失败: {e}"))?;
    Ok(())
}

/// 删除自定义占位符文件
pub fn delete_custom_mock_impl(root: &Path, name: &str) -> Result<(), String> {
    let n = valid_mock_name(name)?;
    let dir = mock_dir(root);
    let path = dir.join(format!("{n}.js"));
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("删除占位符文件失败: {e}"))?;
    }
    Ok(())
}

// ==================== 服务生命周期 ====================

pub async fn start_mock(app: &AppHandle, port: u16) -> Result<MockStatus, String> {
    // 先停掉旧的
    stop_mock(app);

    let root = {
        let state = app.state::<crate::WorkspaceState>();
        let guard = state
            .root
            .lock()
            .map_err(|_| "状态锁错误".to_string())?;
        guard
            .clone()
            .ok_or_else(|| "尚未选择工作目录".to_string())?
    };

    let routes = scan_workspace(&root);
    let routes_arc = Arc::new(RwLock::new(routes));
    let envs = Arc::new(RwLock::new(crate::read_env_map(&root)));
    let server_state = MockServerState {
        routes: routes_arc.clone(),
        envs: envs.clone(),
    };

    let router: Router<MockServerState> = Router::new()
        .fallback(mock_handler)
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|e| format!("端口 {port} 绑定失败: {e}"))?;
    let addr = listener
        .local_addr()
        .map_err(|e| format!("获取地址失败: {e}"))?;

    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, router.with_state(server_state)).await;
    });

    let run_state = app.state::<MockRunState>();
    *run_state.running.lock().unwrap() = true;
    *run_state.port.lock().unwrap() = Some(addr.port());
    *run_state.addr.lock().unwrap() = Some(format!("http://127.0.0.1:{}", addr.port()));
    *run_state.route_count.lock().unwrap() = routes_arc.read().map(|r| r.len()).unwrap_or(0);
    *run_state.routes.lock().unwrap() = Some(routes_arc);
    *run_state.envs.lock().unwrap() = Some(envs);
    *run_state.abort.lock().unwrap() = Some(handle.abort_handle());

    Ok(status(app))
}

pub fn stop_mock(app: &AppHandle) {
    let run_state = app.state::<MockRunState>();
    if let Some(handle) = run_state.abort.lock().unwrap().take() {
        handle.abort();
    }
    *run_state.running.lock().unwrap() = false;
    *run_state.port.lock().unwrap() = None;
    *run_state.addr.lock().unwrap() = None;
    *run_state.route_count.lock().unwrap() = 0;
    *run_state.routes.lock().unwrap() = None;
    *run_state.envs.lock().unwrap() = None;
}

pub fn status(app: &AppHandle) -> MockStatus {
    let run_state = app.state::<MockRunState>();
    let running = *run_state.running.lock().unwrap();
    let url = run_state.addr.lock().unwrap().clone();
    let port = *run_state.port.lock().unwrap();
    let route_count = *run_state.route_count.lock().unwrap();
    MockStatus {
        running,
        url,
        port,
        route_count,
    }
}

pub fn reload_mock(app: &AppHandle) -> Result<(), String> {
    let run_state = app.state::<MockRunState>();
    if !*run_state.running.lock().unwrap() {
        return Ok(());
    }
    let root = {
        let state = app.state::<crate::WorkspaceState>();
        let guard = state
            .root
            .lock()
            .map_err(|_| "状态锁错误".to_string())?;
        guard
            .clone()
            .ok_or_else(|| "尚未选择工作目录".to_string())?
    };
    let routes = scan_workspace(&root);
    let envs = crate::read_env_map(&root);
    let count = {
        let opt = run_state.routes.lock().unwrap().clone();
        match opt {
            Some(routes_arc) => {
                let mut guard = routes_arc.write().unwrap();
                *guard = routes;
                guard.len()
            }
            None => 0,
        }
    };
    if let Some(envs_arc) = run_state.envs.lock().unwrap().clone() {
        *envs_arc.write().unwrap() = envs;
    }
    *run_state.route_count.lock().unwrap() = count;
    Ok(())
}

// ==================== 请求处理 ====================

/// 全局环境变量 {{key}} 替换（保留 path/method 等系统变量不受覆盖）
pub fn apply_env_vars(body: &str, envs: &HashMap<String, String>) -> String {
    let mut out = body.to_string();
    let mut keys: Vec<&String> = envs.keys().collect();
    keys.sort_by_key(|k| std::cmp::Reverse(k.len()));
    for key in keys {
        if key == "path"
            || key == "method"
            || key.starts_with("path.")
            || key.starts_with("query.")
        {
            continue;
        }
        if let Some(v) = envs.get(key) {
            out = out.replace(&format!("{{{{{key}}}}}"), v);
        }
    }
    out
}

async fn mock_handler(
    AxState(state): AxState<MockServerState>,
    req: axum::extract::Request,
) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = uri.path().to_string();
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let query_map: HashMap<String, String> = uri
        .query()
        .map(|q| {
            url::form_urlencoded::parse(q.as_bytes())
                .into_owned()
                .collect()
        })
        .unwrap_or_default();

    // 在锁内匹配路由并克隆出需要的数据，避免跨 await 持有非 Send 的锁守卫
    let matched: Option<(MockRoute, HashMap<String, String>)> = {
        let routes = state.routes.read().unwrap_or_else(|e| e.into_inner());
        let mut found = None;
        for route in routes.iter() {
            // 方法匹配：ANY 或完全一致；HEAD 也接受 GET 路由
            let method_ok = route.method == "ANY"
                || route.method == method.as_str()
                || (route.method == "GET" && method == Method::HEAD);
            if !method_ok {
                continue;
            }
            if route.segments.len() != segs.len() {
                continue;
            }
            let mut params: HashMap<String, String> = HashMap::new();
            let mut matched_segments = true;
            for (rs, ss) in route.segments.iter().zip(segs.iter()) {
                match rs {
                    Segment::Literal(l) => {
                        if l != ss {
                            matched_segments = false;
                            break;
                        }
                    }
                    Segment::Param(name) => {
                        let decoded = percent_decode_str(ss)
                            .decode_utf8_lossy()
                            .to_string();
                        params.insert(name.clone(), decoded);
                    }
                }
            }
            if matched_segments {
                found = Some((route.clone(), params));
                break;
            }
        }
        found
    };

    let Some((route, params)) = matched else {
        return (
            StatusCode::NOT_FOUND,
            axum::Json(json!({
                "code": 404,
                "message": "Mock 路由未匹配",
                "method": method.as_str(),
                "path": path
            })),
        )
            .into_response();
    };

    if route.delay_ms > 0 {
        tokio::time::sleep(Duration::from_millis(route.delay_ms)).await;
    }

    // 模板替换
    let mut body = route.body.clone();
    for (k, v) in &params {
        body = body.replace(&format!("{{{{path.{k}}}}}"), v);
    }
    for (k, v) in &query_map {
        body = body.replace(&format!("{{{{query.{k}}}}}"), v);
    }
    body = body.replace("{{method}}", method.as_str());
    body = body.replace("{{path}}", &path);

    // 全局环境变量 {{key}}
    body = apply_env_vars(&body, &state.envs.read().unwrap_or_else(|e| e.into_inner()));

    // 返回内容不为空：body 为空时给出提示（HEAD / 204 等本就无响应体的除外）
    let is_head = method == Method::HEAD || method == Method::OPTIONS;
    if body.trim().is_empty() && !is_head && route.status != 204 {
        body = format!(
            "{{\"code\":0,\"data\":{{}},\"message\":\"Mock 返回内容为空，请在接口的 Mock 页签填写响应内容\",\"path\":\"{}\"}}",
            path.replace('\\', "\\\\")
        );
    }

    let mut builder = Response::builder()
        .status(StatusCode::from_u16(route.status).unwrap_or(StatusCode::OK));
    let mut has_content_type = false;
    for (k, v) in &route.headers {
        if k.eq_ignore_ascii_case("content-type") {
            has_content_type = true;
        }
        builder = builder.header(k, v);
    }
    if !has_content_type {
        builder = builder.header("content-type", "application/json; charset=utf-8");
    }
    builder
        .body(Body::from(body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

#[cfg(test)]
mod tests {
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
}

use crate::tray::update_tray_mock_item;
use crate::{workspace_root, WorkspaceState};
use tauri::State;


#[tauri::command]
pub(crate) async fn mock_start(app: AppHandle, port: u16) -> Result<MockStatus, String> {
    let res = start_mock(&app, port).await;
    update_tray_mock_item(&app);
    res
}

#[tauri::command]
pub(crate) async fn mock_stop(app: AppHandle) -> Result<MockStatus, String> {
    stop_mock(&app);
    update_tray_mock_item(&app);
    Ok(status(&app))
}

#[tauri::command]
pub(crate) async fn mock_status(app: AppHandle) -> Result<MockStatus, String> {
    Ok(status(&app))
}

#[tauri::command]
pub(crate) async fn mock_reload(app: AppHandle) -> Result<MockStatus, String> {
    reload_mock(&app)?;
    Ok(status(&app))
}


#[tauri::command]
pub(crate) fn list_custom_mocks(
    state: State<'_, WorkspaceState>,
) -> Result<Vec<CustomMock>, String> {
    let root = workspace_root(&state)?;
    Ok(list_custom_mocks_impl(&root))
}

#[tauri::command]
pub(crate) fn save_custom_mock(
    state: State<'_, WorkspaceState>,
    input: CustomMock,
    old_name: Option<String>,
) -> Result<(), String> {
    let root = workspace_root(&state)?;
    save_custom_mock_impl(&root, &input, old_name.as_deref())
}

#[tauri::command]
pub(crate) fn delete_custom_mock(state: State<'_, WorkspaceState>, name: String) -> Result<(), String> {
    let root = workspace_root(&state)?;
    delete_custom_mock_impl(&root, &name)
}
