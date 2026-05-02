use crate::patterns::PATTERNS;

/// Strip AI trail lines from a commit message and tidy whitespace.
///
/// - Lines matching known AI signatures are removed.
/// - Consecutive blank lines are collapsed to one.
/// - Trailing blank lines are stripped.
/// - A trailing newline is preserved if the input had one.
pub fn clean(input: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();

    let kept: Vec<&str> = lines
        .into_iter()
        .filter(|line| !PATTERNS.is_ai_line(line))
        .collect();

    let mut out: Vec<&str> = Vec::with_capacity(kept.len());
    let mut prev_blank = false;
    for line in kept {
        let is_blank = line.trim().is_empty();
        if is_blank && prev_blank {
            continue;
        }
        prev_blank = is_blank;
        out.push(line);
    }

    while out.last().is_some_and(|l| l.trim().is_empty()) {
        out.pop();
    }

    let mut result = out.join("\n");
    if !result.is_empty() && (input.ends_with('\n') || input.ends_with("\r\n")) {
        result.push('\n');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_claude_block() {
        let input = "feat: add thing\n\nDoes the thing.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-authored-by: Claude <noreply@anthropic.com>\n";
        let expected = "feat: add thing\n\nDoes the thing.\n";
        assert_eq!(clean(input), expected);
    }

    #[test]
    fn strips_session_url() {
        let input = "fix: bug\n\nhttps://claude.ai/code/session_018wB7x3KUJLH29rAFb5T33q\n";
        let expected = "fix: bug\n";
        assert_eq!(clean(input), expected);
    }

    #[test]
    fn idempotent_on_clean_message() {
        let input = "feat: add user auth\n\nImplements OAuth flow.\n";
        assert_eq!(clean(input), input);
    }

    #[test]
    fn keeps_user_co_authors() {
        let input = "feat: thing\n\nCo-authored-by: Alice <alice@example.com>\n";
        assert_eq!(clean(input), input);
    }

    #[test]
    fn strips_copilot_coauthor() {
        let input = "fix: oops\n\nCo-authored-by: github-copilot[bot] <copilot@github.com>\n";
        let expected = "fix: oops\n";
        assert_eq!(clean(input), expected);
    }

    #[test]
    fn strips_cursor_coauthor() {
        let input = "fix: thing\n\nCo-authored-by: Cursor Agent <agent@cursor.so>\n";
        let expected = "fix: thing\n";
        assert_eq!(clean(input), expected);
    }

    #[test]
    fn collapses_blank_lines_left_behind() {
        let input = "feat: x\n\nBody.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nMore body.\n";
        let expected = "feat: x\n\nBody.\n\nMore body.\n";
        assert_eq!(clean(input), expected);
    }

    #[test]
    fn empty_input_stays_empty() {
        assert_eq!(clean(""), "");
    }

    #[test]
    fn preserves_user_named_after_ai_in_subject() {
        // Subject line should be untouched, only known signature lines are stripped.
        let input = "fix: rename Claude variable\n";
        assert_eq!(clean(input), input);
    }

    #[test]
    fn idempotent_double_apply() {
        let input = "feat: x\n\nBody.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n";
        let once = clean(input);
        let twice = clean(&once);
        assert_eq!(once, twice);
    }
}
