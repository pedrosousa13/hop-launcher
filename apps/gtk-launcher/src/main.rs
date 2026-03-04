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
use std::path::{Path, PathBuf};

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
    search_query_mode,
    selected_result_id, start_visible_on_launch, toggle_accelerator, ControlMethod, LauncherResult,
    QueryState,
};

fn main() {
    run();
}

#[cfg(feature = "gtk_ui")]
#[derive(Clone, Debug)]
struct LauncherUiSettings {
    overlay_opacity_percent: i32,
    max_results: u32,
    frameless_window: bool,
}

#[cfg(feature = "gtk_ui")]
impl Default for LauncherUiSettings {
    fn default() -> Self {
        Self {
            overlay_opacity_percent: 94,
            max_results: 12,
            frameless_window: true,
        }
    }
}

#[cfg(feature = "gtk_ui")]
fn settings_file_path() -> Option<PathBuf> {
    if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(config_home).join("hop-launcher-gtk/settings.json"));
    }
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".config/hop-launcher-gtk/settings.json"))
}

#[cfg(feature = "gtk_ui")]
fn load_ui_settings() -> LauncherUiSettings {
    let Some(path) = settings_file_path() else {
        return LauncherUiSettings::default();
    };
    let Ok(raw) = fs::read_to_string(path) else {
        return LauncherUiSettings::default();
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return LauncherUiSettings::default();
    };

    let default = LauncherUiSettings::default();
    let overlay = json
        .get("overlay_opacity_percent")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(70, 100) as i32)
        .unwrap_or(default.overlay_opacity_percent);
    let max_results = json
        .get("max_results")
        .and_then(serde_json::Value::as_u64)
        .map(|v| v.clamp(4, 24) as u32)
        .unwrap_or(default.max_results);
    let frameless = json
        .get("frameless_window")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default.frameless_window);

    LauncherUiSettings {
        overlay_opacity_percent: overlay,
        max_results,
        frameless_window: frameless,
    }
}

