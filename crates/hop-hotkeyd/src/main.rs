use std::env;
use std::process;
use std::thread;
use std::time::Duration;

use hop_hotkeyd::{default_control_socket_path, select_backend_mode, send_toggle};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt, GrabMode, ModMask};
use x11rb::protocol::Event;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Run,
    Trigger { socket_path: String },
}

fn usage() -> &'static str {
    "Usage:\n  hop-hotkeyd                 # run daemon backend mode\n  hop-hotkeyd trigger [--socket <path>]\n"
}

fn parse_command(args: &[String]) -> Result<Command, String> {
    if args.len() < 2 {
        return Ok(Command::Run);
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
        Command::Run => run_daemon_mode(),
    }
}

fn run_daemon_mode() -> Result<(), String> {
    let session_type = env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".to_string());
    match select_backend_mode(&session_type) {
        Ok(backend) => {
            eprintln!("hop-hotkeyd backend: {:?}", backend);
            match backend {
                hop_hotkeyd::BackendMode::X11 => run_x11_hotkey_loop(default_control_socket_path()),
                hop_hotkeyd::BackendMode::Wayland => run_wayland_fallback(),
            }
        }
        Err(error) => {
            eprintln!("hop-hotkeyd backend selection failed: {}", error);
            run_wayland_fallback()
        }
    }
}

fn run_wayland_fallback() -> Result<(), String> {
    eprintln!("Wayland fallback active: use `hop-hotkeyd trigger` until native Wayland capture is implemented.");
    loop {
        thread::sleep(Duration::from_secs(30));
    }
}

fn run_x11_hotkey_loop(socket_path: String) -> Result<(), String> {
    let (conn, screen_num) =
        x11rb::connect(None).map_err(|error| format!("x11 connect failed: {}", error))?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    let keycodes = find_toggle_keycodes(&conn)?;
    if keycodes.is_empty() {
        return Err("no keycode found for ampersand/7".to_string());
    }

    for keycode in keycodes {
        conn.grab_key(
            false,
            root,
            ModMask::CONTROL | ModMask::SHIFT,
            keycode,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
        )
        .map_err(|error| format!("x11 grab key failed: {}", error))?;
    }
    conn.flush()
        .map_err(|error| format!("x11 flush failed: {}", error))?;

    loop {
        let event = conn
            .wait_for_event()
            .map_err(|error| format!("x11 wait event failed: {}", error))?;
        if let Event::KeyPress(_) = event {
            if let Err(error) = send_toggle(&socket_path, "hotkey-x11") {
                eprintln!("toggle send failed: {}", error);
            }
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
}
