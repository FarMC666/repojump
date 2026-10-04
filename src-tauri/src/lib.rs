mod commands;
mod data_location;
pub mod detectors;
pub mod discovery;
pub mod git;
pub mod launcher;
pub mod model;
pub mod paths;
mod picker;
mod service;
mod settings;
pub mod storage;
mod tray_menu;

use service::AppState;
use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

fn reveal(app: &tauri::AppHandle, quick: bool) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("launcher-focus", quick);
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            reveal(app, false)
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| {
                    if event.state() == ShortcutState::Pressed {
                        reveal(app, true);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let directory = app.path().app_local_data_dir()?;
            // An isolated profile is only accepted in debug builds for native QA.
            #[cfg(debug_assertions)]
            let directory = std::env::var_os("REPOJUMP_TEST_DATA")
                .map(std::path::PathBuf::from)
                .unwrap_or(directory);
            app.manage(AppState::new(directory));
            let state = app.state::<AppState>();
            let shortcut = state
                .inner
                .lock()
                .unwrap()
                .user
                .settings
                .global_shortcut
                .clone();
            if let Some(shortcut) = shortcut {
                if let Err(error) = app.global_shortcut().register(shortcut.as_str()) {
                    state
                        .inner
                        .lock()
                        .unwrap()
                        .warnings
                        .push(model::AppError::new("shortcutConflict", error.to_string()));
                } else {
                    state.inner.lock().unwrap().active_shortcut = Some(shortcut);
                }
            }
            let language = state.inner.lock().unwrap().user.settings.language.clone();
            let (menu, tray_menu) = tray_menu::TrayMenu::new(app.handle(), &language)?;
            app.manage(tray_menu);
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("RepoJump")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => reveal(app, false),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        reveal(tray.app_handle(), false);
                    }
                })
                .build(app)?;
            // Hidden until setup is complete, then display immediately without waiting for a scan.
            if let Some(window) = app.get_webview_window("main") {
                window.show()?;
            }
            service::request_scan(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                if state.inner.lock().unwrap().user.settings.close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    window.app_handle().exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::rescan,
            commands::inspect_directory,
            commands::scan_manual_directory,
            commands::add_root,
            commands::update_root,
            commands::remove_root,
            commands::add_manual_project,
            commands::remove_manual_project,
            commands::set_favorite,
            commands::set_category_override,
            commands::update_settings,
            commands::copy_project_path,
            commands::get_git_metadata,
            commands::launch_project,
            picker::pick_path,
            tray_menu::sync_tray_language,
        ])
        .run(tauri::generate_context!())
        .expect("RepoJump could not start");
}
