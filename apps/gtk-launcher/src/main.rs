#[cfg(feature = "gtk_ui")]
use std::cell::RefCell;
#[cfg(feature = "gtk_ui")]
use std::fs;
#[cfg(feature = "gtk_ui")]
use std::io::{BufRead, BufReader, Write};
#[cfg(feature = "gtk_ui")]
use std::os::unix::net::{UnixListener, UnixStream};
#[cfg(feature = "gtk_ui")]
use std::rc::Rc;
#[cfg(feature = "gtk_ui")]
use std::sync::mpsc;
#[cfg(feature = "gtk_ui")]
use std::thread;
#[cfg(feature = "gtk_ui")]
use std::time::Duration;

#[cfg(feature = "gtk_ui")]
use gtk::gio;
#[cfg(feature = "gtk_ui")]
use gtk::prelude::*;
#[cfg(feature = "gtk_ui")]
use adw::prelude::*;

#[cfg(feature = "gtk_ui")]
use gtk4 as gtk;

#[cfg(feature = "gtk_ui")]
use libadwaita as adw;
#[cfg(feature = "gtk_ui")]
use hop_launcher_gtk::{
    build_control_error_response, build_control_ok_response, default_control_socket_path,
    default_hopd_socket_path, execute, parse_control_request, search, start_visible_on_launch,
    toggle_accelerator, ControlMethod, LauncherResult,
};

fn main() {
    run();
}

#[cfg(feature = "gtk_ui")]
fn run() {
    adw::init().expect("failed to initialize libadwaita");

    let app = adw::Application::builder()
        .application_id("app.hoplauncher.gtk")
        .build();

    app.connect_activate(|app| {
        let socket_path = Rc::new(default_hopd_socket_path());
        let results = Rc::new(RefCell::new(Vec::<LauncherResult>::new()));

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Hop Launcher")
            .default_width(860)
            .default_height(420)
            .build();

        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .margin_top(16)
            .margin_bottom(16)
            .margin_start(16)
            .margin_end(16)
            .build();

        let entry = gtk::Entry::builder()
            .placeholder_text("Type a utility query (weather/timezone/emoji)…")
            .build();
        let status = gtk::Label::builder()
            .xalign(0.0)
            .build();
        status.add_css_class("dim-label");

        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::Single);
        list.add_css_class("boxed-list");

        content.append(&entry);
        content.append(&status);
        content.append(&list);
        window.set_content(Some(&content));

        let toggle = gio::SimpleAction::new("toggle", None);
        {
            let window = window.clone();
            toggle.connect_activate(move |_, _| {
                toggle_window(&window);
            });
        }
        app.add_action(&toggle);
        app.set_accels_for_action("app.toggle", &[toggle_accelerator()]);

        let (toggle_tx, toggle_rx) = mpsc::channel::<()>();
        {
            let window = window.clone();
            gtk::glib::timeout_add_local(Duration::from_millis(30), move || {
                while toggle_rx.try_recv().is_ok() {
                    toggle_window(&window);
                }
                gtk::glib::ControlFlow::Continue
            });
        }
        if let Err(error) = start_control_listener(default_control_socket_path(), toggle_tx) {
            eprintln!("failed to start control listener: {}", error);
        }

        {
            let list = list.clone();
            let status = status.clone();
            let results = results.clone();
            let socket_path = socket_path.clone();
            entry.connect_changed(move |entry| {
                let query = entry.text().to_string();
                refresh_results(&list, &status, &results, &socket_path, &query);
            });
        }

        {
            let list = list.clone();
            let status = status.clone();
            let results = results.clone();
            let socket_path = socket_path.clone();
            entry.connect_activate(move |_| {
                if let Some(row) = list.selected_row() {
                    let index = row.index() as usize;
                    if let Some(item) = results.borrow().get(index) {
                        if let Err(error) = execute(&socket_path, &item.id) {
                            status.set_text(&format!("Execute failed: {error}"));
                        } else {
                            status.set_text("Executed");
                        }
                    }
                }
            });
        }

        {
            let list = list.clone();
            let status = status.clone();
            let results = results.clone();
            let socket_path = socket_path.clone();
            list.connect_row_activated(move |_, row| {
                let index = row.index() as usize;
                if let Some(item) = results.borrow().get(index) {
                    if let Err(error) = execute(&socket_path, &item.id) {
                        status.set_text(&format!("Execute failed: {error}"));
                    } else {
                        status.set_text("Executed");
                    }
                }
            });
        }

        if start_visible_on_launch() {
            window.present();
        } else {
            window.hide();
        }
    });

    app.run();
}

