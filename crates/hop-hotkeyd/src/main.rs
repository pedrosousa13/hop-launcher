use std::env;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::process;
use std::thread;
use std::time::{Duration, Instant};

use hop_hotkeyd::{
    default_control_socket_path, probe_control_socket, select_backend_mode, send_toggle,
    ControlProbeStatus,
};
use serde_json::json;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt, GrabMode, ModMask};
use x11rb::protocol::Event;

const SWAY_IPC_MAGIC: &[u8; 6] = b"i3-ipc";
const SWAY_MSG_SUBSCRIBE: u32 = 2;
const SWAY_EVENT_TICK: u32 = 0x8000_0007;
const SWAY_TICK_TOGGLE_PAYLOAD: &str = "hop-launcher-toggle";

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Run,
    Trigger { socket_path: String },
    Status,
    Doctor {
        socket_path: String,
        wait_seconds: u64,
        interval_ms: u64,
    },
    PrintBindings { compositor: Option<String> },
}

fn usage() -> &'static str {
    "Usage:\n  hop-hotkeyd                 # run daemon backend mode\n  hop-hotkeyd trigger [--socket <path>]\n  hop-hotkeyd status          # print backend capability status\n  hop-hotkeyd doctor [--socket <path>] [--wait-seconds <n>] [--interval-ms <n>]  # print diagnostics\n  hop-hotkeyd print-bindings [--compositor <name>]  # print compositor binding snippet\n"
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProbeSummary {
    reachable: bool,
    ping_supported: bool,
    status: &'static str,
    error: Option<String>,
}

fn parse_command(args: &[String]) -> Result<Command, String> {
    if args.len() < 2 {
        return Ok(Command::Run);
    }
    if args[1] == "status" {
        return Ok(Command::Status);
    }
    if args[1] == "doctor" {
        let mut socket_path = default_control_socket_path();
        let mut wait_seconds: u64 = 0;
        let mut interval_ms: u64 = 250;
        let mut i = 2;
        while i < args.len() {
            match args[i].as_str() {
                "--socket" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--socket requires a value".to_string());
                    }
                    socket_path = args[i].clone();
                }
                "--wait-seconds" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--wait-seconds requires a value".to_string());
                    }
                    wait_seconds = args[i]
                        .parse::<u64>()
                        .map_err(|_| "--wait-seconds must be an integer".to_string())?;
                }
                "--interval-ms" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--interval-ms requires a value".to_string());
                    }
                    interval_ms = args[i]
                        .parse::<u64>()
                        .map_err(|_| "--interval-ms must be an integer".to_string())?;
                    if interval_ms == 0 {
                        return Err("--interval-ms must be greater than 0".to_string());
                    }
                }
                unknown => return Err(format!("unknown argument: {}", unknown)),
            }
            i += 1;
        }
        return Ok(Command::Doctor {
            socket_path,
            wait_seconds,
            interval_ms,
        });
    }
    if args[1] == "print-bindings" {
        let mut compositor: Option<String> = None;
        let mut i = 2;
        while i < args.len() {
            match args[i].as_str() {
                "--compositor" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--compositor requires a value".to_string());
                    }
                    compositor = Some(args[i].to_ascii_lowercase());
                }
                unknown => return Err(format!("unknown argument: {}", unknown)),
            }
            i += 1;
        }
        return Ok(Command::PrintBindings { compositor });
    }
    if args[1] != "trigger" {
        return Err(usage().to_string());
    }

    let mut socket_path = default_control_socket_path();
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--socket" => {
                i += 1;
                if i >= args.len() {
                    return Err("--socket requires a value".to_string());
                }
                socket_path = args[i].clone();
            }
            unknown => return Err(format!("unknown argument: {}", unknown)),
        }
        i += 1;
    }

    Ok(Command::Trigger { socket_path })
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let command = parse_command(&args)?;
    match command {
        Command::Trigger { socket_path } => send_toggle(&socket_path, "hotkey-trigger"),
        Command::Status => {
            let session_type = env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".to_string());
            println!("{}", build_status_payload(&session_type));
            Ok(())
        }
        Command::Doctor {
            socket_path,
            wait_seconds,
            interval_ms,
        } => {
            let session_type = env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".to_string());
            let backend_payload = build_status_payload(&session_type);
            let summary = summarize_probe_result(run_doctor_probe(
                &socket_path,
                wait_seconds,
                interval_ms,
            ));
            let doctor = json!({
                "status": backend_payload,
                "control_socket_path": socket_path,
                "control_socket_reachable": summary.reachable,
                "control_ping_supported": summary.ping_supported,
                "control_probe_status": summary.status,
                "control_probe_error": summary.error,
                "doctor_wait_seconds": wait_seconds,
                "doctor_interval_ms": interval_ms
            });
            println!("{}", doctor);
            Ok(())
        }
        Command::Run => run_daemon_mode(),
        Command::PrintBindings { compositor } => {
            let control_socket = default_control_socket_path();
            let session_type = env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".to_string());
            let resolved = compositor.unwrap_or_else(|| {
                if session_type.eq_ignore_ascii_case("wayland") {
                    detect_wayland_compositor(
                        &env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
                        &env::var("XDG_SESSION_DESKTOP").unwrap_or_default(),
                        &env::var("SWAYSOCK").unwrap_or_default(),
                        &env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap_or_default(),
                    )
                    .to_string()
                } else {
                    "x11".to_string()
                }
            });
            println!(
                "{}",
                build_binding_snippet_payload(&resolved, &control_socket)
            );
            Ok(())
        }
    }
}

