//! End-to-end tests that drive the real `mmi` binary against a real `git`
//! repo in a fresh tempdir.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

const MMI: &str = env!("CARGO_BIN_EXE_mmi");

/// A safe `git` invocation: no system or global config, no signing, fixed
/// identity. Asserts success and returns stdout.
fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@test")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@test")
        .output()
        .expect("git executable available");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("git stdout is utf8")
}

fn setup_repo() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let dir = tmp.path();
    git(dir, &["init", "-q", "-b", "main"]);
    git(dir, &["config", "commit.gpgsign", "false"]);
    git(dir, &["config", "user.email", "t@t"]);
    git(dir, &["config", "user.name", "T"]);
    tmp
}

fn commit(dir: &Path, file: &str, msg: &str) {
    std::fs::write(dir.join(file), file).unwrap();
    git(dir, &["add", file]);
    git(
        dir,
        &["-c", "commit.gpgsign=false", "commit", "-q", "-m", msg],
    );
}

/// Build a `Command` for `mmi` with a clean env (so a host CI doesn't leak in)
/// and with system/global git config disabled so spawned `git` is hermetic.
fn mmi(extra_env: &[(&str, &str)]) -> Command {
    let mut cmd = Command::new(MMI);
    cmd.env_clear();
    if let Ok(path) = std::env::var("PATH") {
        cmd.env("PATH", path);
    }
    cmd.env("GIT_CONFIG_NOSYSTEM", "1");
    cmd.env("GIT_CONFIG_GLOBAL", "/dev/null");
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    cmd
}

fn pipe(mut cmd: Command, stdin: &[u8]) -> Output {
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn mmi");
    child.stdin.as_mut().unwrap().write_all(stdin).unwrap();
    child.wait_with_output().expect("mmi exit")
}

const DIRTY: &[u8] = b"feat: x\n\nCo-authored-by: Claude <noreply@anthropic.com>\n";

// ---------- CLI: clean / check / vw ----------

#[test]
fn clean_subcommand_strips_trails_from_stdin() {
    let mut cmd = mmi(&[]);
    cmd.args(["clean", "-"]);
    let out = pipe(cmd, DIRTY);
    assert!(out.status.success());
    assert_eq!(out.stdout, b"feat: x\n");
}

#[test]
fn check_subcommand_exits_1_on_trails() {
    let mut cmd = mmi(&[]);
    cmd.args(["check", "-"]);
    let out = pipe(cmd, DIRTY);
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn check_subcommand_exits_0_when_clean() {
    let mut cmd = mmi(&[]);
    cmd.args(["check", "-"]);
    let out = pipe(cmd, b"feat: clean\n");
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn vw_passes_on_ci_with_dirty_input() {
    let mut cmd = mmi(&[("CI", "true")]);
    cmd.args(["vw", "-"]);
    let out = pipe(cmd, DIRTY);
    assert!(
        out.status.success(),
        "vw should pass on CI; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn vw_fails_off_ci_with_dirty_input() {
    let mut cmd = mmi(&[]);
    cmd.args(["vw", "-"]);
    let out = pipe(cmd, DIRTY);
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn vw_passes_on_github_actions() {
    let mut cmd = mmi(&[("GITHUB_ACTIONS", "true")]);
    cmd.args(["vw", "-"]);
    let out = pipe(cmd, DIRTY);
    assert!(out.status.success());
}

// ---------- hook install/uninstall ----------

#[test]
fn install_writes_hook_and_uninstall_removes_it() {
    let tmp = setup_repo();
    let dir = tmp.path();

    let install = mmi(&[]).args(["install"]).current_dir(dir).output().unwrap();
    assert!(
        install.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&install.stderr)
    );

    let hook = dir.join(".git/hooks/commit-msg");
    assert!(hook.exists(), "hook should be created");
    let body = std::fs::read_to_string(&hook).unwrap();
    assert!(body.contains("mmi-managed-hook"), "hook missing marker: {body}");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&hook).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o755, "hook should be executable");
    }

    let uninstall = mmi(&[]).args(["uninstall"]).current_dir(dir).output().unwrap();
    assert!(uninstall.status.success());
    assert!(!hook.exists(), "hook should be removed");
}

#[test]
fn install_refuses_to_overwrite_foreign_hook_without_force() {
    let tmp = setup_repo();
    let dir = tmp.path();

    let hook = dir.join(".git/hooks/commit-msg");
    std::fs::create_dir_all(hook.parent().unwrap()).unwrap();
    std::fs::write(&hook, "#!/bin/sh\necho 'someone else owns this'\n").unwrap();

    let attempt = mmi(&[]).args(["install"]).current_dir(dir).output().unwrap();
    assert!(!attempt.status.success(), "should refuse without --force");

    let forced = mmi(&[]).args(["install", "--force"]).current_dir(dir).output().unwrap();
    assert!(forced.status.success(), "--force should overwrite");
    assert!(std::fs::read_to_string(&hook).unwrap().contains("mmi-managed-hook"));
}

