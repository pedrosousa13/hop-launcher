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
use std::path::Path;

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
    default_hopd_socket_path, execute, parse_control_request, render_status_text, search,
    selected_result_id, start_visible_on_launch, toggle_accelerator, ControlMethod, LauncherResult,
    QueryState,
};

fn main() {
    run();
}

#[cfg(feature = "gtk_ui")]
fn run() {
    adw::init().expect("failed to initialize libadwaita");
    install_css();

    let app = adw::Application::builder()
        .application_id("app.hoplauncher.gtk")
        .build();

    app.connect_activate(|app| {
        let socket_path = Rc::new(default_hopd_socket_path());
        let results = Rc::new(RefCell::new(Vec::<LauncherResult>::new()));

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Hop Launcher")
            .default_width(900)
            .default_height(560)
            .build();
        window.set_opacity(0.94);
        window.add_css_class("hop-launcher-window");

        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(10)
            .margin_top(20)
            .margin_bottom(20)
            .margin_start(20)
            .margin_end(20)
            .build();
        content.add_css_class("hop-launcher-content");

        let title = gtk::Label::builder()
            .label("Hop Launcher")
            .xalign(0.0)
            .build();
        title.add_css_class("title-2");
        title.add_css_class("hop-launcher-title");

        let entry = gtk::Entry::builder()
            .placeholder_text("Search apps, windows, files, recents, settings, weather, timezone, emoji…")
            .build();
        entry.add_css_class("hop-launcher-entry");
        let status = gtk::Label::builder()
            .xalign(0.0)
            .build();
        status.add_css_class("dim-label");
        status.add_css_class("hop-launcher-status");

        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::Single);
        list.add_css_class("boxed-list");
        list.add_css_class("hop-launcher-list");
        let list_scroller = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .min_content_height(320)
            .build();
        list_scroller.set_child(Some(&list));
        list_scroller.add_css_class("hop-launcher-scroll");

        content.append(&title);
        content.append(&entry);
        content.append(&status);
        content.append(&list_scroller);
        window.set_content(Some(&content));

        let toggle = gio::SimpleAction::new("toggle", None);
        {
            let window = window.clone();
            let entry = entry.clone();
            toggle.connect_activate(move |_, _| {
                toggle_window(&window, &entry);
            });
        }
        app.add_action(&toggle);
        app.set_accels_for_action("app.toggle", &[toggle_accelerator()]);

        let (toggle_tx, toggle_rx) = mpsc::channel::<()>();
        {
            let window = window.clone();
            let entry = entry.clone();
            gtk::glib::timeout_add_local(Duration::from_millis(30), move || {
                while toggle_rx.try_recv().is_ok() {
                    toggle_window(&window, &entry);
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
                    if let Some(result_id) = selected_result_id(&results.borrow(), index) {
                        if let Err(error) = execute(&socket_path, result_id) {
                            status.set_text(&render_status_text(QueryState::Error(format!(
                                "execute failed: {error}"
                            ))));
                        } else {
                            status.set_text(&render_status_text(QueryState::Executed));
                        }
                    }
                }
            });
        }

        {
            let list = list.clone();
            let window = window.clone();
            let controller = gtk::EventControllerKey::new();
            controller.connect_key_pressed(move |_, key, _, _| match key {
                gtk::gdk::Key::Down => {
                    move_selection(&list, 1);
                    true.into()
                }
                gtk::gdk::Key::Up => {
                    move_selection(&list, -1);
                    true.into()
                }
                gtk::gdk::Key::Escape => {
                    window.hide();
                    true.into()
                }
                _ => false.into(),
            });
            entry.add_controller(controller);
        }

        {
            let list = list.clone();
            let status = status.clone();
            let results = results.clone();
            let socket_path = socket_path.clone();
            list.connect_row_activated(move |_, row| {
                let index = row.index() as usize;
                if let Some(result_id) = selected_result_id(&results.borrow(), index) {
                    if let Err(error) = execute(&socket_path, result_id) {
                        status.set_text(&render_status_text(QueryState::Error(format!(
                            "execute failed: {error}"
                        ))));
                    } else {
                        status.set_text(&render_status_text(QueryState::Executed));
                    }
                }
            });
        }

        refresh_results(&list, &status, &results, &socket_path, "");

        if start_visible_on_launch() {
            window.present();
            entry.grab_focus();
        } else {
            window.hide();
        }
    });

    app.run();
}

