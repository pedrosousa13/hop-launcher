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
    assert_eq!(parsed.result["service"], "hopd");
    assert!(parsed.result["protocol_version"].is_number());
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
async fn handles_search_query_with_weather_suffix_intent() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(
            r#"{"id":"2","method":"search.query","params":{"query":"zurich weather","limit":5}}"#,
        )
        .await
        .expect("response expected");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    assert_eq!(parsed.id, "2");
    let results = parsed.result["results"].as_array().expect("results array");
    assert!(!results.is_empty(), "expected weather result");
    assert_eq!(results[0]["kind"], "weather");
    assert_eq!(results[0]["title"], "Weather in Zurich");
    assert!(parsed.result["telemetry"]["elapsed_ms"].is_number());
    assert!(parsed.error.is_none());
}

#[tokio::test]
async fn telemetry_reports_elapsed_ms_for_non_utility_query() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(
            r#"{"id":"2h","method":"search.query","params":{"query":"terminal","mode":"apps","limit":5}}"#,
        )
        .await
        .expect("response expected");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    assert!(parsed.result["telemetry"]["elapsed_ms"].is_number());
    assert!(parsed.error.is_none());
}

#[tokio::test]
async fn telemetry_reports_elapsed_ms_for_utility_intent_query() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(
            r#"{"id":"2i","method":"search.query","params":{"query":"weather zurich","mode":"all","limit":5}}"#,
        )
        .await
        .expect("response expected");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    assert!(parsed.result["telemetry"]["elapsed_ms"].is_number());
    assert!(parsed.error.is_none());
}

#[tokio::test]
async fn search_query_returns_ranked_results_when_matches_exist() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(
            r#"{"id":"2b","method":"search.query","params":{"query":"weather","limit":5}}"#,
        )
        .await
        .expect("response expected");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    assert!(!results.is_empty(), "expected at least one result");
    assert_eq!(results[0]["kind"], "weather");
    assert_eq!(results[0]["title"], "Weather");
}

#[tokio::test]
async fn search_query_returns_web_search_action_rows_for_web_prefix() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(
            r#"{"id":"2b-web","method":"search.query","params":{"query":"web rust","limit":5}}"#,
        )
        .await
        .expect("response expected");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    assert!(!results.is_empty(), "expected web action result");
    assert!(results.iter().any(|row| row["kind"] == "action"));
    assert!(results.iter().any(|row| {
        row["kind"] == "action" && row["icon"] == "system-search-symbolic"
    }));
}

#[tokio::test]
async fn search_query_web_prefix_honors_service_keywords_and_order() {
    let server = HopdServer::new();
    server
        .handle_json_line(
            r#"{"id":"ws-cfg","method":"config.set","params":{"key":"web_search.services_json","value":"[{\"id\":\"kagi\",\"name\":\"Kagi\",\"urlTemplate\":\"https://kagi.com/search?q=%s\",\"enabled\":true,\"keyword\":\"kg\"},{\"id\":\"ddg\",\"name\":\"DuckDuckGo\",\"urlTemplate\":\"https://duckduckgo.com/?q=%s\",\"enabled\":true,\"keyword\":\"ddg\"}]"}}"#,
        )
        .await
        .expect("set response");

    let keyword_response = server
        .handle_json_line(
            r#"{"id":"ws-kg","method":"search.query","params":{"query":"kg rust","limit":5}}"#,
        )
        .await
        .expect("response expected");
    let keyword_parsed: IpcResponse =
        serde_json::from_str(&keyword_response).expect("valid json");
    let keyword_results = keyword_parsed.result["results"]
        .as_array()
        .expect("results array");
    assert_eq!(keyword_results.len(), 1);
    assert_eq!(keyword_results[0]["kind"], "action");
    assert_eq!(keyword_results[0]["id"], "web-search:kagi:https%3A%2F%2Fkagi.com%2Fsearch%3Fq%3Drust");
}