fn summarize_probe_result(probe: Result<ControlProbeStatus, String>) -> ProbeSummary {
    match probe {
        Ok(ControlProbeStatus::Healthy) => ProbeSummary {
            reachable: true,
            ping_supported: true,
            status: "healthy",
            error: None,
        },
        Ok(ControlProbeStatus::ReachableNoPing) => ProbeSummary {
            reachable: true,
            ping_supported: false,
            status: "reachable_no_ping",
            error: None,
        },
        Err(error) => ProbeSummary {
            reachable: false,
            ping_supported: false,
            status: "unreachable",
            error: Some(error),
        },
    }
}

fn run_doctor_probe(
    socket_path: &str,
    wait_seconds: u64,
    interval_ms: u64,
) -> Result<ControlProbeStatus, String> {
    let deadline = Instant::now() + Duration::from_secs(wait_seconds);

    loop {
        match probe_control_socket(socket_path) {
            Ok(status) => return Ok(status),
            Err(error) => {
                if Instant::now() >= deadline {
                    return Err(error);
                }
            }
        }

        thread::sleep(Duration::from_millis(interval_ms));
    }
}

fn build_status_payload(session_type: &str) -> serde_json::Value {
    match select_backend_mode(session_type) {
        Ok(hop_hotkeyd::BackendMode::X11) => json!({
            "session_type": session_type,
            "backend": "x11",
            "global_hotkey_supported": true,
            "mode": "daemon"
        }),
        Ok(hop_hotkeyd::BackendMode::Wayland) => {
            let compositor = detect_wayland_compositor(
                &env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
                &env::var("XDG_SESSION_DESKTOP").unwrap_or_default(),
                &env::var("SWAYSOCK").unwrap_or_default(),
                &env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap_or_default(),
            );
            let sway_socket = env::var("SWAYSOCK").unwrap_or_default();
            build_wayland_status_payload(session_type, compositor, &sway_socket)
        }
        Err(error) => json!({
            "session_type": session_type,
            "backend": "unknown",
            "global_hotkey_supported": false,
            "error": error
        }),
    }
}

fn build_wayland_status_payload(
    session_type: &str,
    compositor: &str,
    sway_socket: &str,
) -> serde_json::Value {
    let native_supported = compositor == "sway" && !sway_socket.trim().is_empty();
    json!({
        "session_type": session_type,
        "backend": "wayland",
        "global_hotkey_supported": native_supported,
        "fallback": "hop-hotkeyd trigger",
        "wayland_compositor": compositor,
        "wayland_backend_mode": if native_supported { "sway_tick" } else { "fallback" },
        "next_step": wayland_next_step_hint(compositor)
    })
}

