use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};
use std::process::{self, Command as ProcessCommand, Stdio};
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
const HYPRLAND_TOGGLE_EVENT: &str = "hop-launcher-toggle";
const KDE_TOGGLE_ACTION: &str = "hop-launcher-toggle";
const GNOME_BRIDGE_INTERFACE: &str = "io.github.hop.Hotkeyd";
const GNOME_BRIDGE_MEMBER: &str = "Toggle";

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Run,
    Trigger { socket_path: String },
    ConfigGet,
    ConfigSet { shortcut: String },
    SetupShortcut {
        compositor: Option<String>,
        shortcut: String,
        shortcut_explicit: bool,
        socket_path: String,
        dry_run: bool,
    },
    Status {
        socket_path: String,
        compositor: Option<String>,
    },
    Doctor {
        socket_path: String,
        wait_seconds: u64,
        interval_ms: u64,
        compositor: Option<String>,
        strict: bool,
    },
    PrintBindings {
        compositor: Option<String>,
        socket_path: String,
    },
}

fn usage() -> &'static str {
    "Usage:\n  hop-hotkeyd                 # run daemon backend mode\n  hop-hotkeyd trigger [--socket <path>]\n  hop-hotkeyd config get\n  hop-hotkeyd config set --shortcut <accel>\n  hop-hotkeyd setup-shortcut [--compositor <name>] [--shortcut <accel>] [--socket <path>] [--dry-run]\n  hop-hotkeyd status [--socket <path>] [--compositor <name>]  # print backend capability status\n  hop-hotkeyd doctor [--socket <path>] [--wait-seconds <n>] [--interval-ms <n>] [--compositor <name>] [--strict]  # print diagnostics\n  hop-hotkeyd print-bindings [--compositor <name>] [--socket <path>]  # print compositor binding snippet\n"
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProbeSummary {
    reachable: bool,
    ping_supported: bool,
    status: &'static str,
    error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeBackendProbe {
    ready: bool,
    backend_mode: &'static str,
    socket_path: Option<String>,
    error: Option<String>,
}

fn parse_command(args: &[String]) -> Result<Command, String> {
    if args.len() < 2 {
        return Ok(Command::Run);
    }
    if args[1] == "config" {
        if args.len() < 3 {
            return Err("config requires a subcommand: get|set".to_string());
        }
        if args[2] == "get" {
            return Ok(Command::ConfigGet);
        }
        if args[2] == "set" {
            let mut shortcut: Option<String> = None;
            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "--shortcut" => {
                        i += 1;
                        if i >= args.len() {
                            return Err("--shortcut requires a value".to_string());
                        }
                        shortcut = Some(args[i].clone());
                    }
                    unknown => return Err(format!("unknown argument: {}", unknown)),
                }
                i += 1;
            }
            return Ok(Command::ConfigSet {
                shortcut: shortcut.unwrap_or_else(default_configured_shortcut),
            });
        }
        return Err("config subcommand must be get or set".to_string());
    }
    if args[1] == "status" {
        let mut socket_path = default_control_socket_path();
        let mut compositor: Option<String> = None;
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
        return Ok(Command::Status {
            socket_path,
            compositor,
        });
    }
    if args[1] == "setup-shortcut" {
        let mut compositor: Option<String> = None;
        let mut shortcut = "<Super>space".to_string();
        let mut shortcut_explicit = false;
        let mut socket_path = default_control_socket_path();
        let mut dry_run = false;
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
                "--shortcut" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--shortcut requires a value".to_string());
                    }
                    shortcut = args[i].clone();
                    shortcut_explicit = true;
                }
                "--socket" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--socket requires a value".to_string());
                    }
                    socket_path = args[i].clone();
                }
                "--dry-run" => {
                    dry_run = true;
                }
                unknown => return Err(format!("unknown argument: {}", unknown)),
            }
            i += 1;
        }
        return Ok(Command::SetupShortcut {
            compositor,
            shortcut,
            shortcut_explicit,
            socket_path,
            dry_run,
        });
    }
    if args[1] == "doctor" {
        let mut socket_path = default_control_socket_path();
        let mut wait_seconds: u64 = 0;
        let mut interval_ms: u64 = 250;
        let mut compositor: Option<String> = None;
        let mut strict = false;
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
                "--compositor" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--compositor requires a value".to_string());
                    }
                    compositor = Some(args[i].to_ascii_lowercase());
                }
                "--strict" => {
                    strict = true;
                }
                unknown => return Err(format!("unknown argument: {}", unknown)),
            }
            i += 1;
        }
        return Ok(Command::Doctor {
            socket_path,
            wait_seconds,
            interval_ms,
            compositor,
            strict,
        });
    }
    if args[1] == "print-bindings" {
        let mut compositor: Option<String> = None;
        let mut socket_path = default_control_socket_path();
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
        return Ok(Command::PrintBindings {
            compositor,
            socket_path,
        });
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
        Command::ConfigGet => {
            let path = configured_shortcut_path();
            println!(
                "{}",
                json!({
                    "shortcut": load_configured_shortcut(&path).unwrap_or_else(|_| default_configured_shortcut()),
                })
            );
            Ok(())
        }
        Command::ConfigSet { shortcut } => {
            let path = configured_shortcut_path();
            let (applied, shortcut, warnings) = match store_configured_shortcut(&path, &shortcut) {
                Ok(saved) => (true, saved, Vec::<String>::new()),
                Err(error) => (
                    false,
                    load_configured_shortcut(&path)
                        .unwrap_or_else(|_| default_configured_shortcut()),
                    vec![error],
                ),
            };
            println!(
                "{}",
                json!({
                    "shortcut": shortcut,
                    "applied": applied,
                    "requires_manual_step": !applied,
                    "warnings": warnings,
                })
            );
            Ok(())
        }
        Command::SetupShortcut {
            compositor,
            shortcut,
            shortcut_explicit,
            socket_path,
            dry_run,
        } => {
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
            let effective_shortcut = if shortcut_explicit {
                shortcut
            } else {
                let path = configured_shortcut_path();
                load_configured_shortcut(&path).unwrap_or_else(|_| default_configured_shortcut())
            };
            setup_shortcut(&resolved, &effective_shortcut, &socket_path, dry_run)
        }
        Command::Status {
            socket_path,
            compositor,
        } => {
            let session_type = env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".to_string());
            let mut payload = build_status_payload(&session_type, compositor.as_deref());
            add_runtime_shortcut_fields(&mut payload);
            let summary = summarize_probe_result(probe_control_socket(&socket_path));
            add_control_probe_fields(&mut payload, &socket_path, summary);
            println!("{}", payload);
            Ok(())
        }
        Command::Doctor {
            socket_path,
            wait_seconds,
            interval_ms,
            compositor,
            strict,
        } => {
            let session_type = env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".to_string());
            let mut backend_payload = build_status_payload(&session_type, compositor.as_deref());
            add_runtime_shortcut_fields(&mut backend_payload);
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
                "doctor_interval_ms": interval_ms,
                "doctor_strict": strict
            });
            println!("{}", doctor);
            if should_fail_doctor_strict(strict, &summary, &backend_payload) {
                return Err(
                    "doctor strict check failed: control socket unreachable or shortcut unapplied"
                        .to_string(),
                );
            }
            Ok(())
        }
        Command::Run => run_daemon_mode(),
        Command::PrintBindings {
            compositor,
            socket_path,
        } => {
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
                build_binding_snippet_payload(&resolved, &socket_path)
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

fn normalize_compositor_name(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "sway" => "sway",
        "hyprland" | "hypr" => "hyprland",
        "kde" | "plasma" => "kde",
        "gnome" => "gnome",
        "x11" => "x11",
        _ => "unknown",
    }
}

