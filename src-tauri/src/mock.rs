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
use chrono::{Datelike, Timelike};
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
    /// 工作区根目录（读取 .mock 自定义占位符）
    pub root: PathBuf,
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
    root.join(crate::MOCK_DATA_DIR)
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
        root,
    };

    let router: Router<MockServerState> = Router::new()
        .route("/mock-list", axum::routing::get(mock_list_handler))
        .fallback(mock_handler)
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
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
    // 本机局域网 IP（UDP connect 不发送数据，仅用于探测路由）；失败时回退 127.0.0.1
    let lan = std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("8.8.8.8:80")?;
            Ok(s.local_addr()?.ip().to_string())
        })
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    *run_state.addr.lock().unwrap() = Some(format!("http://{lan}:{}", addr.port()));
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

/// 路由路径字符串（/a/{param}/b，参数保留花括号形式）
fn route_path_str(route: &MockRoute) -> String {
    let mut p = String::from("/");
    for (i, seg) in route.segments.iter().enumerate() {
        if i > 0 {
            p.push('/');
        }
        match seg {
            Segment::Literal(l) => p.push_str(l),
            Segment::Param(n) => {
                p.push('{');
                p.push_str(n);
                p.push('}');
            }
        }
    }
    p
}

/// HTML 转义（mock-list 页面展示路径/方法，避免特殊字符破坏页面）
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// 内置接口 GET /mock-list：以 HTML 表格列出所有 Mock 路由（method 按类型着色）
async fn mock_list_handler(AxState(state): AxState<MockServerState>) -> Response {
    let routes = state.routes.read().unwrap_or_else(|e| e.into_inner());
    let rows: Vec<(String, String)> = routes
        .iter()
        .map(|r| (r.method.clone(), route_path_str(r)))
        .collect();
    drop(routes);
    let mut table = String::new();
    for (m, p) in &rows {
        // method 颜色与客户端一致（GET 绿 / POST 橙 / PUT 蓝 / DELETE 红 / PATCH 紫 / HEAD 青 / OPTIONS 灰）
        let color = match m.as_str() {
            "GET" => "#2ec27e",
            "POST" => "#f5a623",
            "PUT" => "#3b82f6",
            "DELETE" => "#f26d6d",
            "PATCH" => "#a855f7",
            "HEAD" => "#14b8a6",
            "OPTIONS" => "#7d8590",
            _ => "#8ab0e8",
        };
        table.push_str(&format!(
            "<tr><td><span class=\"m\" style=\"color:{color}\">{}</span></td><td>{}</td></tr>",
            escape_html(m),
            escape_html(p)
        ));
    }
    let html = format!(
        r#"<!doctype html>
<html lang="zh-CN"><head><meta charset="utf-8"/>
<title>Mock 路由列表</title>
<style>
body{{margin:0;padding:32px;background:#16171a;color:#e8e9eb;font-family:system-ui,-apple-system,"Segoe UI","Microsoft YaHei",sans-serif;}}
h1{{font-size:20px;font-weight:600;margin:0 0 6px;}}
p.sub{{color:#9ba0a8;font-size:13px;margin:0 0 20px;}}
table{{border-collapse:collapse;width:100%;max-width:720px;background:#1e1f22;border:1px solid #35383e;border-radius:8px;overflow:hidden;}}
th{{text-align:left;padding:10px 16px;font-size:12px;color:#9ba0a8;border-bottom:1px solid #35383e;}}
td{{padding:9px 16px;border-bottom:1px solid #2a2d31;font-family:Consolas,"Courier New",monospace;font-size:13px;}}
tr:last-child td{{border-bottom:none;}}
.m{{font-weight:700;}}
</style></head>
<body>
<h1>Mock 路由列表</h1>
<p class="sub">共 {} 条路由</p>
<table><thead><tr><th style="width:120px">Method</th><th>Path</th></tr></thead><tbody>{}</tbody></table>
</body></html>"#,
        rows.len(),
        table
    );
    (
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
        .into_response()
}

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

    // mock.js 占位符 + 自定义占位符渲染（响应体支持 @cname / @integer(1,100) / "list|1-5": […] 等语法）
    let customs = list_custom_mocks_impl(&state.root);
    body = render_mock_body(&body, &customs);

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

// ==================== mock.js 响应体渲染（Mock 服务返回时生效） ====================
// 支持：字符串值内 @占位符（含参数、自定义占位符）；键规则 key|count / key|min-max / key|min-max.d / key|1 / key|+step。
// 与前端 src/utils/mockData.ts 的 renderMockBody 行为保持一致。

fn mock_rnd_u64() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static STATE: AtomicU64 = AtomicU64::new(0);
    let mut x = STATE.load(Ordering::Relaxed);
    if x == 0 {
        x = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9e37_79b9_7f4a_7c15);
    }
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    STATE.store(x, Ordering::Relaxed);
    x
}

fn mock_rnd() -> f64 {
    (mock_rnd_u64() >> 11) as f64 / (1u64 << 53) as f64
}

fn mock_rand_range(a: i64, b: i64) -> i64 {
    if b <= a {
        return a;
    }
    a + (mock_rnd_u64() % ((b - a + 1) as u64)) as i64
}

fn mock_pick<T: Clone>(arr: &[T]) -> T {
    if arr.is_empty() {
        panic!("mock_pick empty");
    }
    arr[mock_rand_range(0, arr.len() as i64 - 1) as usize].clone()
}

fn mock_shuffle<T>(arr: &mut [T]) {
    for i in (1..arr.len()).rev() {
        let j = mock_rand_range(0, i as i64) as usize;
        arr.swap(i, j);
    }
}

const SURNAMES: &[&str] = &[
    "赵", "钱", "孙", "李", "周", "吴", "郑", "王", "冯", "陈", "褚", "卫", "蒋", "沈", "韩", "杨", "朱", "秦",
    "许", "何", "吕", "施", "张", "孔", "曹", "严", "华", "金", "魏", "陶", "姜", "戚", "谢", "邹", "喻", "柏",
    "水", "窦", "章", "云", "苏", "潘", "葛", "奚", "范", "彭", "郎", "鲁", "韦", "昌", "马", "苗", "凤", "花",
    "方", "俞", "任", "袁", "柳", "酆", "鲍", "史", "唐", "费", "廉", "岑", "薛", "雷", "贺", "倪", "汤", "滕",
    "殷", "罗", "毕", "郝", "邬", "安", "常", "乐", "于", "时", "傅", "皮", "卞", "齐", "康", "伍", "余", "元",
    "卜", "顾", "孟", "平", "黄", "和", "穆", "萧", "尹", "姚", "邵", "湛", "汪", "祁", "毛", "禹", "狄", "米",
    "贝", "明", "臧", "计", "伏", "成", "戴", "谈", "宋", "茅", "庞", "熊", "纪", "舒", "屈", "项", "祝", "董",
    "梁", "杜", "阮", "蓝", "闵", "席", "季", "麻", "强", "贾", "路", "娄", "危", "江", "童", "颜", "郭", "梅",
    "盛", "林", "刁", "钟", "徐", "邱", "骆", "高", "夏", "蔡", "田", "樊", "胡", "凌", "霍", "虞", "万", "支",
    "柯", "昝", "管", "卢", "莫", "经", "房", "裘", "缪", "干", "解", "应", "宗", "丁", "宣", "贲", "邓", "郁",
    "单", "杭", "洪", "包", "诸", "左", "石", "崔", "吉", "钮", "龚", "程", "嵇", "邢", "滑", "裴", "陆", "荣",
    "翁", "荀", "羊", "於", "惠", "甄", "麹", "家", "封", "芮", "羿", "储", "靳", "汲", "邴", "糜", "松", "井",
    "段", "富", "巫", "乌", "焦", "巴", "弓", "牧", "隗", "山", "谷", "车", "侯", "宓", "蓬", "全", "郗", "班",
    "仰", "秋", "仲", "伊", "宫", "宁", "仇", "栾", "暴", "甘", "斜", "厉", "戎", "祖", "武", "符", "刘", "景",
    "詹", "束", "龙", "叶", "幸", "司", "韶", "郜", "黎", "蓟", "薄", "印", "宿", "白", "怀", "蒲", "邰", "从",
    "鄂", "索", "咸", "籍", "赖", "卓", "蔺", "屠", "蒙", "池", "乔", "阴", "郁", "胥", "能", "苍", "双", "闻",
    "莘", "党", "翟", "谭", "贡", "劳", "逄", "姬", "申", "扶", "堵", "冉", "宰", "郦", "雍", "却", "璩", "桑",
    "桂", "濮", "牛", "寿", "通", "边", "扈", "燕", "冀", "郏", "浦", "尚", "农", "温", "别", "庄", "晏", "柴",
    "瞿", "阎", "充", "慕", "连", "茹", "习", "宦", "艾", "鱼", "容", "向", "古", "易", "慎", "戈", "廖", "庾",
    "终", "暨", "居", "衡", "步", "都", "耿", "满", "弘", "匡", "国", "文", "寇", "广", "禄", "阙", "东", "欧",
    "殳", "沃", "利", "蔚", "越", "夔", "隆", "师", "巩", "厍", "聂", "晁", "勾", "敖", "融", "冷", "訾", "辛",
    "阚", "那", "简", "饶", "空", "曾", "毋", "沙", "乜", "养", "鞠", "须", "丰", "巢", "关", "蒯", "相", "查",
    "后", "荆", "红", "游", "竺", "权", "逯", "盖", "益", "桓", "公",
];
const GIVEN: &[&str] = &[
    "伟", "刚", "勇", "毅", "俊", "峰", "强", "军", "平", "保", "东", "文", "辉", "力", "明", "永", "健", "世",
    "广", "志", "义", "兴", "良", "海", "山", "仁", "波", "宁", "贵", "福", "生", "龙", "元", "全", "国", "胜",
    "学", "祥", "才", "发", "武", "新", "利", "清", "飞", "彬", "富", "顺", "信", "子", "杰", "涛", "昌", "成",
    "康", "光", "星", "天", "达", "安", "岩", "中", "茂", "进", "林", "有", "坚", "和", "彪", "博", "诚", "先",
    "敬", "震", "振", "壮", "会", "思", "群", "豪", "心", "邦", "承", "乐", "绍", "功", "松", "善", "厚", "庆",
    "磊", "民", "友", "裕", "河", "哲", "江", "超", "浩", "亮", "政", "谦", "亨", "奇", "固", "之", "轮", "翰",
    "朗", "伯", "宏", "言", "若", "鸣", "朋", "斌", "梁", "栋", "维", "启", "克", "伦", "翔", "旭", "鹏", "泽",
    "晨", "辰", "士", "以", "建", "家", "致", "树", "炎", "德", "行", "时", "泰", "盛", "雄", "琛", "钧", "冠",
    "策", "腾", "楠", "榕", "风", "航", "弘",
];
const CITIES: &[&str] = &[
    "北京", "上海", "广州", "深圳", "杭州", "成都", "武汉", "西安", "南京", "天津", "重庆", "苏州", "长沙", "郑州",
    "青岛", "大连", "宁波", "厦门", "福州", "济南", "合肥", "昆明", "哈尔滨", "沈阳", "长春", "石家庄", "太原",
    "南昌", "无锡", "温州", "兰州", "南宁", "贵阳", "海口", "银川", "西宁", "呼和浩特", "拉萨", "乌鲁木齐",
];
const PROVINCES: &[&str] = &[
    "北京", "上海", "天津", "重庆", "河北", "山西", "辽宁", "吉林", "黑龙江", "江苏", "浙江", "安徽", "福建", "江西",
    "山东", "河南", "湖北", "湖南", "广东", "海南", "四川", "贵州", "云南", "陕西", "甘肃", "青海", "内蒙古", "广西",
    "西藏", "宁夏", "新疆",
];
const WORDS: &[&str] = &[
    "apple", "banana", "cloud", "data", "element", "field", "group", "house", "image", "jacket", "kernel", "light",
    "model", "node", "object", "pixel", "query", "river", "system", "table", "unit", "value", "window", "yield",
    "zone", "alpha", "beta", "delta", "gamma", "lambda",
];
const DOMAINS: &[&str] = &[
    "example.com", "test.com", "mail.com", "demo.net", "sample.org", "api.com", "cloud.io", "data.cn",
];
const FIRST_NAMES: &[&str] = &[
    "James", "John", "Robert", "Michael", "William", "David", "Richard", "Joseph", "Thomas", "Charles", "Mary",
    "Patricia", "Jennifer", "Linda", "Elizabeth", "Barbara", "Susan", "Jessica", "Sarah", "Karen", "Emma",
    "Olivia", "Liam", "Noah", "Ethan", "Aiden", "Lucas", "Mason", "Logan", "Daniel",
];
const LAST_NAMES: &[&str] = &[
    "Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis", "Rodriguez", "Martinez",
    "Lee", "Chen", "Wang", "Li", "Zhang", "Liu", "Yang", "Huang", "Zhao", "Wu",
];
const PROTOCOLS: &[&str] = &["http", "https", "ws", "wss", "ftp"];

fn mock_date_str(t: Option<chrono::DateTime<chrono::Utc>>) -> String {
    let dt = t.unwrap_or_else(|| {
        let now = chrono::Utc::now();
        let start = chrono::DateTime::from_timestamp(0, 0).unwrap_or(now);
        let secs = mock_rand_range(0, now.timestamp() - start.timestamp());
        chrono::DateTime::from_timestamp(secs, 0).unwrap_or(now)
    });
    format!("{:04}-{:02}-{:02}", dt.year(), dt.month(), dt.day())
}

fn mock_time_str(t: Option<chrono::DateTime<chrono::Utc>>) -> String {
    let dt = t.unwrap_or_else(chrono::Utc::now);
    format!("{:02}:{:02}:{:02}", dt.hour(), dt.minute(), dt.second())
}

fn mock_datetime_str(t: Option<chrono::DateTime<chrono::Utc>>) -> String {
    let dt = t.unwrap_or_else(chrono::Utc::now);
    format!("{} {}", mock_date_str(Some(dt)), mock_time_str(Some(dt)))
}

fn mock_guid() -> String {
    let b: Vec<u8> = (0..16).map(|_| mock_rand_range(0, 255) as u8).collect();
    let h = |n: u8| format!("{:02x}", n);
    format!(
        "{}{}{}{}-{}{}-{}{}-{}{}-{}{}{}{}{}{}",
        h(b[0]), h(b[1]), h(b[2]), h(b[3]), h(b[4]), h(b[5]), h(b[6]), h(b[7]),
        h(b[8]), h(b[9]), h(b[10]), h(b[11]), h(b[12]), h(b[13]), h(b[14]), h(b[15])
    )
}

fn mock_email() -> String {
    format!("{}@{}", format!("{}{}", mock_pick(WORDS), mock_rand_range(1, 999)), mock_pick(DOMAINS))
}

fn mock_id_card() -> String {
    let base = format!(
        "{}{:04}{:02}{:02}{:03}",
        mock_rand_range(110000, 659000),
        mock_rand_range(1900, 2023),
        mock_rand_range(1, 12),
        mock_rand_range(1, 28),
        mock_rand_range(0, 999)
    );
    format!("{}{}", base, mock_rand_range(0, 9))
}

fn mock_cname() -> String {
    let sur = mock_pick(SURNAMES);
    let given = if mock_rnd() < 0.5 {
        mock_pick(GIVEN).to_string()
    } else {
        format!("{}{}", mock_pick(GIVEN), mock_pick(GIVEN))
    };
    format!("{}{}", sur, given)
}

/// 内置 mock.js 占位符生成（args 为括号内参数文本，如 "1, 100"）
fn builtin_mock_value(name: &str, args: &str) -> Option<String> {
    let arg_num = |i: usize, def: i64| -> i64 {
        let v = args.split(',').nth(i).map(|x| x.trim()).unwrap_or("");
        if v.is_empty() {
            def
        } else {
            v.parse::<i64>().unwrap_or(def)
        }
    };
    let s = match name {
        "cname" => mock_cname(),
        "name" => format!("{} {}", mock_pick(FIRST_NAMES), mock_pick(LAST_NAMES)),
        "first" => mock_pick(FIRST_NAMES).to_string(),
        "last" => mock_pick(LAST_NAMES).to_string(),
        "email" => mock_email(),
        "phone" => format!(
            "1{}{:09}",
            mock_pick(&[3, 4, 5, 6, 7, 8, 9]),
            mock_rand_range(0, 999_999_999)
        ),
        "id" => mock_id_card(),
        "guid" => mock_guid(),
        "integer" => {
            let a = arg_num(0, 0);
            let b = arg_num(1, 10000);
            mock_rand_range(a.min(b), a.max(b)).to_string()
        }
        "natural" => {
            let a = arg_num(0, 0);
            let b = arg_num(1, 1000);
            mock_rand_range(a.min(b), a.max(b)).to_string()
        }
        "float" => {
            let a = arg_num(0, 0) as f64;
            let b = arg_num(1, 100) as f64;
            let dp = arg_num(2, 2) as usize;
            let v = a + mock_rnd() * (b - a);
            format!("{:.dp$}", v, dp = dp)
        }
        "boolean" => (mock_rnd() < 0.5).to_string(),
        "date" => mock_date_str(None),
        "time" => mock_time_str(None),
        "datetime" => mock_datetime_str(None),
        "now" => mock_datetime_str(Some(chrono::Utc::now())),
        "url" => format!(
            "https://www.{}/{}/{}",
            mock_pick(DOMAINS),
            mock_pick(WORDS),
            mock_rand_range(1, 999)
        ),
        "domain" => mock_pick(DOMAINS).to_string(),
        "ip" => format!(
            "{}.{}.{}.{}",
            mock_rand_range(1, 223),
            mock_rand_range(0, 255),
            mock_rand_range(0, 255),
            mock_rand_range(1, 254)
        ),
        "protocol" => mock_pick(PROTOCOLS).to_string(),
        "city" => mock_pick(CITIES).to_string(),
        "province" => mock_pick(PROVINCES).to_string(),
        "county" => format!(
            "{}市{}{}",
            mock_pick(CITIES),
            mock_pick(&["东", "西", "南", "北", "新", "老"]),
            mock_pick(&["城区", "区", "县", "镇"])
        ),
        "zip" => mock_rand_range(100000, 999999).to_string(),
        "word" => mock_pick(WORDS).to_string(),
        "title" => {
            let mut w = format!("{} {}", mock_pick(WORDS), mock_pick(WORDS));
            let c = w.chars().next().unwrap_or(' ');
            w.replace_range(..c.len_utf8(), &c.to_uppercase().to_string());
            w
        }
        "sentence" => format!(
            "{} {} {} {}.",
            mock_pick(WORDS),
            mock_pick(WORDS),
            mock_pick(WORDS),
            mock_pick(WORDS)
        ),
        "paragraph" => {
            let n = mock_rand_range(2, 4);
            let mut sents = String::new();
            for _ in 0..n {
                sents.push_str(&format!(
                    "{} {} {}. ",
                    mock_pick(WORDS),
                    mock_pick(WORDS),
                    mock_pick(WORDS)
                ));
            }
            sents.trim().to_string()
        }
        "color" => format!(
            "#{:02x}{:02x}{:02x}",
            mock_rand_range(0, 255),
            mock_rand_range(0, 255),
            mock_rand_range(0, 255)
        ),
        "image" => format!(
            "https://picsum.photos/seed/{}/400/300",
            mock_guid().replace('-', "")
        ),
        "avatar" => format!("https://i.pravatar.cc/150?u={}", mock_guid().replace('-', "")),
        "string" => {
            let n = mock_rand_range(1, arg_num(0, 8).max(1)) as usize;
            let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
            (0..n).map(|_| chars.as_bytes()[mock_rand_range(0, chars.len() as i64 - 1) as usize] as char).collect()
        }
        "character" => {
            let chars = "abcdefghijklmnopqrstuvwxyz0123456789";
            (chars.as_bytes()[mock_rand_range(0, chars.len() as i64 - 1) as usize] as char).to_string()
        }
        _ => return None,
    };
    Some(s)
}

/// 执行自定义占位符 JS 代码：代码为 (ctx) => 返回值 的函数（boa 纯 Rust JS 引擎）。
/// 完整支持 ctx.randInt / ctx.pick / ctx.random / ctx.pad / ctx.seq；失败返回 None（保留原样）。
fn run_custom_mock_code(code: &str) -> Option<String> {
    let mut ctx = boa_engine::Context::default();
    let src = format!(
        "const ctx = {{ randInt: (a, b) => a + Math.floor(Math.random() * (b - a + 1)), pick: (arr) => arr[Math.floor(Math.random() * arr.length)], random: Math.random, pad: (n) => String(n).padStart(2, '0'), seq: (() => {{ let i = 0; return () => Date.now().toString(36) + (i++).toString(36); }})() }}; (() => {{ const v = ({0})(ctx); if (typeof v === 'string') return v; if (v === undefined || v === null) return ''; return JSON.stringify(v); }})()",
        code
    );
    let v = ctx
        .eval(boa_engine::Source::from_bytes(src.as_bytes()))
        .ok()?;
    let s = v.to_string(&mut ctx).ok()?;
    Some(s.to_std_string_escaped())
}

/// 字符串值：替换其中的 @占位符（未命中内置 / 未启用自定义的保留原样）
fn render_mock_string(s: &str, customs: &[CustomMock]) -> String {
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        let (_, c) = chars[i];
        if c != '@' {
            out.push(c);
            i += 1;
            continue;
        }
        // 解析名字
        let mut j = i + 1;
        let mut name = String::new();
        while j < chars.len() && (chars[j].1.is_ascii_alphanumeric() || chars[j].1 == '_') {
            name.push(chars[j].1);
            j += 1;
        }
        if name.is_empty() {
            out.push('@');
            i += 1;
            continue;
        }
        // 可选参数 (...)
        let mut args = "";
        let mut end = j;
        if j < chars.len() && chars[j].1 == '(' {
            let mut close = j + 1;
            while close < chars.len() && chars[close].1 != ')' {
                close += 1;
            }
            if close < chars.len() {
                let arg_start = chars[j].0 + 1;
                let arg_end = chars[close].0;
                args = &s[arg_start..arg_end];
                end = close + 1;
            }
        }
        let whole_start = chars[i].0;
        let whole_end = chars[end - 1].0 + chars[end - 1].1.len_utf8();
        let whole = &s[whole_start..whole_end];
        if let Some(v) = builtin_mock_value(&name, args) {
            out.push_str(&v);
        } else if let Some(cus) = customs
            .iter()
            .find(|c| c.enabled && c.name == name && !c.code.trim().is_empty())
        {
            if let Some(v) = run_custom_mock_code(&cus.code) {
                out.push_str(&v);
            } else {
                out.push_str(whole);
            }
        } else {
            out.push_str(whole);
        }
        i = end;
    }
    out
}

/// 解析键规则 "key|count" / "key|min-max" / "key|min-max.d"
fn parse_key_rule(key: &str) -> (String, String) {
    match key.find('|') {
        Some(i) => (key[..i].to_string(), key[i + 1..].to_string()),
        None => (key.to_string(), String::new()),
    }
}

/// 解析 min-max / min-max.d 范围规则；非范围返回 None
fn parse_mock_range(rule: &str) -> Option<(i64, i64, i32)> {
    let dash = rule.find('-')?;
    let a: i64 = rule[..dash].trim().parse().ok()?;
    let rest = &rule[dash + 1..];
    let (b, dp) = match rest.find('.') {
        Some(p) => (rest[..p].trim().parse::<i64>().ok()?, (rest[p + 1..].len()) as i32),
        None => (rest.trim().parse::<i64>().ok()?, 0),
    };
    Some((a.min(b), a.max(b), dp))
}

/// 按键规则渲染单个字段值
fn render_mock_rule(rule: &str, val: &serde_json::Value, customs: &[CustomMock]) -> serde_json::Value {
    use serde_json::Value;
    if rule.is_empty() {
        return render_mock_value(val, customs);
    }
    match val {
        Value::Array(arr) => {
            if rule == "1" {
                if arr.is_empty() {
                    return Value::Array(vec![]);
                }
                return render_mock_value(&mock_pick(arr), customs);
            }
            let n = parse_mock_range(rule)
                .map(|(a, b, _)| mock_rand_range(a, b) as usize)
                .or_else(|| rule.parse::<usize>().ok());
            if let Some(n) = n {
                if arr.is_empty() {
                    return Value::Array(vec![]);
                }
                // mock.js 语义：生成 n 个元素，每个从模板数组随机选取（可重复）并渲染
                let mut out = Vec::with_capacity(n);
                for _ in 0..n {
                    out.push(render_mock_value(&mock_pick(arr), customs));
                }
                return Value::Array(out);
            }
            render_mock_value(val, customs)
        }
        Value::String(s) => {
            let n = parse_mock_range(rule)
                .map(|(a, b, _)| mock_rand_range(a, b) as usize)
                .or_else(|| rule.parse::<usize>().ok());
            if let Some(n) = n {
                return Value::String(render_mock_string(s, customs).repeat(n));
            }
            Value::String(render_mock_string(s, customs))
        }
        Value::Number(_) => {
            if let Some((a, b, dp)) = parse_mock_range(rule) {
                if dp > 0 {
                    let v = a as f64 + mock_rnd() * (b - a) as f64;
                    return serde_json::json!(v);
                }
                return serde_json::json!(mock_rand_range(a, b));
            }
            if rule.starts_with('+') {
                return val.clone();
            }
            val.clone()
        }
        Value::Object(map) => {
            let (n, is_rule) = match parse_mock_range(rule) {
                Some((a, b, _)) => (mock_rand_range(a, b) as usize, true),
                None => match rule.parse::<usize>() {
                    Ok(c) => (c, true),
                    Err(_) => (0, false),
                },
            };
            if is_rule {
                let mut keys: Vec<&String> = map.keys().collect();
                mock_shuffle(&mut keys);
                keys.truncate(n);
                let mut out = serde_json::Map::new();
                for k in keys {
                    let (base, rr) = parse_key_rule(k);
                    out.insert(base, render_mock_rule(&rr, &map[k], customs));
                }
                return Value::Object(out);
            }
            render_mock_value(val, customs)
        }
        Value::Bool(_) => Value::Bool(mock_rnd() < 0.5),
        _ => render_mock_value(val, customs),
    }
}

/// 递归渲染 JSON 值
fn render_mock_value(v: &serde_json::Value, customs: &[CustomMock]) -> serde_json::Value {
    use serde_json::Value;
    match v {
        Value::String(s) => Value::String(render_mock_string(s, customs)),
        Value::Array(arr) => Value::Array(arr.iter().map(|x| render_mock_value(x, customs)).collect()),
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, val) in map {
                let (base, rule) = parse_key_rule(k);
                out.insert(base, render_mock_rule(&rule, val, customs));
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

/// 渲染 mock 响应体：JSON 解析失败则原样返回（由调用方决定是否提示）
pub fn render_mock_body(body: &str, customs: &[CustomMock]) -> String {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else {
        return body.to_string();
    };
    let out = render_mock_value(&v, customs);
    serde_json::to_string_pretty(&out).unwrap_or_else(|_| body.to_string())
}

#[cfg(test)]
#[path = "mock_test.rs"]
mod tests;
