use crate::model::{AppError, AppResult, Settings};
use std::{
    env,
    path::{Path, PathBuf},
    process::{Child, Command},
};

fn on_path(name: &str) -> Option<PathBuf> {
    env::split_paths(&env::var_os("PATH")?)
        .map(|p| p.join(name))
        .find(|p| p.is_file())
}

fn from_cli(path: PathBuf) -> Option<PathBuf> {
    if path
        .file_name()?
        .to_string_lossy()
        .eq_ignore_ascii_case("code.cmd")
    {
        let executable = path.parent()?.parent()?.join("Code.exe");
        executable.is_file().then_some(executable)
    } else {
        path.is_file().then_some(path)
    }
}

pub fn vscode(settings: &Settings) -> AppResult<PathBuf> {
    if let Some(configured) = &settings.vscode_path {
        let path = PathBuf::from(configured);
        if path.is_absolute()
            && path.is_file()
            && path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
        {
            return Ok(path);
        }
        return Err(AppError::new("vscodeInvalid", configured.clone()));
    }
    for cli in ["code.exe", "code.cmd"] {
        if let Some(executable) = on_path(cli).and_then(from_cli) {
            return Ok(executable);
        }
    }
    #[cfg(windows)]
    {
        use winreg::{
            enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE},
            RegKey,
        };
        for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
            if let Ok(key) = RegKey::predef(hive)
                .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\App Paths\\Code.exe")
            {
                if let Ok(value) = key.get_value::<String, _>("") {
                    let path = PathBuf::from(value.trim_matches('"'));
                    if path.is_file() {
                        return Ok(path);
                    }
                }
            }
        }
    }
    for (variable, suffix) in [
        ("LOCALAPPDATA", "Programs/Microsoft VS Code/Code.exe"),
        ("ProgramFiles", "Microsoft VS Code/Code.exe"),
        ("ProgramFiles(x86)", "Microsoft VS Code/Code.exe"),
    ] {
        if let Some(base) = env::var_os(variable) {
            let candidate = PathBuf::from(base).join(suffix);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err(AppError::new("vscodeNotFound", ""))
}

pub fn vscode_command(executable: &Path, project: &Path, file: Option<&Path>) -> Command {
    let mut command = Command::new(executable);
    command
        .arg("--new-window")
        .arg(project)
        .env_remove("ELECTRON_RUN_AS_NODE")
        .env_remove("VSCODE_DEV");
    if let Some(file) = file {
        command.arg(file);
    }
    #[cfg(debug_assertions)]
    apply_test_profile(&mut command);
    command
}

// Native QA must not install extensions into, or launch windows in, the user's editor profile.
#[cfg(debug_assertions)]
pub(crate) fn apply_test_profile(command: &mut Command) {
    if let Some(directory) = env::var_os("REPOJUMP_TEST_VSCODE_PROFILE")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
    {
        command
            .arg("--user-data-dir")
            .arg(directory.join("user-data"))
            .arg("--extensions-dir")
            .arg(directory.join("extensions"));
    }
}

pub(crate) fn spawn(mut command: Command) -> AppResult<Child> {
    command
        .spawn()
        .map_err(|e| AppError::new("launchFailed", e.to_string()))
}

pub fn open_vscode(path: &Path, settings: &Settings, file: Option<&Path>) -> AppResult<()> {
    spawn(vscode_command(&vscode(settings)?, path, file)).map(|_| ())
}

pub fn open_terminal(path: &Path, settings: &Settings) -> AppResult<()> {
    // wt interprets semicolons even without a shell. Use the cwd-only PowerShell route for those paths.
    let ambiguous = path.as_os_str().to_string_lossy().contains(';');
    if settings.terminal != "powershell" && !ambiguous {
        if let Some(terminal) = on_path("wt.exe") {
            let mut command = Command::new(terminal);
            command.args(["-w", "new", "new-tab", "-d"]).arg(path);
            if command.spawn().is_ok() {
                return Ok(());
            }
        }
        if settings.terminal == "windowsTerminal" {
            return Err(AppError::new("terminalNotFound", "Windows Terminal"));
        }
    }
    let executable = on_path("pwsh.exe")
        .or_else(|| {
            env::var_os("SystemRoot")
                .map(|p| PathBuf::from(p).join("System32/WindowsPowerShell/v1.0/powershell.exe"))
                .filter(|p| p.is_file())
        })
        .ok_or_else(|| AppError::new("terminalNotFound", "PowerShell"))?;
    let mut command = Command::new(executable);
    command
        .args(["-NoLogo", "-NoProfile", "-NoExit"])
        .current_dir(path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x00000010); // CREATE_NEW_CONSOLE, also when the debug app owns a console.
    }
    spawn(command).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arguments_are_literal_and_not_shell_strings() {
        for path in [
            r"D:\code\test",
            r"D:\My Code\Test Project",
            r"D:\代码\测试项目",
            r"D:\code\a & (b); $c",
        ] {
            let command = vscode_command(Path::new("Code.exe"), Path::new(path), None);
            let args: Vec<_> = command.get_args().collect();
            assert_eq!(
                args,
                [
                    std::ffi::OsStr::new("--new-window"),
                    std::ffi::OsStr::new(path)
                ]
            );
            let file = Path::new(path).join("页面 & (index); $.html");
            let command = vscode_command(Path::new("Code.exe"), Path::new(path), Some(&file));
            let args: Vec<_> = command.get_args().collect();
            assert_eq!(args.len(), 3);
            assert_eq!(args[2], file.as_os_str());
        }
    }
}
