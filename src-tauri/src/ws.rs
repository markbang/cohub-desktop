//! WebSocket 实时事件客户端。
//! 协议（握手/订阅/事件形状）来自实测：docs/experiments/2026-07-02-cohub-companion-apps.md
//! 事件字段对照 ~/code/cohub 的 protocol/realtime 与 web/ProcessCard 渲染逻辑。
//!
//! 解析的事件类型：
//! - system.subscribe.ok          → Connected（事件流已建立）
//! - session.turn.lifecycle       → TurnProgress（llm_round / provider / model）
//! - session.turn.patch           → TurnProgress（累积 messageOrdinal=steps、tool_use id=tools）
//! - session.turn.finalized       → TurnFinalized（完整 intermediateSummary：steps/tools/usage/duration/lastText）
//! 其余业务事件 → Activity（仅 type + sessionId + spaceId，不转储 payload）。
//!
//! live 计数策略对齐 web ProcessCard：
//!   steps = distinct messageOrdinal 数量；tools = distinct tool_use id 数量。
//!   token 用量与时长仅在 finalized 时可得（来自 turn.intermediateSummary）。

use std::collections::{HashMap, HashSet};

use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::{json, Value};
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

/// 进行中的对话进度（流式过程中持续更新）。
#[derive(Debug, Clone, Serialize)]
pub struct TurnProgress {
    pub space_id: Option<String>,
    pub session_id: Option<String>,
    pub turn_id: Option<String>,
    pub llm_round: Option<u64>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub steps: u64,
    pub tools: u64,
}

/// 对话完结快照（来自 session.turn.finalized.payload.turn）。
#[derive(Debug, Clone, Serialize)]
pub struct TurnFinalized {
    pub space_id: Option<String>,
    pub session_id: Option<String>,
    pub turn_id: Option<String>,
    pub sequence: Option<u64>,
    pub status: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub message_count: u64,
    pub tool_call_count: u64,
    pub usage_input: u64,
    pub usage_output: u64,
    pub usage_cache_read: u64,
    pub duration_ms: Option<u64>,
    /// 最后一轮回复文本（intermediateSummary.lastMessageText，回退 assistantText）。
    pub last_text: Option<String>,
    pub has_error: bool,
    /// 发起该 turn 的用户 uuid（turn.userUuid），用于「仅监听自己的对话」过滤。
    pub user_uuid: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum WsEvent {
    Connected,
    Activity {
        event_type: String,
        space_id: Option<String>,
        session_id: Option<String>,
    },
    TurnProgress(TurnProgress),
    TurnFinalized(TurnFinalized),
}

/// 单个进行中 turn 的累积状态。
struct TurnLiveState {
    space_id: Option<String>,
    session_id: Option<String>,
    llm_round: Option<u64>,
    provider: Option<String>,
    model: Option<String>,
    /// 已见过的 messageOrdinal（用于 steps 计数）。
    ordinals: HashSet<u64>,
    /// 已见过的 tool_use 块 id（用于 tools 计数）。
    tool_ids: HashSet<String>,
}

impl TurnLiveState {
    fn new(space_id: Option<String>, session_id: Option<String>) -> Self {
        Self {
            space_id,
            session_id,
            llm_round: None,
            provider: None,
            model: None,
            ordinals: HashSet::new(),
            tool_ids: HashSet::new(),
        }
    }

    fn progress(&self, turn_id: Option<String>) -> TurnProgress {
        TurnProgress {
            space_id: self.space_id.clone(),
            session_id: self.session_id.clone(),
            turn_id,
            llm_round: self.llm_round,
            provider: self.provider.clone(),
            model: self.model.clone(),
            steps: self.ordinals.len() as u64,
            tools: self.tool_ids.len() as u64,
        }
    }
}

fn str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.as_str()).map(String::from)
}

fn u64_field(v: &Value, key: &str) -> Option<u64> {
    v.get(key).and_then(|x| x.as_u64())
}