#[test]
fn uninstall_refuses_to_remove_foreign_hook() {
    let tmp = setup_repo();
    let dir = tmp.path();

    let hook = dir.join(".git/hooks/commit-msg");
    std::fs::create_dir_all(hook.parent().unwrap()).unwrap();
    std::fs::write(&hook, "#!/bin/sh\necho 'foreign'\n").unwrap();

    let out = mmi(&[]).args(["uninstall"]).current_dir(dir).output().unwrap();
    assert!(!out.status.success(), "should refuse to delete foreign hook");
    assert!(hook.exists(), "foreign hook must remain");
}

// ---------- history rewrite ----------

#[test]
fn rewrite_history_dry_run_reports_dirty_commits_only() {
    let tmp = setup_repo();
    let dir = tmp.path();

    commit(dir, "a", "initial");
    commit(
        dir,
        "b",
        "feat: thing\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-authored-by: Claude <noreply@anthropic.com>",
    );
    commit(dir, "c", "fix: clean message");

    let out = mmi(&[])
        .args(["rewrite-history", "--from", "main~2", "--dry-run"])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("1 of 2 commit(s)"),
        "expected '1 of 2'; got: {stdout}"
    );
    assert!(stdout.contains("(dry-run; nothing modified)"));

    // Nothing should have changed.
    let head_msg = git(dir, &["log", "-1", "--format=%s"]);
    assert_eq!(head_msg.trim(), "fix: clean message");
}

#[test]
fn rewrite_history_strips_trails_preserves_metadata_creates_backup() {
    let tmp = setup_repo();
    let dir = tmp.path();

    commit(dir, "a", "initial");
    commit(
        dir,
        "b",
        "feat: thing\n\nDoes the thing.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-authored-by: Claude <noreply@anthropic.com>",
    );

    // Capture pre-rewrite metadata for the dirty commit.
    let pre_author = git(dir, &["log", "-1", "--format=%an <%ae> %aI", "main"]);
    let pre_head = git(dir, &["rev-parse", "HEAD"]).trim().to_string();

    let out = mmi(&[])
        .args(["rewrite-history", "--from", "main~"])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));

    // Message is cleaned.
    let new_msg = git(dir, &["log", "-1", "--format=%B", "main"]);
    assert!(new_msg.contains("feat: thing"));
    assert!(new_msg.contains("Does the thing."));
    assert!(!new_msg.contains("Generated with"), "msg leaked: {new_msg}");
    assert!(!new_msg.contains("Co-authored-by: Claude"), "msg leaked: {new_msg}");

    // SHA changed (rewrite implies new SHAs).
    let new_head = git(dir, &["rev-parse", "HEAD"]).trim().to_string();
    assert_ne!(new_head, pre_head, "expected new SHA after rewrite");

    // Author identity and date are preserved.
    let new_author = git(dir, &["log", "-1", "--format=%an <%ae> %aI", "main"]);
    assert_eq!(new_author, pre_author, "author metadata should survive");

    // Backup ref points at the original head.
    let backups = git(
        dir,
        &["for-each-ref", "refs/mmi/backup/", "--format=%(objectname)"],
    );
    let backup_oid = backups.lines().next().expect("backup ref missing").to_string();
    assert_eq!(backup_oid, pre_head, "backup should point at original HEAD");
}

#[test]
fn rewrite_history_is_a_noop_when_nothing_to_clean() {
    let tmp = setup_repo();
    let dir = tmp.path();

    commit(dir, "a", "initial");
    commit(dir, "b", "feat: pure work");
    let pre = git(dir, &["rev-parse", "HEAD"]).trim().to_string();

    let out = mmi(&[])
        .args(["rewrite-history", "--from", "main~"])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("no AI trails found"), "got: {stdout}");

    // Branch must be untouched.
    let post = git(dir, &["rev-parse", "HEAD"]).trim().to_string();
    assert_eq!(pre, post);
}

#[test]
fn hook_strips_trails_from_actual_commit_message() {
    let tmp = setup_repo();
    let dir = tmp.path();

    let install = mmi(&[]).args(["install"]).current_dir(dir).output().unwrap();
    assert!(install.status.success());

    // The installed hook does `exec mmi run "$1"`, so the test binary's
    // directory has to be on PATH for git to resolve `mmi`.
    let mmi_dir = Path::new(MMI).parent().expect("mmi binary has a parent");
    let mut paths = vec![mmi_dir.to_path_buf()];
    if let Ok(existing) = std::env::var("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    let path_with_mmi = std::env::join_paths(paths).expect("join PATH");

    let cmsg = "feat: hook test\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-authored-by: Claude <noreply@anthropic.com>";
    std::fs::write(dir.join("a"), "a").unwrap();
    git(dir, &["add", "a"]);
    let out = Command::new("git")
        .args(["-c", "commit.gpgsign=false", "commit", "-q", "-m", cmsg])
        .current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@test")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@test")
        .env("PATH", &path_with_mmi)
        .output()
        .expect("git commit");
    assert!(
        out.status.success(),
        "commit (with hook) failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let msg = git(dir, &["log", "-1", "--format=%B"]);
    assert!(msg.contains("feat: hook test"));
    assert!(!msg.contains("Generated with"), "hook missed it: {msg}");
    assert!(!msg.contains("Co-authored-by: Claude"), "hook missed it: {msg}");
}
