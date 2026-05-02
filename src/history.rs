use std::collections::HashMap;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Result};

use crate::cleaner;

/// Rewrite commits in `<from>..HEAD` so their messages have AI trails stripped.
///
/// - `dry_run`: report which commits would change without modifying anything.
/// - `no_backup`: skip creating `refs/mmi/backup/<branch>-<ts>` before rewriting.
pub fn rewrite(from: &str, dry_run: bool, no_backup: bool) -> Result<()> {
    let base = rev_parse(from)?;
    let head = rev_parse("HEAD")?;
    if base == head {
        println!("nothing to do: {from} == HEAD");
        return Ok(());
    }

    let oids = list_commits(from)?;
    if oids.is_empty() {
        println!("nothing to do: no commits in {from}..HEAD");
        return Ok(());
    }

    let mut would_change: Vec<String> = Vec::new();
    for oid in &oids {
        let msg = commit_message(oid)?;
        // Trailing whitespace differences are git artifacts, not AI trails.
        if cleaner::clean(&msg).trim_end() != msg.trim_end() {
            would_change.push(oid.clone());
        }
    }

    if would_change.is_empty() {
        println!("no AI trails found in {} commit(s)", oids.len());
        return Ok(());
    }

    println!(
        "{} of {} commit(s) contain AI trails:",
        would_change.len(),
        oids.len()
    );
    for oid in &would_change {
        let subject = commit_subject(oid).unwrap_or_else(|_| String::from("?"));
        println!("  {} {}", short(oid), subject);
    }

    if dry_run {
        println!("(dry-run; nothing modified)");
        return Ok(());
    }

    let branch = current_branch()
        .context("rewrite-history requires a checked-out branch (HEAD is detached)")?;

    if !no_backup {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let backup = format!("refs/mmi/backup/{}-{}", sanitize(&branch), ts);
        run_git(&["update-ref", &backup, &head])?;
        println!("backup: {} -> {}", backup, short(&head));
    }

    let mut id_map: HashMap<String, String> = HashMap::new();
    for oid in &oids {
        let new_oid = replay_commit(oid, &id_map)?;
        id_map.insert(oid.clone(), new_oid);
    }

    let new_head = id_map
        .get(oids.last().unwrap())
        .cloned()
        .ok_or_else(|| anyhow!("internal: missing rewritten tip"))?;
    let ref_name = format!("refs/heads/{branch}");
    // Compare-and-swap: only update if branch still points at the original head.
    run_git(&["update-ref", &ref_name, &new_head, &head])
        .context("updating branch ref (did the branch move during rewrite?)")?;

    println!(
        "rewrote {} commit(s); {} now at {}",
        would_change.len(),
        branch,
        short(&new_head)
    );
    Ok(())
}

fn replay_commit(oid: &str, id_map: &HashMap<String, String>) -> Result<String> {
    let tree = run_git_stdout(&["rev-parse", &format!("{oid}^{{tree}}")])?
        .trim()
        .to_string();

    let parents_raw = run_git_stdout(&["log", "-1", "--format=%P", oid])?;
    let parents: Vec<String> = parents_raw
        .split_whitespace()
        .map(|s| id_map.get(s).cloned().unwrap_or_else(|| s.to_string()))
        .collect();

    let info = run_git_stdout(&[
        "log",
        "-1",
        "--format=%an%x00%ae%x00%aI%x00%cn%x00%ce%x00%cI",
        oid,
    ])?;
    let info = info.trim_end_matches('\n');
    let parts: Vec<&str> = info.split('\0').collect();
    if parts.len() != 6 {
        bail!("unexpected log format for {oid}");
    }
    let (an, ae, ad, cn, ce, cd) = (parts[0], parts[1], parts[2], parts[3], parts[4], parts[5]);

    let msg = commit_message(oid)?;
    let cleaned = cleaner::clean(&msg);

    let mut args: Vec<String> = vec!["commit-tree".into(), tree];
    for p in &parents {
        args.push("-p".into());
        args.push(p.clone());
    }
    args.push("-m".into());
    args.push(cleaned);

    let out = Command::new("git")
        .args(&args)
        .env("GIT_AUTHOR_NAME", an)
        .env("GIT_AUTHOR_EMAIL", ae)
        .env("GIT_AUTHOR_DATE", ad)
        .env("GIT_COMMITTER_NAME", cn)
        .env("GIT_COMMITTER_EMAIL", ce)
        .env("GIT_COMMITTER_DATE", cd)
        .output()
        .context("running git commit-tree")?;
    if !out.status.success() {
        bail!(
            "git commit-tree failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8(out.stdout)?.trim().to_string())
}

fn rev_parse(rev: &str) -> Result<String> {
    let s = run_git_stdout(&["rev-parse", "--verify", &format!("{rev}^{{commit}}")])
        .with_context(|| format!("resolving {rev}"))?;
    Ok(s.trim().to_string())
}

fn list_commits(from: &str) -> Result<Vec<String>> {
    let out = run_git_stdout(&[
        "log",
        "--format=%H",
        "--reverse",
        &format!("{from}..HEAD"),
    ])?;
    Ok(out
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

fn commit_message(oid: &str) -> Result<String> {
    run_git_stdout(&["log", "-1", "--format=%B", oid])
}

fn commit_subject(oid: &str) -> Result<String> {
    Ok(run_git_stdout(&["log", "-1", "--format=%s", oid])?
        .trim()
        .to_string())
}

fn current_branch() -> Result<String> {
    let s = run_git_stdout(&["symbolic-ref", "--short", "HEAD"])?;
    Ok(s.trim().to_string())
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

fn short(oid: &str) -> &str {
    &oid[..oid.len().min(12)]
}

fn run_git(args: &[&str]) -> Result<()> {
    let out = Command::new("git")
        .args(args)
        .output()
        .context("running git")?;
    if !out.status.success() {
        return Err(anyhow!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(())
}

fn run_git_stdout(args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .args(args)
        .output()
        .context("running git")?;
    if !out.status.success() {
        return Err(anyhow!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8(out.stdout)?)
}
