use tauri::{
    AppHandle, Emitter, Manager,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    Wry,
};

use crate::db::Database;
use crate::db::queries;
use std::sync::Arc;

/// 创建系统托盘图标和菜单
pub fn setup_tray(app: &AppHandle<Wry>) -> Result<(), Box<dyn std::error::Error>> {
    // 构建菜单项
    let open = MenuItemBuilder::with_id("open", "打开面板")
        .build(app)?;
    let separator1 = tauri::menu::PredefinedMenuItem::separator(app)?;
    let clear = MenuItemBuilder::with_id("clear", "清空历史")
        .build(app)?;
    let separator2 = tauri::menu::PredefinedMenuItem::separator(app)?;
    let quit = MenuItemBuilder::with_id("quit", "退出")
        .build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&open)
        .item(&separator1)
        .item(&clear)
        .item(&separator2)
        .item(&quit)
        .build()?;

    // 构建托盘图标
    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("剪贴板历史")
        .on_menu_event(move |app, event| {
            match event.id().as_ref() {
                "open" => {
                    log::info!("托盘菜单：打开面板");
                    toggle_panel(app, true);
                }
                "clear" => {
                    log::info!("托盘菜单：清空历史");
                    clear_history(app);
                }
                "quit" => {
                    log::info!("托盘菜单：退出程序");
                    app.exit(0);
                }
                _ => {}
            }
        })
        .build(app)?;

    log::info!("系统托盘已创建");
    Ok(())
}

/// 切换面板可见性
fn toggle_panel(app: &AppHandle<Wry>, show: bool) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_focus();
        let _ = window.show();
    }
}

/// 清空历史（直接使用数据库状态）
fn clear_history(app: &AppHandle<Wry>) {
    if let Some(db) = app.try_state::<Arc<Database>>() {
        let conn = db.conn.lock().unwrap();
        match queries::clear_unpinned(&conn) {
            Ok(count) => {
                log::info!("已清空 {} 条历史", count);
                let _ = app.emit("history-cleared", count);
            }
            Err(e) => {
                log::error!("清空历史失败: {}", e);
            }
        }
    } else {
        log::warn!("数据库状态不可用，无法清空历史");
    }
}
