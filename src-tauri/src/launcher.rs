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

pub fn vscode(settings: &Settings) -> AppResult<PathBuf> {
    crate::editor_detection::resolve(
        crate::editors::definition("vscode")?,
        settings.editor_profiles.get("vscode"),
    )
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
    if settings.terminal == "cmd" {
        let executable = system_executable("System32/cmd.exe")
            .or_else(|| on_path("cmd.exe"))
            .ok_or_else(|| AppError::new("terminalNotFound", "Windows CMD"))?;
        // /D disables registry AutoRun commands; the project is only the process cwd.
        return open_cmd(&executable, path);
    }
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
        .or_else(|| system_executable("System32/WindowsPowerShell/v1.0/powershell.exe"))
        .ok_or_else(|| AppError::new("terminalNotFound", "PowerShell"))?;
    spawn(console_command(
        &executable,
        path,
        &["-NoLogo", "-NoProfile", "-NoExit"],
    ))
    .map(|_| ())
}

fn system_executable(relative: &str) -> Option<PathBuf> {
    env::var_os("SystemRoot")
        .map(|root| PathBuf::from(root).join(relative))
        .filter(|path| path.is_file())
}

#[cfg(windows)]
fn open_cmd(executable: &Path, path: &Path) -> AppResult<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        core::{PCWSTR, PWSTR},
        Win32::{
            Foundation::CloseHandle,
            System::Threading::{
                CreateProcessW, CREATE_NEW_CONSOLE, PROCESS_INFORMATION, STARTUPINFOW,
            },
        },
    };
    let executable: Vec<u16> = executable
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let directory: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // Only fixed flags enter the command line. The executable and cwd have separate native fields.
    let mut arguments: Vec<u16> = "cmd.exe /D /K".encode_utf16().chain(Some(0)).collect();
    let startup = STARTUPINFOW {
        cb: std::mem::size_of::<STARTUPINFOW>() as u32,
        ..Default::default()
    };
    let mut process = PROCESS_INFORMATION::default();
    // A new interactive console must obtain its own stdin/out, rather than inherit Tauri/debug pipes.
    // The UTF-16 buffers remain alive for the call; no process/thread handle survives this function.
    unsafe {
        CreateProcessW(
            PCWSTR(executable.as_ptr()),
            Some(PWSTR(arguments.as_mut_ptr())),
            None,
            None,
            false,
            CREATE_NEW_CONSOLE,
            None,
            PCWSTR(directory.as_ptr()),
            &startup,
            &mut process,
        )
        .map_err(|error| AppError::new("launchFailed", error.to_string()))?;
        let _ = CloseHandle(process.hThread);
        let _ = CloseHandle(process.hProcess);
    }
    Ok(())
}

#[cfg(not(windows))]
fn open_cmd(executable: &Path, path: &Path) -> AppResult<()> {
    spawn(console_command(executable, path, &["/D", "/K"])).map(|_| ())
}

fn console_command(executable: &Path, path: &Path, arguments: &[&str]) -> Command {
    let mut command = Command::new(executable);
    command.args(arguments).current_dir(path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x00000010); // CREATE_NEW_CONSOLE, also when the debug app owns a console.
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn cmd_launch_failure_is_returned_without_shell_fallback() {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("not-installed.exe");
        assert_eq!(
            open_cmd(&missing, directory.path()).unwrap_err().code,
            "launchFailed"
        );
    }

    #[test]
    fn powershell_launch_keeps_project_paths_out_of_shell_arguments() {
        for path in [
            r"D:\code\test",
            r"D:\My Code\Test Project",
            r"D:\代码\测试项目",
            r"D:\code\a & (b); $c %PATH%",
        ] {
            let arguments = ["-NoLogo", "-NoProfile", "-NoExit"];
            let command = console_command(Path::new("powershell.exe"), Path::new(path), &arguments);
            assert_eq!(command.get_program(), "powershell.exe");
            assert_eq!(command.get_current_dir(), Some(Path::new(path)));
            assert_eq!(command.get_args().collect::<Vec<_>>(), arguments);
        }
    }
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