fn detect_wayland_compositor(
    current_desktop: &str,
    session_desktop: &str,
    sway_sock: &str,
    hyprland_signature: &str,
) -> &'static str {
    if !sway_sock.trim().is_empty() {
        return "sway";
    }
    if !hyprland_signature.trim().is_empty() {
        return "hyprland";
    }

    let merged = format!(
        "{}:{}",
        current_desktop.to_ascii_lowercase(),
        session_desktop.to_ascii_lowercase()
    );
    if merged.contains("gnome") {
        "gnome"
    } else if merged.contains("kde") || merged.contains("plasma") {
        "kde"
    } else if merged.contains("sway") {
        "sway"
    } else if merged.contains("hyprland") {
        "hyprland"
    } else {
        "unknown"
    }
}

fn wayland_next_step_hint(compositor: &str) -> &'static str {
    match compositor {
        "gnome" => "implement gnome-shell integration path for global shortcut capture",
        "kde" => "implement KGlobalAccel integration path for global shortcut capture",
        "sway" => "configure sway `send_tick hop-launcher-toggle` binding for native daemon toggle",
        "hyprland" => "use compositor config binding to call `hop-hotkeyd trigger`",
        _ => "use fallback trigger and detect compositor-specific integration strategy",
    }
}

fn build_binding_snippet_payload(compositor: &str, control_socket: &str) -> serde_json::Value {
    let snippet = match compositor {
        "sway" => format!(
            "Add to ~/.config/sway/config:\nbindsym Ctrl+Shift+ampersand exec swaymsg -q -t send_tick {}\nFallback: ~/.local/bin/hop-hotkeyd trigger --socket {}",
            SWAY_TICK_TOGGLE_PAYLOAD, control_socket
        ),
        "hyprland" => format!(
            "Add to ~/.config/hypr/hyprland.conf:\nbind = CTRL SHIFT, ampersand, exec, ~/.local/bin/hop-hotkeyd trigger --socket {}",
            control_socket
        ),
        "kde" => format!(
            "Use System Settings > Shortcuts > Custom Shortcuts to run: ~/.local/bin/hop-hotkeyd trigger --socket {}",
            control_socket
        ),
        "gnome" => format!(
            "GNOME Wayland requires GNOME Shell extension/API path for true global capture; use fallback trigger for now: ~/.local/bin/hop-hotkeyd trigger --socket {}",
            control_socket
        ),
        "x11" => "X11 uses built-in hotkey daemon capture; no compositor binding snippet needed.".to_string(),
        _ => format!(
            "Unknown compositor. Use fallback command in your compositor config:\n~/.local/bin/hop-hotkeyd trigger --socket {}",
            control_socket
        ),
    };
    json!({
        "compositor": compositor,
        "control_socket_path": control_socket,
        "snippet": snippet
    })
}

fn run_daemon_mode() -> Result<(), String> {
    let session_type = env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".to_string());
    match select_backend_mode(&session_type) {
        Ok(backend) => {
            eprintln!("hop-hotkeyd backend: {:?}", backend);
            match backend {
                hop_hotkeyd::BackendMode::X11 => run_x11_daemon_loop(default_control_socket_path()),
                hop_hotkeyd::BackendMode::Wayland => run_wayland_daemon_mode(),
            }
        }
        Err(error) => {
            eprintln!("hop-hotkeyd backend selection failed: {}", error);
            run_wayland_fallback()
        }
    }
}

fn run_x11_daemon_loop(socket_path: String) -> Result<(), String> {
    let mut attempt: u32 = 0;
    loop {
        match run_x11_hotkey_loop(socket_path.clone()) {
            Ok(()) => return Ok(()),
            Err(error) => {
                let delay = reconnect_backoff_secs(attempt);
                eprintln!(
                    "x11 hotkey loop error: {}. reconnecting in {}s",
                    error, delay
                );
                thread::sleep(Duration::from_secs(delay));
                attempt = attempt.saturating_add(1);
            }
        }
    }
}

fn reconnect_backoff_secs(attempt: u32) -> u64 {
    let capped = attempt.min(6);
    1u64 << capped
}