#[cfg(feature = "gtk_ui")]
fn toggle_window(window: &adw::ApplicationWindow, entry: &gtk::Entry) {
    if window.is_visible() {
        window.hide();
    } else {
        window.present();
        entry.grab_focus();
        entry.set_position(-1);
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
fn install_css() {
    let css = r#"
.hop-launcher-window {
  background: transparent;
}

.hop-launcher-content {
  border-radius: 18px;
  border: 1px solid alpha(@accent_bg_color, 0.22);
  background: linear-gradient(160deg, rgba(20, 26, 34, 0.58), rgba(17, 21, 30, 0.52));
}

.hop-launcher-title {
  letter-spacing: 0.02em;
}

.hop-launcher-entry {
  min-height: 44px;
}

.hop-launcher-status {
  margin-bottom: 2px;
}

.hop-launcher-scroll {
  border-radius: 12px;
  border: 1px solid alpha(@headerbar_border_color, 0.35);
  background: alpha(@view_bg_color, 0.42);
}

.hop-launcher-list row {
  margin: 2px 4px;
  border-radius: 10px;
  background: alpha(@view_bg_color, 0.18);
}

.hop-launcher-list row:selected {
  background: alpha(@accent_bg_color, 0.34);
}
"#;

    let provider = gtk::CssProvider::new();
    provider.load_from_data(css);
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

#[cfg(feature = "gtk_ui")]
fn move_selection(list: &gtk::ListBox, delta: i32) {
    let current = list.selected_row().map(|row| row.index()).unwrap_or(-1);
    let next = if current < 0 {
        0
    } else {
        (current + delta).max(0)
    };
    if let Some(target) = list.row_at_index(next) {
        list.select_row(Some(&target));
    }
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

    status.set_text(&render_status_text(QueryState::Searching));
    match search(socket_path, query, 8) {
        Ok(rows) => {
            results.borrow_mut().clear();
            results.borrow_mut().extend(rows.iter().cloned());
            for row in &rows {
                let icon = build_result_icon(row);
                icon.set_pixel_size(20);
                let title = gtk::Label::builder()
                    .xalign(0.0)
                    .label(&row.title)
                    .build();
                let subtitle_text = if row.subtitle.is_empty() {
                    row.kind.clone()
                } else {
                    format!("{}  ·  {}", row.subtitle, row.kind)
                };
                let subtitle = gtk::Label::builder()
                    .xalign(0.0)
                    .label(&subtitle_text)
                    .build();
                subtitle.add_css_class("dim-label");
                let text = gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .spacing(2)
                    .hexpand(true)
                    .build();
                text.append(&title);
                text.append(&subtitle);
                let body = gtk::Box::builder()
                    .orientation(gtk::Orientation::Horizontal)
                    .spacing(10)
                    .margin_top(6)
                    .margin_bottom(6)
                    .margin_start(8)
                    .margin_end(8)
                    .build();
                body.append(&icon);
                body.append(&text);
                let item_row = gtk::ListBoxRow::new();
                item_row.set_child(Some(&body));
                list.append(&item_row);
            }
            if list.first_child().is_some() {
                if let Some(first) = list.row_at_index(0) {
                    list.select_row(Some(&first));
                }
            }
            if rows.is_empty() {
                status.set_text(&render_status_text(QueryState::Empty));
            } else {
                status.set_text(&render_status_text(QueryState::Results { count: rows.len() }));
            }
        }
        Err(error) => {
            results.borrow_mut().clear();
            status.set_text(&render_status_text(QueryState::Error(format!(
                "hopd unavailable: {error}"
            ))));
        }
    }
}

#[cfg(feature = "gtk_ui")]
fn build_result_icon(row: &LauncherResult) -> gtk::Image {
    let raw = row.icon.trim();
    if !raw.is_empty() {
        if Path::new(raw).is_absolute() {
            return gtk::Image::from_file(raw);
        }
        return gtk::Image::from_icon_name(raw);
    }

    let fallback = match row.kind.as_str() {
        "app" => "application-x-executable-symbolic",
        "window" => "window-symbolic",
        "file" => "text-x-generic-symbolic",
        "recent" => "document-open-recent-symbolic",
        "setting" => "preferences-system-symbolic",
        "weather" => "weather-clear-symbolic",
        "timezone" => "preferences-system-time-symbolic",
        "emoji" => "face-smile-symbolic",
        _ => "system-search-symbolic",
    };
    gtk::Image::from_icon_name(fallback)
}

#[cfg(not(feature = "gtk_ui"))]
fn run() {
    println!("hop-launcher-gtk scaffold ready. Rebuild with --features gtk_ui to run the GTK UI.");
}
