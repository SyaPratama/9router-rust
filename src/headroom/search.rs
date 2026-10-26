use std::collections::BTreeMap;

pub struct SearchCompressor;

impl SearchCompressor {
    pub fn compress(input: &str) -> Option<String> {
        let lines: Vec<&str> = input.lines().collect();
        if lines.len() < 2 {
            return None;
        }

        // Group by file path
        let mut grouped: BTreeMap<String, Vec<(usize, String)>> = BTreeMap::new();
        let mut unparsed: Vec<String> = Vec::new();

        for line in lines {
            let parts: Vec<&str> = line.splitn(3, ':').collect();
            if parts.len() == 3 {
                if let Ok(line_num) = parts[1].trim().parse::<usize>() {
                    let file = parts[0].trim().to_string();
                    let content = parts[2].trim().to_string();
                    let short_content = if content.len() > 160 {
                        format!("{}...", &content[..157])
                    } else {
                        content
                    };
                    grouped.entry(file).or_default().push((line_num, short_content));
                    continue;
                }
            }
            unparsed.push(line.to_string());
        }

        if grouped.is_empty() {
            return None;
        }

        let mut output = Vec::new();
        for (file, entries) in grouped {
            output.push(format!("{}:", file));
            for (line_num, content) in entries {
                output.push(format!("  {}: {}", line_num, content));
            }
        }
        for u in unparsed {
            output.push(u);
        }

        let res = output.join("\n");
        if res.len() < input.len() {
            Some(res)
        } else {
            None
        }
    }
}
