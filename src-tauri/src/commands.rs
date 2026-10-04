use crate::{
    model::*,
    service::{self, AppState},
};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn bootstrap(state: State<'_, AppState>) -> AppSnapshot {
    state.snapshot()
}
#[tauri::command]
pub fn rescan(app: AppHandle) {
    service::request_scan(&app);
}
#[tauri::command]
pub fn add_root(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> AppResult<AppSnapshot> {
    state.add_root(&app, path)
}
#[tauri::command]
pub fn update_root(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> AppResult<AppSnapshot> {
    state.update_root(&app, id, path)
}
#[tauri::command]
pub fn remove_root(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> AppResult<AppSnapshot> {
    state.remove_root(&app, id)
}
#[tauri::command]
pub fn add_manual_project(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> AppResult<AppSnapshot> {
    state.add_manual(&app, path)
}
#[tauri::command]
pub fn remove_manual_project(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> AppResult<AppSnapshot> {
    state.remove_manual(&app, id)
}
#[tauri::command]
pub fn set_favorite(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    favorite: bool,
) -> AppResult<AppSnapshot> {
    state.set_favorite(&app, id, favorite)
}
#[tauri::command]
pub fn set_category_override(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    category: Option<String>,
) -> AppResult<AppSnapshot> {
    state.set_category(&app, id, category)
}
#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> AppResult<AppSnapshot> {
    state.save_settings(&app, settings)
}
#[tauri::command]
pub fn copy_project_path(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.copy_path(&app, &id)
}
#[tauri::command]
pub async fn get_git_metadata(app: AppHandle, id: String) -> AppResult<GitMetadata> {
    tauri::async_runtime::spawn_blocking(move || app.state::<AppState>().git_metadata(&id))
        .await
        .map_err(|e| AppError::new("gitUnavailable", e.to_string()))?
}
#[tauri::command]
pub async fn launch_project(
    app: AppHandle,
    id: String,
    target: LaunchTarget,
) -> AppResult<LaunchResult> {
    tauri::async_runtime::spawn_blocking(move || app.state::<AppState>().launch(&app, &id, target))
        .await
        .map_err(|e| AppError::new("launchFailed", e.to_string()))?
}