fn default_configured_shortcut() -> String {
    "<Super>space".to_string()
}

fn should_fail_doctor_strict(
    strict: bool,
    summary: &ProbeSummary,
    backend_payload: &serde_json::Value,
) -> bool {
    if !strict {
        return false;
    }
    if !summary.reachable {
        return true;
    }
    !backend_payload
        .get("applied")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn configured_shortcut_path() -> PathBuf {
    if let Ok(path) = env::var("HOP_HOTKEYD_CONFIG") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    if let Ok(config_home) = env::var("XDG_CONFIG_HOME") {
        let trimmed = config_home.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed).join("hop").join("hotkeyd.json");
        }
    }

    if let Ok(home) = env::var("HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed)
                .join(".config")
                .join("hop")
                .join("hotkeyd.json");
        }
    }

    PathBuf::from("/tmp/hop-hotkeyd.json")
}

fn validate_shortcut(shortcut: &str) -> Result<(), String> {
    let trimmed = shortcut.trim();
    if trimmed.is_empty() {
        return Err("shortcut must not be empty".to_string());
    }

    let mut has_modifier = false;
    let mut index = 0usize;
    let bytes = trimmed.as_bytes();
    while index < bytes.len() && bytes[index] == b'<' {
        let Some(end_rel) = trimmed[index + 1..].find('>') else {
            return Err("shortcut has unterminated modifier".to_string());
        };
        let end = index + 1 + end_rel;
        let modifier = trimmed[index + 1..end].trim();
        if modifier.is_empty() {
            return Err("shortcut contains empty modifier".to_string());
        }
        has_modifier = true;
        index = end + 1;
    }

    if !has_modifier {
        return Err("shortcut must include at least one modifier".to_string());
    }

    let key = trimmed[index..].trim();
    if key.is_empty() {
        return Err("shortcut must include a key".to_string());
    }
    if key.starts_with('<') || key.contains('>') {
        return Err("shortcut key segment is invalid".to_string());
    }

    Ok(())
}

fn load_configured_shortcut(path: &Path) -> Result<String, String> {
    if !path.exists() {
        return Ok(default_configured_shortcut());
    }

    let content = fs::read_to_string(path)
        .map_err(|error| format!("read shortcut config failed: {}", error))?;
    let payload: serde_json::Value = serde_json::from_str(content.trim())
        .map_err(|error| format!("parse shortcut config failed: {}", error))?;
    let shortcut = payload
        .get("shortcut")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "shortcut config missing `shortcut`".to_string())?
        .trim()
        .to_string();
    validate_shortcut(&shortcut)?;
    Ok(shortcut)
}

fn store_configured_shortcut(path: &Path, shortcut: &str) -> Result<String, String> {
    validate_shortcut(shortcut)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create config directory failed: {}", error))?;
    }

    let parent = path
        .parent()
        .ok_or_else(|| "invalid config path".to_string())?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "invalid config file name".to_string())?;
    let temp_name = format!(
        ".{}.tmp-{}-{}",
        file_name,
        process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| format!("clock failure: {}", error))?
            .as_nanos()
    );
    let temp_path = parent.join(temp_name);

    let mut file = fs::File::create(&temp_path)
        .map_err(|error| format!("create temp config failed: {}", error))?;
    let payload = json!({ "shortcut": shortcut.trim() }).to_string();
    file.write_all(payload.as_bytes())
        .map_err(|error| format!("write temp config failed: {}", error))?;
    file.write_all(b"\n")
        .map_err(|error| format!("write temp newline failed: {}", error))?;
    file.sync_all()
        .map_err(|error| format!("sync temp config failed: {}", error))?;

    fs::rename(&temp_path, path).map_err(|error| format!("replace config failed: {}", error))?;
    Ok(shortcut.trim().to_string())
}

fn add_runtime_shortcut_fields(payload: &mut serde_json::Value) {
    let path = configured_shortcut_path();
    match load_configured_shortcut(&path) {
        Ok(shortcut) => {
            if let Some(object) = payload.as_object_mut() {
                object.insert(
                    "configured_shortcut".to_string(),
                    serde_json::Value::String(shortcut),
                );
            }
        }
        Err(error) => {
            if let Some(object) = payload.as_object_mut() {
                object.insert(
                    "configured_shortcut".to_string(),
                    serde_json::Value::String(default_configured_shortcut()),
                );
                let warnings = object
                    .entry("warnings".to_string())
                    .or_insert_with(|| serde_json::Value::Array(Vec::new()));
                if let Some(arr) = warnings.as_array_mut() {
                    arr.push(serde_json::Value::String(format!(
                        "config read failed: {}",
                        error
                    )));
                }
            }
        }
    }
}

