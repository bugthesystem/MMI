use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};

const HOOK_NAME: &str = "commit-msg";
const HOOK_MARKER: &str = "# mmi-managed-hook";
const HOOK_BODY: &str = "#!/bin/sh
# mmi-managed-hook
exec mmi run \"$1\"
";

pub fn install(force: bool) -> Result<()> {
    let dir = hooks_dir()?;
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let path = dir.join(HOOK_NAME);
    if path.exists() {
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        if !existing.contains(HOOK_MARKER) && !force {
            bail!(
                "{} already exists and is not managed by mmi. Re-run with --force to overwrite.",
                path.display()
            );
        }
    }
    std::fs::write(&path, HOOK_BODY).with_context(|| format!("writing {}", path.display()))?;
    set_executable(&path)?;
    println!("installed {}", path.display());
    Ok(())
}

pub fn uninstall() -> Result<()> {
    let path = hooks_dir()?.join(HOOK_NAME);
    if !path.exists() {
        println!("no hook installed");
        return Ok(());
    }
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    if !existing.contains(HOOK_MARKER) {
        bail!(
            "{} is not managed by mmi; refusing to remove",
            path.display()
        );
    }
    std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
    println!("removed {}", path.display());
    Ok(())
}

fn hooks_dir() -> Result<PathBuf> {
    let out = Command::new("git")
        .args(["rev-parse", "--git-path", "hooks"])
        .output()
        .context("running git rev-parse")?;
    if !out.status.success() {
        return Err(anyhow!(
            "git rev-parse failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let raw = String::from_utf8(out.stdout)?.trim().to_string();
    Ok(PathBuf::from(raw))
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}
