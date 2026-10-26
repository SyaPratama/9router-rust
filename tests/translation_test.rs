use router9::translator::{
    ChatMessage, ChatRequest, ClaudeTranslator, GeminiTranslator, ResponsesTranslator,
};
use serde_json::json;

#[test]
fn test_openai_to_claude_translation() {
    let req = ChatRequest {
        model: "claude-3-5-sonnet".to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: Some(json!("You are an AI assistant.")),
                name: None,
                tool_call_id: None,
                tool_calls: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: Some(json!("Check weather")),
                name: None,
                tool_call_id: None,
                tool_calls: None,
            },
            ChatMessage {
                role: "assistant".to_string(),
                content: Some(json!("Checking")),
                name: None,
                tool_call_id: None,
                tool_calls: Some(vec![json!({
                    "id": "call_123",
                    "type": "function",
                    "function": {
                        "name": "get_weather",
                        "arguments": "{\"city\":\"Tokyo\"}"
                    }
                })]),
            },
            ChatMessage {
                role: "tool".to_string(),
                content: Some(json!("{\"temp\": 22}")),
                name: None,
                tool_call_id: Some("call_123".to_string()),
                tool_calls: None,
            },
        ],
        stream: false,
        temperature: Some(0.7),
        max_tokens: Some(2048),
        tools: None,
    };

    let claude_payload = ClaudeTranslator::to_anthropic_payload(&req);

    // System prompt extracted
    assert_eq!(claude_payload["system"], "You are an AI assistant.");
    assert_eq!(claude_payload["max_tokens"], 2048);

    let messages = claude_payload["messages"].as_array().expect("messages array");
    // tool result mapped to user message with tool_result block
    let last_msg = messages.last().expect("last message");
    assert_eq!(last_msg["role"], "user");
    let content_blocks = last_msg["content"].as_array().expect("content blocks");
    assert_eq!(content_blocks[0]["type"], "tool_result");
    assert_eq!(content_blocks[0]["tool_use_id"], "call_123");
}

#[test]
fn test_claude_to_openai_response() {
    let claude_resp = json!({
        "id": "msg_01X9Z",
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "text",
            "text": "Hello world from Claude!"
        }],
        "model": "claude-3-5-sonnet",
        "stop_reason": "end_turn"
    });

    let openai_resp = ClaudeTranslator::anthropic_to_openai_response(&claude_resp);
    assert_eq!(openai_resp["object"], "chat.completion");
    assert_eq!(
        openai_resp["choices"][0]["message"]["content"],
        "Hello world from Claude!"
    );
}

#[test]
fn test_gemini_translation() {
    let req = ChatRequest {
        model: "gemini-1.5-pro".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: Some(json!("Hello Gemini")),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }],
        stream: false,
        temperature: None,
        max_tokens: None,
        tools: None,
    };

    let gemini_payload = GeminiTranslator::to_gemini_payload(&req);
    let contents = gemini_payload["contents"].as_array().expect("contents array");
    assert_eq!(contents[0]["role"], "user");
    assert_eq!(contents[0]["parts"][0]["text"], "Hello Gemini");
}

#[test]
fn test_responses_api_roundtrip() {
    let responses_req = json!({
        "model": "gpt-4o",
        "instructions": "Be concise.",
        "input": "Summarize this article",
        "stream": false
    });

    let chat_req = ResponsesTranslator::responses_to_chat_request(&responses_req);
    assert_eq!(chat_req.messages.len(), 2);
    assert_eq!(chat_req.messages[0].role, "system");
    assert_eq!(chat_req.messages[1].role, "user");

    let chat_completion_resp = json!({
        "id": "chatcmpl_999",
        "choices": [{
            "message": {
                "role": "assistant",
                "content": "Article summary."
            }
        }]
    });

    let responses_resp = ResponsesTranslator::chat_completion_to_responses(&chat_completion_resp);
    assert_eq!(responses_resp["object"], "response");
    assert_eq!(responses_resp["status"], "completed");
    assert_eq!(
        responses_resp["output"][0]["content"][0]["text"],
        "Article summary."
    );
}
