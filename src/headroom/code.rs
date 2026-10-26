pub struct CodeCompressor;

impl CodeCompressor {
    pub fn compress(input: &str) -> Option<String> {
        let lines: Vec<&str> = input.lines().collect();
        if lines.len() < 10 {
            return None;
        }

        let mut output = Vec::new();
        let mut in_body = false;
        let mut brace_depth = 0;

        for line in lines {
            let trimmed = line.trim();

            if trimmed.starts_with("import ")
                || trimmed.starts_with("use ")
                || trimmed.starts_with("export ")
                || trimmed.starts_with("pub struct ")
                || trimmed.starts_with("pub enum ")
                || trimmed.starts_with("pub trait ")
                || trimmed.starts_with("struct ")
                || trimmed.starts_with("enum ")
                || trimmed.starts_with("trait ")
                || trimmed.starts_with("type ")
            {
                output.push(line.to_string());
                continue;
            }

            if trimmed.contains("fn ") || trimmed.contains("def ") || trimmed.contains("function ") {
                output.push(line.to_string());
                if trimmed.ends_with('{') || trimmed.ends_with(':') {
                    in_body = true;
                    brace_depth = 1;
                    output.push("    /* body omitted */".to_string());
                }
                continue;
            }

            if in_body {
                brace_depth += line.matches('{').count() as i32;
                brace_depth -= line.matches('}').count() as i32;
                if brace_depth <= 0 {
                    in_body = false;
                    output.push(line.to_string());
                }
                continue;
            }

            output.push(line.to_string());
        }

        let res = output.join("\n");
        if res.len() < input.len() {
            Some(res)
        } else {
            None
        }
    }
}
