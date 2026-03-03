use hopd::kde_adapter::build_hopd_search_request;

#[test]
fn builds_search_query_request_for_utility_intent() {
    let req = build_hopd_search_request("weather zurich", 5).expect("request");
    assert_eq!(req["method"], "search.query");
    assert_eq!(req["params"]["query"], "weather zurich");
    assert_eq!(req["params"]["limit"], 5);
}

#[test]
fn ignores_non_utility_query() {
    let req = build_hopd_search_request("firefox", 5);
    assert!(req.is_none());
}

