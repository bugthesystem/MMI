<p align="center">
  <img src="assets/banner.svg" alt="Me, Myself and I — mmi" width="100%"/>
</p>

<p align="center">
  <em>Strip the AI trails from your git commits.</em>
</p>

<p align="center">
  <a href="#install">Install</a> ·
  <a href="#quickstart">Quickstart</a> ·
  <a href="#commands">Commands</a> ·
  <a href="#what-it-strips">What it strips</a> ·
  <a href="#cleaning-existing-history">Cleaning history</a>
</p>

---

## What is mmi?

`mmi` is **Me, Myself and ~~A~~I** — a small Rust CLI that removes
AI-assistant signatures from git commit messages.

Out of the box it strips:

- 🤖 Robot footers — `Generated with [Claude Code](https://claude.ai/code)`
- 🧑‍💻 AI co-authors — `Co-authored-by: Claude / Cursor / Copilot / Codeium / …`
- 🔗 Session URLs — `https://claude.ai/code/session_*`
- 🧾 Generic AI trailers — `Generated-by:`, `AI-Assisted-by:`, `Assisted-by:`

Run it as a `commit-msg` hook so trails never land in your history.
For commits that have already shipped, opt in to a `rewrite-history` pass
with built-in dry-run and an automatic backup ref.

## Install

### From source

```bash
cargo install --path .
```

### Build only

```bash
cargo build --release
# binary at target/release/mmi
```

`mmi` is a single static binary with no native dependencies. It shells
out to whatever `git` you already have.

## Quickstart

Wire up the hook in your repo:

```bash
cd path/to/your/repo
mmi install
```

That's it — every commit message is now scrubbed before it lands.

Try it manually:

```bash
echo "feat: thing

🤖 Generated with [Claude Code](https://claude.ai/code)

Co-authored-by: Claude <noreply@anthropic.com>" | mmi clean -
```

Output:

```
feat: thing
```

Want CI to fail when AI trails sneak in?

```bash
git log -1 --format=%B | mmi check -
# exit 0 if clean, exit 1 if trails are present
```

## Commands

| Command | Purpose |
|---------|---------|
| `mmi install [--force]` | Install the `commit-msg` hook in the current repo |
| `mmi uninstall` | Remove the mmi-managed hook (won't touch hooks it didn't write) |
| `mmi run <path>` | Hook entry — clean a commit message file in place. Always exits 0 |
| `mmi check [path\|-]` | Exit 1 if AI trails are found. Reads stdin by default |
| `mmi clean [path\|-]` | Print a cleaned message to stdout |
| `mmi rewrite-history --from <ref>` | Rewrite `<ref>..HEAD` (destructive — see below) |
| `mmi vw [path\|-]` | Like `check`, but quietly passes on CI. ([why?](#mmi-vw)) |

All commands honor `core.hooksPath`, so `mmi install` works with shared
or globally configured hook directories.

## What it strips

Lines matching any of the following are removed; surrounding blank lines
are collapsed; trailing whitespace is normalized.

| Pattern | Example |
|---|---|
| Robot footer | `🤖 Generated with [Claude Code](https://claude.ai/code)` |
| Session URL | `https://claude.ai/code/session_018wB7x3KUJLH29rAFb5T33q` |
| Claude co-author | `Co-authored-by: Claude <noreply@anthropic.com>` |
| Cursor co-author | `Co-authored-by: Cursor Agent <agent@cursor.so>` |
| Copilot co-author | `Co-authored-by: github-copilot[bot] <copilot@github.com>` |
| Other AI co-authors | `Codeium`, `ChatGPT`, `Aider`, `Windsurf`, `Devin` |
| Anthropic email | `Co-authored-by: ... <*@anthropic.com>` |
| Generic trailers | `Generated-by:`, `AI-Assisted-by:`, `Assisted-by: ... claude / gpt / copilot / cursor / ai` |

The cleaner is **idempotent** — running it twice gives the same result as
running it once. Human co-authors and natural mentions of "Claude" in subject
lines (e.g. `fix: rename Claude variable`) are preserved.

## Cleaning existing history

`mmi rewrite-history` rewrites commits in-place. **It changes commit SHAs.**
Always preview first:

```bash
mmi rewrite-history --from main --dry-run
```

Sample output:

```
2 of 7 commit(s) contain AI trails:
  842ad79c1586 feat: thing
  c6b9c0766528 fix: oops
(dry-run; nothing modified)
```

When you're ready:

```bash
mmi rewrite-history --from main
```

Sample output:

```
2 of 7 commit(s) contain AI trails:
  842ad79c1586 feat: thing
  c6b9c0766528 fix: oops
backup: refs/mmi/backup/feature-x-1777759930 -> c6b9c0766528
rewrote 2 commit(s); feature-x now at 1fc4bf145671
```

### Safety net

Before any rewrite, `mmi` creates a backup ref at
`refs/mmi/backup/<branch>-<timestamp>` pointing at your previous tip. If
something looks wrong, restore with:

```bash
git update-ref refs/heads/<branch> refs/mmi/backup/<branch>-<timestamp>
```

To skip backup creation (not recommended): pass `--no-backup`.

After a rewrite, you'll need to force-push the branch — and only do that
if you understand the implications for collaborators.

## How it works

- Single Rust binary; no `libgit2` or other native deps. Shells out to `git`.
- Pattern matching is a single `regex::RegexSet` over individual lines —
  fast and conservative. Only known AI signatures match.
- The `commit-msg` hook is one line: `exec mmi run "$1"`. The hook entry
  is idempotent and always exits 0, so it never blocks a commit.
- `rewrite-history` walks `<from>..HEAD` in topological order, rebuilds
  each commit via `git commit-tree`, preserves author/committer/dates, and
  uses a compare-and-swap on the branch ref so concurrent moves don't
  clobber.

## `mmi vw`

A loving homage to [`volkswagen`](https://github.com/auchenberg/volkswagen),
which detected CI environments and made tests pass. `mmi vw` does the same:
behaves like `check` everywhere except CI, where it always exits 0.

```bash
echo "feat: x

Co-authored-by: Claude <noreply@anthropic.com>" | mmi vw -
# locally: exit 1, "mmi: AI trails detected."
# on CI:   exit 0, "mmi: ✓ all clear (nothing to see here)"
```

CI providers detected via env: `CI`, `CONTINUOUS_INTEGRATION`,
`GITHUB_ACTIONS`, `GITLAB_CI`, `CIRCLECI`, `TRAVIS`, `JENKINS_URL`,
`BUILDKITE`, `DRONE`, `TEAMCITY_VERSION`, `TF_BUILD`, `APPVEYOR`,
`SEMAPHORE`, `CODEBUILD_BUILD_ID`, and friends.

For real CI, use `mmi check`.

## Project name

**Me, Myself and ~~A~~I.** The struck-through A is the AI trail getting
removed. The result — Me, Myself and I — is your commit history,
restored to its rightful authors.

## License

[MIT](LICENSE).
