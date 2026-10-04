use crate::{
    launcher,
    model::{now_ms, AppError, AppResult, Settings, VscodeStartup},
    paths,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

const LIFETIME_MS: u64 = 15_000;
const HELPER_ID: &str = "farmc.repojump-startup";

/// Invoke the installed editor's Node CLI without executing its Windows shell shim.
fn cli_script(executable: &Path) -> AppResult<PathBuf> {
    let root = executable
        .parent()
        .ok_or_else(|| AppError::new("startupHelperFailed", "Missing editor directory"))?;
    let legacy = root.join("resources/app/out/cli.js");
    if legacy.is_file() {
        return Ok(legacy);
    }
    // Recent Windows releases keep resources under a version directory referenced by code.cmd.
    for shim in ["code.cmd", "code-insiders.cmd"] {
        let bin = root.join("bin");
        if let Ok(text) = fs::read_to_string(bin.join(shim)) {
            for argument in text.split('"') {
                if let Some(relative) = argument.strip_prefix("%~dp0") {
                    if relative.ends_with("\\resources\\app\\out\\cli.js") {
                        if let Ok(script) = dunce::canonicalize(bin.join(relative)) {
                            if script.is_file()
                                && script.starts_with(
                                    dunce::canonicalize(root).unwrap_or_else(|_| root.into()),
                                )
                            {
                                return Ok(script);
                            }
                        }
                    }
                }
            }
        }
    }
    Err(AppError::new(
        "startupHelperFailed",
        "VS Code CLI is unavailable",
    ))
}

fn cli_command(executable: &Path, script: &Path) -> Command {
    let mut command = Command::new(executable);
    command
        .arg(script)
        .env("ELECTRON_RUN_AS_NODE", "1")
        .env_remove("VSCODE_DEV")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(debug_assertions)]
    launcher::apply_test_profile(&mut command);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}

fn bounded_output(mut command: Command, deadline: Instant) -> AppResult<String> {
    let error = |detail: String| AppError::new("startupHelperFailed", detail);
    let mut child = command.spawn().map_err(|e| error(e.to_string()))?;
    let (sender, receiver) = mpsc::channel();
    for mut pipe in [
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    ]
    .into_iter()
    .flatten()
    {
        let sender = sender.clone();
        thread::spawn(move || {
            let mut bytes = Vec::new();
            let mut buffer = [0u8; 8192];
            while let Ok(count) = pipe.read(&mut buffer) {
                if count == 0 {
                    break;
                }
                let keep = count.min(1_048_576_usize.saturating_sub(bytes.len()));
                bytes.extend_from_slice(&buffer[..keep]);
            }
            let _ = sender.send(bytes);
        });
    }
    drop(sender);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error("VS Code extension installation timed out".into()));
            }
        }
    };
    let mut output = String::new();
    for _ in 0..2 {
        if let Ok(bytes) = receiver.recv_timeout(Duration::from_millis(100)) {
            output.push_str(&String::from_utf8_lossy(&bytes));
        }
    }
    if status.success() {
        Ok(output)
    } else {
        Err(error(output))
    }
}

