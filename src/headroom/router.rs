#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ContentKind {
    GitStatus,
    JsonArray,
    GitDiff,
    Log,
    Search,
    Code,
    Config,
    Rag,
    Generic,
}

pub struct ContentRouter;

impl ContentRouter {
    pub fn route(text: &str) -> ContentKind {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return ContentKind::Generic;
        }

        // Invariant: GitStatus is lossless passthrough
        if trimmed.contains("On branch ")
            || (trimmed.contains("Changes to be committed:") && trimmed.contains("Untracked files:"))
            || trimmed.contains("nothing to commit, working tree clean")
        {
            return ContentKind::GitStatus;
        }

        // Git diff detection
        if trimmed.starts_with("diff --git ")
            || (trimmed.contains("--- a/") && trimmed.contains("+++ b/"))
            || (trimmed.starts_with("@@ -") && trimmed.contains(" @@"))
        {
            return ContentKind::GitDiff;
        }

        // JSON array or JSON object with array
        if (trimmed.starts_with('[') && trimmed.ends_with(']'))
            || (trimmed.starts_with('{') && trimmed.ends_with('}') && trimmed.contains("\": ["))
        {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
                if v.is_array() || (v.is_object() && v.as_object().map_or(false, |o| o.values().any(|val| val.is_array()))) {
                    return ContentKind::JsonArray;
                }
            }
        }

        // Search / Grep output: file:line:content
        let search_lines = trimmed.lines().take(10).filter(|l| {
            let parts: Vec<&str> = l.splitn(3, ':').collect();
            parts.len() >= 3 && parts[1].trim().parse::<usize>().is_ok()
        }).count();
        if search_lines >= 2 {
            return ContentKind::Search;
        }

        // Log detection: timestamps or log tags
        let log_lines = trimmed.lines().take(10).filter(|l| {
            l.contains("[INFO]")
                || l.contains("[ERROR]")
                || l.contains("[WARN]")
                || l.contains("[DEBUG]")
                || (l.len() >= 19 && &l[4..5] == "-" && &l[7..8] == "-" && &l[10..11] == "T")
                || (l.len() >= 10 && &l[4..5] == "-" && &l[7..8] == "-")
        }).count();
        if log_lines >= 3 {
            return ContentKind::Log;
        }

        // Config: TOML/YAML markers
        if (trimmed.starts_with("---") && trimmed.lines().any(|l| l.contains(':')))
            || (trimmed.lines().any(|l| l.starts_with('[') && l.ends_with(']') && !l.contains(' ')))
        {
            return ContentKind::Config;
        }

        // Code detection
        let code_indicators = ["fn ", "def ", "pub fn ", "class ", "import ", "use ", "interface ", "function "];
        let code_matches = code_indicators.iter().filter(|&&kw| trimmed.contains(kw)).count();
        if code_matches >= 2 {
            return ContentKind::Code;
        }

        // RAG context detection
        if trimmed.contains("Context:") || trimmed.contains("Document [") || trimmed.contains("<document>") {
            return ContentKind::Rag;
        }

        ContentKind::Generic
    }
}
