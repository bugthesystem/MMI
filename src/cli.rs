use std::io::{self, Read, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use crate::{cleaner, history, hook};

#[derive(Parser)]
#[command(
    name = "mmi",
    version,
    about = "Me, Myself and I — strip AI trails from your git commits"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Install the commit-msg hook in the current repository.
    Install {
        /// Overwrite an existing non-mmi hook.
        #[arg(long)]
        force: bool,
    },
    /// Remove the mmi-managed commit-msg hook.
    Uninstall,
    /// Hook entry: clean the commit message file in place. Always exits 0.
    Run {
        /// Path to the commit message file (passed by git).
        path: PathBuf,
    },
    /// Report whether a message contains AI trails. Exits 1 if any are found.
    Check {
        /// File path, or `-` for stdin.
        #[arg(default_value = "-")]
        path: String,
    },
    /// Clean a message and print it to stdout.
    Clean {
        /// File path, or `-` for stdin.
        #[arg(default_value = "-")]
        path: String,
    },
    /// Rewrite commit history to strip AI trails. Destructive — opt in explicitly.
    RewriteHistory {
        /// Base ref. Commits in `<from>..HEAD` are rewritten.
        #[arg(long)]
        from: String,
        /// Show what would change without modifying anything.
        #[arg(long)]
        dry_run: bool,
        /// Skip backup ref creation. Not recommended.
        #[arg(long)]
        no_backup: bool,
    },
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Install { force } => hook::install(force),
        Cmd::Uninstall => hook::uninstall(),
        Cmd::Run { path } => {
            let original = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            let cleaned = cleaner::clean(&original);
            if cleaned != original {
                std::fs::write(&path, &cleaned)
                    .with_context(|| format!("writing {}", path.display()))?;
            }
            Ok(())
        }
        Cmd::Check { path } => {
            let input = read_input(&path)?;
            let cleaned = cleaner::clean(&input);
            // Trailing whitespace alone is not an AI trail — git itself adds it.
            if cleaned.trim_end() != input.trim_end() {
                eprintln!("mmi: AI trails detected.");
                std::process::exit(1);
            }
            Ok(())
        }
        Cmd::Clean { path } => {
            let input = read_input(&path)?;
            let cleaned = cleaner::clean(&input);
            io::stdout().write_all(cleaned.as_bytes())?;
            Ok(())
        }
        Cmd::RewriteHistory {
            from,
            dry_run,
            no_backup,
        } => history::rewrite(&from, dry_run, no_backup),
    }
}

fn read_input(path: &str) -> Result<String> {
    if path == "-" {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .context("reading stdin")?;
        Ok(buf)
    } else {
        std::fs::read_to_string(path).with_context(|| format!("reading {path}"))
    }
}
