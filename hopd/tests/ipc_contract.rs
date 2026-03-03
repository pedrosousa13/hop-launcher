use hopd::{
    IpcRequest,
    IpcResponse,
    HopdServer,
};

#[tokio::test]
async fn handles_health_ping_request() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(r#"{"id":"1","method":"health.ping"}"#)
        .await
        .expect("response expected");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    assert_eq!(parsed.id, "1");
    assert_eq!(parsed.result["ok"], true);
}

#[test]
fn parses_search_query_request() {
    let input = r#"{"id":"42","method":"search.query","params":{"query":"emoji smile","limit":8}}"#;
    let req: IpcRequest = serde_json::from_str(input).expect("valid request");

    assert_eq!(req.id, "42");
    assert_eq!(req.method, "search.query");
    assert_eq!(req.params["query"], "emoji smile");
    assert_eq!(req.params["limit"], 8);
}

#[tokio::test]
async fn handles_search_query_with_empty_result_set() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(
            r#"{"id":"2","method":"search.query","params":{"query":"zurich weather","limit":5}}"#,
        )
        .await
        .expect("response expected");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    assert_eq!(parsed.id, "2");
    assert_eq!(parsed.result["results"], serde_json::json!([]));
    assert!(parsed.result["telemetry"]["elapsed_ms"].is_number());
    assert!(parsed.error.is_none());
}

#[tokio::test]
async fn handles_actions_execute_acknowledgement() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(
            r#"{"id":"3","method":"actions.execute","params":{"result_id":"abc","action":"enter"}}"#,
        )
        .await
        .expect("response expected");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    assert_eq!(parsed.id, "3");
    assert_eq!(parsed.result["ok"], true);
    assert_eq!(parsed.result["executed"], true);
    assert!(parsed.error.is_none());
}

#[tokio::test]
async fn supports_config_set_and_get_roundtrip() {
    let server = HopdServer::new();
    server
        .handle_json_line(
            r#"{"id":"4","method":"config.set","params":{"key":"features.weather","value":false}}"#,
        )
        .await
        .expect("set response");

    let response = server
        .handle_json_line(
            r#"{"id":"5","method":"config.get","params":{"key":"features.weather"}}"#,
        )
        .await
        .expect("get response");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    assert_eq!(parsed.id, "5");
    assert_eq!(parsed.result["value"], false);
    assert!(parsed.error.is_none());
}

#[tokio::test]
async fn returns_error_for_unknown_method() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(r#"{"id":"x","method":"unknown.method"}"#)
        .await
        .expect("response expected");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    assert_eq!(parsed.id, "x");
    assert!(parsed.error.is_some());
    assert!(parsed.result.is_null());
}
