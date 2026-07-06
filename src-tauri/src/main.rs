//! Tauri 入口：注册前端可调用的 commands，串联 auth/api/ws 三个模块。
//!
//! 闭环：
//! 1. device flow 登录 -> 拿账号 + 全部 Space -> 一次性订阅全部 Space 的实时事件。
//! 2. 把结构化事件（auth-status / spaces / subscription-status / turn / activity）转发给前端。
//! 3. turn.finalized 时发一条系统通知（push）。
//! 4. 启动时若本地有可用 token，自动恢复订阅。
//! 5. 云盘挂载为 Roadmap，命令存在但返回未实现错误，前端如实展示。

mod api;
mod auth;
mod settings;
mod tray;
mod ws;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{Emitter, Manager};

// ---------- 事件 payload ----------

#[derive(Debug, Serialize, Clone)]
struct LogEvent {
    message: String,
}

#[derive(Debug, Serialize, Clone)]
struct AccountInfo {
    uuid: String,
    display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    avatar_url: Option<String>,
}

#[derive(Debug, Serialize, Clone, Default)]
struct AuthStatus {
    phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    verification_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    verification_uri_complete: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    account: Option<AccountInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct SpacesEvent {
    spaces: Vec<api::SpaceListItem>,
}

#[derive(Debug, Serialize, Clone)]
struct SubscriptionStatus {
    phase: String,
    space_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

// ---------- 应用状态 ----------

struct AppState {
    account: Mutex<Option<AccountInfo>>,
    subscription: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    /// 尚未被用户查看的完结对话数（用于托盘红点）。
    unread_finalized: AtomicUsize,
    /// 当前账号可见的全部 Space（用于 only_my_spaces 过滤时查 owner）。
    spaces: Mutex<Vec<api::SpaceListItem>>,
}

// ---------- emit 辅助 ----------

fn emit_log(app: &tauri::AppHandle, message: impl Into<String>) {
    let message = message.into();
    println!("{message}");
    let _ = app.emit("cohub-log", LogEvent { message });
}

fn emit_auth(app: &tauri::AppHandle, status: AuthStatus) {
    let _ = app.emit("auth-status", status);
}

fn emit_spaces(app: &tauri::AppHandle, spaces: Vec<api::SpaceListItem>) {
    let _ = app.emit("spaces", SpacesEvent { spaces });
}

fn emit_subscription(app: &tauri::AppHandle, status: SubscriptionStatus) {
    let _ = app.emit("subscription-status", status);
}

/// 构造网页端 session 深链：https://cohub.run/spaces/{spaceId}/sessions/{sessionId}?turn={sequence}
fn session_url(
    space_id: &Option<String>,
    session_id: &Option<String>,
    sequence: Option<u64>,
) -> Option<String> {
    let (space, session) = (space_id.as_ref()?, session_id.as_ref()?);
    let mut url = format!("https://cohub.run/spaces/{space}/sessions/{session}");
    if let Some(seq) = sequence {
        url.push_str(&format!("?turn={seq}"));
    }
    Some(url)
}

/// 截断文本并加省略号。
fn truncate_preview(text: &str, max: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max {
        trimmed.to_string()
    } else {
        let end = trimmed
            .char_indices()
            .take(max)
            .last()
            .map(|(i, _)| i)
            .unwrap_or(max);
        format!("{}…", &trimmed[..end])
    }
}

/// 发送一条可点击的系统通知：点击跳转到网页端对应 session。
/// notify-rust 的 default action 在 Linux 对应点击通知体；wait_for_action 阻塞，故起独立线程。
fn show_finalized_notification(fin: &ws::TurnFinalized) {
    let url = session_url(&fin.space_id, &fin.session_id, fin.sequence);
    let summary_label = format!(
        "{} step{} · {} tool{}",
        fin.message_count,
        if fin.message_count == 1 { "" } else { "s" },
        fin.tool_call_count,
        if fin.tool_call_count == 1 { "" } else { "s" },
    );
    let body = match fin.last_text.as_ref() {
        Some(t) if !t.is_empty() => truncate_preview(t, 100),
        _ => summary_label,
    };
    let has_link = url.is_some();
    let url = url.unwrap_or_default();

    std::thread::spawn(move || {
        let mut n = notify_rust::Notification::new();
        n.summary("Cohub · 对话完成").body(&body);
        if has_link {
            n.action("default", "在网页打开");
        }
        if let Ok(handle) = n.show() {
            handle.wait_for_action(|action| {
                if action == "default" && !url.is_empty() {
                    let _ = open::that(&url);
                }
            });
        }
    });
}

/// 判断某 space 的事件是否应被处理（受 only_my_spaces 设置约束）。
fn should_process_space(app: &tauri::AppHandle, space_id: &Option<String>) -> bool {
    let settings = settings::load();
    if !settings.only_my_spaces {
        return true;
    }
    let Some(sid) = space_id else {
        return false;
    };
    let Some(state) = app.try_state::<AppState>() else {
        return false;
    };
    let account = state.account.lock().unwrap().clone();
    let spaces = state.spaces.lock().unwrap().clone();
    let owned = spaces.iter().any(|s| {
        s.id == *sid && s.user_uuid.as_deref() == account.as_ref().map(|a| a.uuid.as_str())
    });
    owned
}

/// 判断某 turn 是否应被处理（受 only_my_turns 设置约束）。
fn should_process_turn(app: &tauri::AppHandle, user_uuid: &Option<String>) -> bool {
    let settings = settings::load();
    if !settings.only_my_turns {
        return true;
    }
    let Some(uuid) = user_uuid else {
        return false;
    };
    let Some(state) = app.try_state::<AppState>() else {
        return false;
    };
    let account = state.account.lock().unwrap().clone();
    account.map(|a| a.uuid == *uuid).unwrap_or(false)
}

/// 处理一条 WS 事件：转发给前端；完结时按设置发通知 + 累计未读 + 更新托盘。
fn handle_ws_event(app: &tauri::AppHandle, ev: ws::WsEvent) {
    match ev {
        ws::WsEvent::Activity {
            event_type,
            space_id,
            session_id,
        } => {
            if !should_process_space(app, &space_id) {
                return;
            }
            let _ = app.emit(
                "activity",
                serde_json::json!({
                    "eventType": event_type,
                    "spaceId": space_id,
                    "sessionId": session_id,
                }),
            );
        }
        ws::WsEvent::TurnProgress(p) => {
            if !should_process_space(app, &p.space_id) {
                return;
            }
            let _ = app.emit("turn-progress", p);
        }
        ws::WsEvent::TurnFinalized(fin) => {
            if !should_process_space(app, &fin.space_id)
                || !should_process_turn(app, &fin.user_uuid)
            {
                return;
            }
            let s = settings::load();
            if s.notify_finalized {
                show_finalized_notification(&fin);
            }
            if s.in_app_toast {
                let _ = app.emit(
                    "in-app-notify",
                    serde_json::json!({
                        "lastText": fin.last_text,
                        "messageCount": fin.message_count,
                        "toolCallCount": fin.tool_call_count,
                        "hasError": fin.has_error,
                    }),
                );
            }
            // 未读计数 + 托盘红点。
            if let Some(state) = app.try_state::<AppState>() {
                let n = state.unread_finalized.fetch_add(1, Ordering::SeqCst) + 1;
                tray::update_badge(app, n);
            }
            let _ = app.emit("turn-finalized", fin);
        }
        // Connected 已在 start_subscription 里处理为 subscription-status。
        ws::WsEvent::Connected => {}
    }
}

/// 启动（或重启）WS 订阅任务：订阅全部 Space，断线自动重连。
fn start_subscription(app: &tauri::AppHandle, space_ids: Vec<String>) {
    let count = space_ids.len();
    let app_for_ws = app.clone();
    let handle = tauri::async_runtime::spawn(async move {
        loop {
            emit_subscription(
                &app_for_ws,
                SubscriptionStatus {
                    phase: "connecting".into(),
                    space_count: count,
                    error: None,
                },
            );

            let client = reqwest::Client::new();
            let token = match auth::get_valid_access_token(&client).await {
                Ok(t) => t,
                Err(e) => {
                    emit_subscription(
                        &app_for_ws,
                        SubscriptionStatus {
                            phase: "error".into(),
                            space_count: count,
                            error: Some(format!("获取 token 失败：{e}")),
                        },
                    );
                    emit_log(&app_for_ws, format!("[ws] 获取 token 失败：{e}，停止重连"));
                    break;
                }
            };

            let app_inner = app_for_ws.clone();
            let result = ws::run_subscription(&token, &space_ids, move |ev| match ev {
                ws::WsEvent::Connected => emit_subscription(
                    &app_inner,
                    SubscriptionStatus {
                        phase: "connected".into(),
                        space_count: count,
                        error: None,
                    },
                ),
                other => handle_ws_event(&app_inner, other),
            })
            .await;

            match result {
                Ok(()) => {
                    emit_log(&app_for_ws, "[ws] 连接正常关闭，5 秒后重连");
                }
                Err(e) => {
                    emit_log(&app_for_ws, format!("[ws] 连接断开：{e}，5 秒后重连"));
                    emit_subscription(
                        &app_for_ws,
                        SubscriptionStatus {
                            phase: "reconnecting".into(),
                            space_count: count,
                            error: Some(e.to_string()),
                        },
                    );
                }
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    // 取消旧的订阅任务（若有），登记新的。
    if let Some(state) = app.try_state::<AppState>() {
        let mut sub = state.subscription.lock().unwrap();
        if let Some(old) = sub.take() {
            old.abort();
        }
        *sub = Some(handle);
    }
}

/// 登录/恢复后共用：取账号信息 -> 存状态 -> emit success -> 取 Space -> emit -> 订阅全部。
async fn post_auth(
    app: &tauri::AppHandle,
    client: &reqwest::Client,
    access_token: &str,
) -> Result<(), String> {
    // 并行拉取账号信息与 Space 列表，减少启动耗时。
    let (me, spaces) = tokio::try_join!(
        api::get_me(client, access_token),
        api::list_spaces(client, access_token),
    )
    .map_err(|e| e.to_string())?;
    let display_name = me
        .profile
        .as_ref()
        .and_then(|p| p.display_name.clone())
        .unwrap_or_else(|| me.uuid.clone());
    let avatar_url = me
        .profile
        .as_ref()
        .and_then(|p| p.avatar_url.clone())
        .filter(|u| !u.is_empty());
    let account = AccountInfo {
        uuid: me.uuid,
        display_name,
        avatar_url,
    };

    if let Some(state) = app.try_state::<AppState>() {
        *state.account.lock().unwrap() = Some(account.clone());
    }
    emit_auth(
        app,
        AuthStatus {
            phase: "success".into(),
            account: Some(account),
            ..Default::default()
        },
    );

    emit_spaces(app, spaces.clone());
    // 存入 state，供 only_my_spaces 过滤时查 owner。
    if let Some(state) = app.try_state::<AppState>() {
        *state.spaces.lock().unwrap() = spaces.clone();
    }

    if spaces.is_empty() {
        emit_subscription(
            app,
            SubscriptionStatus {
                phase: "idle".into(),
                space_count: 0,
                error: Some("当前账号没有可订阅的 Space".into()),
            },
        );
        return Ok(());
    }

    let ids: Vec<String> = spaces.iter().map(|s| s.id.clone()).collect();
    start_subscription(app, ids);
    // 登录成功：托盘应用模式下隐藏窗口，后续交互走托盘。
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
    Ok(())
}

// ---------- commands ----------

/// 开始 device flow 登录：请求 device code -> 自动打开浏览器 -> 轮询 -> 成功后订阅全部 Space。
#[tauri::command]
async fn login(app: tauri::AppHandle) -> Result<(), String> {
    let client = reqwest::Client::new();
    emit_log(&app, "开始 device flow 登录…");

    emit_auth(
        &app,
        AuthStatus {
            phase: "requesting_device".into(),
            ..Default::default()
        },
    );

    let device = auth::request_device_code(&client).await.map_err(|e| {
        let msg = e.to_string();
        emit_auth(
            &app,
            AuthStatus {
                phase: "error".into(),
                error: Some(msg.clone()),
                ..Default::default()
            },
        );
        msg
    })?;

    // 自动打开授权页（device flow 标准做法，类似 gh cli）；UI 也会展示链接/码供手动复制。
    emit_log(
        &app,
        format!("设备授权码已获取，用户码：{}", device.user_code),
    );
    let _ = open::that(&device.verification_uri_complete);

    emit_auth(
        &app,
        AuthStatus {
            phase: "awaiting_user".into(),
            user_code: Some(device.user_code.clone()),
            verification_uri: Some(device.verification_uri.clone()),
            verification_uri_complete: Some(device.verification_uri_complete.clone()),
            ..Default::default()
        },
    );
    emit_auth(
        &app,
        AuthStatus {
            phase: "polling".into(),
            ..Default::default()
        },
    );

    let session = auth::poll_for_token(&client, &device).await.map_err(|e| {
        let msg = e.to_string();
        emit_auth(
            &app,
            AuthStatus {
                phase: "error".into(),
                error: Some(msg.clone()),
                ..Default::default()
            },
        );
        msg
    })?;

    post_auth(&app, &client, &session.access_token)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            emit_auth(
                &app,
                AuthStatus {
                    phase: "error".into(),
                    error: Some(msg.clone()),
                    ..Default::default()
                },
            );
            msg
        })?;

    Ok(())
}

/// 登出：停止订阅、清状态、删本地 session、清托盘红点。
#[tauri::command]
fn logout(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(state) = app.try_state::<AppState>() {
        if let Some(h) = state.subscription.lock().unwrap().take() {
            h.abort();
        }
        *state.account.lock().unwrap() = None;
        *state.spaces.lock().unwrap() = vec![];
        state.unread_finalized.store(0, Ordering::SeqCst);
    }
    auth::logout().map_err(|e| e.to_string())?;
    tray::update_badge(&app, 0);
    emit_subscription(
        &app,
        SubscriptionStatus {
            phase: "disconnected".into(),
            space_count: 0,
            error: None,
        },
    );
    emit_auth(
        &app,
        AuthStatus {
            phase: "logged_out".into(),
            ..Default::default()
        },
    );
    emit_spaces(&app, vec![]);
    // 登出后显示登录窗口。
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
    Ok(())
}

#[tauri::command]
fn check_login() -> bool {
    auth::load_session().is_some()
}

#[tauri::command]
fn get_account(app: tauri::AppHandle) -> Option<AccountInfo> {
    app.try_state::<AppState>()?.account.lock().unwrap().clone()
}

/// 重新拉取 Space 列表（并 emit），供前端刷新。
#[tauri::command]
async fn list_spaces(app: tauri::AppHandle) -> Result<Vec<api::SpaceListItem>, String> {
    let client = reqwest::Client::new();
    let token = auth::get_valid_access_token(&client)
        .await
        .map_err(|e| e.to_string())?;
    let spaces = api::list_spaces(&client, &token)
        .await
        .map_err(|e| e.to_string())?;
    emit_spaces(&app, spaces.clone());
    if let Some(state) = app.try_state::<AppState>() {
        *state.spaces.lock().unwrap() = spaces.clone();
    }
    Ok(spaces)
}

/// 用系统默认浏览器打开 URL（device flow 授权页 / 网页端 session 深链）。
#[tauri::command]
fn open_url(url: String) {
    let _ = open::that(&url);
}

/// 读取应用设置。
#[tauri::command]
fn get_settings() -> settings::Settings {
    settings::load()
}

/// 保存应用设置并广播给前端。
#[tauri::command]
fn set_settings(app: tauri::AppHandle, settings: settings::Settings) -> Result<(), String> {
    settings::save(&settings).map_err(|e| e.to_string())?;
    let _ = app.emit("settings-updated", &settings);
    // 设置变更可能影响托盘红点显示。
    let unread = app
        .try_state::<AppState>()
        .map(|s| s.unread_finalized.load(Ordering::SeqCst))
        .unwrap_or(0);
    tray::update_badge(&app, unread);
    Ok(())
}

/// 标记未读完结对话为已查看（清零托盘红点）。窗口获得焦点时调用。
#[tauri::command]
fn mark_finalized_read(app: tauri::AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        state.unread_finalized.store(0, Ordering::SeqCst);
    }
    tray::update_badge(&app, 0);
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            account: Mutex::new(None),
            subscription: Mutex::new(None),
            unread_finalized: AtomicUsize::new(0),
            spaces: Mutex::new(vec![]),
        })
        .setup(|app| {
            let handle = app.handle().clone();
            // 系统托盘（显示窗口入口 + 未读完结计数红点）。
            tray::build(&handle)?;
            // 窗口事件：获得焦点清零未读红点；关闭按钮隐藏到托盘而非退出。
            if let Some(window) = app.get_webview_window("main") {
                let h = handle.clone();
                window.on_window_event(move |event| {
                    match event {
                        tauri::WindowEvent::Focused(true) => {
                            if let Some(state) = h.try_state::<AppState>() {
                                state.unread_finalized.store(0, Ordering::SeqCst);
                            }
                            tray::update_badge(&h, 0);
                        }
                        tauri::WindowEvent::CloseRequested { api, .. } => {
                            // 关闭窗口 = 隐藏到托盘；软件只能从托盘「退出」结束。
                            api.prevent_close();
                            if let Some(w) = h.get_webview_window("main") {
                                let _ = w.hide();
                            }
                        }
                        _ => {}
                    }
                });
            }
            if auth::load_session().is_some() {
                // 托盘应用：本地有登录态则隐藏窗口静默恢复，恢复失败再显示登录窗口。
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
                tauri::async_runtime::spawn(async move {
                    let client = reqwest::Client::new();
                    match auth::get_valid_access_token(&client).await {
                        Ok(token) => {
                            if let Err(e) = post_auth(&handle, &client, &token).await {
                                emit_log(&handle, format!("自动恢复失败：{e}"));
                                emit_auth(
                                    &handle,
                                    AuthStatus {
                                        phase: "error".into(),
                                        error: Some("本地登录态已失效，请重新登录".into()),
                                        ..Default::default()
                                    },
                                );
                                if let Some(w) = handle.get_webview_window("main") {
                                    let _ = w.show();
                                    let _ = w.set_focus();
                                }
                            }
                        }
                        Err(e) => {
                            emit_log(&handle, format!("本地 token 不可用：{e}"));
                            emit_auth(
                                &handle,
                                AuthStatus {
                                    phase: "logged_out".into(),
                                    ..Default::default()
                                },
                            );
                            if let Some(w) = handle.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                });
            } else {
                emit_auth(
                    &handle,
                    AuthStatus {
                        phase: "logged_out".into(),
                        ..Default::default()
                    },
                );
                // 未登录：显示登录窗口。
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            login,
            logout,
            check_login,
            get_account,
            list_spaces,
            open_url,
            get_settings,
            set_settings,
            mark_finalized_read
        ])
        .run(tauri::generate_context!())
        .expect("error while running cohub-desktop");
}
