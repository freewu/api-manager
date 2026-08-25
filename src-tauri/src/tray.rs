//! 系统托盘：菜单构建、环境/Mock/语言/更新菜单项联动。

use crate::mock;
use crate::update::{fetch_latest_release, UpdateInfo, RELEASES_PAGE};
use crate::{load_settings, read_env_file, read_info_file, save_settings, MockRunState,
            TrayState, WorkspaceState};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};


fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// 当前激活环境名（从工作区 __envs.json 读取）
fn active_env_name(app: &AppHandle) -> String {
    let root = app
        .state::<WorkspaceState>()
        .root
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    match root {
        Some(r) => read_env_file(&r).active,
        None => String::new(),
    }
}

/// 当前设置语言（"zh" / "zh-tw" / "en"，读取 settings.json，兼容旧值）
fn settings_lang(app: &AppHandle) -> String {
    let lang = load_settings(app.clone()).unwrap_or_default().language;
    normalize_lang(&lang)
}

/// 归一化语言：旧配置 "" / "zh" / "en" 与新值 "zh-tw" 统一处理
fn normalize_lang(lang: &str) -> String {
    let l = lang.trim().to_lowercase().replace('_', "-");
    if l == "en" {
        "en".into()
    } else if l == "zh-tw" || l == "zh-hant" || l == "zh-cht" || l == "tw" || l == "cht" {
        "zh-tw".into()
    } else {
        "zh".into()
    }
}

/// 按语言取托盘文案（简体中文 / 繁體中文 / English）
fn tray_text(lang: &str, zh: &str, tw: &str, en: &str) -> String {
    if lang == "en" {
        en.into()
    } else if lang == "zh-tw" {
        tw.into()
    } else {
        zh.into()
    }
}

/// 更新托盘菜单中的环境变量菜单项文字
pub fn update_tray_env_item(app: &AppHandle) {
    let lang = settings_lang(app);
    let name = active_env_name(app);
    let text = if name.trim().is_empty() {
        tray_text(&lang, "环境：未设置（点击编辑）", "環境：未設置（點擊編輯）", "Env: unset (click to edit)")
    } else {
        tray_text(&lang, "环境：{name}（点击编辑）", "環境：{name}（點擊編輯）", "Env: {name} (click to edit)")
            .replace("{name}", name.trim())
    };
    let state = app.state::<TrayState>();
    let guard = state.env_item.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(item) = guard.as_ref() {
        let _ = item.set_text(&text);
    }
}

/// 前端保存/切换环境后同步托盘文字
#[tauri::command]
pub(crate) fn update_tray_env(app: AppHandle, name: String) {
    let lang = settings_lang(&app);
    let text = if name.trim().is_empty() {
        tray_text(&lang, "环境：未设置（点击编辑）", "環境：未設置（點擊編輯）", "Env: unset (click to edit)")
    } else {
        tray_text(&lang, "环境：{name}（点击编辑）", "環境：{name}（點擊編輯）", "Env: {name} (click to edit)")
            .replace("{name}", name.trim())
    };
    let state = app.state::<TrayState>();
    let guard = state.env_item.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(item) = guard.as_ref() {
        let _ = item.set_text(&text);
    }
}

/// 更新托盘菜单中 Mock 菜单项文字
pub fn update_tray_mock_item(app: &AppHandle) {
    let state = app.state::<TrayState>();
    let running = *app
        .state::<MockRunState>()
        .running
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let lang = settings_lang(app);
    let guard = state.mock_item.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(item) = guard.as_ref() {
        let _ = item.set_text(if running {
            tray_text(&lang, "停止 Mock 服务", "停止 Mock 服務", "Stop Mock Server")
        } else {
            tray_text(&lang, "启动 Mock 服务", "啟動 Mock 服務", "Start Mock Server")
        });
    }
}

