mod clipboard;
mod commands;
mod db;
mod hotkey;
mod settings;
mod tray;
mod utils;

use clipboard::dedup::DedupEngine;
use clipboard::listener::ClipboardListener;
use db::Database;
use db::queries;

use std::sync::Arc;

use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{Shortcut, ShortcutEvent, ShortcutState};

/// 应用入口
pub fn run() {
    // 初始化文件日志（%APPDATA%/clipboard-history/clipboard-history.log）
    let db_dir = std::env::var("APPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default())
        .join("clipboard-history");
    utils::logger::FileLogger::init(
        &db_dir,
        "clipboard-history.log",
        log::LevelFilter::Info,  // Release 也记录 Info 级别
    ).expect("日志初始化失败");

    log::info!("🚀 剪贴板历史 v{} 启动中...", env!("CARGO_PKG_VERSION"));
    log::info!("📂 数据库目录: {:?}", db_dir);

    // 单实例检测：使用 Windows 命名 Mutex 防止重复启动
    {
        extern "system" {
            fn CreateMutexW(
                lpMutexAttributes: *const std::ffi::c_void,
                bInitialOwner: i32,
                lpName: *const u16,
            ) -> isize;
            fn GetLastError() -> u32;
        }
        const ERROR_ALREADY_EXISTS: u32 = 183u32;
        let name: Vec<u16> = "Local\\ClipboardHistory_InstanceMutex\0".encode_utf16().collect();
        let mutex = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if mutex == 0 {
            log::error!("创建实例互斥锁失败: {}", unsafe { GetLastError() });
        } else {
            let err = unsafe { GetLastError() };
            if err == ERROR_ALREADY_EXISTS {
                log::warn!("程序已在运行中，退出");
                std::process::exit(0);
            }
            // mutex 保持打开直到进程退出，防止后续实例启动
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |app: &tauri::AppHandle, _shortcut: &Shortcut, event: ShortcutEvent| {
                if event.state == ShortcutState::Pressed {
                    log::info!("全局热键触发：切换面板");
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.set_focus();
                        let _ = window.show();
                    }
                }
            })
            .build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            // 1. 初始化数据库
            let db = Arc::new(Database::new("clipboard-history").map_err(|e| {
                log::error!("数据库初始化失败: {}", e);
                e
            })?);
            log::info!("数据库初始化完成");

            // 2. 初始化去重引擎
            let dedup = Arc::new(DedupEngine::new(50)); // 50ms 防抖

            // 3. 启动剪贴板监听器
            let listener = ClipboardListener::start(dedup.clone());

            // 4. 从监听器获取事件接收器
            let event_rx = listener.get_event_receiver();

            // 5. 启动事件处理循环（在独立线程中）
            let db_clone = db.clone();
            let dedup_clone = dedup.clone();
            let app_handle_clone = app_handle.clone();
            std::thread::spawn(move || {
                loop {
                    match event_rx.lock().unwrap().recv() {
                        Ok(event) => {
                            let preview_short: String = event.text_preview.chars().take(40).collect();
                            log::info!("收到剪贴板事件: {}", preview_short);

                            // 锁定数据库连接写入
                            let conn = db_clone.conn.lock().unwrap();
                            let content_type = db::models::ContentType::from_str(&event.content_type);
                            if let Ok(Some(_id)) = queries::insert_or_update_entry(
                                &conn,
                                &event.content_hash,
                                &event.text_preview,
                                &content_type,
                                event.raw_content.as_deref(),
                                event.content_size,
                                &event.source_app,
                                &event.source_class,
                            ) {
                                log::info!("已存储剪贴板条目, id={}", _id);

                                // 释放连接锁后再调用 SettingsManager（它内部自己锁）
                                drop(conn);
                                let max_history: i64 = settings::manager::SettingsManager::get(
                                    db_clone.as_ref(), "max_history"
                                )
                                    .ok()
                                    .flatten()
                                    .and_then(|v| v.parse().ok())
                                    .unwrap_or(500);

                                // 重新锁连接执行清理
                                let conn = db_clone.conn.lock().unwrap();
                                if let Ok(count) = queries::cleanup_old_entries(&conn, max_history) {
                                    if count > 0 {
                                        log::info!("已清理 {} 条旧记录", count);
                                    }
                                }
                                drop(conn);

                                // 通知前端（通过 Tauri event）
                                let _ = app_handle_clone.emit("new-clip", serde_json::json!({
                                    "id": _id,
                                    "text_preview": event.text_preview,
                                    "content_type": event.content_type,
                                }));
                            }
                        }
                        Err(e) => {
                            log::error!("接收剪贴板事件失败: {}", e);
                            break;
                        }
                    }
                }
            });

            // 6. 注册全局热键（从设置中读取键位，失败不崩溃只警告）
            if let Err(e) = hotkey::register_hotkey(&app_handle, &db) {
                log::warn!("全局热键注册失败（面板只能通过托盘打开）: {}", e);
            }

            // 7. 注册系统托盘
            tray::handler::setup_tray(&app_handle)?;

            // 7. 管理状态（供 Tauri 命令访问）
            app.manage(db.clone());
            app.manage(dedup.clone());

            log::info!("✅ 剪贴板历史启动完成");

            Ok(())
        })
        // 注册 Tauri IPC 命令
        .invoke_handler(tauri::generate_handler![
            commands::history::get_history,
            commands::history::search_history,
            commands::history::copy_to_clipboard,
            commands::history::get_raw_content,
            commands::history::clear_history,
            commands::history::delete_entry,
            commands::history::toggle_pin,
            commands::folders::list_folders,
            commands::folders::create_folder,
            commands::folders::rename_folder,
            commands::folders::delete_folder,
            commands::folders::add_to_folder,
            commands::folders::remove_from_folder,
            commands::folders::get_folder_entries,
            commands::folders::get_entry_folders,
            settings::manager::get_settings,
            settings::manager::update_settings,
        ])
        .run(tauri::generate_context!())
        .expect("启动失败");
}
