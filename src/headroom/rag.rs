use std::collections::HashSet;

pub struct RagCompressor;

impl RagCompressor {
    pub fn compress(input: &str) -> Option<String> {
        let paragraphs: Vec<&str> = input.split("\n\n").collect();
        if paragraphs.len() < 2 {
            return None;
        }

        let mut seen = HashSet::new();
        let mut output = Vec::new();

        for para in paragraphs {
            let trimmed = para.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Fingerprint paragraph by taking first 60 chars
            let fp = if trimmed.len() > 60 {
                &trimmed[..60]
            } else {
                trimmed
            };

            if seen.contains(fp) {
                // Deduplicate repetitive retrieved chunks
                continue;
            }
            seen.insert(fp.to_string());
            output.push(trimmed.to_string());
        }

        let res = output.join("\n\n");
        if res.len() < input.len() {
            Some(res)
        } else {
            None
        }
    }
}

pub struct ConfigCompressor;

impl ConfigCompressor {
    pub fn compress(input: &str) -> Option<String> {
        // Strip comment lines starting with #
        let lines: Vec<&str> = input.lines().collect();
        let filtered: Vec<&str> = lines.into_iter().filter(|l| {
            let t = l.trim();
            !t.starts_with('#') && !t.is_empty()
        }).collect();

        let res = filtered.join("\n");
        if res.len() < input.len() {
            Some(res)
        } else {
            None
        }
    }
}
