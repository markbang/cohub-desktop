//! 最小 Cohub HTTP API client，仅覆盖登录后引导所需的两个端点。
//! 端点来自实测：docs/experiments/2026-07-02-cohub-companion-apps.md

use serde::Deserialize;
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
    pub username: Option<String>,
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

#[derive(Debug, Deserialize)]
pub struct SpaceListItem {
    pub id: String,
    pub name: String,
    // 注意：完整 /api/spaces 响应中的 `meta.extraEnv` 可能含明文密钥，
    // 这里只反序列化 id/name，其余字段一律不解析、不落盘、不打印。
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
