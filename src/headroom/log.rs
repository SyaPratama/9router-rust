pub struct LogCompressor;

impl LogCompressor {
    pub fn compress(input: &str) -> Option<String> {
        let lines: Vec<&str> = input.lines().collect();
        if lines.len() < 3 {
            return None;
        }

        let mut output = Vec::new();
        let mut last_norm_line: Option<String> = None;
        let mut repeat_count = 0;

        for line in lines {
            let is_critical = line.contains("ERROR")
                || line.contains("panic")
                || line.contains("FATAL")
                || line.contains("Exception")
                || line.contains("Traceback")
                || line.contains("failed")
                || line.contains("stack backtrace");

            let norm = if is_critical {
                line.to_string()
            } else {
                Self::normalize_log_line(line)
            };

            if let Some(last) = &last_norm_line {
                if !is_critical && last == &norm {
                    repeat_count += 1;
                    continue;
                } else if repeat_count > 1 {
                    output.push(format!("[... repeated {} times: {}]", repeat_count, last));
                    repeat_count = 0;
                } else if repeat_count == 1 {
                    output.push(last.clone());
                    repeat_count = 0;
                }
            }

            if !is_critical {
                last_norm_line = Some(norm);
                repeat_count = 1;
            } else {
                if repeat_count > 1 {
                    if let Some(last) = &last_norm_line {
                        output.push(format!("[... repeated {} times: {}]", repeat_count, last));
                    }
                } else if repeat_count == 1 {
                    if let Some(last) = &last_norm_line {
                        output.push(last.clone());
                    }
                }
                last_norm_line = None;
                repeat_count = 0;
                output.push(line.to_string());
            }
        }

        if repeat_count > 1 {
            if let Some(last) = &last_norm_line {
                output.push(format!("[... repeated {} times: {}]", repeat_count, last));
            }
        } else if repeat_count == 1 {
            if let Some(last) = &last_norm_line {
                output.push(last.clone());
            }
        }

        let res = output.join("\n");
        if res.len() < input.len() {
            Some(res)
        } else {
            None
        }
    }

    fn normalize_log_line(line: &str) -> String {
        let trimmed = line.trim();
        if trimmed.len() >= 20 && &trimmed[4..5] == "-" && &trimmed[7..8] == "-" {
            if let Some(pos) = trimmed.find(|c: char| c == '[' || c == '{') {
                return trimmed[pos..].trim().to_string();
            } else if let Some(space_pos) = trimmed.find(' ') {
                return trimmed[space_pos..].trim().to_string();
            }
        }
        trimmed.to_string()
    }
}