#[tokio::test]
async fn search_query_web_prefix_excludes_disabled_services() {
    let server = HopdServer::new();
    server
        .handle_json_line(
            r#"{"id":"ws-cfg-disabled","method":"config.set","params":{"key":"web_search.services_json","value":"[{\"id\":\"kagi\",\"name\":\"Kagi\",\"urlTemplate\":\"https://kagi.com/search?q=%s\",\"enabled\":true,\"keyword\":\"kg\"},{\"id\":\"google\",\"name\":\"Google\",\"urlTemplate\":\"https://www.google.com/search?q=%s\",\"enabled\":false,\"keyword\":\"g\"}]"}}"#,
        )
        .await
        .expect("set response");

    let response = server
        .handle_json_line(
            r#"{"id":"ws-web-disabled","method":"search.query","params":{"query":"web rust","limit":5}}"#,
        )
        .await
        .expect("response expected");
    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    let action_ids = results
        .iter()
        .filter(|row| row["kind"] == "action")
        .filter_map(|row| row["id"].as_str())
        .collect::<Vec<_>>();
    assert!(
        action_ids.iter().any(|id| id.starts_with("web-search:kagi:")),
        "expected enabled provider result"
    );
    assert!(
        !action_ids.iter().any(|id| id.starts_with("web-search:google:")),
        "disabled provider should not appear"
    );
}

#[tokio::test]
async fn search_query_respects_limit() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(
            r#"{"id":"2c","method":"search.query","params":{"query":"e","limit":1}}"#,
        )
        .await
        .expect("response expected");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn search_query_aggregates_multiple_providers() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(
            r#"{"id":"2d","method":"search.query","params":{"query":"weather emoji","limit":5}}"#,
        )
        .await
        .expect("response expected");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    assert!(results.iter().any(|row| row["kind"] == "weather"));
    assert!(results.iter().any(|row| row["kind"] == "emoji"));
}

#[tokio::test]
async fn search_query_handles_timezone_intent_for_city_phrase() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(
            r#"{"id":"2e","method":"search.query","params":{"query":"time tokyo","limit":3}}"#,
        )
        .await
        .expect("response expected");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    assert!(!results.is_empty(), "expected timezone result");
    assert_eq!(results[0]["kind"], "timezone");
    assert_eq!(results[0]["title"], "Time in Tokyo");
}

#[tokio::test]
async fn search_query_returns_calculator_row_for_math_expression() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(
            r#"{"id":"2f","method":"search.query","params":{"query":"2+2","limit":3}}"#,
        )
        .await
        .expect("response expected");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    assert!(!results.is_empty(), "expected calculator result");
    assert_eq!(results[0]["kind"], "calculator");
    assert_eq!(results[0]["id"], "utility:calculator:2+2");
}

#[tokio::test]
async fn search_query_returns_weather_row_for_city_phrase() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(
            r#"{"id":"2f-weather","method":"search.query","params":{"query":"weather zurich","limit":3}}"#,
        )
        .await
        .expect("response expected");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    assert!(!results.is_empty(), "expected weather result");
    assert_eq!(results[0]["kind"], "weather");
    assert_eq!(results[0]["id"], "utility:weather:Zurich");
    assert_eq!(results[0]["title"], "Weather in Zurich");
}

#[tokio::test]
async fn search_query_weather_mode_accepts_bare_city_query() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(
            r#"{"id":"2f-weather-mode","method":"search.query","params":{"query":"zurich","mode":"weather","limit":3}}"#,
        )
        .await
        .expect("response expected");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    assert!(!results.is_empty(), "expected weather result");
    assert_eq!(results[0]["kind"], "weather");
    assert_eq!(results[0]["id"], "utility:weather:Zurich");
    assert_eq!(results[0]["title"], "Weather in Zurich");
}

#[tokio::test]
async fn search_query_returns_currency_row_for_conversion_phrase() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(
            r#"{"id":"2g","method":"search.query","params":{"query":"12 usd to chf","limit":3}}"#,
        )
        .await
        .expect("response expected");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    assert!(!results.is_empty(), "expected currency result");
    assert_eq!(results[0]["kind"], "currency");
    assert_eq!(results[0]["id"], "utility:currency:12:USD:CHF");
}

#[tokio::test]
async fn search_query_handles_timezone_intent_for_suffix_phrase() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(
            r#"{"id":"2g-time","method":"search.query","params":{"query":"zurich time","limit":3}}"#,
        )
        .await
        .expect("response expected");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    assert!(!results.is_empty(), "expected timezone result");
    assert_eq!(results[0]["kind"], "timezone");
    assert_eq!(results[0]["title"], "Time in Zurich");
}

#[tokio::test]
async fn search_query_timezone_mode_accepts_bare_city_query() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(
            r#"{"id":"2g-time-mode","method":"search.query","params":{"query":"tokyo","mode":"timezone","limit":3}}"#,
        )
        .await
        .expect("response expected");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    assert!(!results.is_empty(), "expected timezone result");
    assert_eq!(results[0]["kind"], "timezone");
    assert_eq!(results[0]["title"], "Time in Tokyo");
}