#[cfg(feature = "gtk_ui")]
fn save_ui_settings(settings: &LauncherUiSettings) -> Result<(), String> {
    let Some(path) = settings_file_path() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create settings dir failed: {error}"))?;
    }
    let payload = serde_json::json!({
        "overlay_opacity_percent": settings.overlay_opacity_percent,
        "max_results": settings.max_results,
        "frameless_window": settings.frameless_window,
    });
    let encoded = serde_json::to_string_pretty(&payload)
        .map_err(|error| format!("encode settings failed: {error}"))?;
    fs::write(path, encoded).map_err(|error| format!("write settings failed: {error}"))
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
        let ui_settings = Rc::new(RefCell::new(load_ui_settings()));

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Hop Launcher")
            .default_width(900)
            .default_height(560)
            .build();
        {
            let settings = ui_settings.borrow().clone();
            window.set_opacity(settings.overlay_opacity_percent as f64 / 100.0);
            window.set_decorated(!settings.frameless_window);
        }
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
            .hexpand(true)
            .build();
        title.add_css_class("title-2");
        title.add_css_class("hop-launcher-title");
        let settings_button = gtk::Button::from_icon_name("preferences-system-symbolic");
        settings_button.add_css_class("flat");
        settings_button.add_css_class("hop-launcher-settings-button");
        settings_button.set_tooltip_text(Some("Launcher Settings"));
        let header = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .build();
        header.append(&title);
        header.append(&settings_button);
        let hints = build_mode_hints();

        let entry = gtk::Entry::builder()
            .placeholder_text("Search apps, windows, files, recents, settings, weather, timezone, emoji, calculations, currency…")
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

        content.append(&header);
        content.append(&hints);
        content.append(&entry);
        content.append(&status);
        content.append(&list_scroller);
        window.set_content(Some(&content));

        {
            let app = app.clone();
            let parent = window.clone();
            let settings = ui_settings.clone();
            settings_button.connect_clicked(move |_| {
                open_settings_window(&app, &parent, settings.clone());
            });
        }

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
            let ui_settings = ui_settings.clone();
            entry.connect_changed(move |entry| {
                let query = entry.text().to_string();
                let max_results = ui_settings.borrow().max_results;
                refresh_results(&list, &status, &results, &socket_path, max_results, &query);
            });
        }

        {
            let list = list.clone();
            let status = status.clone();
            let results = results.clone();
            let socket_path = socket_path.clone();
            let window = window.clone();
            let entry = entry.clone();
            entry.clone().connect_activate(move |_| {
                if let Some(row) = list.selected_row() {
                    let index = row.index() as usize;
                    if let Some(result_id) = selected_result_id(&results.borrow(), index) {
                        if let Err(error) = execute(&socket_path, result_id) {
                            status.set_text(&render_status_text(QueryState::Error(format!(
                                "execute failed: {error}"
                            ))));
                        } else {
                            status.set_text(&render_status_text(QueryState::Executed));
                            window.hide();
                            entry.set_text("");
                        }
                    }
                }
            });
        }

        {
            let list = list.clone();
            let window = window.clone();
            let app = app.clone();
            let ui_settings = ui_settings.clone();
            let controller = gtk::EventControllerKey::new();
            controller.connect_key_pressed(move |_, key, _, state| {
                let is_ctrl = state.contains(gtk::gdk::ModifierType::CONTROL_MASK);
                let is_shift = state.contains(gtk::gdk::ModifierType::SHIFT_MASK);
                match key {
                    gtk::gdk::Key::Down => {
                        move_selection(&list, 1);
                        true.into()
                    }
                    gtk::gdk::Key::Up => {
                        move_selection(&list, -1);
                        true.into()
                    }
                    gtk::gdk::Key::j if is_ctrl => {
                        move_selection(&list, 1);
                        true.into()
                    }
                    gtk::gdk::Key::k if is_ctrl => {
                        move_selection(&list, -1);
                        true.into()
                    }
                    gtk::gdk::Key::Tab if is_shift => {
                        move_selection(&list, -1);
                        true.into()
                    }
                    gtk::gdk::Key::ISO_Left_Tab => {
                        move_selection(&list, -1);
                        true.into()
                    }
                    gtk::gdk::Key::Tab => {
                        move_selection(&list, 1);
                        true.into()
                    }
                    gtk::gdk::Key::Escape => {
                        window.hide();
                        true.into()
                    }
                    gtk::gdk::Key::comma if is_ctrl => {
                        open_settings_window(&app, &window, ui_settings.clone());
                        true.into()
                    }
                    _ => false.into(),
                }
            });
            entry.add_controller(controller);
        }

        {
            let list = list.clone();
            let status = status.clone();
            let results = results.clone();
            let socket_path = socket_path.clone();
            let window = window.clone();
            let entry = entry.clone();
            list.connect_row_activated(move |_, row| {
                let index = row.index() as usize;
                if let Some(result_id) = selected_result_id(&results.borrow(), index) {
                    if let Err(error) = execute(&socket_path, result_id) {
                        status.set_text(&render_status_text(QueryState::Error(format!(
                            "execute failed: {error}"
                        ))));
                    } else {
                        status.set_text(&render_status_text(QueryState::Executed));
                        window.hide();
                        entry.set_text("");
                    }
                }
            });
        }

        refresh_results(
            &list,
            &status,
            &results,
            &socket_path,
            ui_settings.borrow().max_results,
            "",
        );

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
  border: 1px solid alpha(@accent_bg_color, 0.20);
  background: linear-gradient(160deg, rgba(20, 26, 34, 0.62), rgba(17, 21, 30, 0.56));
}

.hop-launcher-title {
  letter-spacing: 0.02em;
}

.hop-launcher-settings-button {
  min-width: 32px;
  min-height: 32px;
}

.hop-launcher-hints {
  margin-bottom: 2px;
}

.hop-launcher-hint-chip {
  padding: 3px 8px;
  border-radius: 999px;
  border: 1px solid alpha(@headerbar_border_color, 0.35);
  background: alpha(@view_bg_color, 0.22);
  font-size: 0.78em;
}

.hop-launcher-entry {
  min-height: 44px;
}

.hop-launcher-status {
  margin-bottom: 2px;
}

.hop-launcher-subtitle {
  font-size: 0.92em;
}

.hop-launcher-scroll {
  border-radius: 12px;
  border: 1px solid alpha(@headerbar_border_color, 0.35);
  background: alpha(@view_bg_color, 0.48);
}

.hop-launcher-list row {
  margin: 1px 4px;
  border-radius: 10px;
  background: alpha(@view_bg_color, 0.20);
  transition: 130ms ease;
}

.hop-launcher-list row:hover {
  background: alpha(@view_bg_color, 0.30);
}

.hop-launcher-list row:selected {
  background: alpha(@accent_bg_color, 0.40);
}

.hop-launcher-kind-badge {
  min-width: 60px;
  padding: 2px 8px;
  border-radius: 999px;
  border: 1px solid alpha(@accent_bg_color, 0.35);
  background: alpha(@accent_bg_color, 0.16);
  font-size: 0.72em;
  font-weight: 600;
  letter-spacing: 0.04em;
}