/// 若事件带了有效的 session_id + space_id，则更新 session→space 映射。
/// envelope 顶层 spaceId 可能为 null（finalized 尤甚），靠这个累积映射回退补全。
fn update_session_space(
    map: &mut HashMap<String, String>,
    session_id: &Option<String>,
    space_id: &Option<String>,
) {
    if let (Some(sid), Some(spid)) = (session_id.as_ref(), space_id.as_ref()) {
        if !sid.is_empty() && !spid.is_empty() {
            map.insert(sid.clone(), spid.clone());
        }
    }
}

/// 处理 session.turn.lifecycle（phase=llm_call_started）。
fn parse_lifecycle(envelope: &Value) -> Option<(Option<String>, TurnProgress)> {
    let payload = envelope.get("payload")?;
    if payload.get("phase").and_then(|v| v.as_str()) != Some("llm_call_started") {
        return None;
    }
    let turn_id = str_field(payload, "turnId");
    Some((
        turn_id.clone(),
        TurnProgress {
            space_id: str_field(envelope, "spaceId"),
            session_id: str_field(envelope, "sessionId"),
            turn_id,
            llm_round: u64_field(payload, "llmRound"),
            provider: str_field(payload, "provider"),
            model: str_field(payload, "model"),
            steps: 0,
            tools: 0,
        },
    ))
}

/// 处理 session.turn.patch：累积 ordinals / tool_ids，返回更新后的进度。
fn parse_patch(
    envelope: &Value,
    states: &mut HashMap<String, TurnLiveState>,
) -> Option<(Option<String>, TurnProgress)> {
    let payload = envelope.get("payload")?;
    let turn_id = str_field(payload, "turnId")?;
    let space_id = str_field(envelope, "spaceId");
    let session_id = str_field(envelope, "sessionId");

    let state = states
        .entry(turn_id.clone())
        .or_insert_with(|| TurnLiveState::new(space_id.clone(), session_id.clone()));
    if state.space_id.is_none() {
        state.space_id = space_id;
    }
    if state.session_id.is_none() {
        state.session_id = session_id;
    }

    if let Some(ord) = u64_field(payload, "messageOrdinal") {
        state.ordinals.insert(ord);
    }

    // 扫描 ops，收集 tool_use 块的 id。ops 形如 {o, p, v}，v 可能是 content block。
    if let Some(ops) = payload.get("ops").and_then(|v| v.as_array()) {
        for op in ops {
            let v = match op.get("v") {
                Some(v) if v.is_object() => v,
                _ => continue,
            };
            if v.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                if let Some(id) = v.get("id").and_then(|i| i.as_str()) {
                    state.tool_ids.insert(id.to_string());
                }
            }
        }
    }

    Some((Some(turn_id.clone()), state.progress(Some(turn_id))))
}

/// 处理 session.turn.finalized：从 payload.turn.intermediateSummary 提取完整快照。
fn parse_finalized(envelope: &Value) -> Option<TurnFinalized> {
    let turn = envelope.get("payload")?.get("turn")?;
    let summary = turn.get("intermediateSummary");
    let usage = summary.and_then(|s| s.get("usage"));

    let last_text = summary
        .and_then(|s| str_field(s, "lastMessageText"))
        .filter(|s| !s.is_empty())
        .or_else(|| str_field(turn, "assistantText").filter(|s| !s.is_empty()));

    Some(TurnFinalized {
        space_id: str_field(envelope, "spaceId"),
        session_id: str_field(turn, "sessionId").or_else(|| str_field(envelope, "sessionId")),
        turn_id: str_field(turn, "id"),
        sequence: u64_field(turn, "sequence"),
        status: str_field(turn, "status"),
        provider: str_field(turn, "provider"),
        model: str_field(turn, "model"),
        message_count: summary
            .and_then(|s| u64_field(s, "messageCount"))
            .unwrap_or(0),
        tool_call_count: summary
            .and_then(|s| u64_field(s, "toolCallCount"))
            .unwrap_or(0),
        usage_input: usage
            .and_then(|u| u.get("input").and_then(|v| v.as_u64()))
            .unwrap_or(0),
        usage_output: usage
            .and_then(|u| u.get("output").and_then(|v| v.as_u64()))
            .unwrap_or(0),
        usage_cache_read: usage
            .and_then(|u| u.get("cacheRead").and_then(|v| v.as_u64()))
            .unwrap_or(0),
        duration_ms: summary
            .and_then(|s| u64_field(s, "durationMs"))
            .or_else(|| u64_field(turn, "durationMs")),
        last_text,
        has_error: summary
            .and_then(|s| s.get("hasError").and_then(|v| v.as_bool()))
            .unwrap_or(false),
        user_uuid: str_field(turn, "userUuid"),
    })
}

