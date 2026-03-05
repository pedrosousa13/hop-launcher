use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use hopd::kde_adapter::{
    build_hopd_execute_request, build_hopd_search_request, request_hopd_execute_over_socket,
    request_hopd_search_over_socket,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

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

fn test_socket_path() -> PathBuf {
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let seq = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("hopd-kde-adapter-{nonce}-{seq}.sock"))
}

#[tokio::test]
async fn request_hopd_search_over_socket_returns_response_for_utility_query() {
    let socket_path = test_socket_path();
    let listener = UnixListener::bind(&socket_path).expect("bind socket");

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut reader = BufReader::new(stream);
        let mut request_line = String::new();
        reader
            .read_line(&mut request_line)
            .await
            .expect("read request line");
        assert!(request_line.contains("\"method\":\"search.query\""));
        assert!(request_line.contains("\"query\":\"weather zurich\""));
        let mut stream = reader.into_inner();
        stream
            .write_all(br#"{"id":"kde-proto-1","result":{"ok":true,"results":[{"id":"utility:weather","kind":"weather"}]}}"#)
            .await
            .expect("write response");
        stream.write_all(b"\n").await.expect("write newline");
    });

    let response = request_hopd_search_over_socket(&socket_path, "weather zurich", 5)
        .await
        .expect("request succeeds")
        .expect("utility query response");
    assert_eq!(response["result"]["ok"], true);
    assert_eq!(response["result"]["results"][0]["kind"], "weather");

    server.await.expect("server join");
    let _ = std::fs::remove_file(&socket_path);
}

#[tokio::test]
async fn request_hopd_search_over_socket_skips_non_utility_query() {
    let response = request_hopd_search_over_socket("/tmp/unused.sock", "firefox", 8)
        .await
        .expect("call succeeds");
    assert!(response.is_none());
}

#[test]
fn builds_execute_request_payload() {
    let req = build_hopd_execute_request("app:org.gnome.Nautilus.desktop", "enter");
    assert_eq!(req["method"], "actions.execute");
    assert_eq!(req["params"]["result_id"], "app:org.gnome.Nautilus.desktop");
    assert_eq!(req["params"]["action"], "enter");
}

#[tokio::test]
async fn request_hopd_execute_over_socket_returns_response() {
    let socket_path = test_socket_path();
    let listener = UnixListener::bind(&socket_path).expect("bind socket");

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut reader = BufReader::new(stream);
        let mut request_line = String::new();
        reader
            .read_line(&mut request_line)
            .await
            .expect("read request line");
        assert!(request_line.contains("\"method\":\"actions.execute\""));
        assert!(request_line.contains("\"result_id\":\"app:org.gnome.Nautilus.desktop\""));
        let mut stream = reader.into_inner();
        stream
            .write_all(br#"{"id":"kde-proto-exec-1","result":{"ok":true,"executed":true}}"#)
            .await
            .expect("write response");
        stream.write_all(b"\n").await.expect("write newline");
    });

    let response =
        request_hopd_execute_over_socket(&socket_path, "app:org.gnome.Nautilus.desktop", "enter")
            .await
            .expect("execute request succeeds");
    assert_eq!(response["result"]["ok"], true);
    assert_eq!(response["result"]["executed"], true);

    server.await.expect("server join");
    let _ = std::fs::remove_file(&socket_path);
}
