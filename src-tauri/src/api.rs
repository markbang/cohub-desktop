//! 最小 Cohub HTTP API client，仅覆盖登录后引导所需的两个端点。
//! 端点来自实测：docs/experiments/2026-07-02-cohub-companion-apps.md

use serde::{Deserialize, Serialize};
use thiserror::Error;

const API_BASE_URL: &str = "https://api.cohub.run";

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("http request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("unexpected status {0}")]
    Status(u16),
}

#[derive(Debug, Deserialize)]
pub struct MeResponse {
    pub uuid: String,
    pub profile: Option<MeProfile>,
}

#[derive(Debug, Deserialize)]
pub struct MeProfile {
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[allow(dead_code)]
    pub username: Option<String>,
    #[serde(rename = "avatarUrl")]
    pub avatar_url: Option<String>,
}

pub async fn get_me(client: &reqwest::Client, access_token: &str) -> Result<MeResponse, ApiError> {
    let resp = client
        .get(format!("{API_BASE_URL}/api/me"))
        .bearer_auth(access_token)
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(ApiError::Status(resp.status().as_u16()));
    }
    Ok(resp.json().await?)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpaceListItem {
    pub id: String,
    pub name: String,
    /// Space 创建者的 userUuid（space.userUuid）。用于「仅监听自己创建的 Space」过滤。
    #[serde(rename = "userUuid", default)]
    pub user_uuid: Option<String>,
    // 注意：完整 /api/spaces 响应中的 `meta.extraEnv` 可能含明文密钥，
    // 这里只反序列化 id/name/userUuid，其余字段一律不解析、不落盘、不打印。
}

#[derive(Debug, Deserialize)]
struct SpaceSessionItem {
    id: String,
    // spaceId 字段在响应中存在，但我们已知道它属于哪个 space，无需解析。
}

#[derive(Debug, Deserialize)]
struct SpaceSessionsResponse {
    sessions: Vec<SpaceSessionItem>,
}

/// 拉取某个 space 下最近的 session id 列表，用于预建 session→space 映射。
/// 这样即使 finalized 是某 session 的首个事件，也能回退查到 spaceId。
pub async fn list_space_session_ids(
    client: &reqwest::Client,
    access_token: &str,
    space_id: &str,
) -> Result<Vec<String>, ApiError> {
    let resp = client
        .get(format!(
            "{API_BASE_URL}/api/spaces/{space_id}/sessions?limit=50"
        ))
        .bearer_auth(access_token)
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(ApiError::Status(resp.status().as_u16()));
    }
    Ok(resp
        .json::<SpaceSessionsResponse>()
        .await?
        .sessions
        .into_iter()
        .map(|s| s.id)
        .collect())
}

pub async fn list_spaces(
    client: &reqwest::Client,
    access_token: &str,
) -> Result<Vec<SpaceListItem>, ApiError> {
    let resp = client
        .get(format!("{API_BASE_URL}/api/spaces"))
        .bearer_auth(access_token)
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(ApiError::Status(resp.status().as_u16()));
    }
    Ok(resp.json().await?)
}