/// 连接、认证、订阅全部 Space room，把每个事件传给 on_event。
/// 阻塞运行直到连接关闭或出错；调用方负责放到独立 tokio task 里跑，并负责重连。
pub async fn run_subscription<F>(
    access_token: &str,
    space_ids: &[String],
    initial_session_space: HashMap<String, String>,
    mut on_event: F,
) -> Result<(), WsError>
where
    F: FnMut(WsEvent) + Send,
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
    // 每个 turn 的累积状态；finalized 后清除。
    let mut live: HashMap<String, TurnLiveState> = HashMap::new();
    // session → space 映射：envelope 顶层 spaceId 可能是 null（finalized 尤甚），
    // 靠之前该 session 收到的任何带 spaceId 的事件补全。对齐 web 的
    // `input.spaceId ?? current?.spaceId` 回退策略。
    let mut session_space: HashMap<String, String> = initial_session_space;

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

        let envelope: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let event_type = envelope
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        match event_type.as_str() {
            "system.ready" => {}
            "system.auth.ok" => {
                authed = true;
                let rooms: Vec<String> = space_ids.iter().map(|id| format!("space:{id}")).collect();
                write
                    .send(Message::Text(
                        json!({ "type": "subscribe", "payload": { "rooms": rooms } }).to_string(),
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
                on_event(WsEvent::Connected);
            }
            "system.pong" => {}
            "session.turn.lifecycle" => {
                if let Some((turn_id, mut progress)) = parse_lifecycle(&envelope) {
                    update_session_space(
                        &mut session_space,
                        &progress.session_id,
                        &progress.space_id,
                    );
                    progress.space_id = progress.space_id.or_else(|| {
                        progress
                            .session_id
                            .as_ref()
                            .and_then(|s| session_space.get(s).cloned())
                    });
                    // 合并进持久状态（保留 patch 累积的 steps/tools）。
                    let space_id = progress.space_id.clone();
                    let session_id = progress.session_id.clone();
                    let state = live
                        .entry(turn_id.clone().unwrap_or_default())
                        .or_insert_with(|| TurnLiveState::new(space_id, session_id));
                    state.llm_round = progress.llm_round;
                    if progress.provider.is_some() {
                        state.provider = progress.provider.take();
                    }
                    if progress.model.is_some() {
                        state.model = progress.model.take();
                    }
                    on_event(WsEvent::TurnProgress(state.progress(turn_id)));
                }
            }
            "session.turn.patch" => {
                if let Some((_turn_id, mut progress)) = parse_patch(&envelope, &mut live) {
                    update_session_space(
                        &mut session_space,
                        &progress.session_id,
                        &progress.space_id,
                    );
                    progress.space_id = progress.space_id.or_else(|| {
                        progress
                            .session_id
                            .as_ref()
                            .and_then(|s| session_space.get(s).cloned())
                    });
                    on_event(WsEvent::TurnProgress(progress));
                }
            }
            "session.turn.finalized" => {
                if let Some(mut fin) = parse_finalized(&envelope) {
                    update_session_space(&mut session_space, &fin.session_id, &fin.space_id);
                    fin.space_id = fin.space_id.or_else(|| {
                        fin.session_id
                            .as_ref()
                            .and_then(|s| session_space.get(s).cloned())
                    });
                    let key = fin.turn_id.clone().unwrap_or_default();
                    live.remove(&key);
                    on_event(WsEvent::TurnFinalized(fin));
                }
            }
            _ => {
                let sid = str_field(&envelope, "sessionId");
                let spid = str_field(&envelope, "spaceId");
                update_session_space(&mut session_space, &sid, &spid);
                on_event(WsEvent::Activity {
                    event_type,
                    space_id: spid
                        .or_else(|| sid.as_ref().and_then(|s| session_space.get(s).cloned())),
                    session_id: sid,
                });
            }
        }
    }

    Ok(())
}
