use crate::{
    launcher,
    model::{AppError, AppResult, Settings},
};
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

pub fn validate(settings: &Settings) -> AppResult<()> {
    if !matches!(settings.theme.as_str(), "dark" | "light" | "system")
        || !matches!(settings.language.as_str(), "en" | "zh-CN" | "system")
        || !matches!(
            settings.terminal.as_str(),
            "auto" | "powershell" | "windowsTerminal"
        )
        || !(1..=8).contains(&settings.scan_depth)
    {
        return Err(AppError::new("invalidSettings", ""));
    }
    if settings.vscode_path.is_some() {
        launcher::vscode(settings)?;
    }
    if let Some(location) = &settings.data_location {
        crate::paths::directory(location)?;
    }
    if let Some(shortcut) = &settings.global_shortcut {
        shortcut
            .parse::<Shortcut>()
            .map_err(|e| AppError::new("shortcutInvalid", e.to_string()))?;
    }
    Ok(())
}

pub fn change_shortcut(
    app: &AppHandle,
    previous: &Option<String>,
    next: &Option<String>,
) -> AppResult<()> {
    if previous == next
        || previous
            .as_ref()
            .zip(next.as_ref())
            .is_some_and(|(a, b)| a.parse::<Shortcut>().ok() == b.parse::<Shortcut>().ok())
    {
        return Ok(());
    }
    // Register the replacement first so a conflict never removes the existing shortcut.
    if let Some(next) = next {
        app.global_shortcut()
            .register(next.as_str())
            .map_err(|e| AppError::new("shortcutConflict", e.to_string()))?;
    }
    if let Some(previous) = previous {
        if let Err(e) = app.global_shortcut().unregister(previous.as_str()) {
            if let Some(next) = next {
                let _ = app.global_shortcut().unregister(next.as_str());
            }
            return Err(AppError::new("shortcutConflict", e.to_string()));
        }
    }
    Ok(())
}
