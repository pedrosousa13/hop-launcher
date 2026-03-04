use std::env;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendMode {
    Wayland,
    X11,
}

pub fn default_control_socket_path() -> String {
    if let Ok(path) = env::var("HOP_LAUNCHER_CONTROL_SOCKET") {
        return path;
    }
    if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
        return format!("{runtime_dir}/hop-launcher-control.sock");
    }
    "/tmp/hop-launcher-control.sock".to_string()
}

pub fn select_backend_mode(session_type: &str) -> Result<BackendMode, String> {
    match session_type.trim().to_ascii_lowercase().as_str() {
        "wayland" => Ok(BackendMode::Wayland),
        "x11" => Ok(BackendMode::X11),
        other => Err(format!("unsupported session type: {}", other)),
    }
}

pub fn build_toggle_request(id: &str) -> Value {
    json!({
        "id": id,
        "method": "ui.toggle"
    })
}

pub fn send_toggle(socket_path: &str, request_id: &str) -> Result<(), String> {
    let mut stream = UnixStream::connect(socket_path)
        .map_err(|error| format!("connect {} failed: {}", socket_path, error))?;

    let request = build_toggle_request(request_id);
    let encoded = serde_json::to_string(&request)
        .map_err(|error| format!("encode request failed: {}", error))?;
    stream
        .write_all(encoded.as_bytes())
        .map_err(|error| format!("write request failed: {}", error))?;
    stream
        .write_all(b"\n")
        .map_err(|error| format!("write newline failed: {}", error))?;

    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|error| format!("read response failed: {}", error))?;

    let payload: Value = serde_json::from_str(line.trim())
        .map_err(|error| format!("decode response failed: {}", error))?;
    let ok = payload
        .get("result")
        .and_then(|row| row.get("ok"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !ok {
        return Err(format!("toggle not acknowledged: {}", payload));
    }
    Ok(())
}

pub fn probe_control_socket(socket_path: &str) -> Result<(), String> {
    let mut stream = UnixStream::connect(socket_path)
        .map_err(|error| format!("connect {} failed: {}", socket_path, error))?;

    let request = json!({
        "id": "doctor",
        "method": "ui.ping"
    });
    let encoded = serde_json::to_string(&request)
        .map_err(|error| format!("encode request failed: {}", error))?;
    stream
        .write_all(encoded.as_bytes())
        .map_err(|error| format!("write request failed: {}", error))?;
    stream
        .write_all(b"\n")
        .map_err(|error| format!("write newline failed: {}", error))?;

    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|error| format!("read response failed: {}", error))?;
    let _payload: Value = serde_json::from_str(line.trim())
        .map_err(|error| format!("decode response failed: {}", error))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    use std::path::Path;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temp_socket_path(name: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock drift")
            .as_nanos();
        format!("/tmp/{}-{}-{}.sock", name, std::process::id(), nanos)
    }

    #[test]
    fn selects_backend_mode_from_session_type() {
        assert_eq!(select_backend_mode("wayland").unwrap(), BackendMode::Wayland);
        assert_eq!(select_backend_mode("x11").unwrap(), BackendMode::X11);
        assert!(select_backend_mode("tty").is_err());
    }

    #[test]
    fn builds_ui_toggle_request_payload() {
        let payload = build_toggle_request("hk-1");
        assert_eq!(payload["id"], "hk-1");
        assert_eq!(payload["method"], "ui.toggle");
    }

    #[test]
    fn sends_toggle_and_accepts_ok_response() {
        let socket_path = temp_socket_path("hop-hotkeyd-test");
        let listener = UnixListener::bind(&socket_path).expect("bind test socket");
        let server = thread::spawn(move || {
            let (mut conn, _) = listener.accept().expect("accept connection");
            let mut line = String::new();
            let mut reader = BufReader::new(conn.try_clone().expect("clone conn"));
            reader.read_line(&mut line).expect("read request");
            assert!(line.contains("\"method\":\"ui.toggle\""));

            let response = "{\"id\":\"hk-1\",\"result\":{\"ok\":true}}\n";
            conn.write_all(response.as_bytes()).expect("write response");
        });

        let result = send_toggle(&socket_path, "hk-1");
        server.join().expect("server thread");

        if Path::new(&socket_path).exists() {
            let _ = std::fs::remove_file(&socket_path);
        }

        assert!(result.is_ok(), "toggle should succeed");
    }

    #[test]
    fn probe_control_socket_accepts_error_reply_as_reachable() {
        let socket_path = temp_socket_path("hop-hotkeyd-probe");
        let listener = UnixListener::bind(&socket_path).expect("bind test socket");
        let server = thread::spawn(move || {
            let (mut conn, _) = listener.accept().expect("accept connection");
            let mut line = String::new();
            let mut reader = BufReader::new(conn.try_clone().expect("clone conn"));
            reader.read_line(&mut line).expect("read request");
            assert!(line.contains("\"method\":\"ui.ping\""));

            let response = "{\"id\":\"doctor\",\"error\":{\"code\":-32601,\"message\":\"unsupported method\"}}\n";
            conn.write_all(response.as_bytes()).expect("write response");
        });

        let reachable = probe_control_socket(&socket_path);
        server.join().expect("server thread");

        if Path::new(&socket_path).exists() {
            let _ = std::fs::remove_file(&socket_path);
        }

        assert!(reachable.is_ok(), "probe should treat error response as reachable");
    }
}
