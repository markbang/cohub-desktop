//! 系统托盘：显示窗口入口 + 未读完结对话计数。
//! 红点近似：Linux/Windows 用 set_title 显示未读数字；tooltip 显示详情。
//! 左键单击显示/聚焦窗口；右键菜单提供「显示窗口」「退出」。

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};

const TRAY_ID: &str = "cohub-main";

/// 切换主窗口显示状态：隐藏则显示并聚焦，可见则隐藏到托盘。
fn toggle_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        if w.is_visible().unwrap_or(false) {
            let _ = w.hide();
        } else {
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
}

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, "toggle", "显示/隐藏窗口", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出 Cohub Desktop", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &quit])?;

    let icon = app.default_window_icon().cloned();
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle" => toggle_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // 单击托盘图标切换窗口显示/隐藏。
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_window(tray.app_handle());
            }
        });

    if let Some(ic) = icon {
        builder = builder.icon(ic);
    }
    builder = builder.tooltip("Cohub Desktop");

    builder.build(app)?;
    Ok(())
}

/// 根据未读完结对话数与设置更新托盘显示。
/// unread=0 时清掉 title；>0 且开启 tray_badge 时显示数字。
pub fn update_badge<R: Runtime>(app: &AppHandle<R>, unread: usize) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let settings = crate::settings::load();
    let show_badge = settings.tray_badge && unread > 0;
    let title = if show_badge {
        Some(format!("{unread}"))
    } else {
        None
    };
    let _ = tray.set_title(title.as_deref());
    let tooltip = if unread > 0 {
        format!("Cohub · {unread} 个对话完结待查看")
    } else {
        "Cohub Desktop".to_string()
    };
    let _ = tray.set_tooltip(Some(&tooltip));
}
