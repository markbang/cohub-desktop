//! Tauri 入口：注册前端可调用的 commands，串联 auth/api/ws 三个模块。
//! 最小闭环：登录 -> 拿第一个 space -> 订阅 -> 把 TurnSignal 转发给前端展示。
//! 未在本环境编译验证（无 Rust 工具链），逻辑基于已实测协议镜像实现。

mod api;
mod auth;
mod ws;

use serde::Serialize;
use tauri::{Emitter, Manager};

#[derive(Debug, Serialize, Clone)]
struct LogEvent {
    message: String,
}

fn emit_log(app: &tauri::AppHandle, message: impl Into<String>) {
    let message = message.into();
    println!("{message}"); // 同时打到终端，方便本地调试时不依赖前端。
    let _ = app.emit("cohub-log", LogEvent { message });
}

/// 前端调用：开始 device flow 登录。
/// 流程：请求 device code -> 打印/展示登录链接 -> 轮询直到用户完成授权。
#[tauri::command]
async fn login(app: tauri::AppHandle) -> Result<(), String> {
    let client = reqwest::Client::new();

    let device = auth::request_device_code(&client)
        .await
        .map_err(|e| e.to_string())?;

    emit_log(&app, format!("打开链接登录: {}", device.verification_uri_complete));
    emit_log(&app, format!("用户码: {}", device.user_code));

    let session = auth::poll_for_token(&client, &device)
        .await
        .map_err(|e| e.to_string())?;

    emit_log(&app, "登录成功");

    // 登录后立即拿用户信息 + space 列表，验证 token 可用，并选第一个 space 订阅。
    let me = api::get_me(&client, &session.access_token)
        .await
        .map_err(|e| e.to_string())?;
    let display_name = me
        .profile
        .as_ref()
        .and_then(|p| p.display_name.clone())
        .unwrap_or_else(|| me.uuid.clone());
    emit_log(&app, format!("已登录账号: {display_name}"));

    let spaces = api::list_spaces(&client, &session.access_token)
        .await
        .map_err(|e| e.to_string())?;
    let Some(first_space) = spaces.into_iter().next() else {
        emit_log(&app, "当前账号没有可用的 Space，未开始订阅");
        return Ok(());
    };
    emit_log(&app, format!("订阅 Space: {} ({})", first_space.name, first_space.id));

    // WebSocket 订阅放到独立 task 里跑，不阻塞 command 返回。
    let app_for_ws = app.clone();
    let space_id = first_space.id.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let client = reqwest::Client::new();
            let token = match auth::get_valid_access_token(&client).await {
                Ok(t) => t,
                Err(e) => {
                    emit_log(&app_for_ws, format!("[ws] 获取 token 失败: {e}"));
                    break;
                }
            };

            let app_inner = app_for_ws.clone();
            let result = ws::run_subscription(&token, &space_id, move |signal| {
                match &signal {
                    ws::TurnSignal::LlmRoundStarted { llm_round, provider, model, .. } => {
                        emit_log(
                            &app_inner,
                            format!(
                                "[turn] llmRound={} provider={} model={}",
                                llm_round.map(|v| v.to_string()).unwrap_or_default(),
                                provider.clone().unwrap_or_default(),
                                model.clone().unwrap_or_default(),
                            ),
                        );
                    }
                    ws::TurnSignal::TurnFinalized { status, .. } => {
                        emit_log(
                            &app_inner,
                            format!("[turn] ✅ finalized status={}", status.clone().unwrap_or_default()),
                        );
                    }
                }
            })
            .await;

            if let Err(e) = result {
                emit_log(&app_for_ws, format!("[ws] 连接断开: {e}，5秒后重连"));
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });

    Ok(())
}

/// 前端调用：检查是否已有本地登录态（不会自动发起登录）。
#[tauri::command]
fn check_login() -> bool {
    auth::load_session().is_some()
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            if auth::load_session().is_some() {
                emit_log(&handle, "检测到本地登录态，可点击「连接」直接订阅（当前原型未自动恢复订阅，需要重新点登录）");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![login, check_login])
        .run(tauri::generate_context!())
        .expect("error while running cohub-desktop");
}
