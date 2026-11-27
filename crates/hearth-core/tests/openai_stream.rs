//! OpenAI-compatible adapter against a mock llama.cpp-style SSE stream.

use hearth_core::provider::OpenAiProvider;
use hearth_core::*;
use std::sync::Mutex;
use wiremock::matchers::{body_partial_json, method, path};
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
        // Nemotron's tuned settings reach llama.cpp.
        .and(body_partial_json(serde_json::json!({"model": "nemotron-3.5-bolt-74b", "chat_template_kwargs": {"enable_thinking": true}})))
        .respond_with(ResponseTemplate::new(200).insert_header("content-type", "text/event-stream").set_body_string(sse))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data":[{"id":"qwen3.6-35b-a3b"},{"id":"nemotron-3.5-bolt-74b"},{"id":"gemma-4-12b"}]})))
        .mount(&server)
        .await;

    let p = OpenAiProvider::new("box", &format!("{}/v1", server.uri()), None);
    assert_eq!(p.list_models().await.unwrap(), vec!["gemma-4-12b", "nemotron-3.5-bolt-74b", "qwen3.6-35b-a3b"]);

    let seen = Mutex::new(String::new());
    let sink = |t: &str| seen.lock().unwrap().push_str(t);
    let model = ModelRef::new("hearth-box", "nemotron-3.5-bolt-74b");
    let req = ChatRequest {
        model: model.model.clone(),
        messages: vec![Message::user("hi")],
        tools: vec![],
        max_tokens: 100,
        settings: hearth_core::models::settings_for(&model),
    };
    let c = p.complete(&req, &sink).await.unwrap();
    assert_eq!(c.content, "Let me check.");
    assert_eq!(*seen.lock().unwrap(), "Let me check.");
    assert_eq!(c.tool_calls.len(), 1);
    assert_eq!(c.tool_calls[0].arguments, r#"{"path":"a.txt"}"#);
    assert_eq!(c.usage.output_tokens, 7);
    assert_eq!(c.stop_reason, "tool_calls");
}

#[tokio::test]
async fn sends_gemma_sampling_and_nothing_extra_for_cloud() {
    let server = MockServer::start().await;
    let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";
    let bodies = std::sync::Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
    let seen = bodies.clone();
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(move |r: &wiremock::Request| {
            seen.lock().unwrap().push(serde_json::from_slice(&r.body).unwrap());
            ResponseTemplate::new(200).insert_header("content-type", "text/event-stream").set_body_string(sse)
        })
        .mount(&server)
        .await;
    let p = OpenAiProvider::new("box", &format!("{}/v1", server.uri()), None);
    let sink = |_: &str| {};
    for m in [ModelRef::new("hearth-box", "gemma-4-12b"), ModelRef::new("openai", "gpt-5")] {
        let req = ChatRequest { model: m.model.clone(), messages: vec![Message::user("hi")], tools: vec![], max_tokens: 10, settings: hearth_core::models::settings_for(&m) };
        p.complete(&req, &sink).await.unwrap();
    }
    let b = bodies.lock().unwrap();
    assert_eq!(b[0]["chat_template_kwargs"]["enable_thinking"], true);
    assert!((b[0]["temperature"].as_f64().unwrap() - 1.3).abs() < 1e-6);
    assert_eq!(b[0]["top_p"], 1.0);
    assert!((b[0]["repeat_penalty"].as_f64().unwrap() - 1.05).abs() < 1e-6);
    for k in ["chat_template_kwargs", "temperature", "top_p", "repeat_penalty"] {
        assert!(b[1].get(k).is_none(), "{k} sent to a cloud model");
    }
}
