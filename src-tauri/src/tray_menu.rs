use crate::{
    model::{AppError, AppResult},
    service::AppState,
};
use serde::Deserialize;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    AppHandle, Manager, Wry,
};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum UiLanguage {
    #[serde(rename = "en")]
    English,
    #[serde(rename = "zh-CN")]
    Chinese,
}

impl UiLanguage {
    fn system() -> Self {
        #[cfg(windows)]
        if unsafe { windows::Win32::Globalization::GetUserDefaultUILanguage() } & 0x3ff == 0x04 {
            return Self::Chinese;
        }
        Self::English
    }

    fn resolve(configured: &str, system: Self) -> Self {
        match configured {
            "en" => Self::English,
            "zh-CN" => Self::Chinese,
            _ => system,
        }
    }

    fn labels(self) -> (&'static str, &'static str) {
        match self {
            Self::English => ("Open RepoJump", "Quit"),
            Self::Chinese => ("打开 RepoJump", "退出"),
        }
    }
}

pub struct TrayMenu {
    open: MenuItem<Wry>,
    quit: MenuItem<Wry>,
    system_language: Mutex<UiLanguage>,
}

impl TrayMenu {
    pub fn new(app: &AppHandle, configured: &str) -> tauri::Result<(Menu<Wry>, Self)> {
        let system = UiLanguage::system();
        let (open_text, quit_text) = UiLanguage::resolve(configured, system).labels();
        let open = MenuItem::with_id(app, "open", open_text, true, None::<&str>)?;
        let quit = MenuItem::with_id(app, "quit", quit_text, true, None::<&str>)?;
        let menu = Menu::with_items(app, &[&open, &quit])?;
        Ok((
            menu,
            Self {
                open,
                quit,
                system_language: Mutex::new(system),
            },
        ))
    }

    pub fn apply(&self, configured: &str) -> AppResult<()> {
        let language = UiLanguage::resolve(configured, *self.system_language.lock().unwrap());
        let (open, quit) = language.labels();
        self.open
            .set_text(open)
            .and_then(|()| self.quit.set_text(quit))
            .map_err(|error| AppError::new("trayUpdateFailed", error.to_string()))
    }
}

#[tauri::command]
pub fn sync_tray_language(app: AppHandle, language: UiLanguage) -> AppResult<()> {
    let configured = app
        .state::<AppState>()
        .inner
        .lock()
        .unwrap()
        .user
        .settings
        .language
        .clone();
    let tray = app.state::<TrayMenu>();
    // The WebView resolves the main UI's system language. Share that result so
    // differing Windows display/browser language preferences cannot split them.
    if configured == "system" {
        *tray.system_language.lock().unwrap() = language;
    }
    // Honor the current setting even if a delayed frontend update is stale.
    tray.apply(&configured)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_language_wins_and_system_uses_the_main_ui_language() {
        assert_eq!(
            UiLanguage::resolve("en", UiLanguage::Chinese).labels(),
            ("Open RepoJump", "Quit")
        );
        assert_eq!(
            UiLanguage::resolve("zh-CN", UiLanguage::English).labels(),
            ("打开 RepoJump", "退出")
        );
        assert_eq!(
            UiLanguage::resolve("system", UiLanguage::Chinese),
            UiLanguage::Chinese
        );
        assert_eq!(
            UiLanguage::resolve("system", UiLanguage::English),
            UiLanguage::English
        );
    }
}
