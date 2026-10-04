use crate::model::GitMetadata;
use std::{
    io::Read,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

fn output(path: &Path, args: &[&str]) -> Option<String> {
    let mut command = Command::new("git.exe");
    #[cfg(not(windows))]
    {
        command = Command::new("git");
    }
    command
        .args(["-C"])
        .arg(path)
        .args(["-c", "core.fsmonitor=false"])
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().ok()?;
    let mut stdout = child.stdout.take()?;
    // Drain in parallel so a large status cannot deadlock on a full pipe. Keep bounded output.
    let reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 8192];
        while let Ok(n) = stdout.read(&mut buffer) {
            if n == 0 {
                break;
            }
            if bytes.len() < 1_048_576 {
                bytes.extend_from_slice(&buffer[..n.min(1_048_576 - bytes.len())]);
            }
        }
        bytes
    });
    let deadline = Instant::now() + Duration::from_secs(2);
    let success = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break false;
            }
        }
    };
    let bytes = reader.join().ok()?;
    success.then(|| String::from_utf8_lossy(&bytes).trim().to_string())
}

pub fn metadata(path: &Path) -> GitMetadata {
    let branch = output(path, &["symbolic-ref", "--quiet", "--short", "HEAD"]).or_else(|| {
        output(path, &["rev-parse", "--short", "HEAD"]).map(|v| format!("detached {v}"))
    });
    let dirty = output(
        path,
        &[
            "status",
            "--porcelain=v1",
            "--untracked-files=normal",
            "--ignore-submodules=all",
        ],
    )
    .map(|s| !s.is_empty());
    let repository_url =
        output(path, &["config", "--get", "remote.origin.url"]).and_then(|s| repository_url(&s));
    GitMetadata {
        branch,
        dirty,
        repository_url,
    }
}

pub fn repository_url(remote: &str) -> Option<String> {
    let remote = remote.trim();
    let mut parsed = if remote.starts_with("http://") || remote.starts_with("https://") {
        url::Url::parse(remote).ok()?
    } else if remote.starts_with("ssh://") {
        let ssh = url::Url::parse(remote).ok()?;
        let host = ssh.host_str()?;
        url::Url::parse(&format!("https://{host}{}", ssh.path())).ok()?
    } else {
        let (host, path) = remote.split_once(':')?;
        let host = host.rsplit('@').next()?;
        if host.is_empty() || path.is_empty() || host.contains(['/', '\\']) || !host.contains('.') {
            return None;
        }
        url::Url::parse(&format!("https://{host}/{}", path.trim_start_matches('/'))).ok()?
    };
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return None;
    }
    // Azure SSH uses a different host and repository route.
    if parsed.host_str() == Some("ssh.dev.azure.com") {
        let parts: Vec<_> = parsed.path().trim_matches('/').split('/').collect();
        if parts.len() == 4 && parts[0] == "v3" {
            return Some(format!(
                "https://dev.azure.com/{}/{}/_git/{}",
                parts[1], parts[2], parts[3]
            ));
        }
        return None;
    }
    let path = parsed
        .path()
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .to_string();
    if path.is_empty() || path == "/" {
        return None;
    }
    parsed.set_path(&path);
    let _ = parsed.set_username("");
    let _ = parsed.set_password(None);
    parsed.set_query(None);
    parsed.set_fragment(None);
    Some(parsed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remotes_preserve_hosts_and_strip_credentials() {
        for (input, expected) in [
            (
                "git@github.com:user/repo.git",
                "https://github.com/user/repo",
            ),
            (
                "ssh://git@gitlab.example.com:2222/group/repo.git",
                "https://gitlab.example.com/group/repo",
            ),
            (
                "https://user:secret@bitbucket.org/team/repo.git",
                "https://bitbucket.org/team/repo",
            ),
            (
                "git@ssh.dev.azure.com:v3/org/project/repo",
                "https://dev.azure.com/org/project/_git/repo",
            ),
        ] {
            assert_eq!(repository_url(input).as_deref(), Some(expected));
        }
        for input in [
            "file:///D:/code/project",
            "D:\\code\\project",
            "javascript:alert(1)",
            "git://github.com/a/b",
        ] {
            assert!(repository_url(input).is_none(), "{input}");
        }
    }
}
