//! WebSocket 实时事件客户端。
//! 协议（握手/订阅/事件形状）来自实测：docs/experiments/2026-07-02-cohub-companion-apps.md
//! 镜像自 Node.js 验证脚本 scripts/experiments/cohub-companion/ws-listen.mjs。

use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::json;
use thiserror::Error;
use tokio_tungstenite::tungstenite::Message;

const WS_URL: &str = "wss://gateway.cohub.run/ws";

#[derive(Debug, Error)]
pub enum WsError {
    #[error("websocket error: {0}")]
    Ws(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("auth failed: {0}")]
    Auth(String),
    #[error("connection closed before auth completed")]
    ClosedBeforeAuth,
}

/// 从原始 envelope 中提取出我们关心的两类信号，其余事件类型忽略。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum TurnSignal {
    /// session.turn.lifecycle, phase=llm_call_started
    /// llmRound 随每次模型调用（含 tool call 往返）递增，用作"第几轮"计数。
    LlmRoundStarted {
        session_id: Option<String>,
        turn_id: Option<String>,
        llm_round: Option<u64>,
        provider: Option<String>,
        model: Option<String>,
    },
    /// session.turn.finalized, payload.turn.status
    TurnFinalized {
        session_id: Option<String>,
        turn_id: Option<String>,
        status: Option<String>,
    },
}

fn parse_turn_signal(envelope: &serde_json::Value) -> Option<TurnSignal> {
    let event_type = envelope.get("type")?.as_str()?;
    let session_id = envelope
        .get("sessionId")
        .and_then(|v| v.as_str())
        .map(String::from);
    let payload = envelope.get("payload")?;

    match event_type {
        "session.turn.lifecycle" => {
            if payload.get("phase").and_then(|v| v.as_str()) != Some("llm_call_started") {
                return None;
            }
            Some(TurnSignal::LlmRoundStarted {
                session_id,
                turn_id: payload
                    .get("turnId")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                llm_round: payload.get("llmRound").and_then(|v| v.as_u64()),
                provider: payload
                    .get("provider")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                model: payload
                    .get("model")
                    .and_then(|v| v.as_str())
                    .map(String::from),
            })
        }
        "session.turn.finalized" => {
            let turn = payload.get("turn")?;
            Some(TurnSignal::TurnFinalized {
                session_id,
                turn_id: turn.get("id").and_then(|v| v.as_str()).map(String::from),
                status: turn
                    .get("status")
                    .and_then(|v| v.as_str())
                    .map(String::from),
            })
        }
        _ => None,
    }
}

/// 连接、认证、订阅一个 space room，并把每个解析出的 TurnSignal 传给 on_signal。
/// 阻塞运行直到连接关闭或出错；调用方负责放到独立 tokio task 里跑。
pub async fn run_subscription<F>(
    access_token: &str,
    space_id: &str,
    mut on_signal: F,
) -> Result<(), WsError>
where
    F: FnMut(TurnSignal) + Send,
{
    let (ws_stream, _) = tokio_tungstenite::connect_async(WS_URL).await?;
    let (mut write, mut read) = ws_stream.split();

    write
        .send(Message::Text(
            json!({
                "type": "auth",
                "payload": {
                    "token": access_token,
                    "capabilities": ["compact-stream", "room-subscription"]
                }
            })
            .to_string(),
        ))
        .await?;

    let mut authed = false;
    let mut subscribed = false;

    while let Some(msg) = read.next().await {
        let msg = msg?;
        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => {
                if !authed {
                    return Err(WsError::ClosedBeforeAuth);
                }
                break;
            }
            _ => continue,
        };

        let envelope: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let event_type = envelope.get("type").and_then(|v| v.as_str()).unwrap_or("");

        match event_type {
            "system.ready" => {
                // 握手第一步确认，无需动作。
            }
            "system.auth.ok" => {
                authed = true;
                write
                    .send(Message::Text(
                        json!({
                            "type": "subscribe",
                            "payload": { "rooms": [format!("space:{space_id}")] }
                        })
                        .to_string(),
                    ))
                    .await?;
            }
            "system.request.error" => {
                let message = envelope
                    .get("payload")
                    .and_then(|p| p.get("message"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("auth failed")
                    .to_string();
                return Err(WsError::Auth(message));
            }
            "system.subscribe.ok" => {
                subscribed = true;
            }
            "system.pong" => {
                // 心跳响应，忽略。
            }
            _ => {
                if let Some(signal) = parse_turn_signal(&envelope) {
                    on_signal(signal);
                }
            }
        }

        let _ = subscribed; // 保留变量供未来判断订阅状态使用，避免 unused warning。
    }

    Ok(())
}