/// 按当前语言刷新托盘菜单全部文字（语言切换后调用）
pub fn update_tray_language(app: &AppHandle) {
    let lang = settings_lang(app);
    let st = app.state::<TrayState>();
    if let Some(i) = st.show_item.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        let _ = i.set_text(&tray_text(&lang, "显示窗口", "顯示窗口", "Show Window"));
    }
    if let Some(i) = st.github_item.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        let _ = i.set_text(&tray_text(&lang, "GitHub 仓库", "GitHub 倉庫", "GitHub Repository"));
    }
    if let Some(i) = st.issue_item.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        let _ = i.set_text(&tray_text(&lang, "提交 Issue", "提交 Issue", "Submit Issue"));
    }
    if let Some(i) = st.quit_item.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        let _ = i.set_text(&tray_text(&lang, "退出", "退出", "Quit"));
    }
    // 语言子菜单标题 + 子项勾选态（单行入口，展开后勾选当前语言）
    if let Some(m) = st
        .lang_submenu
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
    {
        let _ = m.set_text(&tray_text(&lang, "语言", "語言", "Language"));
    }
    let set_checked = |item: &Mutex<Option<tauri::menu::CheckMenuItem<tauri::Wry>>>, active: bool| {
        if let Some(i) = item.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            let _ = i.set_checked(active);
        }
    };
    set_checked(&st.lang_zh_item, lang == "zh");
    set_checked(&st.lang_tw_item, lang == "zh-tw");
    set_checked(&st.lang_en_item, lang == "en");
    update_tray_env_item(app);
    update_tray_mock_item(app);
    update_tray_update_item(app);
}

/// 按当前语言刷新「检查更新」菜单项文字（无待提醒版本时显示默认文字）
pub fn update_tray_update_item(app: &AppHandle) {
    let lang = settings_lang(app);
    let st = app.state::<TrayState>();
    let pending = st
        .latest_version
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let guard = st.update_item.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(i) = guard.as_ref() {
        let text = match pending {
            Some(v) => tray_text(
                &lang,
                &format!("发现新版本 v{v}"),
                &format!("發現新版本 v{v}"),
                &format!("New version v{v} available"),
            ),
            None => tray_text(&lang, "检查更新", "檢查更新", "Check for Updates"),
        };
        let _ = i.set_text(&text);
    }
}

/// 切换界面语言：保存设置 + 刷新托盘菜单 + 通知前端刷新文案
#[tauri::command]
pub(crate) fn set_language(app: AppHandle, lang: String) -> Result<(), String> {
    let normalized = normalize_lang(&lang);
    let mut s = load_settings(app.clone())?;
    if s.language == normalized {
        // 已是当前语言，无需重复刷新
        return Ok(());
    }
    s.language = normalized.clone();
    save_settings(app.clone(), s)?;
    update_tray_language(&app);
    let _ = app.emit("language-changed", normalized);
    Ok(())
}

/// 托盘菜单：启动/停止 Mock 服务
fn tray_toggle_mock(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let status = mock::status(&app);
        if status.running {
            mock::stop_mock(&app);
        } else {
            // 从工作区 __info.json 读取端口，默认 5050
            let port = {
                let root = app
                    .state::<WorkspaceState>()
                    .root
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                match root {
                    Some(r) => read_info_file(&r).mock_port.unwrap_or(5050),
                    None => 5050,
                }
            };
            let _ = mock::start_mock(&app, port).await;
        }
        update_tray_mock_item(&app);
        // 托盘操作 Mock 后通知主页面刷新状态（启动/停止联动）
        let _ = app.emit("mock-status-changed", ());
    });
}

/// 标记发现新版本：刷新托盘菜单文字 + 记录版本号 + 通知前端弹窗提醒
fn mark_update_available(app: &AppHandle, info: &UpdateInfo) {
    *app.state::<TrayState>()
        .latest_version
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = Some(info.latest.clone());
    update_tray_update_item(app);
    let _ = app.emit("update-available", info);
}