fn build_status_payload(session_type: &str, compositor_override: Option<&str>) -> serde_json::Value {
    match select_backend_mode(session_type) {
        Ok(hop_hotkeyd::BackendMode::X11) => json!({
            "session_type": session_type,
            "backend": "x11",
            "global_hotkey_supported": true,
            "mode": "daemon",
            "configured_shortcut": default_configured_shortcut(),
            "applied": true,
            "requires_manual_step": false,
            "warnings": [],
        }),
        Ok(hop_hotkeyd::BackendMode::Wayland) => {
            let compositor = compositor_override
                .map(normalize_compositor_name)
                .unwrap_or_else(|| {
                    detect_wayland_compositor(
                        &env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
                        &env::var("XDG_SESSION_DESKTOP").unwrap_or_default(),
                        &env::var("SWAYSOCK").unwrap_or_default(),
                        &env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap_or_default(),
                    )
                });
            let sway_socket = env::var("SWAYSOCK").unwrap_or_default();
            let hyprland_signature = env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap_or_default();
            let runtime_dir = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
            build_wayland_status_payload(
                session_type,
                compositor,
                &sway_socket,
                &hyprland_signature,
                &runtime_dir,
            )
        }
        Err(error) => json!({
            "session_type": session_type,
            "backend": "unknown",
            "global_hotkey_supported": false,
            "applied": false,
            "requires_manual_step": true,
            "warnings": [error.clone()],
            "error": error
        }),
    }
}

fn build_wayland_status_payload(
    session_type: &str,
    compositor: &str,
    sway_socket: &str,
    hyprland_signature: &str,
    runtime_dir: &str,
) -> serde_json::Value {
    let native_probe =
        probe_wayland_native_backend(compositor, sway_socket, hyprland_signature, runtime_dir);
    let recommended_binding =
        recommended_wayland_binding(compositor, native_probe.backend_mode, native_probe.socket_path.as_deref());
    let warning = native_probe.error.clone();
    json!({
        "session_type": session_type,
        "backend": "wayland",
        "configured_shortcut": default_configured_shortcut(),
        "global_hotkey_supported": native_probe.ready,
        "applied": native_probe.ready,
        "requires_manual_step": !native_probe.ready,
        "warnings": warning.into_iter().collect::<Vec<String>>(),
        "fallback": "hop-hotkeyd trigger",
        "wayland_compositor": compositor,
        "wayland_backend_mode": native_probe.backend_mode,
        "native_backend_ready": native_probe.ready,
        "native_backend_socket": native_probe.socket_path,
        "native_backend_error": native_probe.error,
        "recommended_binding": recommended_binding,
        "next_step": wayland_next_step_hint(compositor),
    })
}

fn recommended_wayland_binding(
    compositor: &str,
    backend_mode: &str,
    socket_path: Option<&str>,
) -> Option<String> {
    if compositor == "sway" && backend_mode == "sway_tick" {
        return Some(format!(
            "bindsym Ctrl+Shift+ampersand exec swaymsg -q -t send_tick {}",
            SWAY_TICK_TOGGLE_PAYLOAD
        ));
    }
    if compositor == "hyprland" && backend_mode == "hyprland_event" {
        return Some(format!(
            "bind = CTRL SHIFT, ampersand, exec, hyprctl dispatch event {}",
            HYPRLAND_TOGGLE_EVENT
        ));
    }
    if compositor == "kde" && backend_mode == "kde_dbus_bridge" {
        return Some(format!(
            "qdbus org.kde.kglobalaccel /component/hoplauncher org.kde.kglobalaccel.Component.invokeShortcut {}",
            KDE_TOGGLE_ACTION
        ));
    }
    if compositor == "gnome" && backend_mode == "gnome_shell_bridge" {
        return Some(format!(
            "gdbus emit --session --object-path /io/github/hop/Hotkeyd --signal {}.{} {}",
            GNOME_BRIDGE_INTERFACE, GNOME_BRIDGE_MEMBER, KDE_TOGGLE_ACTION
        ));
    }

    socket_path.map(|path| format!("~/.local/bin/hop-hotkeyd trigger --socket {}", path))
}

fn add_control_probe_fields(
    payload: &mut serde_json::Value,
    socket_path: &str,
    summary: ProbeSummary,
) {
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "control_socket_path".to_string(),
            serde_json::Value::String(socket_path.to_string()),
        );
        object.insert(
            "control_socket_reachable".to_string(),
            serde_json::Value::Bool(summary.reachable),
        );
        object.insert(
            "control_ping_supported".to_string(),
            serde_json::Value::Bool(summary.ping_supported),
        );
        object.insert(
            "control_probe_status".to_string(),
            serde_json::Value::String(summary.status.to_string()),
        );
        object.insert(
            "control_probe_error".to_string(),
            match summary.error {
                Some(error) => serde_json::Value::String(error),
                None => serde_json::Value::Null,
            },
        );
    }
}

fn probe_wayland_native_backend(
    compositor: &str,
    sway_socket: &str,
    hyprland_signature: &str,
    runtime_dir: &str,
) -> NativeBackendProbe {
    if compositor == "sway" {
        if sway_socket.trim().is_empty() {
            return NativeBackendProbe {
                ready: false,
                backend_mode: "fallback",
                socket_path: None,
                error: Some("SWAYSOCK is not set".to_string()),
            };
        }
        let socket_path = sway_socket.to_string();
        return check_unix_socket_path("sway_tick", socket_path);
    }

    if compositor == "hyprland" {
        if hyprland_signature.trim().is_empty() {
            return NativeBackendProbe {
                ready: false,
                backend_mode: "fallback",
                socket_path: None,
                error: Some("HYPRLAND_INSTANCE_SIGNATURE is not set".to_string()),
            };
        }
        let socket_path = format!("{}/hypr/{}/.socket2.sock", runtime_dir, hyprland_signature);
        return check_unix_socket_path("hyprland_event", socket_path);
    }
    if compositor == "kde" {
        let monitor_ready = command_exists_in_path("dbus-monitor");
        return NativeBackendProbe {
            ready: monitor_ready,
            backend_mode: "kde_dbus_bridge",
            socket_path: None,
            error: if monitor_ready {
                None
            } else {
                Some("dbus-monitor is not available in PATH".to_string())
            },
        };
    }
    if compositor == "gnome" {
        let monitor_ready = command_exists_in_path("dbus-monitor");
        return NativeBackendProbe {
            ready: monitor_ready,
            backend_mode: "gnome_shell_bridge",
            socket_path: None,
            error: if monitor_ready {
                None
            } else {
                Some("dbus-monitor is not available in PATH".to_string())
            },
        };
    }

    NativeBackendProbe {
        ready: false,
        backend_mode: "fallback",
        socket_path: None,
        error: None,
    }
}

