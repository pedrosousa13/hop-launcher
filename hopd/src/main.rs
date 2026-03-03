use std::env;
use std::io;
use std::path::Path;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

use hopd::HopdServer;

#[tokio::main]
async fn main() -> io::Result<()> {
    let socket_path = resolve_socket_path();
    if Path::new(&socket_path).exists() {
        let _ = std::fs::remove_file(&socket_path);
    }

    let listener = UnixListener::bind(&socket_path)?;
    let server = std::sync::Arc::new(HopdServer::new());
    eprintln!("hopd listening on {}", socket_path);

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                break;
            }
            accepted = listener.accept() => {
                match accepted {
                    Ok((stream, _)) => {
                        let server = server.clone();
                        tokio::spawn(async move {
                            if let Err(error) = handle_client(stream, server).await {
                                eprintln!("hopd client error: {}", error);
                            }
                        });
                    }
                    Err(error) => eprintln!("hopd accept error: {}", error),
                }
            }
        }
    }

    let _ = std::fs::remove_file(&socket_path);
    Ok(())
}

async fn handle_client(stream: UnixStream, server: std::sync::Arc<HopdServer>) -> io::Result<()> {
    let (read_half, mut write_half) = stream.into_split();
    let mut lines = BufReader::new(read_half).lines();

    while let Some(line) = lines.next_line().await? {
        let response = match server.handle_json_line(&line).await {
            Ok(payload) => payload,
            Err(error) => {
                format!(
                    "{{\"id\":\"\",\"result\":null,\"error\":{{\"code\":-32700,\"message\":\"{}\"}}}}",
                    error
                )
            }
        };

        write_half.write_all(response.as_bytes()).await?;
        write_half.write_all(b"\n").await?;
    }

    Ok(())
}

fn resolve_socket_path() -> String {
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--socket" {
            return args
                .next()
                .unwrap_or_else(default_socket_path);
        }
    }

    default_socket_path()
}

fn default_socket_path() -> String {
    if let Ok(path) = env::var("HOPD_SOCKET") {
        return path;
    }

    if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
        return format!("{}/hopd.sock", runtime_dir);
    }

    "/tmp/hopd.sock".to_string()
}
