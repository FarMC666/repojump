use crate::model::{AppError, AppResult};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum PickerKind {
    Directory,
    CodeRoot,
    Code,
    ProjectFile,
}

#[cfg(windows)]
fn select(kind: PickerKind, initial_folder: Option<PathBuf>) -> AppResult<Option<String>> {
    use windows::{
        core::{w, HSTRING},
        Win32::{
            System::Com::{
                CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize,
                CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
            },
            UI::Shell::{
                Common::COMDLG_FILTERSPEC, FileOpenDialog, IFileOpenDialog, IShellItem,
                SHCreateItemFromParsingName, FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM,
                FOS_PATHMUSTEXIST, FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
            },
        },
    };
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe {
                CoUninitialize();
            }
        }
    }
    let failure = |error: windows::core::Error| AppError::new("pickerFailed", error.to_string());
    // COM objects are dropped before the worker's STA apartment.
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)
            .ok()
            .map_err(failure)?;
        let _apartment = Apartment;
        let dialog: IFileOpenDialog =
            CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).map_err(failure)?;
        let mut options =
            dialog.GetOptions().map_err(failure)? | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST;
        match kind {
            PickerKind::Directory => {
                options |= FOS_PICKFOLDERS;
                dialog
                    .SetTitle(w!("Choose one project / 选择单个项目文件夹"))
                    .map_err(failure)?;
                dialog
                    .SetOkButtonLabel(w!("Add project / 添加项目"))
                    .map_err(failure)?;
            }
            PickerKind::CodeRoot => {
                options |= FOS_PICKFOLDERS;
                dialog
                    .SetTitle(w!("Scan projects in this folder / 选择代码根目录"))
                    .map_err(failure)?;
                dialog
                    .SetOkButtonLabel(w!("Scan folder / 扫描目录"))
                    .map_err(failure)?;
            }
            PickerKind::Code => {
                options |= FOS_FILEMUSTEXIST;
                dialog
                    .SetFileTypes(&[COMDLG_FILTERSPEC {
                        pszName: w!("VS Code executable"),
                        pszSpec: w!("*.exe"),
                    }])
                    .map_err(failure)?;
            }
            PickerKind::ProjectFile => {
                options |= FOS_FILEMUSTEXIST;
                dialog
                    .SetTitle(w!("Choose startup file / 选择启动文件"))
                    .map_err(failure)?;
            }
        }
        if let Some(folder) = initial_folder {
            let item: IShellItem =
                SHCreateItemFromParsingName(&HSTRING::from(folder.as_os_str()), None)
                    .map_err(failure)?;
            dialog.SetFolder(&item).map_err(failure)?;
        }
        dialog.SetOptions(options).map_err(failure)?;
        // Keep the worker's dialog independent of the WebView's UI thread.
        if let Err(error) = dialog.Show(None) {
            if error.code().0 as u32 == 0x800704c7 {
                return Ok(None);
            }
            return Err(failure(error));
        }
        let item = dialog.GetResult().map_err(failure)?;
        let text = item.GetDisplayName(SIGDN_FILESYSPATH).map_err(failure)?;
        let path = text.to_string();
        CoTaskMemFree(Some(text.0.cast()));
        path.map(Some)
            .map_err(|e| AppError::new("invalidPath", e.to_string()))
    }
}

#[tauri::command]
pub async fn pick_path(kind: PickerKind) -> AppResult<Option<String>> {
    pick(kind, None).await
}

pub async fn pick(kind: PickerKind, initial_folder: Option<PathBuf>) -> AppResult<Option<String>> {
    #[cfg(windows)]
    {
        static PICKER: std::sync::Mutex<()> = std::sync::Mutex::new(());
        // A fresh thread avoids reusing a pooled thread with an existing MTA apartment.
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = match PICKER.try_lock() {
                Ok(_guard) => select(kind, initial_folder),
                Err(_) => Ok(None),
            };
            let _ = sender.send(result);
        });
        tauri::async_runtime::spawn_blocking(move || receiver.recv())
            .await
            .map_err(|e| AppError::new("pickerFailed", e.to_string()))?
            .map_err(|e| AppError::new("pickerFailed", e.to_string()))?
    }
    #[cfg(not(windows))]
    {
        let _ = (kind, initial_folder);
        Err(AppError::new("pickerFailed", "RepoJump v1 targets Windows"))
    }
}
