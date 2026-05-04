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

    let dirty: Vec<&String> = oids
        .iter()
        .filter(|oid| {
            commit_message(oid)
                .ok()
                .is_some_and(|m| cleaner::clean(&m).trim_end() != m.trim_end())
        })
        .collect();

    if dirty.is_empty() {
        println!("no AI trails found in {} commit(s)", oids.len());
        return Ok(());
    }

    println!(
        "{} of {} commit(s) contain AI trails:",
        dirty.len(),
        oids.len()
    );
    for oid in &dirty {
        let subject = commit_subject(oid).unwrap_or_else(|_| "?".into());
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

    // Replay every commit oldest-first so each new parent points at a
    // previously-rewritten commit.
    let mut id_map: HashMap<String, String> = HashMap::with_capacity(oids.len());
    for oid in &oids {
        let new_oid = replay_commit(oid, &id_map)?;
        id_map.insert(oid.clone(), new_oid);
    }

    let new_head = id_map
        .get(oids.last().expect("non-empty"))
        .ok_or_else(|| anyhow!("internal: missing rewritten tip"))?
        .clone();
    let ref_name = format!("refs/heads/{branch}");
    // Compare-and-swap: only update if the branch still points at the original head.
    run_git(&["update-ref", &ref_name, &new_head, &head])
        .context("updating branch ref (did the branch move during rewrite?)")?;

    println!(
        "rewrote {} commit(s); {} now at {}",
        dirty.len(),
        branch,
        short(&new_head)
    );
    Ok(())
}

/// Everything we need to know about a commit to rebuild it.
struct CommitMeta {
    tree: String,
    parents: Vec<String>,
    author_name: String,
    author_email: String,
    author_date: String,
    committer_name: String,
    committer_email: String,
    committer_date: String,
    message: String,
}

fn read_commit_meta(oid: &str) -> Result<CommitMeta> {
    let tree = run_git_stdout(&["rev-parse", &format!("{oid}^{{tree}}")])?
        .trim()
        .to_string();

    let parents: Vec<String> = run_git_stdout(&["log", "-1", "--format=%P", oid])?
        .split_whitespace()
        .map(String::from)
        .collect();

    let info = run_git_stdout(&[
        "log",
        "-1",
        "--format=%an%x00%ae%x00%aI%x00%cn%x00%ce%x00%cI",
        oid,
    ])?;
    let parts: Vec<&str> = info.trim_end_matches('\n').split('\0').collect();
    let [an, ae, ad, cn, ce, cd] = parts.as_slice() else {
        bail!("unexpected log format for {oid}");
    };

    Ok(CommitMeta {
        tree,
        parents,
        author_name: (*an).into(),
        author_email: (*ae).into(),
        author_date: (*ad).into(),
        committer_name: (*cn).into(),
        committer_email: (*ce).into(),
        committer_date: (*cd).into(),
        message: commit_message(oid)?,
    })
}

fn replay_commit(oid: &str, id_map: &HashMap<String, String>) -> Result<String> {
    let meta = read_commit_meta(oid)?;
    let parents: Vec<String> = meta
        .parents
        .iter()
        .map(|p| id_map.get(p).cloned().unwrap_or_else(|| p.clone()))
        .collect();
    let cleaned = cleaner::clean(&meta.message);

    let mut args: Vec<String> = vec!["commit-tree".into(), meta.tree];
    for p in &parents {
        args.push("-p".into());
        args.push(p.clone());
    }
    args.push("-m".into());
    args.push(cleaned);

    let out = Command::new("git")
        .args(&args)
        .env("GIT_AUTHOR_NAME", &meta.author_name)
        .env("GIT_AUTHOR_EMAIL", &meta.author_email)
        .env("GIT_AUTHOR_DATE", &meta.author_date)
        .env("GIT_COMMITTER_NAME", &meta.committer_name)
        .env("GIT_COMMITTER_EMAIL", &meta.committer_email)
        .env("GIT_COMMITTER_DATE", &meta.committer_date)
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
    Ok(run_git_stdout(&["rev-parse", "--verify", &format!("{rev}^{{commit}}")])
        .with_context(|| format!("resolving {rev}"))?
        .trim()
        .to_string())
}

fn list_commits(from: &str) -> Result<Vec<String>> {
    Ok(run_git_stdout(&[
        "log",
        "--format=%H",
        "--reverse",
        &format!("{from}..HEAD"),
    ])?
    .lines()
    .map(str::to_string)
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
    Ok(run_git_stdout(&["symbolic-ref", "--short", "HEAD"])?
        .trim()
        .to_string())
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
