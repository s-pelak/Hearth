//! OpenAI-compatible adapter against a mock llama.cpp-style SSE stream.

use hearth_core::provider::OpenAiProvider;
use hearth_core::*;
use std::sync::Mutex;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn parses_streamed_text_and_tool_calls() {
    let server = MockServer::start().await;
    let sse = [
        r#"{"choices":[{"delta":{"content":"Let me "}}]}"#,
        r#"{"choices":[{"delta":{"content":"check."}}]}"#,
        r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","function":{"name":"read_file","arguments":"{\"pa"}}]}}]}"#,
        r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"th\":\"a.txt\"}"}}]},"finish_reason":"tool_calls"}]}"#,
        r#"{"choices":[],"usage":{"prompt_tokens":12,"completion_tokens":7}}"#,
    ]
    .iter()
    .map(|d| format!("data: {d}\n\n"))
    .collect::<String>()
        + "data: [DONE]\n\n";
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-type", "text/event-stream").set_body_string(sse))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data":[{"id":"qwen3.5-9b"},{"id":"gemma-4-12b"}]})))
        .mount(&server)
        .await;

    let p = OpenAiProvider::new("box", &format!("{}/v1", server.uri()), None);
    assert_eq!(p.list_models().await.unwrap(), vec!["gemma-4-12b", "qwen3.5-9b"]);

    let seen = Mutex::new(String::new());
    let sink = |t: &str| seen.lock().unwrap().push_str(t);
    let req = ChatRequest { model: "qwen3.5-9b".into(), messages: vec![Message::user("hi")], tools: vec![], max_tokens: 100 };
    let c = p.complete(&req, &sink).await.unwrap();
    assert_eq!(c.content, "Let me check.");
    assert_eq!(*seen.lock().unwrap(), "Let me check.");
    assert_eq!(c.tool_calls.len(), 1);
    assert_eq!(c.tool_calls[0].arguments, r#"{"path":"a.txt"}"#);
    assert_eq!(c.usage.output_tokens, 7);
    assert_eq!(c.stop_reason, "tool_calls");
}
