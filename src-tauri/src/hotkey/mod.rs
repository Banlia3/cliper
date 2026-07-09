use tauri::AppHandle;
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tauri_plugin_global_shortcut::Shortcut;

/// 注册全局热键（使用 Tauri 插件封装）
pub fn register_hotkey(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let shortcuts = app.global_shortcut();

    // 注册 Ctrl+Shift+V（使用字符串解析方式）
    shortcuts.register("Ctrl+Shift+V")?;

    log::info!("全局热键已注册: Ctrl+Shift+V");
    Ok(())
}

/// 注销全局热键
pub fn unregister_hotkey(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let shortcuts = app.global_shortcut();

    if shortcuts.is_registered("Ctrl+Shift+V") {
        shortcuts.unregister("Ctrl+Shift+V")?;
        log::info!("全局热键已注销");
    }

    Ok(())
}
