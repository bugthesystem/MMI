use once_cell::sync::Lazy;
use regex::RegexSet;

pub static PATTERNS: Lazy<Patterns> = Lazy::new(Patterns::default);

pub struct Patterns {
    set: RegexSet,
}

impl Default for Patterns {
    fn default() -> Self {
        let raw = &[
            // Claude Code "Generated with" footer
            r"^\s*🤖\s*Generated with .*$",
            r"^\s*Generated with\s*\[?Claude Code\]?.*$",
            // Claude session URLs
            r"^\s*https?://claude\.ai/code/\S+\s*$",
            // AI co-authors — match by tool name OR by known bot email domain
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
        ];
        Self {
            set: RegexSet::new(raw).expect("valid regex set"),
        }
    }
}

impl Patterns {
    pub fn is_ai_line(&self, line: &str) -> bool {
        self.set.is_match(line)
    }
}
