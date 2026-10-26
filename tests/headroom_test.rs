use router9::headroom::{
    CodeCompressor, ContentKind, ContentRouter, DiffCompressor, HeadroomPipeline,
    LogCompressor, SearchCompressor, SmartCrusher,
};

#[test]
fn test_git_status_lossless_passthrough() {
    let git_status_output = "On branch master\nYour branch is up to date with 'origin/master'.\n\nChanges to be committed:\n\tmodified:   src/main.rs\n\nUntracked files:\n\tnew_file.txt\n";
    let kind = ContentRouter::route(git_status_output);
    assert_eq!(kind, ContentKind::GitStatus);

    let pipeline = HeadroomPipeline::new();
    let (compressed, transforms) = pipeline.compress_text(git_status_output);
    // Git status MUST NOT be modified
    assert_eq!(compressed, git_status_output);
    assert!(transforms.is_empty());
}

#[test]
fn test_smart_crusher_json_array() {
    let mut items = Vec::new();
    for i in 0..50 {
        items.push(serde_json::json!({
            "id": i,
            "uuid": format!("550e8400-e29b-41d4-a716-44665544{:04x}", i),
            "status": "active",
            "description": format!("This is item number {} in the dataset representing a mock user record", i),
            "score": i as f64 * 1.5,
            "isActive": true
        }));
    }
    let input = serde_json::to_string(&items).unwrap();
    let compressed = SmartCrusher::compress(&input).expect("SmartCrusher should compress uniform JSON array");

    assert!(compressed.contains("__schema"));
    assert!(compressed.contains("__rows"));
    assert!(compressed.len() < input.len());
    let savings_pct = (input.len() - compressed.len()) as f64 / input.len() as f64;
    assert!(savings_pct > 0.25, "Expected at least 25% savings, got {:.2}%", savings_pct * 100.0);
}

#[test]
fn test_diff_compressor() {
    let diff_input = "diff --git a/src/lib.rs b/src/lib.rs\nindex 1234..5678 100644\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -10,8 +10,8 @@\n context line 1\n context line 2\n context line 3\n-old line\n+new line\n context line 4\n context line 5\n context line 6\n";
    let compressed = DiffCompressor::compress(diff_input).expect("DiffCompressor should compress context lines");

    assert!(compressed.contains("diff --git a/src/lib.rs b/src/lib.rs"));
    assert!(compressed.contains("-old line"));
    assert!(compressed.contains("+new line"));
    assert!(compressed.contains("..."));
    assert!(compressed.len() < diff_input.len());
}

#[test]
fn test_log_compressor_deduplication() {
    let log_input = "2026-03-29T10:00:00Z [INFO] Processing request id=123\n\
2026-03-29T10:00:01Z [INFO] Processing request id=123\n\
2026-03-29T10:00:02Z [INFO] Processing request id=123\n\
2026-03-29T10:00:03Z [INFO] Processing request id=123\n\
2026-03-29T10:00:04Z [ERROR] Critical failure in module X: panic occurred at line 42\n\
2026-03-29T10:00:05Z [INFO] Resuming loop\n";

    let compressed = LogCompressor::compress(log_input).expect("LogCompressor should collapse repeated lines");
    assert!(compressed.contains("[... repeated 4 times: [INFO] Processing request id=123]"));
    // Error panic line MUST be preserved 100%
    assert!(compressed.contains("[ERROR] Critical failure in module X: panic occurred at line 42"));
    assert!(compressed.len() < log_input.len());
}

#[test]
fn test_search_compressor_grouping() {
    let search_input = "src/router.rs:12: pub fn route() {\n\
src/router.rs:45: match kind {\n\
src/router.rs:88: }\n\
src/main.rs:5: use router9::*;\n";

    let compressed = SearchCompressor::compress(search_input).expect("SearchCompressor should group matches");
    assert!(compressed.contains("src/router.rs:"));
    assert!(compressed.contains("  12: pub fn route() {"));
    assert!(compressed.contains("  45: match kind {"));
}

#[test]
fn test_code_compressor() {
    let code_input = "use std::collections::HashMap;\n\
\n\
pub struct MyStruct {\n\
    pub field: String,\n\
}\n\
\n\
impl MyStruct {\n\
    pub fn execute(&self) -> bool {\n\
        let mut x = 1;\n\
        for _ in 0..10 {\n\
            x += 1;\n\
        }\n\
        x > 5\n\
    }\n\
}\n";

    let compressed = CodeCompressor::compress(code_input).expect("CodeCompressor should compact function bodies");
    assert!(compressed.contains("pub struct MyStruct"));
    assert!(compressed.contains("pub fn execute(&self) -> bool"));
    assert!(compressed.contains("/* body omitted */"));
    assert!(compressed.len() < code_input.len());
}

#[test]
fn test_headroom_pipeline_fail_open() {
    let pipeline = HeadroomPipeline::new();
    let small_input = "Short string < 500 bytes";
    let (compressed, transforms) = pipeline.compress_text(small_input);
    assert_eq!(compressed, small_input);
    assert!(transforms.is_empty());
}