fn check_unix_socket_path(backend_mode: &'static str, socket_path: String) -> NativeBackendProbe {
    let path = Path::new(&socket_path);
    match fs::metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_socket() {
                NativeBackendProbe {
                    ready: true,
                    backend_mode,
                    socket_path: Some(socket_path),
                    error: None,
                }
            } else {
                NativeBackendProbe {
                    ready: false,
                    backend_mode: "fallback",
                    socket_path: Some(socket_path),
                    error: Some("path exists but is not a unix socket".to_string()),
                }
            }
        }
        Err(error) => NativeBackendProbe {
            ready: false,
            backend_mode: "fallback",
            socket_path: Some(socket_path),
            error: Some(format!("socket metadata check failed: {}", error)),
        },
    }
}

fn command_exists_in_path(command_name: &str) -> bool {
    let Some(paths) = env::var_os("PATH") else {
        return false;
    };
    for path in env::split_paths(&paths) {
        let candidate = path.join(command_name);
        if candidate.is_file() {
            return true;
        }
    }
    false
}

fn run_external_command(dry_run: bool, program: &str, args: &[&str]) -> Result<(), String> {
    if dry_run {
        println!("dry-run: {} {}", program, args.join(" "));
        return Ok(());
    }

    let status = ProcessCommand::new(program)
        .args(args)
        .status()
        .map_err(|error| format!("failed to start {}: {}", program, error))?;
    if !status.success() {
        return Err(format!("{} exited with {}", program, status));
    }
    Ok(())
}

fn read_command_stdout(program: &str, args: &[&str]) -> Result<String, String> {
    let output = ProcessCommand::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("failed to start {}: {}", program, error))?;
    if !output.status.success() {
        return Err(format!("{} exited with {}", program, output.status));
    }
    String::from_utf8(output.stdout).map_err(|error| format!("{} output decode failed: {}", program, error))
}

fn append_gsettings_array_path(current: &str, path: &str) -> String {
    let desired = format!("'{}'", path);
    if current.contains(&desired) {
        return current.to_string();
    }
    if current.trim() == "@as []" || current.trim() == "[]" {
        return format!("[{}]", desired);
    }
    let trimmed = current.trim();
    if let Some(prefix) = trimmed.strip_suffix(']') {
        return format!("{}, {}]", prefix, desired);
    }
    format!("[{}]", desired)
}

fn setup_shortcut(
    compositor: &str,
    shortcut: &str,
    socket_path: &str,
    dry_run: bool,
) -> Result<(), String> {
    match compositor {
        "gnome" => setup_gnome_shortcut(shortcut, socket_path, dry_run),
        "kde" => setup_kde_shortcut(shortcut, socket_path, dry_run),
        "x11" => {
            println!("x11 uses built-in key grab; setup-shortcut is not required.");
            Ok(())
        }
        other => Err(format!(
            "shortcut setup not implemented for compositor '{}'; use `hop-hotkeyd print-bindings --compositor {}`",
            other, other
        )),
    }
}

fn setup_gnome_shortcut(shortcut: &str, socket_path: &str, dry_run: bool) -> Result<(), String> {
    if !command_exists_in_path("gsettings") {
        return Err("gsettings is not available in PATH".to_string());
    }
    let binding_path =
        "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/hop-launcher-hotkeyd/";
    let schema = "org.gnome.settings-daemon.plugins.media-keys";
    let key = "custom-keybindings";
    let entry_schema = format!(
        "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}",
        binding_path
    );
    let current = if dry_run {
        "@as []".to_string()
    } else {
        read_command_stdout("gsettings", &["get", schema, key])?
    };
    let updated = append_gsettings_array_path(current.trim(), binding_path);
    let home = env::var("HOME").unwrap_or_else(|_| "~".to_string());
    let command = format!("{}/.local/bin/hop-hotkeyd trigger --socket {}", home, socket_path);
    let binding = shortcut.to_string();

    run_external_command(dry_run, "gsettings", &["set", schema, key, updated.as_str()])?;
    run_external_command(
        dry_run,
        "gsettings",
        &["set", entry_schema.as_str(), "name", "Hop Launcher"],
    )?;
    run_external_command(
        dry_run,
        "gsettings",
        &["set", entry_schema.as_str(), "command", command.as_str()],
    )?;
    run_external_command(
        dry_run,
        "gsettings",
        &["set", entry_schema.as_str(), "binding", binding.as_str()],
    )?;

    println!(
        "configured gnome shortcut {} -> ~/.local/bin/hop-hotkeyd trigger --socket {}",
        shortcut, socket_path
    );
    Ok(())
}