#[tokio::test]
async fn search_query_returns_normalized_rows_with_metadata() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(
            r#"{"id":"n1","method":"search.query","params":{"query":"weather","limit":5}}"#,
        )
        .await
        .expect("response expected");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let first = parsed.result["results"]
        .as_array()
        .expect("results array")
        .first()
        .expect("at least one row");
    assert!(first["subtitle"].is_string());
    assert!(first["icon"].is_string());
    assert_eq!(first["primary_action"], "enter");
    assert!(first["score"].is_number());
}

#[tokio::test]
async fn search_query_returns_default_suggestions_for_empty_query() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(r#"{"id":"n2","method":"search.query","params":{"query":"","limit":8}}"#)
        .await
        .expect("response expected");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    assert!(!results.is_empty(), "expected default suggestions");
    assert!(
        results.iter().any(|row| row["kind"] == "setting"),
        "expected settings suggestions for empty query"
    );
}

#[tokio::test]
async fn search_query_respects_feature_toggles_for_settings() {
    let server = HopdServer::new();
    server
        .handle_json_line(
            r#"{"id":"fs1","method":"config.set","params":{"key":"features.settings","value":false}}"#,
        )
        .await
        .expect("set response");

    let response = server
        .handle_json_line(
            r#"{"id":"fs2","method":"search.query","params":{"query":"settings","mode":"settings","limit":8}}"#,
        )
        .await
        .expect("response expected");
    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    assert!(
        results.is_empty(),
        "expected no settings rows when settings feature is disabled"
    );

    let all_mode = server
        .handle_json_line(
            r#"{"id":"fs3","method":"search.query","params":{"query":"settings","mode":"all","limit":20}}"#,
        )
        .await
        .expect("response expected");
    let parsed: IpcResponse = serde_json::from_str(&all_mode).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    assert!(
        results.iter().all(|row| row["kind"] != "setting"),
        "settings kind should be filtered in all mode when disabled"
    );
}

#[tokio::test]
async fn search_query_respects_feature_toggles_for_utility_family() {
    let server = HopdServer::new();
    server
        .handle_json_line(
            r#"{"id":"fu1","method":"config.set","params":{"key":"features.utility","value":false}}"#,
        )
        .await
        .expect("set response");

    for (id, query, mode) in [
        ("fu2", "2+2", "all"),
        ("fu3", "weather", "all"),
        ("fu4", "time tokyo", "all"),
        ("fu5", "emoji smile", "all"),
        ("fu6", "12 usd to chf", "all"),
    ] {
        let response = server
            .handle_json_line(&format!(
                r#"{{"id":"{id}","method":"search.query","params":{{"query":"{query}","mode":"{mode}","limit":20}}}}"#
            ))
            .await
            .expect("response expected");
        let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
        let results = parsed.result["results"].as_array().expect("results array");
        assert!(
            results.iter().all(|row| {
                let kind = row["kind"].as_str().unwrap_or_default();
                !["utility", "emoji", "calculator", "currency", "weather", "timezone"]
                    .contains(&kind)
            }),
            "utility-family kinds should be filtered when utility feature is disabled"
        );
    }
}

#[tokio::test]
async fn search_query_respects_feature_toggles_for_primary_provider_kinds() {
    let server = HopdServer::new();
    for (set_id, key, mode, query, blocked_kind) in [
        ("fp1", "features.apps", "apps", "a", "app"),
        ("fp2", "features.windows", "windows", "a", "window"),
        ("fp3", "features.files", "files", "a", "file"),
        ("fp4", "features.recents", "recents", "a", "recent"),
    ] {
        server
            .handle_json_line(&format!(
                r#"{{"id":"{set_id}","method":"config.set","params":{{"key":"{key}","value":false}}}}"#
            ))
            .await
            .expect("set response");

        let response = server
            .handle_json_line(&format!(
                r#"{{"id":"q-{set_id}","method":"search.query","params":{{"query":"{query}","mode":"{mode}","limit":20}}}}"#
            ))
            .await
            .expect("response expected");
        let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
        let results = parsed.result["results"].as_array().expect("results array");
        assert!(
            results.is_empty() || results.iter().all(|row| row["kind"] != blocked_kind),
            "mode {mode} should not return disabled kind {blocked_kind}"
        );
    }
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
    assert_eq!(parsed.result["ok"], false);
    assert_eq!(parsed.result["success"], false);
    assert_eq!(parsed.result["executed"], false);
    assert_eq!(parsed.result["action_resolved"], false);
    assert_eq!(parsed.result["execution_status"], "unresolved");
    assert_eq!(parsed.result["resolved_command"], serde_json::Value::Null);
    assert_eq!(parsed.result["resolved_args"], serde_json::Value::Null);
    assert!(parsed.error.is_none());
}

