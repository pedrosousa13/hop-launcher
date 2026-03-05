use hopd::{HopdServer, IpcResponse};

#[tokio::test]
async fn actions_execute_returns_kind_specific_ack() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(
            r#"{"id":"e1","method":"actions.execute","params":{"result_id":"app:terminal","action":"enter"}}"#,
        )
        .await
        .expect("response expected");
    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    assert_eq!(parsed.result["ok"], true);
    assert_eq!(parsed.result["executed"], true);
    assert_eq!(parsed.result["action_resolved"], true);
    assert!(parsed.result["success"].is_boolean());
    assert!(parsed.result["launch_spawned"].is_boolean());
    assert!(parsed.result["resolved_command"].is_string());
    assert!(parsed.result["resolved_args"].is_array());
    assert_eq!(parsed.result["result_id"], "app:terminal");
    assert_eq!(parsed.result["action"], "enter");
}

#[tokio::test]
async fn actions_execute_reports_null_resolution_metadata_for_unresolved_result() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(
            r#"{"id":"e2","method":"actions.execute","params":{"result_id":"unknown-id","action":"enter"}}"#,
        )
        .await
        .expect("response expected");
    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    assert_eq!(parsed.result["ok"], false);
    assert_eq!(parsed.result["success"], false);
    assert_eq!(parsed.result["action_resolved"], false);
    assert_eq!(parsed.result["resolved_command"], serde_json::Value::Null);
    assert_eq!(parsed.result["resolved_args"], serde_json::Value::Null);
}
