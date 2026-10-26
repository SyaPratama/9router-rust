pub struct DiffCompressor;

impl DiffCompressor {
    pub fn compress(input: &str) -> Option<String> {
        let lines: Vec<&str> = input.lines().collect();
        if lines.is_empty() {
            return None;
        }

        let mut output = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            // Always preserve headers
            if line.starts_with("diff --git")
                || line.starts_with("index ")
                || line.starts_with("--- ")
                || line.starts_with("+++ ")
                || line.starts_with("@@")
            {
                output.push(line.to_string());
                i += 1;
                continue;
            }

            // If it's a context line (starts with space or empty), determine if it's adjacent to +/-
            if line.starts_with(' ') || line.is_empty() {
                let prev_is_change = i > 0 && (lines[i - 1].starts_with('+') || lines[i - 1].starts_with('-'));
                let next_is_change = i + 1 < lines.len() && (lines[i + 1].starts_with('+') || lines[i + 1].starts_with('-'));

                if prev_is_change || next_is_change {
                    output.push(line.to_string());
                } else if output.last().map_or(false, |l| !l.starts_with("...")) {
                    output.push("...".to_string());
                }
            } else {
                // Modified (+ or -) or other line
                output.push(line.to_string());
            }

            i += 1;
        }

        let res = output.join("\n");
        if res.len() < input.len() {
            Some(res)
        } else {
            None
        }
    }
}