/// 清除「发现新版本」状态，恢复默认「检查更新」文字
fn reset_update_item(app: &AppHandle) {
    *app.state::<TrayState>()
        .latest_version
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = None;
    update_tray_update_item(app);
}

/// 托盘菜单：检查更新（异步访问 GitHub Releases，发现新版本时提醒）
pub fn tray_check_update(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        match fetch_latest_release().await {
            Ok(info) if info.has_update => mark_update_available(&app, &info),
            // 已是最新或检查失败：恢复默认文字
            _ => reset_update_item(&app),
        }
    });
}

/// 创建系统托盘图标与菜单
pub(crate) fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::{CheckMenuItem, IconMenuItem, Menu, PredefinedMenuItem, Submenu};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    use tauri_plugin_opener::OpenerExt;

    app.manage(TrayState {
        mock_item: Mutex::new(None),
        env_item: Mutex::new(None),
        show_item: Mutex::new(None),
        github_item: Mutex::new(None),
        issue_item: Mutex::new(None),
        quit_item: Mutex::new(None),
        lang_submenu: Mutex::new(None),
        lang_zh_item: Mutex::new(None),
        lang_tw_item: Mutex::new(None),
        lang_en_item: Mutex::new(None),
        update_item: Mutex::new(None),
        latest_version: Mutex::new(None),
        exiting: AtomicBool::new(false),
    });

    // 托盘菜单图标：由 gen-tray-icons.mjs 生成的 16x16 单色图标
    let icon_info = tauri::image::Image::from_bytes(include_bytes!("../tray-icons/info.png"))?;
    let icon_window =
        tauri::image::Image::from_bytes(include_bytes!("../tray-icons/window.png"))?;
    let icon_env = tauri::image::Image::from_bytes(include_bytes!("../tray-icons/env.png"))?;
    let icon_mock = tauri::image::Image::from_bytes(include_bytes!("../tray-icons/mock.png"))?;
    let icon_github =
        tauri::image::Image::from_bytes(include_bytes!("../tray-icons/github.png"))?;
    let icon_issue = tauri::image::Image::from_bytes(include_bytes!("../tray-icons/issue.png"))?;
    let icon_quit = tauri::image::Image::from_bytes(include_bytes!("../tray-icons/quit.png"))?;

    let version = IconMenuItem::with_id(
        app,
        "tray_version",
        format!("API Manager v{}", env!("CARGO_PKG_VERSION")),
        false,
        Some(icon_info.clone()),
        None::<&str>,
    )?;
    let show = IconMenuItem::with_id(app, "show", "显示窗口", true, Some(icon_window), None::<&str>)?;
    let env_item = IconMenuItem::with_id(
        app,
        "edit_env",
        "环境：未设置（点击编辑）",
        true,
        Some(icon_env),
        None::<&str>,
    )?;
    let toggle_mock = IconMenuItem::with_id(
        app,
        "toggle_mock",
        "启动 Mock 服务",
        true,
        Some(icon_mock),
        None::<&str>,
    )?;
    let github = IconMenuItem::with_id(
        app,
        "open_github",
        "GitHub 仓库",
        true,
        Some(icon_github),
        None::<&str>,
    )?;
    let issue = IconMenuItem::with_id(
        app,
        "open_issue",
        "提交 Issue",
        true,
        Some(icon_issue),
        None::<&str>,
    )?;
    let quit = IconMenuItem::with_id(app, "quit", "退出", true, Some(icon_quit), None::<&str>)?;
    // 语言切换：单行「语言」子菜单，内含简体中文 / 繁體中文 / English 勾选项
    let lang_zh = CheckMenuItem::with_id(app, "lang_zh", "简体中文", true, false, None::<&str>)?;
    let lang_tw = CheckMenuItem::with_id(app, "lang_tw", "繁體中文", true, false, None::<&str>)?;
    let lang_en = CheckMenuItem::with_id(app, "lang_en", "English", true, false, None::<&str>)?;
    let lang_menu = Submenu::with_items(app, "语言", true, &[&lang_zh, &lang_tw, &lang_en])?;
    // 检查更新（异步访问 GitHub Releases；发现新版本时文字变为「发现新版本 vX.Y.Z」）
    let check_update = IconMenuItem::with_id(
        app,
        "check_update",
        "检查更新",
        true,
        Some(icon_info.clone()),
        None::<&str>,
    )?;
    let menu = Menu::with_items(
        app,
        &[
            &version,
            &PredefinedMenuItem::separator(app)?,
            &show,
            &PredefinedMenuItem::separator(app)?,
            &env_item,
            &PredefinedMenuItem::separator(app)?,
            &toggle_mock,
            &PredefinedMenuItem::separator(app)?,
            &check_update,
            &PredefinedMenuItem::separator(app)?,
            &github,
            &issue,
            &PredefinedMenuItem::separator(app)?,
            &lang_menu,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    *app.state::<TrayState>().mock_item.lock().unwrap() = Some(toggle_mock.clone());
    *app.state::<TrayState>().env_item.lock().unwrap() = Some(env_item.clone());
    *app.state::<TrayState>().show_item.lock().unwrap() = Some(show.clone());
    *app.state::<TrayState>().github_item.lock().unwrap() = Some(github.clone());
    *app.state::<TrayState>().issue_item.lock().unwrap() = Some(issue.clone());
    *app.state::<TrayState>().quit_item.lock().unwrap() = Some(quit.clone());
    *app.state::<TrayState>().lang_submenu.lock().unwrap() = Some(lang_menu.clone());
    *app.state::<TrayState>().lang_zh_item.lock().unwrap() = Some(lang_zh.clone());
    *app.state::<TrayState>().lang_tw_item.lock().unwrap() = Some(lang_tw.clone());
    *app.state::<TrayState>().lang_en_item.lock().unwrap() = Some(lang_en.clone());
    *app.state::<TrayState>().update_item.lock().unwrap() = Some(check_update.clone());
    // 用当前设置语言 + 工作区环境名刷新托盘文字
    update_tray_language(app.handle());

    TrayIconBuilder::with_id("main")
        // 使用项目 logo 生成的 32px 方形图标作为托盘图标（小尺寸显示更清晰）
        .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))?)
        .menu(&menu)
        .tooltip(format!("API Manager v{}", env!("CARGO_PKG_VERSION")))
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app),
            "edit_env" => {
                // 显示窗口并通知前端打开环境变量编辑器
                show_main_window(app);
                let _ = app.emit("open-env-editor", ());
            }
            "toggle_mock" => tray_toggle_mock(app),
            "check_update" => {
                // 已发现新版本时点击直接打开 GitHub 发布页；否则发起检查
                let pending = app
                    .state::<TrayState>()
                    .latest_version
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .is_some();
                if pending {
                    use tauri_plugin_opener::OpenerExt;
                    let _ = app.opener().open_url(RELEASES_PAGE, None::<&str>);
                } else {
                    tray_check_update(app);
                }
            }
            "open_github" => {
                // 打开项目 GitHub 仓库
                let _ = app
                    .opener()
                    .open_url("https://github.com/freewu/api-manager", None::<&str>);
            }
            "open_issue" => {
                // 快速跳转到新建 Issue 页面
                let _ = app
                    .opener()
                    .open_url("https://github.com/freewu/api-manager/issues/new", None::<&str>);
            }
            "quit" => {
                app.state::<TrayState>()
                    .exiting
                    .store(true, Ordering::Relaxed);
                app.exit(0);
            }
            "lang_zh" => {
                let _ = crate::set_language(app.clone(), "zh".to_string());
            }
            "lang_tw" => {
                let _ = crate::set_language(app.clone(), "zh-tw".to_string());
            }
            "lang_en" => {
                let _ = crate::set_language(app.clone(), "en".to_string());
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // 左键单击托盘图标 -> 显示窗口
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}