fn run_wayland_daemon_mode() -> Result<(), String> {
    let compositor = detect_wayland_compositor(
        &env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
        &env::var("XDG_SESSION_DESKTOP").unwrap_or_default(),
        &env::var("SWAYSOCK").unwrap_or_default(),
        &env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap_or_default(),
    );
    if compositor == "sway" {
        let sway_socket = env::var("SWAYSOCK").unwrap_or_default();
        if sway_socket.trim().is_empty() {
            eprintln!("sway compositor detected but SWAYSOCK is empty; using fallback mode");
            return run_wayland_fallback();
        }
        return run_sway_daemon_loop(default_control_socket_path(), sway_socket);
    }
    run_wayland_fallback()
}

fn run_wayland_fallback() -> Result<(), String> {
    eprintln!("Wayland fallback active: use `hop-hotkeyd trigger` until native Wayland capture is implemented.");
    loop {
        thread::sleep(Duration::from_secs(30));
    }
}

fn run_sway_daemon_loop(control_socket_path: String, sway_socket_path: String) -> Result<(), String> {
    let mut attempt: u32 = 0;
    loop {
        match run_sway_tick_loop(control_socket_path.clone(), sway_socket_path.clone()) {
            Ok(()) => return Ok(()),
            Err(error) => {
                let delay = reconnect_backoff_secs(attempt);
                eprintln!(
                    "sway tick loop error: {}. reconnecting in {}s",
                    error, delay
                );
                thread::sleep(Duration::from_secs(delay));
                attempt = attempt.saturating_add(1);
            }
        }
    }
}

fn run_sway_tick_loop(control_socket_path: String, sway_socket_path: String) -> Result<(), String> {
    let mut stream = UnixStream::connect(&sway_socket_path)
        .map_err(|error| format!("sway socket connect failed: {}", error))?;
    subscribe_sway_tick_events(&mut stream)?;

    let mut last_toggle_at: Option<Instant> = None;
    loop {
        let (msg_type, payload) = read_sway_message(&mut stream)?;
        if msg_type == SWAY_EVENT_TICK && parse_sway_tick_toggle_event(&payload) {
            let now = Instant::now();
            if should_emit_toggle(now, &mut last_toggle_at, Duration::from_millis(220)) {
                if let Err(error) = send_toggle(&control_socket_path, "hotkey-sway-tick") {
                    eprintln!("toggle send failed: {}", error);
                }
            }
        }
    }
}

