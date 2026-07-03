//! Device flow 登录，端点/字段来自已实测验证的协议
//! （见 docs/experiments/2026-07-02-cohub-companion-apps.md）。
//! 未在本环境编译验证，逻辑镜像自 Node.js 验证脚本
//! scripts/experiments/cohub-companion/device-login.mjs。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

const ISSUER: &str = "https://auth.neta.art";
const CLIENT_ID: &str = "f8d26cdlwx85b0e5l3om2";
const RESOURCE: &str = "https://api.talesofai";
const SCOPE: &str = "openid profile email offline_access";

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("http request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("device auth error: {0}")]
    DeviceAuth(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("login timed out")]
    Timeout,
    #[error("not authenticated")]
    NotAuthenticated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub expires_at_ms: u64,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn session_path() -> PathBuf {
    // 与 CLI 不同：独立配置目录，不与 cohub-cli 共享登录态。
    let dirs = directories::ProjectDirs::from("run", "cohub", "cohub-desktop")
        .expect("failed to resolve config dir");
    let dir = dirs.config_dir().to_path_buf();
    std::fs::create_dir_all(&dir).ok();
    dir.join("session.json")
}

pub fn load_session() -> Option<Session> {
    let path = session_path();
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// 清除本地登录态。对应登出操作。不撤销服务端 token（device flow 无 refresh 撤销端点）。
pub fn logout() -> Result<(), AuthError> {
    let path = session_path();
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

fn save_session(session: &Session) -> Result<(), AuthError> {
    let path = session_path();
    let raw = serde_json::to_string_pretty(session)?;
    std::fs::write(path, raw)?;
    Ok(())
}

/// 请求 device code。对应实测端点：POST {issuer}/oidc/device/auth
pub async fn request_device_code(client: &reqwest::Client) -> Result<DeviceCode, AuthError> {
    let resp = client
        .post(format!("{ISSUER}/oidc/device/auth"))
        .form(&[
            ("client_id", CLIENT_ID),
            ("scope", SCOPE),
            ("resource", RESOURCE),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(AuthError::DeviceAuth(format!(
            "device code request failed: HTTP {status} {text}"
        )));
    }

    #[derive(Deserialize)]
    struct RawDeviceCode {
        device_code: String,
        user_code: String,
        verification_uri: String,
        verification_uri_complete: Option<String>,
        expires_in: u64,
        interval: Option<u64>,
    }

    let raw: RawDeviceCode = resp.json().await?;
    let verification_uri_complete = raw
        .verification_uri_complete
        .unwrap_or_else(|| raw.verification_uri.clone());

    Ok(DeviceCode {
        device_code: raw.device_code,
        user_code: raw.user_code,
        verification_uri: raw.verification_uri,
        verification_uri_complete,
        expires_in: raw.expires_in,
        interval: raw.interval.unwrap_or(5),
    })
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
    expires_in: u64,
}

#[derive(Deserialize)]
struct TokenErrorResponse {
    error: Option<String>,
}

/// 轮询直到用户完成授权。对应实测端点：
/// POST {issuer}/oidc/token, grant_type=urn:ietf:params:oauth:grant-type:device_code
pub async fn poll_for_token(
    client: &reqwest::Client,
    device: &DeviceCode,
) -> Result<Session, AuthError> {
    let deadline = now_ms() + device.expires_in * 1000;
    let mut interval_secs = device.interval;

    loop {
        if now_ms() >= deadline {
            return Err(AuthError::Timeout);
        }
        tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;

        let resp = client
            .post(format!("{ISSUER}/oidc/token"))
            .form(&[
                ("client_id", CLIENT_ID),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", &device.device_code),
                ("resource", RESOURCE),
            ])
            .send()
            .await?;

        if resp.status().is_success() {
            let token: TokenResponse = resp.json().await?;
            let session = Session {
                access_token: token.access_token,
                refresh_token: token.refresh_token,
                id_token: token.id_token,
                expires_at_ms: now_ms() + token.expires_in * 1000,
            };
            save_session(&session)?;
            return Ok(session);
        }

        let err: TokenErrorResponse = resp
            .json()
            .await
            .unwrap_or(TokenErrorResponse { error: None });
        match err.error.as_deref() {
            Some("authorization_pending") => continue,
            Some("slow_down") => {
                interval_secs += 5;
                continue;
            }
            other => {
                return Err(AuthError::DeviceAuth(format!(
                    "token poll failed: {:?}",
                    other
                )))
            }
        }
    }
}

/// 刷新 access token。对应实测端点：POST {issuer}/oidc/token, grant_type=refresh_token
pub async fn refresh_token(client: &reqwest::Client) -> Result<Session, AuthError> {
    let current = load_session().ok_or(AuthError::NotAuthenticated)?;
    let refresh = current
        .refresh_token
        .clone()
        .ok_or(AuthError::NotAuthenticated)?;

    let resp = client
        .post(format!("{ISSUER}/oidc/token"))
        .form(&[
            ("client_id", CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh.as_str()),
            ("scope", SCOPE),
            ("resource", RESOURCE),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(AuthError::DeviceAuth("refresh failed".into()));
    }

    let token: TokenResponse = resp.json().await?;
    let session = Session {
        access_token: token.access_token,
        refresh_token: token.refresh_token.or(current.refresh_token),
        id_token: token.id_token.or(current.id_token),
        expires_at_ms: now_ms() + token.expires_in * 1000,
    };
    save_session(&session)?;
    Ok(session)
}

/// 返回可直接使用的 access token，过期前 5 分钟自动刷新。
pub async fn get_valid_access_token(client: &reqwest::Client) -> Result<String, AuthError> {
    let session = load_session().ok_or(AuthError::NotAuthenticated)?;
    const SKEW_MS: u64 = 5 * 60 * 1000;
    if session.expires_at_ms > now_ms() + SKEW_MS {
        return Ok(session.access_token);
    }
    let refreshed = refresh_token(client).await?;
    Ok(refreshed.access_token)
}
