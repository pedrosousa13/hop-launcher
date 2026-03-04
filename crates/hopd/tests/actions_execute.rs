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
    assert_eq!(parsed.result["result_id"], "app:terminal");
    assert_eq!(parsed.result["action"], "enter");
}
