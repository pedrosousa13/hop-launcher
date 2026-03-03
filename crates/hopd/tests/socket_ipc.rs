use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static SOCKET_COUNTER: AtomicU64 = AtomicU64::new(0);

fn socket_path() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let seq = SOCKET_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("/tmp/hopd-test-{}-{}-{}.sock", std::process::id(), nanos, seq)
}

fn spawn_hopd(path: &str) -> Child {
    Command::new(env!("CARGO_BIN_EXE_hopd"))
        .arg("--socket")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn hopd")
}

fn wait_for_socket(path: &str) {
    for _ in 0..100 {
        if Path::new(path).exists() {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("socket not created at {}", path);
}

fn send_line(path: &str, line: &str) -> serde_json::Value {
    let mut stream = UnixStream::connect(path).expect("connect socket");
    stream.write_all(line.as_bytes()).expect("write request");
    stream.write_all(b"\n").expect("write newline");

    let mut response = String::new();
    let mut reader = BufReader::new(stream);
    reader.read_line(&mut response).expect("read response");
    serde_json::from_str(response.trim()).expect("parse json response")
}

fn send_lines_in_single_connection(path: &str, lines: &[&str]) -> Vec<serde_json::Value> {
    let mut stream = UnixStream::connect(path).expect("connect socket");
    for line in lines {
        stream.write_all(line.as_bytes()).expect("write request");
        stream.write_all(b"\n").expect("write newline");
    }
    stream.flush().expect("flush request bytes");

    let mut reader = BufReader::new(stream);
    let mut responses = Vec::new();
    for _ in lines {
        let mut response = String::new();
        reader.read_line(&mut response).expect("read response");
        responses.push(serde_json::from_str(response.trim()).expect("parse json response"));
    }

    responses
}

#[test]
fn daemon_serves_health_ping_over_socket() {
    let path = socket_path();
    let mut child = spawn_hopd(&path);
    wait_for_socket(&path);

    let response = send_line(&path, r#"{"id":"s1","method":"health.ping"}"#);
    assert_eq!(response["id"], "s1");
    assert_eq!(response["result"]["ok"], true);
    assert_eq!(response["result"]["service"], "hopd");

    let _ = child.kill();
    let _ = std::fs::remove_file(path);
}

#[test]
fn daemon_returns_standard_parse_error_for_invalid_json() {
    let path = socket_path();
    let mut child = spawn_hopd(&path);
    wait_for_socket(&path);

    let response = send_line(&path, r#"{"id":"bad","method":"health.ping""#);
    assert!(response["id"].is_null());
    assert_eq!(response["error"]["code"], -32700);

    let _ = child.kill();
    let _ = std::fs::remove_file(path);
}

#[test]
fn daemon_handles_search_and_metrics_over_socket() {
    let path = socket_path();
    let mut child = spawn_hopd(&path);
    wait_for_socket(&path);

    let responses = send_lines_in_single_connection(
        &path,
        &[
            r#"{"id":"s2","method":"search.query","params":{"query":"weather emoji","limit":5}}"#,
            r#"{"id":"s3","method":"metrics.snapshot"}"#,
        ],
    );

    assert_eq!(responses[0]["id"], "s2");
    assert!(responses[0]["result"]["results"].is_array());
    let results = responses[0]["result"]["results"].as_array().expect("results array");
    assert!(results.iter().any(|row| row["kind"] == "weather"));
    assert!(results.iter().any(|row| row["kind"] == "emoji"));

    assert_eq!(responses[1]["id"], "s3");
    assert_eq!(responses[1]["result"]["total_requests"], 1);

    let _ = child.kill();
    let _ = std::fs::remove_file(path);
}

#[test]
fn daemon_supports_config_roundtrip_over_socket() {
    let path = socket_path();
    let mut child = spawn_hopd(&path);
    wait_for_socket(&path);

    let responses = send_lines_in_single_connection(
        &path,
        &[
            r#"{"id":"s4","method":"config.set","params":{"key":"features.weather","value":false}}"#,
            r#"{"id":"s5","method":"config.get","params":{"key":"features.weather"}}"#,
        ],
    );

    assert_eq!(responses[0]["id"], "s4");
    assert_eq!(responses[0]["result"]["ok"], true);
    assert!(responses[0]["error"].is_null());

    assert_eq!(responses[1]["id"], "s5");
    assert_eq!(responses[1]["result"]["value"], false);
    assert!(responses[1]["error"].is_null());

    let _ = child.kill();
    let _ = std::fs::remove_file(path);
}
