use tauri::Manager;

/// 打开 DevTools（前端按 F12 时调用）
#[tauri::command]
pub fn open_devtools(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(webview) = app.get_webview_window("main") {
        webview.open_devtools();
        log::info!("DevTools 已打开");
        Ok(())
    } else {
        Err("找不到主窗口".to_string())
    }
}