fn setup_kde_shortcut(shortcut: &str, socket_path: &str, dry_run: bool) -> Result<(), String> {
    let trigger_command = format!("~/.local/bin/hop-hotkeyd trigger --socket {}", socket_path);
    if command_exists_in_path("qdbus6") || command_exists_in_path("qdbus") {
        println!(
            "kde shortcut setup: assign '{}' to command: {}",
            shortcut, trigger_command
        );
        println!(
            "kde trigger probe: qdbus org.kde.kglobalaccel /component/hoplauncher org.kde.kglobalaccel.Component.invokeShortcut {}",
            KDE_TOGGLE_ACTION
        );
        if dry_run {
            println!("dry-run: no KDE setting was modified.");
        }
        return Ok(());
    }
    Err("kde setup requires qdbus or qdbus6 in PATH".to_string())
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
        "gnome" => "configure gnome shell extension bridge to emit io.github.hop.Hotkeyd.Toggle",
        "kde" => "configure KGlobalAccel action and DBus bridge event for hop-launcher-toggle",
        "sway" => "configure sway `send_tick hop-launcher-toggle` binding for native daemon toggle",
        "hyprland" => "configure hyprland `dispatch event hop-launcher-toggle` binding for native daemon toggle",
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
            "Add to ~/.config/hypr/hyprland.conf:\nbind = CTRL SHIFT, ampersand, exec, hyprctl dispatch event {}\nFallback: ~/.local/bin/hop-hotkeyd trigger --socket {}",
            HYPRLAND_TOGGLE_EVENT, control_socket
        ),
        "kde" => format!(
            "KDE DBus bridge path (KGlobalAccel):\n1) Create a KGlobalAccel shortcut action named `{}`.\n2) Ensure hop-hotkeyd daemon is running (it listens via dbus-monitor).\n3) Manual test command:\nqdbus org.kde.kglobalaccel /component/hoplauncher org.kde.kglobalaccel.Component.invokeShortcut {}\nFallback: ~/.local/bin/hop-hotkeyd trigger --socket {}",
            KDE_TOGGLE_ACTION, KDE_TOGGLE_ACTION, control_socket
        ),
        "gnome" => format!(
            "GNOME shell bridge path:\nEmit `{}`.`{}` with payload `{}` from extension/API bridge.\nManual emit test:\ngdbus emit --session --object-path /io/github/hop/Hotkeyd --signal {}.{} {}\nFallback: ~/.local/bin/hop-hotkeyd trigger --socket {}",
            GNOME_BRIDGE_INTERFACE, GNOME_BRIDGE_MEMBER, KDE_TOGGLE_ACTION,
            GNOME_BRIDGE_INTERFACE, GNOME_BRIDGE_MEMBER, KDE_TOGGLE_ACTION, control_socket
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
    if compositor == "hyprland" {
        let signature = env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap_or_default();
        if signature.trim().is_empty() {
            eprintln!("hyprland compositor detected but HYPRLAND_INSTANCE_SIGNATURE is empty; using fallback mode");
            return run_wayland_fallback();
        }
        return run_hyprland_daemon_loop(default_control_socket_path(), signature);
    }
    if compositor == "kde" {
        return run_kde_dbus_bridge_loop(default_control_socket_path());
    }
    if compositor == "gnome" {
        return run_gnome_shell_bridge_loop(default_control_socket_path());
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

fn run_hyprland_daemon_loop(control_socket_path: String, signature: String) -> Result<(), String> {
    let mut attempt: u32 = 0;
    loop {
        match run_hyprland_event_loop(control_socket_path.clone(), signature.clone()) {
            Ok(()) => return Ok(()),
            Err(error) => {
                let delay = reconnect_backoff_secs(attempt);
                eprintln!(
                    "hyprland event loop error: {}. reconnecting in {}s",
                    error, delay
                );
                thread::sleep(Duration::from_secs(delay));
                attempt = attempt.saturating_add(1);
            }
        }
    }
}

fn run_kde_dbus_bridge_loop(control_socket_path: String) -> Result<(), String> {
    let mut attempt: u32 = 0;
    loop {
        match run_kde_dbus_monitor_once(control_socket_path.clone()) {
            Ok(()) => return Ok(()),
            Err(error) => {
                let delay = reconnect_backoff_secs(attempt);
                eprintln!(
                    "kde dbus bridge loop error: {}. reconnecting in {}s",
                    error, delay
                );
                thread::sleep(Duration::from_secs(delay));
                attempt = attempt.saturating_add(1);
            }
        }
    }
}

fn run_gnome_shell_bridge_loop(control_socket_path: String) -> Result<(), String> {
    let mut attempt: u32 = 0;
    loop {
        match run_gnome_dbus_monitor_once(control_socket_path.clone()) {
            Ok(()) => return Ok(()),
            Err(error) => {
                let delay = reconnect_backoff_secs(attempt);
                eprintln!(
                    "gnome dbus bridge loop error: {}. reconnecting in {}s",
                    error, delay
                );
                thread::sleep(Duration::from_secs(delay));
                attempt = attempt.saturating_add(1);
            }
        }
    }
}

fn run_kde_dbus_monitor_once(control_socket_path: String) -> Result<(), String> {
    let mut child = ProcessCommand::new("dbus-monitor")
        .args([
            "--session",
            "type='signal',interface='org.kde.kglobalaccel.Component',member='globalShortcutPressed'",
        ])
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to start dbus-monitor for KDE: {}", error))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "dbus-monitor stdout unavailable".to_string())?;
    let mut last_toggle_at: Option<Instant> = None;
    for line in BufReader::new(stdout).lines() {
        let line = line.map_err(|error| format!("dbus-monitor read failed: {}", error))?;
        if parse_kde_dbus_toggle_line(&line) {
            let now = Instant::now();
            if should_emit_toggle(now, &mut last_toggle_at, Duration::from_millis(220)) {
                if let Err(error) = send_toggle(&control_socket_path, "hotkey-kde-dbus") {
                    eprintln!("toggle send failed: {}", error);
                }
            }
        }
    }
    let status = child
        .wait()
        .map_err(|error| format!("dbus-monitor wait failed: {}", error))?;
    Err(format!("kde dbus bridge exited: {}", status))
}

fn run_gnome_dbus_monitor_once(control_socket_path: String) -> Result<(), String> {
    let signal_match = format!(
        "type='signal',interface='{}',member='{}'",
        GNOME_BRIDGE_INTERFACE, GNOME_BRIDGE_MEMBER
    );
    let mut child = ProcessCommand::new("dbus-monitor")
        .args(["--session", signal_match.as_str()])
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to start dbus-monitor for GNOME: {}", error))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "dbus-monitor stdout unavailable".to_string())?;
    let mut last_toggle_at: Option<Instant> = None;
    let mut awaiting_toggle_payload = false;
    for line in BufReader::new(stdout).lines() {
        let line = line.map_err(|error| format!("dbus-monitor read failed: {}", error))?;
        if parse_gnome_dbus_signal_header_line(&line) {
            awaiting_toggle_payload = true;
            continue;
        }
        if awaiting_toggle_payload && parse_gnome_dbus_toggle_line(&line) {
            let now = Instant::now();
            if should_emit_toggle(now, &mut last_toggle_at, Duration::from_millis(220)) {
                if let Err(error) = send_toggle(&control_socket_path, "hotkey-gnome-bridge") {
                    eprintln!("toggle send failed: {}", error);
                }
            }
            awaiting_toggle_payload = false;
            continue;
        }
        if line.trim_start().starts_with("signal ") {
            awaiting_toggle_payload = false;
        }
    }
    let status = child
        .wait()
        .map_err(|error| format!("dbus-monitor wait failed: {}", error))?;
    Err(format!("gnome dbus bridge exited: {}", status))
}

fn run_hyprland_event_loop(control_socket_path: String, signature: String) -> Result<(), String> {
    let runtime_dir = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    let hypr_socket = format!("{}/hypr/{}/.socket2.sock", runtime_dir, signature);
    let stream = UnixStream::connect(&hypr_socket)
        .map_err(|error| format!("hyprland socket connect failed: {}", error))?;
    let mut reader = BufReader::new(stream);

    let mut last_toggle_at: Option<Instant> = None;
    loop {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .map_err(|error| format!("hyprland socket read failed: {}", error))?;
        if read == 0 {
            return Err("hyprland socket closed".to_string());
        }
        if parse_hyprland_toggle_event_line(&line) {
            let now = Instant::now();
            if should_emit_toggle(now, &mut last_toggle_at, Duration::from_millis(220)) {
                if let Err(error) = send_toggle(&control_socket_path, "hotkey-hyprland-event") {
                    eprintln!("toggle send failed: {}", error);
                }
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

fn parse_hyprland_toggle_event_line(line: &str) -> bool {
    let normalized = line.trim();
    normalized == format!("custom>>{}", HYPRLAND_TOGGLE_EVENT)
}

fn parse_kde_dbus_toggle_line(line: &str) -> bool {
    let normalized = line.trim();
    normalized.contains("string \"hop-launcher-toggle\"")
}

fn parse_gnome_dbus_toggle_line(line: &str) -> bool {
    let normalized = line.trim();
    normalized.contains("string \"hop-launcher-toggle\"")
}

fn parse_gnome_dbus_signal_header_line(line: &str) -> bool {
    let normalized = line.trim();
    normalized.starts_with("signal ")
        && normalized.contains("interface=io.github.hop.Hotkeyd")
        && normalized.contains("member=Toggle")
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
    use std::io::Write;
    use std::os::unix::net::UnixListener;
    use std::time::{SystemTime, UNIX_EPOCH};

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
        assert_eq!(
            command,
            Command::Status {
                socket_path: default_control_socket_path(),
                compositor: None,
            }
        );
    }

    #[test]
    fn parse_setup_shortcut_defaults() {
        let args = vec!["hop-hotkeyd".to_string(), "setup-shortcut".to_string()];
        let command = parse_command(&args).expect("setup-shortcut should parse");
        assert_eq!(
            command,
            Command::SetupShortcut {
                compositor: None,
                shortcut: "<Super>space".to_string(),
                shortcut_explicit: false,
                socket_path: default_control_socket_path(),
                dry_run: false,
            }
        );
    }

    #[test]
    fn parse_setup_shortcut_with_options() {
        let args = vec![
            "hop-hotkeyd".to_string(),
            "setup-shortcut".to_string(),
            "--compositor".to_string(),
            "gnome".to_string(),
            "--shortcut".to_string(),
            "<Super>Return".to_string(),
            "--socket".to_string(),
            "/tmp/hop.sock".to_string(),
            "--dry-run".to_string(),
        ];
        let command = parse_command(&args).expect("setup-shortcut should parse");
        assert_eq!(
            command,
            Command::SetupShortcut {
                compositor: Some("gnome".to_string()),
                shortcut: "<Super>Return".to_string(),
                shortcut_explicit: true,
                socket_path: "/tmp/hop.sock".to_string(),
                dry_run: true,
            }
        );
    }

    #[test]
    fn parse_config_get_subcommand() {
        let args = vec![
            "hop-hotkeyd".to_string(),
            "config".to_string(),
            "get".to_string(),
        ];
        let command = parse_command(&args).expect("config get should parse");
        assert_eq!(command, Command::ConfigGet);
    }

    #[test]
    fn parse_config_set_subcommand() {
        let args = vec![
            "hop-hotkeyd".to_string(),
            "config".to_string(),
            "set".to_string(),
            "--shortcut".to_string(),
            "<Super>Return".to_string(),
        ];
        let command = parse_command(&args).expect("config set should parse");
        assert_eq!(
            command,
            Command::ConfigSet {
                shortcut: "<Super>Return".to_string(),
            }
        );
    }

    #[test]
    fn shortcut_config_round_trip_persists_value() {
        let path = unique_temp_path("hotkey-config-roundtrip");
        let written = store_configured_shortcut(&path, "<Super>Return").expect("store shortcut");
        assert_eq!(written, "<Super>Return");
        let loaded = load_configured_shortcut(&path).expect("load shortcut");
        assert_eq!(loaded, "<Super>Return");
        fs::remove_file(&path).ok();
    }

    #[test]
    fn shortcut_config_set_rejects_invalid_and_preserves_previous() {
        let path = unique_temp_path("hotkey-config-invalid");
        store_configured_shortcut(&path, "<Super>space").expect("seed config");
        let result = store_configured_shortcut(&path, "bad");
        assert!(result.is_err());
        let loaded = load_configured_shortcut(&path).expect("load previous");
        assert_eq!(loaded, "<Super>space");
        fs::remove_file(&path).ok();
    }

    #[test]
    fn append_gsettings_array_path_handles_empty_array_forms() {
        let path =
            "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/hop-launcher-hotkeyd/";
        assert_eq!(
            append_gsettings_array_path("@as []", path),
            format!("['{}']", path)
        );
        assert_eq!(
            append_gsettings_array_path("[]", path),
            format!("['{}']", path)
        );
    }

    #[test]
    fn append_gsettings_array_path_appends_when_missing() {
        let path =
            "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/hop-launcher-hotkeyd/";
        let existing = "['/org/example/a/']";
        let updated = append_gsettings_array_path(existing, path);
        assert!(updated.contains("'/org/example/a/'"));
        assert!(updated.contains(path));
    }

    #[test]
    fn append_gsettings_array_path_is_idempotent() {
        let path =
            "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/hop-launcher-hotkeyd/";
        let existing = format!("['{}']", path);
        let updated = append_gsettings_array_path(&existing, path);
        assert_eq!(updated, existing);
    }

    #[test]
    fn parse_status_subcommand_with_custom_socket() {
        let args = vec![
            "hop-hotkeyd".to_string(),
            "status".to_string(),
            "--socket".to_string(),
            "/tmp/status.sock".to_string(),
        ];
        let command = parse_command(&args).expect("status should parse");
        assert_eq!(
            command,
            Command::Status {
                socket_path: "/tmp/status.sock".to_string(),
                compositor: None,
            }
        );
    }

    #[test]
    fn parse_status_subcommand_with_compositor_override() {
        let args = vec![
            "hop-hotkeyd".to_string(),
            "status".to_string(),
            "--compositor".to_string(),
            "hyprland".to_string(),
        ];
        let command = parse_command(&args).expect("status should parse");
        assert_eq!(
            command,
            Command::Status {
                socket_path: default_control_socket_path(),
                compositor: Some("hyprland".to_string()),
            }
        );
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
                interval_ms: 250,
                compositor: None,
                strict: false
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
                interval_ms: 100,
                compositor: None
                ,
                strict: false
            }
        );
    }

    #[test]
    fn parse_doctor_subcommand_with_compositor_override() {
        let args = vec![
            "hop-hotkeyd".to_string(),
            "doctor".to_string(),
            "--compositor".to_string(),
            "kde".to_string(),
        ];
        let command = parse_command(&args).expect("doctor should parse");
        assert_eq!(
            command,
            Command::Doctor {
                socket_path: default_control_socket_path(),
                wait_seconds: 0,
                interval_ms: 250,
                compositor: Some("kde".to_string()),
                strict: false,
            }
        );
    }

    #[test]
    fn parse_doctor_subcommand_with_strict_flag() {
        let args = vec![
            "hop-hotkeyd".to_string(),
            "doctor".to_string(),
            "--strict".to_string(),
        ];
        let command = parse_command(&args).expect("doctor should parse");
        assert_eq!(
            command,
            Command::Doctor {
                socket_path: default_control_socket_path(),
                wait_seconds: 0,
                interval_ms: 250,
                compositor: None,
                strict: true,
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
                compositor: Some("sway".to_string()),
                socket_path: default_control_socket_path()
            }
        );
    }

    #[test]
    fn parse_print_bindings_with_custom_socket() {
        let args = vec![
            "hop-hotkeyd".to_string(),
            "print-bindings".to_string(),
            "--socket".to_string(),
            "/tmp/custom-control.sock".to_string(),
        ];
        let result = parse_command(&args).expect("print-bindings should parse");
        assert_eq!(
            result,
            Command::PrintBindings {
                compositor: None,
                socket_path: "/tmp/custom-control.sock".to_string(),
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
        let payload = build_status_payload("x11", None);
        assert_eq!(payload["backend"], "x11");
        assert_eq!(payload["global_hotkey_supported"], true);
        assert_eq!(payload["configured_shortcut"], "<Super>space");
        assert_eq!(payload["applied"], true);
        assert_eq!(payload["requires_manual_step"], false);
        assert_eq!(payload["warnings"], json!([]));
    }

    #[test]
    fn status_payload_honors_wayland_compositor_override() {
        let payload = build_status_payload("wayland", Some("kde"));
        assert_eq!(payload["backend"], "wayland");
        assert_eq!(payload["wayland_compositor"], "kde");
    }

    #[test]
    fn status_payload_reports_wayland_fallback() {
        let payload = build_wayland_status_payload("wayland", "unknown", "", "", "/tmp");
        assert_eq!(payload["backend"], "wayland");
        assert_eq!(payload["global_hotkey_supported"], false);
        assert_eq!(payload["wayland_backend_mode"], "fallback");
        assert_eq!(payload["applied"], false);
        assert_eq!(payload["requires_manual_step"], true);
    }

    #[test]
    fn doctor_strict_fails_for_unapplied_backend_state() {
        let summary = ProbeSummary {
            reachable: true,
            ping_supported: true,
            status: "healthy",
            error: None,
        };
        let backend = json!({
            "applied": false
        });
        assert!(should_fail_doctor_strict(true, &summary, &backend));
    }

    #[test]
    fn status_payload_reports_kde_dbus_bridge_mode() {
        let payload = build_wayland_status_payload("wayland", "kde", "", "", "/tmp");
        assert_eq!(payload["backend"], "wayland");
        assert_eq!(payload["wayland_backend_mode"], "kde_dbus_bridge");
        assert!(payload["recommended_binding"]
            .as_str()
            .unwrap_or_default()
            .contains("qdbus"));
    }

    #[test]
    fn status_payload_reports_gnome_shell_bridge_mode() {
        let payload = build_wayland_status_payload("wayland", "gnome", "", "", "/tmp");
        assert_eq!(payload["backend"], "wayland");
        assert_eq!(payload["wayland_backend_mode"], "gnome_shell_bridge");
        assert!(payload["recommended_binding"]
            .as_str()
            .unwrap_or_default()
            .contains("gdbus"));
    }

    #[test]
    fn status_payload_reports_sway_native_mode_when_socket_present() {
        let base = unique_temp_path("sway-status");
        fs::create_dir_all(&base).expect("create temp dir");
        let socket_path = base.join("sway.sock");
        let _listener = UnixListener::bind(&socket_path).expect("bind socket");
        let payload = build_wayland_status_payload(
            "wayland",
            "sway",
            socket_path.to_str().unwrap_or_default(),
            "",
            base.to_str().unwrap_or_default(),
        );
        assert_eq!(payload["backend"], "wayland");
        assert_eq!(payload["global_hotkey_supported"], true);
        assert_eq!(payload["wayland_backend_mode"], "sway_tick");
        assert!(payload["recommended_binding"]
            .as_str()
            .unwrap_or_default()
            .contains("send_tick"));
        fs::remove_file(&socket_path).ok();
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn status_payload_reports_hyprland_native_mode_when_signature_present() {
        let base = unique_temp_path("hypr-status");
        let hypr_dir = base.join("hypr").join("abc123");
        fs::create_dir_all(&hypr_dir).expect("create hypr dir");
        let socket_path = hypr_dir.join(".socket2.sock");
        let _listener = UnixListener::bind(&socket_path).expect("bind socket");
        let payload = build_wayland_status_payload(
            "wayland",
            "hyprland",
            "",
            "abc123",
            base.to_str().unwrap_or_default(),
        );
        assert_eq!(payload["backend"], "wayland");
        assert_eq!(payload["global_hotkey_supported"], true);
        assert_eq!(payload["wayland_backend_mode"], "hyprland_event");
        assert!(payload["recommended_binding"]
            .as_str()
            .unwrap_or_default()
            .contains("dispatch event"));
        fs::remove_file(&socket_path).ok();
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn status_payload_reports_sway_missing_socket_error() {
        let payload = build_wayland_status_payload(
            "wayland",
            "sway",
            "/tmp/does-not-exist.sock",
            "",
            "/tmp",
        );
        assert_eq!(payload["global_hotkey_supported"], false);
        assert_eq!(payload["wayland_backend_mode"], "fallback");
        assert!(payload["recommended_binding"]
            .as_str()
            .unwrap_or_default()
            .contains("hop-hotkeyd trigger --socket /tmp/does-not-exist.sock"));
        assert!(payload["native_backend_error"]
            .as_str()
            .unwrap_or_default()
            .contains("socket metadata check failed"));
    }

    #[test]
    fn recommended_wayland_binding_uses_fallback_socket_when_native_unavailable() {
        let binding = recommended_wayland_binding(
            "unknown",
            "fallback",
            Some("/tmp/hop-launcher-control.sock"),
        )
        .expect("fallback should provide command");
        assert!(binding.contains("hop-hotkeyd trigger --socket /tmp/hop-launcher-control.sock"));
    }

    #[test]
    fn add_control_probe_fields_attaches_socket_health() {
        let mut payload = json!({"backend":"x11"});
        add_control_probe_fields(
            &mut payload,
            "/tmp/control.sock",
            ProbeSummary {
                reachable: false,
                ping_supported: false,
                status: "unreachable",
                error: Some("missing socket".to_string()),
            },
        );
        assert_eq!(payload["control_socket_path"], "/tmp/control.sock");
        assert_eq!(payload["control_socket_reachable"], false);
        assert_eq!(payload["control_ping_supported"], false);
        assert_eq!(payload["control_probe_status"], "unreachable");
        assert_eq!(payload["control_probe_error"], "missing socket");
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
    fn sway_message_roundtrip_preserves_type_and_payload() {
        let (mut writer, mut reader) =
            std::os::unix::net::UnixStream::pair().expect("create socket pair");
        write_sway_message(&mut writer, SWAY_MSG_SUBSCRIBE, br#"["tick"]"#)
            .expect("write frame");
        let (msg_type, payload) = read_sway_message(&mut reader).expect("read frame");
        assert_eq!(msg_type, SWAY_MSG_SUBSCRIBE);
        assert_eq!(payload, br#"["tick"]"#);
    }

    #[test]
    fn sway_message_rejects_invalid_magic_header() {
        let (mut writer, mut reader) =
            std::os::unix::net::UnixStream::pair().expect("create socket pair");
        writer
            .write_all(b"badmagicheader")
            .expect("write invalid header");
        let result = read_sway_message(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn parse_hyprland_event_line_matches_toggle_marker() {
        assert!(parse_hyprland_toggle_event_line(
            "custom>>hop-launcher-toggle\n"
        ));
        assert!(!parse_hyprland_toggle_event_line("custom>>other\n"));
    }

    #[test]
    fn parse_kde_dbus_monitor_line_matches_toggle_marker() {
        assert!(parse_kde_dbus_toggle_line(
            "   string \"hop-launcher-toggle\"\n"
        ));
        assert!(!parse_kde_dbus_toggle_line("   string \"other-action\"\n"));
    }

    #[test]
    fn parse_gnome_dbus_monitor_line_matches_toggle_marker() {
        assert!(parse_gnome_dbus_toggle_line(
            "   string \"hop-launcher-toggle\"\n"
        ));
        assert!(!parse_gnome_dbus_toggle_line("   string \"other\"\n"));
    }

    #[test]
    fn parse_gnome_dbus_signal_header_matches_bridge_signal() {
        assert!(parse_gnome_dbus_signal_header_line(
            "signal time=1 sender=:1.2 -> destination=(null destination) serial=5 path=/io/github/hop/Hotkeyd; interface=io.github.hop.Hotkeyd; member=Toggle"
        ));
        assert!(!parse_gnome_dbus_signal_header_line(
            "signal time=1 sender=:1.2 -> destination=(null destination) serial=5 path=/io/github/hop/Hotkeyd; interface=io.github.hop.Hotkeyd; member=Other"
        ));
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
        assert!(wayland_next_step_hint("gnome").contains("io.github.hop.Hotkeyd.Toggle"));
        assert!(wayland_next_step_hint("kde").contains("KGlobalAccel"));
        assert!(wayland_next_step_hint("sway").contains("send_tick"));
        assert!(wayland_next_step_hint("hyprland").contains("dispatch event"));
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
        assert!(snippet.contains("qdbus"));
        assert!(snippet.contains("hop-launcher-toggle"));
    }

    #[test]
    fn binding_payload_for_hyprland_includes_event_and_fallback_command() {
        let payload = build_binding_snippet_payload("hyprland", "/tmp/hop.sock");
        let snippet = payload["snippet"].as_str().unwrap_or_default();
        assert!(snippet.contains("dispatch event"));
        assert!(snippet.contains("hop-launcher-toggle"));
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

    fn unique_temp_path(prefix: &str) -> std::path::PathBuf {
        let mut path = env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time moved backwards")
            .as_nanos();
        let stamp = format!(
            "{}-{}-{}",
            prefix,
            std::process::id(),
            nanos
        );
        path.push(stamp);
        path
    }
}
