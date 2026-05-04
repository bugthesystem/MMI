use std::sync::LazyLock;

use regex::RegexSet;

/// Compiled set of line patterns considered AI trails. Match a single line
/// against this set to decide whether it should be stripped.
pub static PATTERNS: LazyLock<RegexSet> = LazyLock::new(|| {
    RegexSet::new([
        // Claude Code "Generated with" footer
        r"^\s*🤖\s*Generated with .*$",
        r"^\s*Generated with\s*\[?Claude Code\]?.*$",
        // Claude session URLs
        r"^\s*https?://claude\.ai/code/\S+\s*$",
        // AI co-authors — by tool name or known bot email
        r"(?i)^\s*Co-authored-by:\s+Claude(\s|<).*$",
        r"(?i)^\s*Co-authored-by:.*<[^>]*@anthropic\.com>\s*$",
        r"(?i)^\s*Co-authored-by:\s+.*\bcursor\b.*$",
        r"(?i)^\s*Co-authored-by:\s+.*\bcopilot\b.*$",
        r"(?i)^\s*Co-authored-by:.*<copilot@github\.com>\s*$",
        r"(?i)^\s*Co-authored-by:\s+.*\bcodeium\b.*$",
        r"(?i)^\s*Co-authored-by:\s+.*\bchatgpt\b.*$",
        r"(?i)^\s*Co-authored-by:\s+.*\baider\b.*$",
        r"(?i)^\s*Co-authored-by:\s+.*\bwindsurf\b.*$",
        r"(?i)^\s*Co-authored-by:\s+.*\bdevin\b.*$",
        // Generic AI trailers
        r"(?i)^\s*Generated-by:\s+.*$",
        r"(?i)^\s*AI-Assisted-by:\s+.*$",
        r"(?i)^\s*Assisted-by:\s+.*\b(claude|gpt|copilot|cursor|ai)\b.*$",
    ])
    .expect("AI-trail patterns compile")
});