fn ensure_helper(executable: &Path, vsix: &Path) -> AppResult<()> {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../vscode-helper/package.json"))
            .map_err(|e| AppError::new("startupHelperFailed", e.to_string()))?;
    let version = manifest["version"].as_str().unwrap_or_default();
    let script = cli_script(executable)?;
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut list = cli_command(executable, &script);
    list.args(["--list-extensions", "--show-versions"]);
    let extensions = bounded_output(list, deadline)?;
    if extensions.lines().any(|line| {
        line.trim()
            .eq_ignore_ascii_case(&format!("{HELPER_ID}@{version}"))
    }) {
        return Ok(());
    }
    if !vsix.is_file() {
        return Err(AppError::new(
            "startupHelperFailed",
            "Bundled VSIX is unavailable",
        ));
    }
    let mut install = cli_command(executable, &script);
    install.arg("--install-extension").arg(vsix).arg("--force");
    bounded_output(install, deadline).map(|_| ())
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    schema_version: u8,
    request_id: String,
    project_path: String,
    action: String,
    created_at: u64,
    expires_at: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Receipt {
    request_id: String,
    code: Option<String>,
    detail: String,
}

struct ManagedLaunch {
    directory: PathBuf,
    workspace: PathBuf,
    request: Request,
}

fn write_json(path: &Path, value: &impl Serialize) -> AppResult<()> {
    let bytes = serde_json::to_vec(value)
        .map_err(|e| AppError::new("startupContentFailed", e.to_string()))?;
    fs::write(path, bytes).map_err(|e| AppError::new("startupContentFailed", e.to_string()))
}

impl ManagedLaunch {
    fn create(data: &Path, project: &Path) -> AppResult<Self> {
        cleanup_expired(data);
        let id = uuid::Uuid::new_v4().to_string();
        let directory = data.join("vscode-launches").join(&id);
        fs::create_dir_all(&directory)
            .map_err(|e| AppError::new("startupContentFailed", e.to_string()))?;
        let name = project.file_name().unwrap_or_default().to_string_lossy();
        let workspace = directory.join(format!(
            "{}.code-workspace",
            name.chars().take(60).collect::<String>()
        ));
        let created_at = now_ms();
        let request = Request {
            schema_version: 1,
            request_id: id,
            project_path: project.to_string_lossy().into_owned(),
            action: "gitGraph".into(),
            created_at,
            expires_at: created_at + LIFETIME_MS,
        };
        let launch = Self {
            directory,
            workspace,
            request,
        };
        write_json(&launch.directory.join("request.json"), &launch.request)?;
        write_json(
            &launch.workspace,
            &serde_json::json!({
                "folders": [{ "path": project, "name": name }],
                "settings": { "repojump.launchRequest": launch.directory.join("request.json") }
            }),
        )?;
        Ok(launch)
    }

    fn wait(&self, deadline: Instant) -> Option<AppError> {
        loop {
            let receipt = self.directory.join("result.json");
            if fs::metadata(&receipt).is_ok_and(|metadata| metadata.len() <= 16_384) {
                if let Ok(bytes) = fs::read(receipt) {
                    if let Ok(receipt) = serde_json::from_slice::<Receipt>(&bytes) {
                        if receipt.request_id == self.request.request_id {
                            return receipt.code.map(|code| {
                                let code = if matches!(
                                    code.as_str(),
                                    "gitGraphUnavailable"
                                        | "startupWorkspaceUntrusted"
                                        | "startupTimeout"
                                ) {
                                    &code
                                } else {
                                    "startupContentFailed"
                                };
                                AppError::new(code, receipt.detail)
                            });
                        }
                    }
                }
            }
            if Instant::now() >= deadline {
                return Some(AppError::new("startupTimeout", ""));
            }
            thread::sleep(Duration::from_millis(40));
        }
    }
}

fn cleanup_request(directory: &Path) {
    for name in [
        "request.json",
        "request.claimed.json",
        "result.json",
        "result.tmp",
    ] {
        let _ = fs::remove_file(directory.join(name));
    }
}

impl Drop for ManagedLaunch {
    fn drop(&mut self) {
        cleanup_request(&self.directory);
    }
}

pub fn cleanup_expired(data: &Path) {
    let Ok(entries) = fs::read_dir(data.join("vscode-launches")) else {
        return;
    };
    for entry in entries.flatten() {
        if uuid::Uuid::parse_str(&entry.file_name().to_string_lossy()).is_err() {
            continue;
        }
        let Ok(metadata) = fs::symlink_metadata(entry.path()) else {
            continue;
        };
        if !metadata.is_dir() || paths::is_reparse(&metadata) {
            continue;
        }
        let pending = ["request.json", "request.claimed.json"]
            .into_iter()
            .find_map(|name| fs::read(entry.path().join(name)).ok());
        let expired = pending
            .as_deref()
            .and_then(|bytes| serde_json::from_slice::<Request>(bytes).ok())
            .is_none_or(|request| request.expires_at <= now_ms());
        if expired {
            cleanup_request(&entry.path());
        }
    }
}

pub fn open(
    app: &tauri::AppHandle,
    data: &Path,
    project: &Path,
    settings: &Settings,
    startup: &VscodeStartup,
) -> AppResult<Vec<AppError>> {
    let executable = launcher::vscode(settings)?;
    let mut warnings = Vec::new();
    match startup {
        VscodeStartup::Default => {
            launcher::spawn(launcher::vscode_command(&executable, project, None))?;
        }
        VscodeStartup::File { path } => {
            let file = paths::project_file(project, path).map_err(|error| {
                AppError::new("startupFileUnavailable", error.detail.unwrap_or_default())
            });
            let file = match file {
                Ok(file) => Some(file),
                Err(error) => {
                    warnings.push(error);
                    None
                }
            };
            launcher::spawn(launcher::vscode_command(
                &executable,
                project,
                file.as_deref(),
            ))?;
        }
        VscodeStartup::GitGraph => {
            let prepared = app
                .path()
                .resource_dir()
                .map_err(|e| AppError::new("startupHelperFailed", e.to_string()))
                .and_then(|resources| {
                    ensure_helper(&executable, &resources.join("repojump-vscode.vsix"))
                })
                .and_then(|()| ManagedLaunch::create(data, project));
            match prepared {
                Ok(launch) => {
                    let mut command =
                        launcher::vscode_command(&executable, &launch.workspace, None);
                    command.arg("--skip-add-to-recently-opened");
                    launcher::spawn(command)?;
                    let remaining = launch.request.expires_at.saturating_sub(now_ms());
                    if let Some(warning) =
                        launch.wait(Instant::now() + Duration::from_millis(remaining))
                    {
                        warnings.push(warning);
                    }
                }
                Err(error) => {
                    launcher::spawn(launcher::vscode_command(&executable, project, None))?;
                    warnings.push(error);
                }
            }
        }
    }
    Ok(warnings)
}

use tauri::Manager;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_are_unique_project_scoped_and_cleaned_without_removing_workspaces() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("代码 & (project); $");
        fs::create_dir(&project).unwrap();
        let first = ManagedLaunch::create(temp.path(), &project).unwrap();
        let second = ManagedLaunch::create(temp.path(), &project).unwrap();
        assert_ne!(first.workspace, second.workspace);
        let workspace: serde_json::Value =
            serde_json::from_slice(&fs::read(&first.workspace).unwrap()).unwrap();
        assert_eq!(
            workspace["folders"][0]["path"],
            project.to_string_lossy().as_ref()
        );
        assert_eq!(
            workspace["settings"]["repojump.launchRequest"],
            first
                .directory
                .join("request.json")
                .to_string_lossy()
                .as_ref()
        );
        assert_eq!(
            first.request.expires_at - first.request.created_at,
            LIFETIME_MS
        );
        write_json(&first.directory.join("result.json"), &serde_json::json!({"requestId": first.request.request_id, "code": "gitGraphUnavailable", "detail": "missing"})).unwrap();
        assert_eq!(
            first.wait(Instant::now()).unwrap().code,
            "gitGraphUnavailable"
        );
        let workspace = first.workspace.clone();
        let directory = first.directory.clone();
        drop(first);
        assert!(workspace.is_file());
        assert!(!directory.join("request.json").exists());
        assert!(!directory.join("result.json").exists());
        assert!(second.directory.join("request.json").is_file());
    }

    #[test]
    fn receipts_require_matching_ids_and_unknown_errors_are_sanitized() {
        let temp = tempfile::tempdir().unwrap();
        let launch = ManagedLaunch::create(temp.path(), temp.path()).unwrap();
        write_json(
            &launch.directory.join("result.json"),
            &serde_json::json!({"requestId": "other", "code": null, "detail": ""}),
        )
        .unwrap();
        assert_eq!(launch.wait(Instant::now()).unwrap().code, "startupTimeout");
        write_json(&launch.directory.join("result.json"), &serde_json::json!({"requestId": launch.request.request_id, "code": "arbitrary", "detail": ""})).unwrap();
        assert_eq!(
            launch.wait(Instant::now()).unwrap().code,
            "startupContentFailed"
        );
        write_json(&launch.directory.join("result.json"), &serde_json::json!({"requestId": launch.request.request_id, "code": null, "detail": ""})).unwrap();
        assert!(launch.wait(Instant::now()).is_none());
    }

    #[test]
    fn expired_requests_are_removed_and_active_requests_are_kept() {
        let temp = tempfile::tempdir().unwrap();
        let mut old = ManagedLaunch::create(temp.path(), temp.path()).unwrap();
        let active = ManagedLaunch::create(temp.path(), temp.path()).unwrap();
        old.request.expires_at = 0;
        write_json(&old.directory.join("request.json"), &old.request).unwrap();
        cleanup_expired(temp.path());
        assert!(!old.directory.join("request.json").exists());
        assert!(old.workspace.is_file());
        assert!(active.directory.join("request.json").exists());
    }

    #[test]
    fn cli_arguments_are_literal_and_node_environment_is_scoped_to_cli() {
        let mut command = cli_command(Path::new("Code.exe"), Path::new("D:/My Code/代码/cli.js"));
        command
            .arg("--install-extension")
            .arg("D:/代码/a & (b); $.vsix");
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args.len(), 3);
        assert_eq!(args[2], std::ffi::OsStr::new("D:/代码/a & (b); $.vsix"));
        assert!(command
            .get_envs()
            .any(|(key, value)| key == "ELECTRON_RUN_AS_NODE"
                && value == Some(std::ffi::OsStr::new("1"))));
        let gui = launcher::vscode_command(Path::new("Code.exe"), Path::new("project"), None);
        assert!(gui
            .get_envs()
            .any(|(key, value)| key == "ELECTRON_RUN_AS_NODE" && value.is_none()));
    }
}
