//! 请求测试（HTTP 发送），含文件上传/二进制/表单等。

use crate::{HttpRequestData, HttpResult};
use std::time::Instant;
use tauri::AppHandle;


fn decode_body(bytes: &[u8], headers: &[(String, String)]) -> String {
    let charset = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .and_then(|(_, v)| v.split(';').nth(1))
        .and_then(|s| s.trim().strip_prefix("charset="))
        .map(|s| s.trim().trim_matches('"').to_lowercase());
    match charset.as_deref() {
        Some("gbk") | Some("gb2312") | Some("gb18030") | Some("cp936") | Some("gbk-2312") => {
            let (cow, _, _) = encoding_rs::GBK.decode(bytes);
            cow.to_string()
        }
        Some("big5") => {
            let (cow, _, _) = encoding_rs::BIG5.decode(bytes);
            cow.to_string()
        }
        Some("latin1") | Some("iso-8859-1") | Some("windows-1252") => {
            let (cow, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
            cow.to_string()
        }
        Some("utf-8") | Some("utf8") | None => String::from_utf8_lossy(bytes).to_string(),
        Some(other) => {
            let (cow, _, _) = encoding_rs::Encoding::for_label(other.as_bytes())
                .map(|enc| enc.decode(bytes))
                .unwrap_or_else(|| encoding_rs::UTF_8.decode(bytes));
            cow.to_string()
        }
    }
}

#[tauri::command]
pub(crate) fn pick_file(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app.dialog().file().blocking_pick_file();
    Ok(picked
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().to_string()))
}

#[tauri::command]
pub(crate) async fn send_request(req: HttpRequestData) -> Result<HttpResult, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(req.timeout_ms.max(1000)))
        .redirect(reqwest::redirect::Policy::limited(10))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("创建客户端失败: {e}"))?;

    let method =
        reqwest::Method::from_bytes(req.method.as_bytes()).map_err(|e| format!("非法请求方法: {e}"))?;
    let mut rb = client.request(method, &req.url);

    for h in req.headers.iter().filter(|h| h.enabled && !h.key.trim().is_empty()) {
        rb = rb.header(h.key.trim(), h.value.trim());
    }
    // 表单（含文件字段）：multipart/form-data；否则按原始 body 发送
    if let Some(form) = &req.form {
        if !form.is_empty() {
            let mut mp = reqwest::multipart::Form::new();
            for f in form.iter().filter(|f| f.enabled && !f.key.trim().is_empty()) {
                if f.is_file {
                    let path = f.value.trim();
                    if path.is_empty() {
                        return Err(format!("表单文件字段 [{}] 未选择文件", f.key.trim()));
                    }
                    let bytes = tokio::fs::read(path)
                        .await
                        .map_err(|e| format!("读取文件失败 [{}]: {e}", path))?;
                    let fname = std::path::Path::new(path)
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "file".to_string());
                    mp = mp.part(
                        f.key.trim().to_string(),
                        reqwest::multipart::Part::bytes(bytes).file_name(fname),
                    );
                } else {
                    mp = mp.text(f.key.trim().to_string(), f.value.clone());
                }
            }
            rb = rb.multipart(mp);
        }
    } else if let Some(path) = &req.body_file {
        // 二进制模式：读取本地文件字节作为请求体
        if !path.trim().is_empty() {
            let bytes = tokio::fs::read(path.trim())
                .await
                .map_err(|e| format!("读取文件失败 [{path}]: {e}"))?;
            let has_ct = req
                .headers
                .iter()
                .any(|h| h.enabled && h.key.eq_ignore_ascii_case("content-type"));
            if !has_ct {
                rb = rb.header("Content-Type", "application/octet-stream");
            }
            rb = rb.body(bytes);
        }
    } else if let Some(body) = &req.body {
        if !body.is_empty() {
            let has_ct = req
                .headers
                .iter()
                .any(|h| h.enabled && h.key.eq_ignore_ascii_case("content-type"));
            if !has_ct {
                rb = rb.header("Content-Type", "application/json; charset=utf-8");
            }
            rb = rb.body(body.clone());
        }
    }

    let start = Instant::now();
    match rb.send().await {
        Ok(resp) => {
            let status = resp.status();
            let headers: Vec<(String, String)> = resp
                .headers()
                .iter()
                .map(|(k, v)| {
                    (
                        k.as_str().to_string(),
                        v.to_str().unwrap_or("").to_string(),
                    )
                })
                .collect();
            let bytes = resp.bytes().await.unwrap_or_default();
            let time_ms = start.elapsed().as_millis() as u64;
            let text = decode_body(&bytes, &headers);
            Ok(HttpResult {
                ok: true,
                status: status.as_u16(),
                status_text: status
                    .canonical_reason()
                    .unwrap_or("")
                    .to_string(),
                headers,
                body: text,
                time_ms,
                size: bytes.len(),
                url: req.url.clone(),
                error: None,
            })
        }
        Err(e) => {
            let time_ms = start.elapsed().as_millis() as u64;
            let err = if e.is_timeout() {
                "请求超时".to_string()
            } else if e.is_connect() {
                format!("连接失败: {e}")
            } else if e.is_builder() {
                // URL 无法解析为合法地址（缺少 http(s):// 前缀，或包含未替换的 {{变量}}）
                format!("URL 格式不正确: {}（请检查是否缺少 http:// 前缀或存在未替换的 {{变量}}）", req.url)
            } else {
                e.to_string()
            };
            Ok(HttpResult {
                ok: false,
                status: 0,
                status_text: String::new(),
                headers: vec![],
                body: String::new(),
                time_ms,
                size: 0,
                url: req.url.clone(),
                error: Some(err),
            })
        }
    }
}
