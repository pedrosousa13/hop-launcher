use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn socket_path() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_millis();
    format!("/tmp/hopd-test-{}-{}.sock", std::process::id(), millis)
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
