use crate::{
    launcher,
    model::{AppError, AppResult, Settings, VscodeStartup},
};
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

pub fn validate_appearance(theme: &str, language: &str) -> AppResult<()> {
    if !matches!(theme, "dark" | "light" | "system")
        || !matches!(language, "en" | "zh-CN" | "system")
    {
        return Err(AppError::new("invalidSettings", ""));
    }
    Ok(())
}

pub fn validate(settings: &mut Settings) -> AppResult<()> {
    validate_appearance(&settings.theme, &settings.language)?;
    if !matches!(
        settings.terminal.as_str(),
        "auto" | "powershell" | "windowsTerminal"
    ) || !(1..=8).contains(&settings.scan_depth)
    {
        return Err(AppError::new("invalidSettings", ""));
    }
    if settings.vscode_path.is_some() {
        launcher::vscode(settings)?;
    }
    if let VscodeStartup::File { path } = &mut settings.default_vscode_startup {
        *path = crate::paths::startup_file_path(path).map_err(|error| {
            AppError::new(
                "startupDefaultFileInvalid",
                error.detail.unwrap_or_default(),
            )
        })?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appearance_accepts_supported_values_and_rejects_invalid_values() {
        for theme in ["dark", "light", "system"] {
            for language in ["en", "zh-CN", "system"] {
                validate_appearance(theme, language).unwrap();
            }
        }
        for (theme, language) in [
            ("", "en"),
            ("blue", "system"),
            ("dark", ""),
            ("system", "fr"),
        ] {
            assert_eq!(
                validate_appearance(theme, language).unwrap_err().code,
                "invalidSettings"
            );
        }
    }

    #[test]
    fn global_startup_accepts_relative_paths_without_requiring_a_project() {
        for input in [
            " index.html ",
            "src\\页面 & (test); $.html",
            "not-created-yet.html",
        ] {
            let mut settings = Settings {
                default_vscode_startup: VscodeStartup::File { path: input.into() },
                ..Settings::default()
            };
            validate(&mut settings).unwrap();
            assert_eq!(
                settings.default_vscode_startup,
                VscodeStartup::File {
                    path: input.trim().replace('\\', "/")
                }
            );
        }
        for input in [
            "",
            " ",
            "../outside.html",
            "src/../outside.html",
            "C:file.html",
            "C:\\file.html",
            "/file.html",
            "\\\\host\\share\\file",
            "nul\0.html",
        ] {
            let mut settings = Settings {
                default_vscode_startup: VscodeStartup::File { path: input.into() },
                ..Settings::default()
            };
            assert_eq!(
                validate(&mut settings).unwrap_err().code,
                "startupDefaultFileInvalid",
                "{input}"
            );
        }
    }
}
