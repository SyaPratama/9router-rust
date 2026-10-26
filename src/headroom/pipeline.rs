use super::ccr::CcrStore;
use super::code::CodeCompressor;
use super::diff::DiffCompressor;
use super::log::LogCompressor;
use super::rag::{ConfigCompressor, RagCompressor};
use super::router::{ContentKind, ContentRouter};
use super::smart_crusher::SmartCrusher;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionStats {
    #[serde(rename = "tokensBefore")]
    pub tokens_before: usize,
    #[serde(rename = "tokensAfter")]
    pub tokens_after: usize,
    #[serde(rename = "tokensSaved")]
    pub tokens_saved: usize,
    #[serde(rename = "compressionRatio")]
    pub compression_ratio: f64,
    #[serde(rename = "transformsApplied")]
    pub transforms_applied: Vec<String>,
}

pub struct HeadroomPipeline {
    pub ccr_store: CcrStore,
    pub min_payload_bytes: usize,
}

impl Default for HeadroomPipeline {
    fn default() -> Self {
        Self {
            ccr_store: CcrStore::new(),
            min_payload_bytes: 500,
        }
    }
}

impl HeadroomPipeline {
    pub fn new() -> Self {
        Self::default()
    }

    /// Approximate tokens: ~4 characters per token
    pub fn estimate_tokens(text: &str) -> usize {
        (text.len() + 3) / 4
    }

    pub fn compress_text(&self, text: &str) -> (String, Vec<String>) {
        if text.len() < self.min_payload_bytes {
            return (text.to_string(), Vec::new());
        }

        let kind = ContentRouter::route(text);
        let mut transforms = Vec::new();

        let compressed_opt = match kind {
            ContentKind::GitStatus => {
                // Invariant: GitStatus is lossless passthrough
                None
            }
            ContentKind::JsonArray => {
                transforms.push("smart_crusher".to_string());
                SmartCrusher::compress(text)
            }
            ContentKind::GitDiff => {
                transforms.push("diff_compressor".to_string());
                DiffCompressor::compress(text)
            }
            ContentKind::Log => {
                transforms.push("log_compressor".to_string());
                LogCompressor::compress(text)
            }
            ContentKind::Search => {
                transforms.push("search_compressor".to_string());
                super::search::SearchCompressor::compress(text)
            }
            ContentKind::Code => {
                transforms.push("code_compressor".to_string());
                CodeCompressor::compress(text)
            }
            ContentKind::Config => {
                transforms.push("config_compressor".to_string());
                ConfigCompressor::compress(text)
            }
            ContentKind::Rag => {
                transforms.push("rag_compressor".to_string());
                RagCompressor::compress(text)
            }
            ContentKind::Generic => None,
        };

        if let Some(comp) = compressed_opt {
            // Fail-open contract: only keep if saved at least 5%
            let orig_len = text.len();
            let new_len = comp.len();
            if new_len < orig_len && (orig_len - new_len) as f64 / orig_len as f64 >= 0.05 {
                // Cache original in CCR store
                self.ccr_store.store(text);
                (comp, transforms)
            } else {
                (text.to_string(), Vec::new())
            }
        } else {
            (text.to_string(), Vec::new())
        }
    }

    /// Compress OpenAI-compatible messages payload
    pub fn compress_messages(&self, mut messages: Vec<serde_json::Value>) -> (Vec<serde_json::Value>, CompressionStats) {
        let before_json = serde_json::to_string(&messages).unwrap_or_default();
        let tokens_before = Self::estimate_tokens(&before_json);
        let mut all_transforms = Vec::new();

        for msg in &mut messages {
            let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
            let is_error = msg.get("is_error").and_then(|e| e.as_bool()).unwrap_or(false)
                || msg.get("status").and_then(|s| s.as_str()).map_or(false, |s| s == "error");

            if is_error {
                continue;
            }

            // Target tool result or user content
            if role == "tool" || role == "user" || role == "system" {
                if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                    let (compressed, transforms) = self.compress_text(content);
                    if !transforms.is_empty() {
                        msg["content"] = serde_json::Value::String(compressed);
                        all_transforms.extend(transforms);
                    }
                }
            }
        }

        let after_json = serde_json::to_string(&messages).unwrap_or_default();
        let tokens_after = Self::estimate_tokens(&after_json);
        let tokens_saved = if tokens_before > tokens_after {
            tokens_before - tokens_after
        } else {
            0
        };
        let compression_ratio = if tokens_before > 0 {
            tokens_after as f64 / tokens_before as f64
        } else {
            1.0
        };

        (
            messages,
            CompressionStats {
                tokens_before,
                tokens_after,
                tokens_saved,
                compression_ratio,
                transforms_applied: all_transforms,
            },
        )
    }
}
