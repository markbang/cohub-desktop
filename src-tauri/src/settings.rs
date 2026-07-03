//! 应用设置：持久化到 ~/.config/cohub-desktop/settings.json，前端可读写。
//! 用于实时活动监听的过滤选项与通知/托盘行为开关。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// 实时活动监听与通知设置。所有字段都有默认值，缺失字段用默认补全（向前兼容）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// 仅监听自己创建的 Space（space.userUuid === 当前账号 uuid）。
    #[serde(default)]
    pub only_my_spaces: bool,
    /// 仅监听自己发起的对话（turn.userUuid === 当前账号 uuid）。
    #[serde(default)]
    pub only_my_turns: bool,
    /// 对话完结时推送系统通知。
    #[serde(default = "default_true")]
    pub notify_finalized: bool,
    /// 托盘图标显示未读红点（有未查看的完结对话时）。
    #[serde(default = "default_true")]
    pub tray_badge: bool,
    /// 应用内 Toast 提示（对话完结时）。
    #[serde(default = "default_true")]
    pub in_app_toast: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            only_my_spaces: false,
            only_my_turns: false,
            notify_finalized: true,
            tray_badge: true,
            in_app_toast: true,
        }
    }
}

fn default_true() -> bool {
    true
}

fn settings_path() -> PathBuf {
    let dirs = directories::ProjectDirs::from("run", "cohub", "cohub-desktop")
        .expect("failed to resolve config dir");
    let dir = dirs.config_dir().to_path_buf();
    std::fs::create_dir_all(&dir).ok();
    dir.join("settings.json")
}

pub fn load() -> Settings {
    let path = settings_path();
    let raw = match std::fs::read_to_string(&path) {
        Ok(r) => r,
        Err(_) => return Settings::default(),
    };
    // 用 Default 补全缺失字段，保证向前兼容。
    serde_json::from_str::<Settings>(&raw).unwrap_or_else(|_| {
        // 尝试合并默认值（旧版本可能缺字段）。
        let mut defaults = serde_json::to_value(&Settings::default()).unwrap_or_default();
        if let Ok(existing) = serde_json::from_str::<serde_json::Value>(&raw) {
            if let (Some(d), Some(e)) = (defaults.as_object_mut(), existing.as_object()) {
                for (k, v) in e {
                    d.insert(k.clone(), v.clone());
                }
            }
        }
        serde_json::from_value(defaults).unwrap_or_else(|_| Settings::default())
    })
}

pub fn save(settings: &Settings) -> Result<(), SettingsError> {
    let path = settings_path();
    let raw = serde_json::to_string_pretty(settings)?;
    std::fs::write(path, raw)?;
    Ok(())
}
