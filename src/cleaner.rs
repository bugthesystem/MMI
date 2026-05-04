use crate::patterns::PATTERNS;

/// Strip AI trail lines from a commit message and tidy whitespace.
///
/// Single pass: known AI signature lines are dropped, consecutive blank
/// lines collapse to one, trailing blank lines are removed, and a single
/// trailing newline is preserved iff the input had one.
pub fn clean(input: &str) -> String {
    let mut kept: Vec<&str> = Vec::new();
    let mut prev_blank = false;
    for line in input.lines() {
        if PATTERNS.is_match(line) {
            continue;
        }
        let is_blank = line.trim().is_empty();
        if is_blank && prev_blank {
            continue;
        }
        prev_blank = is_blank;
        kept.push(line);
    }
    while kept.last().is_some_and(|l| l.trim().is_empty()) {
        kept.pop();
    }

    let mut out = kept.join("\n");
    if !out.is_empty() && input.ends_with('\n') {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_claude_block() {
        let input = "feat: add thing\n\nDoes the thing.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-authored-by: Claude <noreply@anthropic.com>\n";
        assert_eq!(clean(input), "feat: add thing\n\nDoes the thing.\n");
    }

    #[test]
    fn strips_session_url() {
        let input = "fix: bug\n\nhttps://claude.ai/code/session_018wB7x3KUJLH29rAFb5T33q\n";
        assert_eq!(clean(input), "fix: bug\n");
    }

    #[test]
    fn idempotent_on_clean_message() {
        let input = "feat: add user auth\n\nImplements OAuth flow.\n";
        assert_eq!(clean(input), input);
    }

    #[test]
    fn idempotent_double_apply() {
        let input = "feat: x\n\nBody.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n";
        let once = clean(input);
        assert_eq!(clean(&once), once);
    }

    #[test]
    fn collapses_blank_lines_left_behind() {
        let input = "feat: x\n\nBody.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nMore body.\n";
        assert_eq!(clean(input), "feat: x\n\nBody.\n\nMore body.\n");
    }

    #[test]
    fn empty_input_stays_empty() {
        assert_eq!(clean(""), "");
    }

    #[test]
    fn pure_trail_input_becomes_empty() {
        let input = "🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-authored-by: Claude <noreply@anthropic.com>\n";
        assert_eq!(clean(input), "");
    }

    #[test]
    fn preserves_trailing_newline_iff_input_had_one() {
        assert_eq!(clean("subject\n"), "subject\n");
        assert_eq!(clean("subject"), "subject");
    }

    #[test]
    fn handles_crlf() {
        // `lines()` strips both \n and \r\n, so output is normalized to LF.
        let input = "feat: x\r\n\r\nbody\r\n\r\nCo-authored-by: Claude <noreply@anthropic.com>\r\n";
        assert_eq!(clean(input), "feat: x\n\nbody\n");
    }

    /// Table-driven coverage for every documented pattern, plus
    /// false-positive guards.
    #[test]
    fn pattern_coverage() {
        struct Case<'a> {
            line: &'a str,
            stripped: bool,
            note: &'a str,
        }
        let cases = [
            // positives — should be stripped
            Case { line: "🤖 Generated with [Claude Code](https://claude.ai/code)", stripped: true, note: "claude robot footer" },
            Case { line: "Generated with Claude Code", stripped: true, note: "plain claude footer" },
            Case { line: "https://claude.ai/code/session_abc123", stripped: true, note: "session url" },
            Case { line: "Co-authored-by: Claude <noreply@anthropic.com>", stripped: true, note: "claude coauthor" },
            Case { line: "Co-authored-by: Cursor Agent <agent@cursor.so>", stripped: true, note: "cursor coauthor" },
            Case { line: "Co-authored-by: github-copilot[bot] <copilot@github.com>", stripped: true, note: "copilot via name" },
            Case { line: "Co-authored-by: Bot <copilot@github.com>", stripped: true, note: "copilot via email" },
            Case { line: "Co-authored-by: Codeium AI <bot@codeium.com>", stripped: true, note: "codeium" },
            Case { line: "Co-authored-by: ChatGPT <bot@openai.com>", stripped: true, note: "chatgpt" },
            Case { line: "Co-authored-by: Aider <aider@bot>", stripped: true, note: "aider" },
            Case { line: "Co-authored-by: Windsurf <ws@bot>", stripped: true, note: "windsurf" },
            Case { line: "Co-authored-by: Devin <devin@cognition.ai>", stripped: true, note: "devin" },
            Case { line: "Co-authored-by: Anonymous <bot@anthropic.com>", stripped: true, note: "anthropic email" },
            Case { line: "Generated-by: SomeTool", stripped: true, note: "generic Generated-by" },
            Case { line: "AI-Assisted-by: Tool", stripped: true, note: "AI-Assisted-by" },
            Case { line: "Assisted-by: claude-3", stripped: true, note: "Assisted-by + claude" },
            // negatives — should be preserved
            Case { line: "Co-authored-by: Alice <alice@example.com>", stripped: false, note: "human coauthor" },
            Case { line: "fix: rename Claude variable", stripped: false, note: "Claude in subject" },
            Case { line: "This commit was inspired by an AI but written by hand", stripped: false, note: "casual mention" },
            Case { line: "Reviewed-by: Bob", stripped: false, note: "different trailer" },
            Case { line: "https://github.com/example/repo", stripped: false, note: "non-claude url" },
            Case { line: "Assisted-by: Bob", stripped: false, note: "Assisted-by without AI tool" },
        ];

        for case in &cases {
            let got = clean(case.line);
            if case.stripped {
                assert!(
                    got.trim().is_empty(),
                    "expected `{}` to be stripped ({}), got: `{}`",
                    case.line,
                    case.note,
                    got
                );
            } else {
                assert_eq!(
                    got.trim_end(),
                    case.line.trim_end(),
                    "expected `{}` to be preserved ({})",
                    case.line,
                    case.note
                );
            }
        }
    }
}
