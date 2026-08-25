//! 检查更新（GitHub Releases）

use serde::Serialize;
use serde_json::Value;

// ==================== 检查更新（GitHub Releases） ====================

/// GitHub 仓库与发布页地址
pub(crate) const RELEASES_PAGE: &str = "https://github.com/freewu/api-manager/releases";
const LATEST_RELEASE_API: &str = "https://api.github.com/repos/freewu/api-manager/releases/latest";

/// 更新检查结果
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    /// 最新版本号（去掉 v 前缀，如 "0.2.0"）
    pub latest: String,
    /// 当前应用版本号
    pub current: String,
    /// 是否发现更新（latest > current）
    pub has_update: bool,
    /// 最新版本发布页地址
    pub url: String,
}

/// 解析版本号 "v0.1.5" / "0.1.5-beta" -> 数字段 [0, 1, 5]；忽略非数字部分
pub(crate) fn parse_version(v: &str) -> Vec<u32> {
    v.trim()
        .trim_start_matches(['v', 'V'])
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<u32>().ok())
        .collect()
}

/// 比较两个版本号，a 大于 b 返回 true（数值逐段比较，段数多的更大）
pub(crate) fn version_gt(a: &str, b: &str) -> bool {
    let pa = parse_version(a);
    let pb = parse_version(b);
    for (x, y) in pa.iter().zip(pb.iter()) {
        if x != y {
            return x > y;
        }
    }
    pa.len() > pb.len()
}

/// 异步访问 GitHub Releases API，获取最新版本号并判断是否有更新
pub(crate) async fn fetch_latest_release() -> Result<UpdateInfo, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("api-manager/update-check")
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    let resp = client
        .get(LATEST_RELEASE_API)
        .send()
        .await
        .map_err(|e| format!("访问 GitHub Releases 失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "GitHub Releases 接口返回 {}",
            resp.status().as_u16()
        ));
    }
    let json: Value = resp
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {e}"))?;
    let tag = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .trim_start_matches(['v', 'V'])
        .to_string();
    let current = env!("CARGO_PKG_VERSION").to_string();
    let has_update = !tag.is_empty() && version_gt(&tag, &current);
    let url = json
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or(RELEASES_PAGE)
        .to_string();
    Ok(UpdateInfo {
        latest: tag,
        current,
        has_update,
        url,
    })
}

/// 前端手动触发检查更新
#[tauri::command]
pub(crate) async fn check_update() -> Result<UpdateInfo, String> {
    fetch_latest_release().await
}
