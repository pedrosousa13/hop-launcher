use std::env;
use std::process;

use hop_hotkeyd::{default_control_socket_path, select_backend_mode, send_toggle};

fn usage() -> &'static str {
    "Usage:\n  hop-hotkeyd trigger [--socket <path>]\n"
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return Err(usage().to_string());
    }

    let session_type = env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".to_string());
    let backend = select_backend_mode(&session_type)?;
    eprintln!("hop-hotkeyd backend: {:?}", backend);

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

    send_toggle(&socket_path, "hotkey-trigger")
}

fn main() {
    if let Err(error) = run() {
        eprintln!("hop-hotkeyd: {}", error);
        process::exit(1);
    }
}