#[cfg(feature = "gtk_ui")]
fn toggle_window(window: &adw::ApplicationWindow) {
    if window.is_visible() {
        window.hide();
    } else {
        window.present();
    }
}

#[cfg(feature = "gtk_ui")]
fn start_control_listener(socket_path: String, toggle_tx: mpsc::Sender<()>) -> Result<(), String> {
    thread::Builder::new()
        .name("hop-launcher-control".to_string())
        .spawn(move || {
            if fs::remove_file(&socket_path).is_err() && std::path::Path::new(&socket_path).exists() {
                eprintln!("failed to remove stale control socket: {}", socket_path);
            }

            let listener = match UnixListener::bind(&socket_path) {
                Ok(listener) => listener,
                Err(error) => {
                    eprintln!("control socket bind failed at {}: {}", socket_path, error);
                    return;
                }
            };

            for stream_result in listener.incoming() {
                match stream_result {
                    Ok(stream) => {
                        if let Err(error) = handle_control_stream(stream, &toggle_tx) {
                            eprintln!("control request error: {}", error);
                        }
                    }
                    Err(error) => {
                        eprintln!("control socket accept error: {}", error);
                    }
                }
            }
        })
        .map(|_| ())
        .map_err(|error| format!("spawn control listener failed: {}", error))
}

#[cfg(feature = "gtk_ui")]
fn handle_control_stream(mut stream: UnixStream, toggle_tx: &mpsc::Sender<()>) -> Result<(), String> {
    let mut line = String::new();
    let mut reader = BufReader::new(
        stream
            .try_clone()
            .map_err(|error| format!("clone stream failed: {}", error))?,
    );
    reader
        .read_line(&mut line)
        .map_err(|error| format!("read request failed: {}", error))?;

    let payload: serde_json::Value = serde_json::from_str(line.trim())
        .map_err(|error| format!("decode request failed: {}", error))?;

    let request_id = payload
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");

    let response = match parse_control_request(&payload) {
        Ok(request) => match request.method {
            ControlMethod::Toggle => {
                if toggle_tx.send(()).is_err() {
                    build_control_error_response(request_id, -32000, "toggle dispatch failed")
                } else {
                    build_control_ok_response(request_id)
                }
            }
            ControlMethod::Ping => build_control_ok_response(request_id),
        }
        Err(error) => build_control_error_response(request_id, -32601, &error),
    };

    let encoded = serde_json::to_string(&response)
        .map_err(|error| format!("encode response failed: {}", error))?;
    stream
        .write_all(encoded.as_bytes())
        .map_err(|error| format!("write response failed: {}", error))?;
    stream
        .write_all(b"\n")
        .map_err(|error| format!("write newline failed: {}", error))?;

    Ok(())
}

#[cfg(feature = "gtk_ui")]
fn refresh_results(
    list: &gtk::ListBox,
    status: &gtk::Label,
    results: &Rc<RefCell<Vec<LauncherResult>>>,
    socket_path: &str,
    query: &str,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    if query.trim().is_empty() {
        results.borrow_mut().clear();
        status.set_text("Ready");
        return;
    }

    match search(socket_path, query, 8) {
        Ok(rows) => {
            results.borrow_mut().clear();
            results.borrow_mut().extend(rows.iter().cloned());
            for row in rows {
                let label = gtk::Label::builder()
                    .xalign(0.0)
                    .label(format!("{}  ·  {}", row.title, row.kind))
                    .build();
                let item_row = gtk::ListBoxRow::new();
                item_row.set_child(Some(&label));
                list.append(&item_row);
            }
            if list.first_child().is_some() {
                if let Some(first) = list.row_at_index(0) {
                    list.select_row(Some(&first));
                }
            }
            status.set_text("Connected to hopd");
        }
        Err(error) => {
            results.borrow_mut().clear();
            status.set_text(&format!("hopd unavailable: {error}"));
        }
    }
}

#[cfg(not(feature = "gtk_ui"))]
fn run() {
    println!("hop-launcher-gtk scaffold ready. Rebuild with --features gtk_ui to run the GTK UI.");
}
