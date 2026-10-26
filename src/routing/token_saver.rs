use crate::headroom::HeadroomPipeline;
use crate::translator::ChatMessage;
use serde_json::Value;

pub const CAVEMAN_PROMPT_LITE: &str = "Respond tersely. Strip filler, pleasantries and meta-commentary. Keep code, errors and commands verbatim.";
pub const CAVEMAN_PROMPT_FULL: &str = "Caveman mode: High compression. Minimal words. Retain technical facts, exact code, paths, and commands.";
pub const CAVEMAN_PROMPT_ULTRA: &str = "Respond ultra-terse. Maximum compression. Strip conjunctions. One word when enough. Pattern: [thing] [action] [reason]. [next step]. Keep code exact.";

pub const PONYTAIL_PROMPT_LITE: &str = "You are a lazy senior developer. The best code is the code never written. Build what is asked, but name the lazier alternative in one line.";
pub const PONYTAIL_PROMPT_FULL: &str = "You are a lazy senior developer. Minimal code, YAGNI, standard library and native first. Shortest diff wins.";
pub const PONYTAIL_PROMPT_ULTRA: &str = "You are a lazy senior developer. YAGNI extremist. Deletion before addition. Ship the one-liner and challenge unneeded requirements.";

pub struct TokenSaverPipeline;

impl TokenSaverPipeline {
    pub fn process(
        messages: &mut Vec<ChatMessage>,
        headroom_pipeline: &HeadroomPipeline,
        headroom_enabled: bool,
        caveman_level: Option<&str>,
        ponytail_level: Option<&str>,
    ) {
        // 1. Inject Caveman / Ponytail prompt into system message if requested
        let mut injections = Vec::new();
        if let Some(level) = caveman_level {
            match level {
                "lite" => injections.push(CAVEMAN_PROMPT_LITE),
                "full" => injections.push(CAVEMAN_PROMPT_FULL),
                "ultra" => injections.push(CAVEMAN_PROMPT_ULTRA),
                _ => {}
            }
        }
        if let Some(level) = ponytail_level {
            match level {
                "lite" => injections.push(PONYTAIL_PROMPT_LITE),
                "full" => injections.push(PONYTAIL_PROMPT_FULL),
                "ultra" => injections.push(PONYTAIL_PROMPT_ULTRA),
                _ => {}
            }
        }

        if !injections.is_empty() {
            let combined = injections.join("\n\n");
            // Prepend to first system message or insert new system message at head
            if let Some(sys_msg) = messages.iter_mut().find(|m| m.role == "system") {
                if let Some(ref mut c) = sys_msg.content {
                    if let Some(s) = c.as_str() {
                        *c = Value::String(format!("{}\n\n{}", combined, s));
                    }
                }
            } else {
                messages.insert(0, ChatMessage {
                    role: "system".to_string(),
                    content: Some(Value::String(combined)),
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                });
            }
        }

        // 2. Built-in Headroom compression on tool outputs and content
        if headroom_enabled {
            for msg in messages.iter_mut() {
                if msg.role == "tool" || msg.role == "user" {
                    if let Some(c) = &msg.content {
                        if let Some(s) = c.as_str() {
                            let (compressed, transforms) = headroom_pipeline.compress_text(s);
                            if !transforms.is_empty() {
                                msg.content = Some(Value::String(compressed));
                            }
                        }
                    }
                }
            }
        }
    }
}