#[tokio::test]
async fn actions_execute_resolves_commands_for_representative_result_kinds() {
    let server = HopdServer::new();
    for (id, result_id, expected_command) in [
        ("exec-app", "app:org.gnome.Nautilus.desktop", "gtk-launch"),
        ("exec-file", "file:/tmp/demo.txt", "xdg-open"),
        ("exec-recent", "recent:/tmp/demo.txt", "xdg-open"),
        ("exec-setting", "setting:network", "gnome-control-center"),
        ("exec-window", "window:0x04200004", "wmctrl"),
    ] {
        let response = server
            .handle_json_line(&format!(
                r#"{{"id":"{id}","method":"actions.execute","params":{{"result_id":"{result_id}","action":"enter"}}}}"#
            ))
            .await
            .expect("response expected");
        let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
        assert_eq!(parsed.id, id);
        assert_eq!(parsed.result["ok"], true);
        assert_eq!(parsed.result["action_resolved"], true);
        assert_eq!(parsed.result["resolved_command"], expected_command);
        assert!(parsed.result["resolved_args"].is_array());
        assert!(parsed.error.is_none());
    }

    for (id, result_id) in [
        ("exec-weather", "utility:weather"),
        ("exec-timezone", "utility:timezone"),
        ("exec-emoji", "utility:emoji"),
        ("exec-calc", "utility:calculator:2+2"),
        ("exec-currency", "utility:currency:12:USD:CHF"),
    ] {
        let response = server
            .handle_json_line(&format!(
                r#"{{"id":"{id}","method":"actions.execute","params":{{"result_id":"{result_id}","action":"enter"}}}}"#
            ))
            .await
            .expect("response expected");
        let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
        assert_eq!(parsed.id, id);
        assert_eq!(parsed.result["ok"], true);
        assert_eq!(parsed.result["action_resolved"], true);
        assert_eq!(parsed.result["execution_status"], "copied");
        assert_eq!(parsed.result["resolved_command"], serde_json::Value::Null);
        assert_eq!(parsed.result["launch_spawned"], false);
        assert!(parsed.result["copied_text"].is_string());
        assert_eq!(parsed.result["action_effective"], "copy");
        assert!(parsed.error.is_none());
    }
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
async fn ranking_config_affects_search_order() {
    let server = HopdServer::new();
    let _ = server
        .handle_json_line(
            r#"{"id":"rw1","method":"config.set","params":{"key":"ranking.weight_emoji","value":120}}"#,
        )
        .await
        .expect("set weight emoji");
    let _ = server
        .handle_json_line(
            r#"{"id":"rw2","method":"config.set","params":{"key":"ranking.weight_utility","value":-80}}"#,
        )
        .await
        .expect("set weight utility");

    let response = server
        .handle_json_line(
            r#"{"id":"rw3","method":"search.query","params":{"query":"e","limit":3}}"#,
        )
        .await
        .expect("search response");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    assert!(!results.is_empty(), "expected ranked rows");
    assert!(
        results
            .iter()
            .all(|row| row.get("score").and_then(serde_json::Value::as_i64).is_some()),
        "expected scored rows after ranking overrides"
    );
}

#[tokio::test]
async fn exposes_metrics_snapshot_with_request_count() {
    let server = HopdServer::new();
    let _ = server
        .handle_json_line(r#"{"id":"m1","method":"health.ping"}"#)
        .await
        .expect("health response");
    let _ = server
        .handle_json_line(r#"{"id":"m2","method":"search.query","params":{"query":"weather","limit":2}}"#)
        .await
        .expect("search response");

    let response = server
        .handle_json_line(r#"{"id":"m3","method":"metrics.snapshot"}"#)
        .await
        .expect("metrics response");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    assert_eq!(parsed.id, "m3");
    assert_eq!(parsed.result["total_requests"], 2);
    assert!(parsed.error.is_none());
}

#[tokio::test]
async fn search_query_respects_feature_toggle_for_web_search() {
    let server = HopdServer::new();
    server
        .handle_json_line(
            r#"{"id":"fws1","method":"config.set","params":{"key":"features.web_search","value":false}}"#,
        )
        .await
        .expect("set response");

    let response = server
        .handle_json_line(
            r#"{"id":"fws2","method":"search.query","params":{"query":"web rust","limit":10}}"#,
        )
        .await
        .expect("response expected");
    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    assert!(
        results.iter().all(|row| row["kind"] != "action"),
        "action kind should be filtered when web_search feature is disabled"
    );
}

#[tokio::test]
async fn ranking_weight_web_search_is_independent_from_utility() {
    let server = HopdServer::new();
    let _ = server
        .handle_json_line(
            r#"{"id":"rwws1","method":"config.set","params":{"key":"ranking.weight_web_search","value":200}}"#,
        )
        .await
        .expect("set weight");

    let response = server
        .handle_json_line(
            r#"{"id":"rwws2","method":"search.query","params":{"query":"web rust","limit":10}}"#,
        )
        .await
        .expect("response expected");
    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    let action_rows: Vec<_> = results.iter().filter(|r| r["kind"] == "action").collect();
    assert!(!action_rows.is_empty(), "expected web search action rows");
    let action_score = action_rows[0]["score"].as_i64().unwrap_or(0);
    assert!(
        action_score > 100,
        "expected high score from weight_web_search=200, got {action_score}"
    );
}

#[tokio::test]
async fn search_query_returns_web_search_actions_without_prefix() {
    let server = HopdServer::new();
    let response = server
        .handle_json_line(
            r#"{"id":"ws-noprefix","method":"search.query","params":{"query":"rust programming","limit":10}}"#,
        )
        .await
        .expect("response expected");

    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    assert!(
        results.iter().any(|row| row["kind"] == "action"),
        "expected web search action rows for unprefixed query"
    );
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

#[tokio::test]
async fn search_query_web_search_respects_max_actions_for_unprefixed_query() {
    let server = HopdServer::new();
    server
        .handle_json_line(
            r#"{"id":"wm-cfg","method":"config.set","params":{"key":"web_search.services_json","value":"[{\"id\":\"a\",\"name\":\"Alpha\",\"urlTemplate\":\"https://alpha.example.com/?q=%s\",\"enabled\":true,\"keyword\":\"\"},{\"id\":\"b\",\"name\":\"Bravo\",\"urlTemplate\":\"https://bravo.example.com/?q=%s\",\"enabled\":true,\"keyword\":\"\"},{\"id\":\"c\",\"name\":\"Charlie\",\"urlTemplate\":\"https://charlie.example.com/?q=%s\",\"enabled\":true,\"keyword\":\"\"},{\"id\":\"d\",\"name\":\"Delta\",\"urlTemplate\":\"https://delta.example.com/?q=%s\",\"enabled\":true,\"keyword\":\"\"}]"}}"#,
        )
        .await
        .expect("set services");
    server
        .handle_json_line(
            r#"{"id":"wm-max","method":"config.set","params":{"key":"web_search.max_actions","value":2}}"#,
        )
        .await
        .expect("set max");

    let response = server
        .handle_json_line(
            r#"{"id":"wm-q","method":"search.query","params":{"query":"test query","limit":20}}"#,
        )
        .await
        .expect("response expected");
    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    let action_count = results.iter().filter(|r| r["kind"] == "action").count();
    assert_eq!(
        action_count, 2,
        "expected exactly 2 action rows with max_actions=2 and 4 services, got {action_count}"
    );
}

#[tokio::test]
async fn search_query_keyword_prefix_returns_single_provider_result() {
    let server = HopdServer::new();
    server
        .handle_json_line(
            r#"{"id":"kw-cfg","method":"config.set","params":{"key":"web_search.services_json","value":"[{\"id\":\"google\",\"name\":\"Google\",\"urlTemplate\":\"https://www.google.com/search?q=%s\",\"enabled\":true,\"keyword\":\"g\"},{\"id\":\"ddg\",\"name\":\"DuckDuckGo\",\"urlTemplate\":\"https://duckduckgo.com/?q=%s\",\"enabled\":true,\"keyword\":\"ddg\"}]"}}"#,
        )
        .await
        .expect("set services");

    let response = server
        .handle_json_line(
            r#"{"id":"kw-q","method":"search.query","params":{"query":"g rust","limit":10}}"#,
        )
        .await
        .expect("response expected");
    let parsed: IpcResponse = serde_json::from_str(&response).expect("valid json");
    let results = parsed.result["results"].as_array().expect("results array");
    let action_rows: Vec<_> = results
        .iter()
        .filter(|r| r["kind"] == "action")
        .collect();
    assert_eq!(action_rows.len(), 1, "keyword prefix should return single provider");
    assert!(
        action_rows[0]["id"].as_str().unwrap_or("").starts_with("web-search:google:"),
        "keyword 'g' should match Google provider"
    );
}
