use hopd::{HopdServer, IpcResponse};

#[tokio::test]
async fn search_query_supports_all_primary_modes_without_scaffolds() {
    let server = HopdServer::new();
    for (mode, allowed_kinds) in [
        ("apps", vec!["app"]),
        ("windows", vec!["window"]),
        ("files", vec!["file"]),
        ("recents", vec!["recent"]),
        ("settings", vec!["setting"]),
        ("weather", vec!["weather"]),
        ("timezone", vec!["timezone"]),
        ("emoji", vec!["emoji"]),
        ("calculator", vec!["calculator"]),
        ("currency", vec!["currency"]),
    ] {
        let query = match mode {
            "weather" => "weather zurich",
            "timezone" => "time tokyo",
            "emoji" => "emoji smile",
            "calculator" => "2+2",
            "currency" => "12 usd to chf",
            _ => "a",
        };
        let response = server
            .handle_json_line(&format!(
                r#"{{"id":"mode-{mode}","method":"search.query","params":{{"query":"{query}","mode":"{mode}","limit":20}}}}"#
            ))
            .await
            .expect("response expected");
        let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
        let results = parsed.result["results"].as_array().expect("results array");
        for row in results {
            let kind = row["kind"].as_str().expect("kind string");
            assert!(
                allowed_kinds.contains(&kind),
                "unexpected kind {kind} in mode {mode}"
            );
        }
    }

    let all_mode = server
        .handle_json_line(
            r#"{"id":"mode-all","method":"search.query","params":{"query":"a","mode":"all","limit":20}}"#,
        )
        .await
        .expect("response expected");
    let parsed: IpcResponse = serde_json::from_str(&all_mode).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    for row in results {
        let kind = row["kind"].as_str().expect("kind string");
        assert!(
            [
                "app",
                "window",
                "file",
                "recent",
                "setting",
                "utility",
                "weather",
                "timezone",
                "emoji",
                "calculator",
                "currency",
            ]
            .contains(&kind),
            "unexpected kind {kind} in all mode"
        );
    }

    let utility_mode = server
        .handle_json_line(
            r#"{"id":"mode-utility","method":"search.query","params":{"query":"utilities","limit":20}}"#,
        )
        .await
        .expect("response expected");
    let parsed: IpcResponse = serde_json::from_str(&utility_mode).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    assert!(
        results.iter().any(|row| row["kind"] == "utility"),
        "expected utility row for short generic query"
    );
}

#[tokio::test]
async fn settings_mode_returns_settings_rows() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(
            r#"{"id":"settings-only","method":"search.query","params":{"query":"settings","mode":"settings","limit":5}}"#,
        )
        .await
        .expect("response expected");
    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    assert!(!results.is_empty(), "expected settings rows");
    assert!(results.iter().all(|row| row["kind"] == "setting"));
}
