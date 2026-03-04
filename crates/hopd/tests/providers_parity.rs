use std::collections::HashSet;

use hopd::{HopdServer, IpcResponse};

#[tokio::test]
async fn search_query_can_return_all_primary_kinds() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(r#"{"id":"k1","method":"search.query","params":{"query":"a","limit":20}}"#)
        .await
        .expect("response expected");
    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");

    let kinds: HashSet<&str> = results
        .iter()
        .filter_map(|row| row["kind"].as_str())
        .collect();

    assert!(kinds.contains("app"), "missing app kind");
    assert!(kinds.contains("window"), "missing window kind");
    assert!(kinds.contains("file"), "missing file kind");
    assert!(kinds.contains("recent"), "missing recent kind");
    assert!(kinds.contains("setting"), "missing setting kind");
    assert!(kinds.contains("utility"), "missing utility kind");
}