fn subscribe_sway_tick_events(stream: &mut UnixStream) -> Result<(), String> {
    write_sway_message(stream, SWAY_MSG_SUBSCRIBE, br#"["tick"]"#)?;
    let (_, payload) = read_sway_message(stream)?;
    let parsed: serde_json::Value = serde_json::from_slice(&payload)
        .map_err(|error| format!("sway subscribe response parse failed: {}", error))?;
    let success = parsed
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if success {
        Ok(())
    } else {
        Err("sway subscribe response did not acknowledge success".to_string())
    }
}

fn write_sway_message(stream: &mut UnixStream, msg_type: u32, payload: &[u8]) -> Result<(), String> {
    let mut frame = Vec::with_capacity(SWAY_IPC_MAGIC.len() + 8 + payload.len());
    frame.extend_from_slice(SWAY_IPC_MAGIC);
    frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    frame.extend_from_slice(&msg_type.to_le_bytes());
    frame.extend_from_slice(payload);
    stream
        .write_all(&frame)
        .map_err(|error| format!("sway socket write failed: {}", error))
}

fn read_sway_message(stream: &mut UnixStream) -> Result<(u32, Vec<u8>), String> {
    let mut header = [0u8; 14];
    stream
        .read_exact(&mut header)
        .map_err(|error| format!("sway socket read header failed: {}", error))?;
    if &header[..6] != SWAY_IPC_MAGIC {
        return Err("sway socket returned invalid magic header".to_string());
    }
    let payload_len = u32::from_le_bytes([header[6], header[7], header[8], header[9]]) as usize;
    let msg_type = u32::from_le_bytes([header[10], header[11], header[12], header[13]]);
    let mut payload = vec![0u8; payload_len];
    stream
        .read_exact(&mut payload)
        .map_err(|error| format!("sway socket read payload failed: {}", error))?;
    Ok((msg_type, payload))
}

fn parse_sway_tick_toggle_event(payload: &[u8]) -> bool {
    let parsed: serde_json::Value = match serde_json::from_slice(payload) {
        Ok(value) => value,
        Err(_) => return false,
    };
    parsed
        .get("payload")
        .and_then(|value| value.as_str())
        .map(|value| value == SWAY_TICK_TOGGLE_PAYLOAD)
        .unwrap_or(false)
}

fn run_x11_hotkey_loop(socket_path: String) -> Result<(), String> {
    let (conn, screen_num) =
        x11rb::connect(None).map_err(|error| format!("x11 connect failed: {}", error))?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    let numlock_mask = detect_numlock_mask(&conn)?.unwrap_or(ModMask::M2);
    let keycodes = find_toggle_keycodes(&conn)?;
    if keycodes.is_empty() {
        return Err("no keycode found for ampersand/7".to_string());
    }

    for keycode in keycodes {
        for modifiers in hotkey_modifier_variants(numlock_mask) {
            conn.grab_key(
                false,
                root,
                modifiers,
                keycode,
                GrabMode::ASYNC,
                GrabMode::ASYNC,
            )
            .map_err(|error| format!("x11 grab key failed: {}", error))?;
        }
    }
    conn.flush()
        .map_err(|error| format!("x11 flush failed: {}", error))?;

    let mut last_toggle_at: Option<Instant> = None;
    loop {
        let event = conn
            .wait_for_event()
            .map_err(|error| format!("x11 wait event failed: {}", error))?;
        if let Event::KeyPress(_) = event {
            let now = Instant::now();
            if should_emit_toggle(now, &mut last_toggle_at, Duration::from_millis(220)) {
                if let Err(error) = send_toggle(&socket_path, "hotkey-x11") {
                    eprintln!("toggle send failed: {}", error);
                }
            }
        }
    }
}

fn hotkey_modifier_variants(numlock_mask: ModMask) -> [ModMask; 4] {
    [
        ModMask::CONTROL | ModMask::SHIFT,
        ModMask::CONTROL | ModMask::SHIFT | ModMask::LOCK,
        ModMask::CONTROL | ModMask::SHIFT | numlock_mask,
        ModMask::CONTROL | ModMask::SHIFT | ModMask::LOCK | numlock_mask,
    ]
}

fn should_emit_toggle(now: Instant, last: &mut Option<Instant>, min_gap: Duration) -> bool {
    match *last {
        Some(prev) if now.duration_since(prev) < min_gap => false,
        _ => {
            *last = Some(now);
            true
        }
    }
}

fn find_toggle_keycodes(conn: &impl Connection) -> Result<Vec<u8>, String> {
    const XK_AMPERSAND: u32 = 0x0026;
    const XK_7: u32 = 0x0037;

    let setup = conn.setup();
    let min_keycode = setup.min_keycode;
    let max_keycode = setup.max_keycode;
    let keycode_count = max_keycode.saturating_sub(min_keycode).saturating_add(1);

    let reply = conn
        .get_keyboard_mapping(min_keycode, keycode_count)
        .map_err(|error| format!("x11 get keyboard mapping request failed: {}", error))?
        .reply()
        .map_err(|error| format!("x11 get keyboard mapping reply failed: {}", error))?;
    let per_keycode = reply.keysyms_per_keycode as usize;
    if per_keycode == 0 {
        return Ok(Vec::new());
    }

    let mut keycodes = Vec::new();
    for (i, keysyms) in reply.keysyms.chunks(per_keycode).enumerate() {
        if keysyms.iter().any(|keysym| *keysym == XK_AMPERSAND || *keysym == XK_7) {
            keycodes.push(min_keycode + i as u8);
        }
    }
    Ok(keycodes)
}

fn detect_numlock_mask(conn: &impl Connection) -> Result<Option<ModMask>, String> {
    const XK_NUM_LOCK: u32 = 0xFF7F;

    let setup = conn.setup();
    let min_keycode = setup.min_keycode;
    let max_keycode = setup.max_keycode;
    let keycode_count = max_keycode.saturating_sub(min_keycode).saturating_add(1);

    let mapping = conn
        .get_keyboard_mapping(min_keycode, keycode_count)
        .map_err(|error| format!("x11 get keyboard mapping request failed: {}", error))?
        .reply()
        .map_err(|error| format!("x11 get keyboard mapping reply failed: {}", error))?;
    let per_keycode = mapping.keysyms_per_keycode as usize;
    if per_keycode == 0 {
        return Ok(None);
    }

    let modifier_mapping = conn
        .get_modifier_mapping()
        .map_err(|error| format!("x11 get modifier mapping request failed: {}", error))?
        .reply()
        .map_err(|error| format!("x11 get modifier mapping reply failed: {}", error))?;
    let keys_per_mod = modifier_mapping.keycodes_per_modifier() as usize;
    if keys_per_mod == 0 {
        return Ok(None);
    }

    for (mod_index, keycode_chunk) in modifier_mapping
        .keycodes
        .chunks(keys_per_mod)
        .enumerate()
    {
        for keycode in keycode_chunk {
            if *keycode == 0 {
                continue;
            }
            let idx = (*keycode).saturating_sub(min_keycode) as usize;
            let start = idx.saturating_mul(per_keycode);
            let end = start.saturating_add(per_keycode);
            if end > mapping.keysyms.len() {
                continue;
            }
            if mapping.keysyms[start..end]
                .iter()
                .any(|keysym| *keysym == XK_NUM_LOCK)
            {
                return Ok(mod_index_to_mask(mod_index));
            }
        }
    }

    Ok(None)
}

fn mod_index_to_mask(index: usize) -> Option<ModMask> {
    match index {
        0 => Some(ModMask::SHIFT),
        1 => Some(ModMask::LOCK),
        2 => Some(ModMask::CONTROL),
        3 => Some(ModMask::M1),
        4 => Some(ModMask::M2),
        5 => Some(ModMask::M3),
        6 => Some(ModMask::M4),
        7 => Some(ModMask::M5),
        _ => None,
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("hop-hotkeyd: {}", error);
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_defaults_to_run_when_no_subcommand() {
        let args = vec!["hop-hotkeyd".to_string()];
        let command = parse_command(&args).expect("command should parse");
        assert_eq!(command, Command::Run);
    }

    #[test]
    fn parse_trigger_accepts_custom_socket_path() {
        let args = vec![
            "hop-hotkeyd".to_string(),
            "trigger".to_string(),
            "--socket".to_string(),
            "/tmp/custom.sock".to_string(),
        ];
        let command = parse_command(&args).expect("command should parse");
        assert_eq!(
            command,
            Command::Trigger {
                socket_path: "/tmp/custom.sock".to_string()
            }
        );
    }

    #[test]
    fn parse_unknown_subcommand_fails() {
        let args = vec!["hop-hotkeyd".to_string(), "unknown".to_string()];
        let result = parse_command(&args);
        assert!(result.is_err());
    }

    #[test]
    fn parse_status_subcommand() {
        let args = vec!["hop-hotkeyd".to_string(), "status".to_string()];
        let command = parse_command(&args).expect("status should parse");
        assert_eq!(command, Command::Status);
    }

    #[test]
    fn parse_doctor_subcommand_with_custom_socket() {
        let args = vec![
            "hop-hotkeyd".to_string(),
            "doctor".to_string(),
            "--socket".to_string(),
            "/tmp/doctor.sock".to_string(),
        ];
        let command = parse_command(&args).expect("doctor should parse");
        assert_eq!(
            command,
            Command::Doctor {
                socket_path: "/tmp/doctor.sock".to_string(),
                wait_seconds: 0,
                interval_ms: 250
            }
        );
    }

    #[test]
    fn parse_doctor_subcommand_with_wait_options() {
        let args = vec![
            "hop-hotkeyd".to_string(),
            "doctor".to_string(),
            "--wait-seconds".to_string(),
            "3".to_string(),
            "--interval-ms".to_string(),
            "100".to_string(),
        ];
        let command = parse_command(&args).expect("doctor should parse");
        assert_eq!(
            command,
            Command::Doctor {
                socket_path: default_control_socket_path(),
                wait_seconds: 3,
                interval_ms: 100
            }
        );
    }

    #[test]
    fn parse_doctor_rejects_zero_interval() {
        let args = vec![
            "hop-hotkeyd".to_string(),
            "doctor".to_string(),
            "--interval-ms".to_string(),
            "0".to_string(),
        ];
        let result = parse_command(&args);
        assert!(result.is_err());
    }

    #[test]
    fn parse_doctor_rejects_non_integer_wait() {
        let args = vec![
            "hop-hotkeyd".to_string(),
            "doctor".to_string(),
            "--wait-seconds".to_string(),
            "abc".to_string(),
        ];
        let result = parse_command(&args);
        assert!(result.is_err());
    }

    #[test]
    fn parse_print_bindings_subcommand() {
        let args = vec![
            "hop-hotkeyd".to_string(),
            "print-bindings".to_string(),
            "--compositor".to_string(),
            "sway".to_string(),
        ];
        let result = parse_command(&args).expect("print-bindings should parse");
        assert_eq!(
            result,
            Command::PrintBindings {
                compositor: Some("sway".to_string())
            }
        );
    }

    #[test]
    fn variants_include_lock_modifier_combinations() {
        let variants = hotkey_modifier_variants(ModMask::M2);
        assert!(variants.contains(&(ModMask::CONTROL | ModMask::SHIFT)));
        assert!(variants.contains(&(ModMask::CONTROL | ModMask::SHIFT | ModMask::LOCK)));
        assert!(variants.contains(&(ModMask::CONTROL | ModMask::SHIFT | ModMask::M2)));
    }

    #[test]
    fn variants_use_detected_numlock_mask() {
        let variants = hotkey_modifier_variants(ModMask::M4);
        assert!(variants.contains(&(ModMask::CONTROL | ModMask::SHIFT | ModMask::M4)));
        assert!(variants.contains(
            &(ModMask::CONTROL | ModMask::SHIFT | ModMask::LOCK | ModMask::M4)
        ));
    }

    #[test]
    fn debounce_blocks_rapid_retrigger() {
        let base = Instant::now();
        let mut last = None;
        assert!(should_emit_toggle(base, &mut last, Duration::from_millis(220)));
        assert!(!should_emit_toggle(
            base + Duration::from_millis(50),
            &mut last,
            Duration::from_millis(220)
        ));
        assert!(should_emit_toggle(
            base + Duration::from_millis(300),
            &mut last,
            Duration::from_millis(220)
        ));
    }

    #[test]
    fn modifier_index_mapping_matches_x11_order() {
        assert_eq!(mod_index_to_mask(3), Some(ModMask::M1));
        assert_eq!(mod_index_to_mask(4), Some(ModMask::M2));
        assert_eq!(mod_index_to_mask(7), Some(ModMask::M5));
        assert_eq!(mod_index_to_mask(9), None);
    }

    #[test]
    fn reconnect_backoff_caps_growth() {
        assert_eq!(reconnect_backoff_secs(0), 1);
        assert_eq!(reconnect_backoff_secs(1), 2);
        assert_eq!(reconnect_backoff_secs(2), 4);
        assert_eq!(reconnect_backoff_secs(6), 64);
        assert_eq!(reconnect_backoff_secs(20), 64);
    }

    #[test]
    fn status_payload_reports_x11_capability() {
        let payload = build_status_payload("x11");
        assert_eq!(payload["backend"], "x11");
        assert_eq!(payload["global_hotkey_supported"], true);
    }

    #[test]
    fn status_payload_reports_wayland_fallback() {
        let payload = build_wayland_status_payload("wayland", "unknown", "");
        assert_eq!(payload["backend"], "wayland");
        assert_eq!(payload["global_hotkey_supported"], false);
        assert_eq!(payload["wayland_backend_mode"], "fallback");
    }

    #[test]
    fn status_payload_reports_sway_native_mode_when_socket_present() {
        let payload = build_wayland_status_payload("wayland", "sway", "/run/user/1000/sway-ipc.sock");
        assert_eq!(payload["backend"], "wayland");
        assert_eq!(payload["global_hotkey_supported"], true);
        assert_eq!(payload["wayland_backend_mode"], "sway_tick");
    }

    #[test]
    fn parse_sway_tick_event_payload_matches_toggle_marker() {
        assert!(parse_sway_tick_toggle_event(
            br#"{"first":false,"payload":"hop-launcher-toggle"}"#
        ));
        assert!(!parse_sway_tick_toggle_event(
            br#"{"first":false,"payload":"other"}"#
        ));
        assert!(!parse_sway_tick_toggle_event(br#"{"first":false}"#));
    }

    #[test]
    fn detects_wayland_compositor_from_desktop_env() {
        assert_eq!(detect_wayland_compositor("GNOME", "", "", ""), "gnome");
        assert_eq!(detect_wayland_compositor("KDE", "", "", ""), "kde");
        assert_eq!(detect_wayland_compositor("sway", "", "", ""), "sway");
        assert_eq!(detect_wayland_compositor("Hyprland", "", "", ""), "hyprland");
        assert_eq!(detect_wayland_compositor("", "", "", ""), "unknown");
    }

    #[test]
    fn detects_wayland_compositor_from_runtime_hints() {
        assert_eq!(
            detect_wayland_compositor("", "", "/run/user/1000/sway-ipc.sock", ""),
            "sway"
        );
        assert_eq!(
            detect_wayland_compositor("", "", "", "deadbeef-signature"),
            "hyprland"
        );
    }

    #[test]
    fn provides_wayland_next_step_hint() {
        assert!(wayland_next_step_hint("gnome").contains("gnome-shell"));
        assert!(wayland_next_step_hint("kde").contains("KGlobalAccel"));
        assert!(wayland_next_step_hint("sway").contains("send_tick"));
        assert!(wayland_next_step_hint("unknown").contains("fallback"));
    }

    #[test]
    fn binding_payload_for_sway_includes_trigger_command() {
        let payload = build_binding_snippet_payload("sway", "/tmp/hop.sock");
        let snippet = payload["snippet"].as_str().unwrap_or_default();
        assert!(snippet.contains("bindsym"));
        assert!(snippet.contains("send_tick"));
        assert!(snippet.contains("hop-launcher-toggle"));
        assert!(snippet.contains("hop-hotkeyd trigger"));
        assert!(snippet.contains("/tmp/hop.sock"));
    }

    #[test]
    fn binding_payload_for_x11_states_builtin_support() {
        let payload = build_binding_snippet_payload("x11", "/tmp/hop.sock");
        let snippet = payload["snippet"].as_str().unwrap_or_default();
        assert!(snippet.contains("built-in hotkey daemon capture"));
    }

    #[test]
    fn binding_payload_for_kde_includes_socket_path() {
        let payload = build_binding_snippet_payload("kde", "/tmp/hop.sock");
        let snippet = payload["snippet"].as_str().unwrap_or_default();
        assert!(snippet.contains("hop-hotkeyd trigger"));
        assert!(snippet.contains("/tmp/hop.sock"));
    }

    #[test]
    fn summarize_probe_result_maps_states() {
        assert_eq!(
            summarize_probe_result(Ok(ControlProbeStatus::Healthy)),
            ProbeSummary {
                reachable: true,
                ping_supported: true,
                status: "healthy",
                error: None
            }
        );
        assert_eq!(
            summarize_probe_result(Ok(ControlProbeStatus::ReachableNoPing)),
            ProbeSummary {
                reachable: true,
                ping_supported: false,
                status: "reachable_no_ping",
                error: None
            }
        );
        assert_eq!(
            summarize_probe_result(Err("missing socket".to_string())),
            ProbeSummary {
                reachable: false,
                ping_supported: false,
                status: "unreachable",
                error: Some("missing socket".to_string())
            }
        );
    }
}
