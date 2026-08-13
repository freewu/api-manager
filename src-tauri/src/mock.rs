use crate::{ApiFile, MockRunState, MockStatus, INFO_FILE};
use axum::{
    body::Body,
    extract::State as AxState,
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    Router,
};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
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
