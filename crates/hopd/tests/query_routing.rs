use hopd::{HopdServer, IpcResponse};

#[tokio::test]
async fn search_query_mode_filters_provider_set() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(
            r#"{"id":"r1","method":"search.query","params":{"query":"a","mode":"apps","limit":5}}"#,
        )
        .await
        .expect("response expected");
    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    assert!(!results.is_empty(), "expected routed app results");
    assert!(results.iter().all(|row| row["kind"] == "app"));
}
