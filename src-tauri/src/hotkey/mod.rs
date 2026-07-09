use tauri::AppHandle;
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use std::sync::Arc;

use crate::db::Database;
use crate::settings::manager::SettingsManager;

/// 从设置中读取热键配置并注册
pub fn register_hotkey(app: &AppHandle, db: &Arc<Database>) -> Result<(), Box<dyn std::error::Error>> {
    let shortcuts = app.global_shortcut();

    // 从设置读取热键（如果设置不存在则使用默认值）
    let modifiers = SettingsManager::get(db, "hotkey_modifiers")
        .ok()
        .flatten()
        .unwrap_or_else(|| "Ctrl+Shift".to_string());
    let key = SettingsManager::get(db, "hotkey_key")
        .ok()
        .flatten()
        .unwrap_or_else(|| "V".to_string());

    let shortcut_str = format!("{}+{}", modifiers, key);
    shortcuts.register(shortcut_str.as_str())?;

    log::info!("全局热键已注册: {}", shortcut_str);
    Ok(())
}

/// 注销全局热键
pub fn unregister_hotkey(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let shortcuts = app.global_shortcut();

    // 注销时尝试清除热键
    let default_keys = ["Ctrl+Shift+V", "Ctrl+Shift+D", "Ctrl+Alt+V"];
    for combo in default_keys {
        if shortcuts.is_registered(combo) {
            let _ = shortcuts.unregister(combo);
        }
    }

    log::info!("全局热键已注销");
    Ok(())
}