.hop-launcher-action-hint {
  font-size: 0.8em;
}

.hop-launcher-row-body {
  min-height: 44px;
}

.hop-launcher-title-text {
  font-weight: 600;
}

.hop-launcher-icon {
  margin-end: 2px;
}

.hop-launcher-icon-app {
  padding: 2px;
  border-radius: 9px;
  background: alpha(@view_bg_color, 0.20);
}

.hop-launcher-icon-window {
  padding: 1px;
  border-radius: 7px;
  background: alpha(@view_bg_color, 0.12);
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
fn open_settings_window(
    app: &adw::Application,
    parent: &adw::ApplicationWindow,
    settings: Rc<RefCell<LauncherUiSettings>>,
) {
    let prefs = adw::PreferencesWindow::builder()
        .application(app)
        .title("Hop Launcher Settings")
        .default_width(560)
        .default_height(420)
        .transient_for(parent)
        .modal(true)
        .build();

    let page = adw::PreferencesPage::new();
    let appearance = adw::PreferencesGroup::builder()
        .title("Appearance")
        .description("Tune launcher translucency and window chrome.")
        .build();
    let behavior = adw::PreferencesGroup::builder()
        .title("Behavior")
        .description("Result density and interaction defaults.")
        .build();

    let opacity_row = adw::ActionRow::builder()
        .title("Launcher translucency (%)")
        .subtitle("Higher values are less transparent.")
        .build();
    let opacity_adjustment = gtk::Adjustment::new(
        settings.borrow().overlay_opacity_percent as f64,
        70.0,
        100.0,
        1.0,
        5.0,
        0.0,
    );
    let opacity_spin = gtk::SpinButton::new(Some(&opacity_adjustment), 1.0, 0);
    opacity_spin.set_valign(gtk::Align::Center);
    opacity_row.add_suffix(&opacity_spin);
    opacity_row.set_activatable_widget(Some(&opacity_spin));
    {
        let settings = settings.clone();
        let parent = parent.clone();
        opacity_spin.connect_value_changed(move |spin| {
            let mut next = settings.borrow().clone();
            next.overlay_opacity_percent = spin.value_as_int().clamp(70, 100);
            parent.set_opacity(next.overlay_opacity_percent as f64 / 100.0);
            if let Err(error) = save_ui_settings(&next) {
                eprintln!("failed to save launcher settings: {error}");
            }
            *settings.borrow_mut() = next;
        });
    }
    appearance.add(&opacity_row);

    let frame_row = adw::ActionRow::builder()
        .title("Frameless launcher window")
        .subtitle("Use overlay-style window without titlebar decorations.")
        .build();
    let frame_switch = gtk::Switch::builder()
        .active(settings.borrow().frameless_window)
        .valign(gtk::Align::Center)
        .build();
    frame_row.add_suffix(&frame_switch);
    frame_row.set_activatable_widget(Some(&frame_switch));
    {
        let settings = settings.clone();
        let parent = parent.clone();
        frame_switch.connect_active_notify(move |toggle| {
            let mut next = settings.borrow().clone();
            next.frameless_window = toggle.is_active();
            parent.set_decorated(!next.frameless_window);
            if let Err(error) = save_ui_settings(&next) {
                eprintln!("failed to save launcher settings: {error}");
            }
            *settings.borrow_mut() = next;
        });
    }
    appearance.add(&frame_row);

    let results_row = adw::ActionRow::builder()
        .title("Max results")
        .subtitle("Maximum number of rows returned from hopd per query.")
        .build();
    let results_adjustment = gtk::Adjustment::new(
        settings.borrow().max_results as f64,
        4.0,
        24.0,
        1.0,
        4.0,
        0.0,
    );
    let results_spin = gtk::SpinButton::new(Some(&results_adjustment), 1.0, 0);
    results_spin.set_valign(gtk::Align::Center);
    results_row.add_suffix(&results_spin);
    results_row.set_activatable_widget(Some(&results_spin));
    {
        let settings = settings.clone();
        results_spin.connect_value_changed(move |spin| {
            let mut next = settings.borrow().clone();
            next.max_results = spin.value_as_int().clamp(4, 24) as u32;
            if let Err(error) = save_ui_settings(&next) {
                eprintln!("failed to save launcher settings: {error}");
            }
            *settings.borrow_mut() = next;
        });
    }
    behavior.add(&results_row);

    page.add(&appearance);
    page.add(&behavior);
    prefs.add(&page);
    prefs.present();
}

#[cfg(feature = "gtk_ui")]
fn build_mode_hints() -> gtk::Box {
    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .build();
    row.add_css_class("hop-launcher-hints");

    for text in [
        "a apps",
        "w windows",
        "f files",
        "r recents",
        "settings",
        "weather",
        "time in",
        "emoji",
        "2+2",
        "12 usd to chf",
    ] {
        let chip = gtk::Label::builder().label(text).build();
        chip.add_css_class("hop-launcher-hint-chip");
        row.append(&chip);
    }

    row
}

#[cfg(feature = "gtk_ui")]
fn refresh_results(
    list: &gtk::ListBox,
    status: &gtk::Label,
    results: &Rc<RefCell<Vec<LauncherResult>>>,
    socket_path: &str,
    max_results: u32,
    query: &str,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    status.set_text(&render_status_text(QueryState::Searching));
    let mode_label = search_query_mode(query).to_ascii_uppercase();
    match search(socket_path, query, max_results) {
        Ok(rows) => {
            results.borrow_mut().clear();
            results.borrow_mut().extend(rows.iter().cloned());
            for row in &rows {
                let icon = build_result_icon(row);
                icon.set_pixel_size(icon_size_for_row(row));
                icon.add_css_class("hop-launcher-icon");
                if row.kind == "app" {
                    icon.add_css_class("hop-launcher-icon-app");
                } else if row.kind == "window" {
                    icon.add_css_class("hop-launcher-icon-window");
                }
                let title = gtk::Label::builder()
                    .xalign(0.0)
                    .label(&row.title)
                    .build();
                title.add_css_class("hop-launcher-title-text");
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
                subtitle.add_css_class("hop-launcher-subtitle");
                let text = gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .spacing(2)
                    .hexpand(true)
                    .build();
                text.append(&title);
                text.append(&subtitle);

                let kind_badge = gtk::Label::builder()
                    .label(row.kind.to_uppercase())
                    .xalign(1.0)
                    .build();
                kind_badge.add_css_class("hop-launcher-kind-badge");

                let action_hint = gtk::Label::builder()
                    .label(action_hint_for_row(row))
                    .xalign(1.0)
                    .build();
                action_hint.add_css_class("dim-label");
                action_hint.add_css_class("hop-launcher-action-hint");

                let meta = gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .spacing(3)
                    .valign(gtk::Align::Center)
                    .build();
                meta.append(&kind_badge);
                meta.append(&action_hint);

                let body = gtk::Box::builder()
                    .orientation(gtk::Orientation::Horizontal)
                    .spacing(10)
                    .margin_top(6)
                    .margin_bottom(6)
                    .margin_start(8)
                    .margin_end(8)
                    .build();
                body.add_css_class("hop-launcher-row-body");
                body.append(&icon);
                body.append(&text);
                body.append(&meta);
                let item_row = gtk::ListBoxRow::new();
                item_row.set_child(Some(&body));
                item_row.add_css_class("hop-launcher-row");
                list.append(&item_row);
            }
            if list.first_child().is_some() {
                if let Some(first) = list.row_at_index(0) {
                    list.select_row(Some(&first));
                }
            }
            if rows.is_empty() {
                status.set_text(&format!("{mode_label} · {}", render_status_text(QueryState::Empty)));
            } else {
                status.set_text(&format!(
                    "{mode_label} · {}",
                    render_status_text(QueryState::Results { count: rows.len() })
                ));
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
        "calculator" => "accessories-calculator-symbolic",
        "currency" => "accessories-calculator-symbolic",
        _ => "system-search-symbolic",
    };
    gtk::Image::from_icon_name(fallback)
}

#[cfg(feature = "gtk_ui")]
fn action_hint_for_row(row: &LauncherResult) -> &'static str {
    match row.kind.as_str() {
        "window" => "Focus",
        "setting" => "Open",
        "weather" | "timezone" | "emoji" | "calculator" | "currency" => "Open",
        _ => "Enter",
    }
}

#[cfg(feature = "gtk_ui")]
fn icon_size_for_row(row: &LauncherResult) -> i32 {
    match row.kind.as_str() {
        "app" => 24,
        "window" => 22,
        "setting" => 21,
        "weather" | "timezone" | "emoji" | "calculator" | "currency" => 21,
        _ => 20,
    }
}

#[cfg(not(feature = "gtk_ui"))]
fn run() {
    println!("hop-launcher-gtk scaffold ready. Rebuild with --features gtk_ui to run the GTK UI.");
}
