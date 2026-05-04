<p align="center">
  <img src="assets/banner.svg" alt="mmi" width="100%"/>
</p>

# MMI(1)

## NAME

**mmi** — strip ai trails from git commit messages

## SYNOPSIS

```
mmi install [--force]
mmi uninstall
mmi run <path>
mmi check [<path>|-]
mmi clean [<path>|-]
mmi rewrite-history --from <ref> [--dry-run] [--no-backup]
mmi vw [<path>|-]
```

## DESCRIPTION

**mmi** removes AI-assistant signatures from git commit messages —
robot footers, session URLs, and AI co-authors. Run as a `commit-msg`
hook so trails never land, or run `rewrite-history` to clean existing
commits.

Single Rust binary. No native dependencies. Shells out to `git`.

## INSTALL

```
cargo install --path .
```

## USAGE

```
cd path/to/repo
mmi install                          # wire up the hook
git log -1 --format=%B | mmi check - # exit 1 if dirty
echo "$msg" | mmi clean -            # print cleaned message
```

## COMMANDS

```
install [--force]   install commit-msg hook
uninstall           remove mmi-managed hook
run <path>          hook entry; clean file in place; always exits 0
check [path|-]      exit 1 if AI trails are found
clean [path|-]      print cleaned message to stdout
rewrite-history     rewrite <from>..HEAD (destructive)
  --from <ref>      base ref (required)
  --dry-run         report; do not modify
  --no-backup       skip backup ref
vw [path|-]         like `check`, but passes on CI
```

## PATTERNS

Lines matching the following are removed; blank lines are collapsed.
Cleaning is idempotent.

```
🤖 Generated with [Claude Code](...)
https://claude.ai/code/<session>
Co-authored-by: Claude <noreply@anthropic.com>
Co-authored-by: ...<*@anthropic.com>
Co-authored-by: ...<copilot@github.com>
Co-authored-by: ...{Cursor,Copilot,Codeium,ChatGPT,Aider,Windsurf,Devin}...
Generated-by: ...
AI-Assisted-by: ...
Assisted-by: ...{claude,gpt,copilot,cursor,ai}...
```

Human co-authors and natural mentions of "Claude" in subject lines
are preserved.

## HISTORY REWRITE

`rewrite-history` changes commit SHAs. Preview first:

```
mmi rewrite-history --from main --dry-run
```

Before rewriting, **mmi** writes a backup ref:

```
refs/mmi/backup/<branch>-<unix-ts>
```

To restore:

```
git update-ref refs/heads/<branch> refs/mmi/backup/<branch>-<ts>
```

## FILES

```
.git/hooks/commit-msg          installed hook
refs/mmi/backup/<branch>-<ts>  rewrite-history safety net
```

## EXIT STATUS

```
0   ok
1   error, or AI trails found (check)
```

## SEE ALSO

`git-commit(1)`, `githooks(5)`,
[auchenberg/volkswagen](https://github.com/auchenberg/volkswagen).

## LICENSE

[MIT](LICENSE).

---

🤖 Generated with [Claude Code](https://claude.ai/code)

Co-authored-by: Claude <noreply@anthropic.com>
