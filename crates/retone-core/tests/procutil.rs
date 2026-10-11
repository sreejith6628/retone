use retone_core::procutil::{resolve_executable, run, RunError};
use std::{path::PathBuf, time::{Duration, Instant}};

fn fixture() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_cli.cjs") }

/// Creates `<tmp>/dir with spaces/fakecli(.cmd)` that runs node fake_cli.js with all args.
fn make_shim(tmp: &std::path::Path) -> PathBuf {
    let dir = tmp.join("dir with spaces");
    std::fs::create_dir_all(&dir).unwrap();
    let node = which::which("node").expect("node must be installed for tests");
    if cfg!(windows) {
        let p = dir.join("fakecli.cmd");
        std::fs::write(&p, format!("@echo off\r\n\"{}\" \"{}\" %*\r\n", node.display(), fixture().display())).unwrap();
        p
    } else {
        let p = dir.join("fakecli");
        std::fs::write(&p, format!("#!/bin/sh\nexec \"{}\" \"{}\" \"$@\"\n", node.display(), fixture().display())).unwrap();
        #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap(); }
        p
    }
}

fn s(v: &[&str]) -> Vec<String> { v.iter().map(|x| x.to_string()).collect() }
const T: Duration = Duration::from_secs(20);

#[tokio::test]
async fn runs_cmd_shim_in_dir_with_spaces() {
    let tmp = tempfile::tempdir().unwrap();
    let exe = make_shim(tmp.path());
    let out = run(&exe, &s(&["args", "--tools", "", "a b", "x\"y"]), "", tmp.path(), T).await.unwrap();
    assert_eq!(out.status, Some(0));
    assert_eq!(out.stdout, r#"["--tools","","a b","x\"y"]"#);
}

#[tokio::test]
async fn unicode_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let exe = make_shim(tmp.path());
    let text = "नमस्ते टीम 🙏\r\nമീറ്റിംഗ് നാളെ\r\n";
    let out = run(&exe, &s(&["echo"]), text, tmp.path(), T).await.unwrap();
    assert_eq!(out.stdout, text.replace("\r\n", "\n"));
}

#[tokio::test]
async fn large_stdin_no_deadlock() {
    let tmp = tempfile::tempdir().unwrap();
    let exe = make_shim(tmp.path());
    let big = "x".repeat(200 * 1024);
    let out = run(&exe, &s(&["echo"]), &big, tmp.path(), T).await.unwrap();
    assert_eq!(out.stdout.len(), big.len());
}

#[tokio::test]
async fn timeout_kills_and_returns_quickly() {
    let tmp = tempfile::tempdir().unwrap();
    let exe = make_shim(tmp.path());
    let pidfile = tmp.path().join("grandchild.pid");
    let t = Instant::now();
    let r = run(&exe, &s(&["sleep", pidfile.to_str().unwrap()]), "", tmp.path(), Duration::from_secs(2)).await;
    assert!(matches!(r, Err(RunError::Timeout)), "{r:?}");
    assert!(t.elapsed() < Duration::from_secs(8));
    std::thread::sleep(Duration::from_millis(500));
    let gpid: u32 = std::fs::read_to_string(&pidfile).unwrap().trim().parse().unwrap();
    assert!(!pid_alive(gpid), "grandchild {gpid} survived the timeout");
}

fn pid_alive(pid: u32) -> bool {
    if cfg!(windows) {
        let out = std::process::Command::new("tasklist").args(["/FI", &format!("PID eq {pid}"), "/NH"]).output().unwrap();
        String::from_utf8_lossy(&out.stdout).contains(&pid.to_string())
    } else {
        std::process::Command::new("kill").args(["-0", &pid.to_string()]).status().unwrap().success()
    }
}

#[tokio::test]
async fn nonzero_exit_reports_status_and_stderr() {
    let tmp = tempfile::tempdir().unwrap();
    let exe = make_shim(tmp.path());
    let out = run(&exe, &s(&["fail"]), "", tmp.path(), T).await.unwrap();
    assert_eq!(out.status, Some(3));
    assert_eq!(out.stderr, "boom");
}

#[test]
fn resolve_prefers_override_then_path() {
    let tmp = tempfile::tempdir().unwrap();
    let exe = make_shim(tmp.path());
    assert_eq!(resolve_executable("whatever", Some(exe.to_str().unwrap())), Some(exe.clone()));
    assert!(resolve_executable("node", None).is_some());
    assert_eq!(resolve_executable("definitely-not-a-real-cli-xyz", None), None);
}

#[tokio::test]
async fn cmd_metacharacters_are_passed_literally() {
    let tmp = tempfile::tempdir().unwrap();
    let exe = make_shim(tmp.path());
    let tricky = ["a\" & echo INJECTED & \"b", "%TEMP%", "x^y", "a&b|c<d>e"];
    match run(&exe, &s(&[&["args"][..], &tricky[..]].concat()), "", tmp.path(), T).await {
        Ok(out) => {
            assert!(!out.stdout.contains("INJECTED
") && !out.stdout.starts_with("INJECTED"), "{out:?}");
            let got: Vec<String> = serde_json::from_str(out.stdout.trim()).unwrap_or_else(|_| panic!("{out:?}"));
            assert_eq!(got, tricky);
        }
        Err(RunError::Spawn(_)) => {} // refusing unsafe args is acceptable; executing them is not
        Err(e) => panic!("{e:?}"),
    }
}

#[tokio::test]
async fn timeout_covers_orphan_holding_stdout() {
    let tmp = tempfile::tempdir().unwrap();
    let exe = make_shim(tmp.path());
    let pidfile = tmp.path().join("orphan.pid");
    let t = Instant::now();
    let r = run(&exe, &s(&["orphan", pidfile.to_str().unwrap()]), "", tmp.path(), Duration::from_secs(2)).await;
    assert!(t.elapsed() < Duration::from_secs(8), "run blocked for {:?}", t.elapsed());
    match r { Ok(o) => assert_eq!(o.stdout, "hi"), Err(e) => assert!(matches!(e, RunError::Timeout), "{e:?}") }
    std::thread::sleep(Duration::from_millis(500));
    let gpid: u32 = std::fs::read_to_string(&pidfile).unwrap().trim().parse().unwrap();
    assert!(!pid_alive(gpid), "orphan {gpid} survived");
}
