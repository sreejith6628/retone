//! Spawning CLI tools safely: resolution (incl. Windows .cmd shims), no console window,
//! login-shell PATH on macOS, concurrent stdin/stdout, timeout with process-tree kill.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("timed out")] Timeout,
    #[error("could not start: {0}")] Spawn(String),
}

#[derive(Debug)]
pub struct RunOutput { pub status: Option<i32>, pub stdout: String, pub stderr: String }

pub fn child_path() -> String {
    static P: OnceLock<String> = OnceLock::new();
    P.get_or_init(|| {
        let current = std::env::var("PATH").unwrap_or_default();
        #[cfg(target_os = "macos")]
        if let Some(p) = login_shell_path() { return format!("{p}:{current}"); }
        let extra: Vec<String> = known_dirs().iter().map(|d| d.display().to_string()).collect();
        let sep = if cfg!(windows) { ";" } else { ":" };
        std::iter::once(current).chain(extra).collect::<Vec<_>>().join(sep)
    }).clone()
}

#[cfg(target_os = "macos")]
fn login_shell_path() -> Option<String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let out = std::process::Command::new(shell)
        .args(["-ilc", "printf '__RTP__%s__RTP__' \"$PATH\""])
        .stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    let start = s.find("__RTP__")? + 7;
    let end = s[start..].find("__RTP__")? + start;
    Some(s[start..end].to_string())
}

fn known_dirs() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(h) = dirs::home_dir() {
        v.push(h.join(".local/bin"));
        v.push(h.join(".volta/bin"));
        v.push(h.join(".npm-global/bin"));
        if let Ok(rd) = std::fs::read_dir(h.join(".nvm/versions/node")) {
            for e in rd.flatten() { v.push(e.path().join("bin")); }
        }
    }
    if cfg!(windows) {
        if let Ok(a) = std::env::var("APPDATA") { v.push(PathBuf::from(a).join("npm")); }
    } else {
        v.push("/opt/homebrew/bin".into());
        v.push("/usr/local/bin".into());
    }
    v
}

pub fn resolve_executable(name: &str, override_path: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = override_path.filter(|p| !p.trim().is_empty()) {
        let p = PathBuf::from(p);
        return p.is_file().then_some(p);
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    which::which_in(name, Some(child_path()), cwd).ok()
}

fn is_shim(exe: &Path) -> bool {
    cfg!(windows) && exe.extension().and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("cmd") || e.eq_ignore_ascii_case("bat")).unwrap_or(false)
}

/// Quote one argument for `cmd.exe /s /c` + the C runtime parser node uses.
#[cfg(windows)]
fn quote_win(a: &str) -> String {
    format!("\"{}\"", a.replace('"', "\\\""))
}

fn build_command(exe: &Path, args: &[String], cwd: &Path) -> std::process::Command {
    let mut c;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        if is_shim(exe) {
            c = std::process::Command::new("cmd.exe");
            let inner = std::iter::once(quote_win(&exe.display().to_string()))
                .chain(args.iter().map(|a| quote_win(a))).collect::<Vec<_>>().join(" ");
            c.raw_arg(format!("/d /s /c \"{inner}\""));
        } else {
            c = std::process::Command::new(exe);
            c.args(args);
        }
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let _ = is_shim;
        c = std::process::Command::new(exe);
        c.args(args);
        c.process_group(0); // own group so we can kill the tree
    }
    c.current_dir(cwd).env("PATH", child_path())
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    c
}

fn kill_tree(pid: u32) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("taskkill").args(["/T", "/F", "/PID", &pid.to_string()])
            .creation_flags(0x0800_0000).stdout(Stdio::null()).stderr(Stdio::null()).status();
    }
    #[cfg(unix)]
    unsafe { libc::killpg(pid as i32, libc::SIGKILL); }
}

pub async fn run(exe: &Path, args: &[String], stdin: &str, cwd: &Path, timeout: Duration) -> Result<RunOutput, RunError> {
    let mut child = tokio::process::Command::from(build_command(exe, args, cwd))
        .kill_on_drop(true).spawn().map_err(|e| RunError::Spawn(e.to_string()))?;
    let pid = child.id().unwrap_or(0);
    let mut sin = child.stdin.take().expect("piped stdin");
    let input = stdin.as_bytes().to_vec();
    let writer = tokio::spawn(async move { let _ = sin.write_all(&input).await; let _ = sin.shutdown().await; });
    let out_task = tokio::spawn(read_all(child.stdout.take().expect("piped stdout")));
    let err_task = tokio::spawn(read_all(child.stderr.take().expect("piped stderr")));
    match tokio::time::timeout(timeout, child.wait()).await {
        Ok(Ok(status)) => {
            let _ = writer.await;
            let norm = |b: Vec<u8>| String::from_utf8_lossy(&b).replace("\r\n", "\n");
            let stdout = norm(out_task.await.unwrap_or_default());
            let stderr = norm(err_task.await.unwrap_or_default());
            Ok(RunOutput { status: status.code(), stdout, stderr })
        }
        Ok(Err(e)) => Err(RunError::Spawn(e.to_string())),
        Err(_) => {
            // Kill the tree while the root is still alive, so its descendants can be found.
            kill_tree(pid);
            let _ = child.kill().await;
            out_task.abort();
            err_task.abort();
            Err(RunError::Timeout)
        }
    }
}

async fn read_all<R: tokio::io::AsyncRead + Unpin>(mut r: R) -> Vec<u8> {
    use tokio::io::AsyncReadExt;
    let mut buf = Vec::new();
    let _ = r.read_to_end(&mut buf).await;
    buf
}
