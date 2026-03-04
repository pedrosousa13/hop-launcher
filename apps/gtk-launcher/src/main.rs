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
use std::time::Instant;
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
    config_set, default_hopd_socket_path, execute, parse_control_request, render_status_text, search,
    start_visible_on_launch, toggle_accelerator, ControlMethod, LauncherResult, QueryState,
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
    feature_apps_enabled: bool,
    feature_windows_enabled: bool,
    feature_files_enabled: bool,
    feature_recents_enabled: bool,
    feature_settings_enabled: bool,
    feature_utility_enabled: bool,
    weight_windows: i32,
    weight_apps: i32,
    weight_recents: i32,
    weight_files: i32,
    weight_emoji: i32,
    weight_utility: i32,
    min_fuzzy_score: i32,
    animations_enabled: bool,
    open_animation_ms: i32,
    close_animation_ms: i32,
    debounce_ms: i32,
    density_mode: String,
    indexed_folders: Vec<String>,
    learning_enabled: bool,
    currency_refresh_enabled: bool,
    currency_rate_ttl_hours: i32,
    web_search_enabled: bool,
    web_search_max_actions: i32,
}

#[cfg(feature = "gtk_ui")]
impl Default for LauncherUiSettings {
    fn default() -> Self {
        Self {
            overlay_opacity_percent: 96,
            max_results: 12,
            frameless_window: true,
            feature_apps_enabled: true,
            feature_windows_enabled: true,
            feature_files_enabled: true,
            feature_recents_enabled: true,
            feature_settings_enabled: true,
            feature_utility_enabled: true,
            weight_windows: 30,
            weight_apps: 20,
            weight_recents: 10,
            weight_files: 12,
            weight_emoji: 8,
            weight_utility: 6,
            min_fuzzy_score: 30,
            animations_enabled: true,
            open_animation_ms: 140,
            close_animation_ms: 110,
            debounce_ms: 15,
            density_mode: "default".to_string(),
            indexed_folders: Vec::new(),
            learning_enabled: true,
            currency_refresh_enabled: true,
            currency_rate_ttl_hours: 12,
            web_search_enabled: true,
            web_search_max_actions: 3,
        }
    }
}

#[cfg(feature = "gtk_ui")]
fn sanitize_density_mode(raw: &str) -> String {
    match raw {
        "compact" | "comfortable" | "default" => raw.to_string(),
        _ => "default".to_string(),
    }
}

#[cfg(feature = "gtk_ui")]
fn density_mode_to_index(mode: &str) -> u32 {
    match mode {
        "compact" => 0,
        "comfortable" => 2,
        _ => 1,
    }
}

#[cfg(feature = "gtk_ui")]
fn density_index_to_mode(index: u32) -> String {
    match index {
        0 => "compact".to_string(),
        2 => "comfortable".to_string(),
        _ => "default".to_string(),
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
        .map(|v| v.clamp(80, 100) as i32)
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
    let feature_apps_enabled = json
        .get("feature_apps_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default.feature_apps_enabled);
    let feature_windows_enabled = json
        .get("feature_windows_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default.feature_windows_enabled);
    let feature_files_enabled = json
        .get("feature_files_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default.feature_files_enabled);
    let feature_recents_enabled = json
        .get("feature_recents_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default.feature_recents_enabled);
    let feature_settings_enabled = json
        .get("feature_settings_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default.feature_settings_enabled);
    let feature_utility_enabled = json
        .get("feature_utility_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default.feature_utility_enabled);
    let weight_windows = json
        .get("weight_windows")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(-200, 200) as i32)
        .unwrap_or(default.weight_windows);
    let weight_apps = json
        .get("weight_apps")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(-200, 200) as i32)
        .unwrap_or(default.weight_apps);
    let weight_recents = json
        .get("weight_recents")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(-200, 200) as i32)
        .unwrap_or(default.weight_recents);
    let weight_files = json
        .get("weight_files")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(-200, 200) as i32)
        .unwrap_or(default.weight_files);
    let weight_emoji = json
        .get("weight_emoji")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(-200, 200) as i32)
        .unwrap_or(default.weight_emoji);
    let weight_utility = json
        .get("weight_utility")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(-200, 200) as i32)
        .unwrap_or(default.weight_utility);
    let min_fuzzy_score = json
        .get("min_fuzzy_score")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(0, 400) as i32)
        .unwrap_or(default.min_fuzzy_score);
    let animations_enabled = json
        .get("animations_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default.animations_enabled);
    let open_animation_ms = json
        .get("open_animation_ms")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(1, 500) as i32)
        .unwrap_or(default.open_animation_ms);
    let close_animation_ms = json
        .get("close_animation_ms")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(1, 500) as i32)
        .unwrap_or(default.close_animation_ms);
    let debounce_ms = json
        .get("debounce_ms")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(0, 500) as i32)
        .unwrap_or(default.debounce_ms);
    let density_mode = json
        .get("density_mode")
        .and_then(serde_json::Value::as_str)
        .map(sanitize_density_mode)
        .unwrap_or(default.density_mode);
    let indexed_folders = json
        .get("indexed_folders")
        .and_then(serde_json::Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(serde_json::Value::as_str)
                .map(|path| path.trim().to_string())
                .filter(|path| !path.is_empty())
                .collect::<Vec<String>>()
        })
        .unwrap_or(default.indexed_folders);
    let learning_enabled = json
        .get("learning_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default.learning_enabled);
    let currency_refresh_enabled = json
        .get("currency_refresh_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default.currency_refresh_enabled);
    let currency_rate_ttl_hours = json
        .get("currency_rate_ttl_hours")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(1, 168) as i32)
        .unwrap_or(default.currency_rate_ttl_hours);
    let web_search_enabled = json
        .get("web_search_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default.web_search_enabled);
    let web_search_max_actions = json
        .get("web_search_max_actions")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(1, 10) as i32)
        .unwrap_or(default.web_search_max_actions);

    LauncherUiSettings {
        overlay_opacity_percent: overlay,
        max_results,
        frameless_window: frameless,
        feature_apps_enabled,
        feature_windows_enabled,
        feature_files_enabled,
        feature_recents_enabled,
        feature_settings_enabled,
        feature_utility_enabled,
        weight_windows,
        weight_apps,
        weight_recents,
        weight_files,
        weight_emoji,
        weight_utility,
        min_fuzzy_score,
        animations_enabled,
        open_animation_ms,
        close_animation_ms,
        debounce_ms,
        density_mode,
        indexed_folders,
        learning_enabled,
        currency_refresh_enabled,
        currency_rate_ttl_hours,
        web_search_enabled,
        web_search_max_actions,
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
        "feature_apps_enabled": settings.feature_apps_enabled,
        "feature_windows_enabled": settings.feature_windows_enabled,
        "feature_files_enabled": settings.feature_files_enabled,
        "feature_recents_enabled": settings.feature_recents_enabled,
        "feature_settings_enabled": settings.feature_settings_enabled,
        "feature_utility_enabled": settings.feature_utility_enabled,
        "weight_windows": settings.weight_windows,
        "weight_apps": settings.weight_apps,
        "weight_recents": settings.weight_recents,
        "weight_files": settings.weight_files,
        "weight_emoji": settings.weight_emoji,
        "weight_utility": settings.weight_utility,
        "min_fuzzy_score": settings.min_fuzzy_score,
        "animations_enabled": settings.animations_enabled,
        "open_animation_ms": settings.open_animation_ms,
        "close_animation_ms": settings.close_animation_ms,
        "debounce_ms": settings.debounce_ms,
        "density_mode": settings.density_mode,
        "indexed_folders": settings.indexed_folders,
        "learning_enabled": settings.learning_enabled,
        "currency_refresh_enabled": settings.currency_refresh_enabled,
        "currency_rate_ttl_hours": settings.currency_rate_ttl_hours,
        "web_search_enabled": settings.web_search_enabled,
        "web_search_max_actions": settings.web_search_max_actions,
    });
    let encoded = serde_json::to_string_pretty(&payload)
        .map_err(|error| format!("encode settings failed: {error}"))?;
    fs::write(path, encoded).map_err(|error| format!("write settings failed: {error}"))
}

#[cfg(feature = "gtk_ui")]
fn sync_settings_to_hopd(socket_path: &str, settings: &LauncherUiSettings) {
    let values = [
        ("ui.overlay_opacity_percent", serde_json::json!(settings.overlay_opacity_percent)),
        ("ui.max_results", serde_json::json!(settings.max_results)),
        ("ui.frameless_window", serde_json::json!(settings.frameless_window)),
        ("features.apps", serde_json::json!(settings.feature_apps_enabled)),
        ("features.windows", serde_json::json!(settings.feature_windows_enabled)),
        ("features.files", serde_json::json!(settings.feature_files_enabled)),
        ("features.recents", serde_json::json!(settings.feature_recents_enabled)),
        ("features.settings", serde_json::json!(settings.feature_settings_enabled)),
        ("features.utility", serde_json::json!(settings.feature_utility_enabled)),
        ("ranking.weight_windows", serde_json::json!(settings.weight_windows)),
        ("ranking.weight_apps", serde_json::json!(settings.weight_apps)),
        ("ranking.weight_recents", serde_json::json!(settings.weight_recents)),
        ("ranking.weight_files", serde_json::json!(settings.weight_files)),
        ("ranking.weight_emoji", serde_json::json!(settings.weight_emoji)),
        ("ranking.weight_utility", serde_json::json!(settings.weight_utility)),
        ("ranking.min_fuzzy_score", serde_json::json!(settings.min_fuzzy_score)),
        ("ui.animations_enabled", serde_json::json!(settings.animations_enabled)),
        ("ui.open_animation_ms", serde_json::json!(settings.open_animation_ms)),
        ("ui.close_animation_ms", serde_json::json!(settings.close_animation_ms)),
        ("ui.debounce_ms", serde_json::json!(settings.debounce_ms)),
        ("ui.density_mode", serde_json::json!(settings.density_mode)),
        ("search.indexed_folders", serde_json::json!(settings.indexed_folders)),
        ("learning.enabled", serde_json::json!(settings.learning_enabled)),
        (
            "currency.refresh_enabled",
            serde_json::json!(settings.currency_refresh_enabled),
        ),
        (
            "currency.rate_ttl_hours",
            serde_json::json!(settings.currency_rate_ttl_hours),
        ),
        ("web_search.enabled", serde_json::json!(settings.web_search_enabled)),
        (
            "web_search.max_actions",
            serde_json::json!(settings.web_search_max_actions),
        ),
    ];
    for (key, value) in values {
        if let Err(error) = config_set(socket_path, key, value) {
            eprintln!("failed to sync setting {key} to hopd: {error}");
        }
    }
}

#[cfg(feature = "gtk_ui")]
fn apply_density_class(window: &adw::ApplicationWindow, mode: &str) {
    for class_name in [
        "hop-density-compact",
        "hop-density-default",
        "hop-density-comfortable",
    ] {
        window.remove_css_class(class_name);
    }
    let class_name = match mode {
        "compact" => "hop-density-compact",
        "comfortable" => "hop-density-comfortable",
        _ => "hop-density-default",
    };
    window.add_css_class(class_name);
}

#[cfg(feature = "gtk_ui")]
fn set_settings_feedback(status_label: &gtk::Label, message: &str, is_error: bool) {
    status_label.set_text(message);
    if is_error {
        status_label.add_css_class("hop-settings-status-error");
    } else {
        status_label.remove_css_class("hop-settings-status-error");
    }
}

#[cfg(feature = "gtk_ui")]
fn validate_indexed_folders(folders: &[String]) -> Result<(), String> {
    use std::path::Path;

    let mut invalid = Vec::new();
    for folder in folders {
        let path = Path::new(folder);
        if !path.is_absolute() {
            invalid.push(format!("{folder} (not absolute)"));
            continue;
        }
        if !path.exists() {
            invalid.push(format!("{folder} (missing)"));
            continue;
        }
        if !path.is_dir() {
            invalid.push(format!("{folder} (not directory)"));
        }
    }

    if invalid.is_empty() {
        Ok(())
    } else {
        Err(format!("Invalid indexed folders: {}", invalid.join(", ")))
    }
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
        let pending_search = Rc::new(RefCell::new(None::<gtk::glib::SourceId>));

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
            apply_density_class(&window, &settings.density_mode);
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

        content.append(&entry);
        content.append(&list_scroller);
        window.set_content(Some(&content));

        let open_settings = gio::SimpleAction::new("open-settings", None);
        {
            let app = app.clone();
            let parent = window.clone();
            let settings = ui_settings.clone();
            let socket_path = socket_path.clone();
            open_settings.connect_activate(move |_, _| {
                open_settings_window(&app, &parent, settings.clone(), &socket_path);
            });
        }
        app.add_action(&open_settings);
        app.set_accels_for_action(
            "app.open-settings",
            &["<Primary>comma", "<Super>comma", "<Meta>comma"],
        );

        let toggle = gio::SimpleAction::new("toggle", None);
        {
            let window = window.clone();
            let entry = entry.clone();
            let ui_settings = ui_settings.clone();
            toggle.connect_activate(move |_, _| {
                toggle_window(&window, &entry, &ui_settings.borrow());
            });
        }
        app.add_action(&toggle);
        app.set_accels_for_action("app.toggle", &[toggle_accelerator()]);

        let (toggle_tx, toggle_rx) = mpsc::channel::<()>();
        {
            let window = window.clone();
            let entry = entry.clone();
            let ui_settings = ui_settings.clone();
            gtk::glib::timeout_add_local(Duration::from_millis(30), move || {
                while toggle_rx.try_recv().is_ok() {
                    toggle_window(&window, &entry, &ui_settings.borrow());
                }
                gtk::glib::ControlFlow::Continue
            });
        }
        if let Err(error) = start_control_listener(default_control_socket_path(), toggle_tx) {
            eprintln!("failed to start control listener: {}", error);
        }
        sync_settings_to_hopd(&socket_path, &ui_settings.borrow());

        {
            let list = list.clone();
            let status = status.clone();
            let results = results.clone();
            let socket_path = socket_path.clone();
            let ui_settings = ui_settings.clone();
            let pending_search = pending_search.clone();
            entry.connect_changed(move |entry| {
                let query = entry.text().to_string();
                status.set_text(&render_status_text(QueryState::Searching));
                if let Some(source) = pending_search.borrow_mut().take() {
                    source.remove();
                }
                let debounce_ms = ui_settings.borrow().debounce_ms.max(0) as u64;
                let list = list.clone();
                let status = status.clone();
                let results = results.clone();
                let socket_path = socket_path.clone();
                let ui_settings = ui_settings.clone();
                let pending_search_for_timeout = pending_search.clone();
                let source = gtk::glib::timeout_add_local_once(
                    Duration::from_millis(debounce_ms),
                    move || {
                        let current_settings = ui_settings.borrow().clone();
                        refresh_results(
                            &list,
                            &status,
                            &results,
                            &socket_path,
                            current_settings.max_results,
                            &current_settings,
                            &query,
                        );
                        pending_search_for_timeout.borrow_mut().take();
                    },
                );
                *pending_search.borrow_mut() = Some(source);
            });
        }

        {
            let list = list.clone();
            let status = status.clone();
            let results = results.clone();
            let socket_path = socket_path.clone();
            let window = window.clone();
            let entry = entry.clone();
            let app = app.clone();
            let ui_settings = ui_settings.clone();
            let socket_path_for_settings = socket_path.clone();
            entry.clone().connect_activate(move |_| {
                let index = list
                    .selected_row()
                    .map(|row| row.index() as usize)
                    .unwrap_or(0);
                if let Some(result) = results.borrow().get(index).cloned() {
                    if result.id == "hop-launcher-settings" {
                        open_settings_window(
                            &app,
                            &window,
                            ui_settings.clone(),
                            &socket_path_for_settings,
                        );
                        status.set_text("Opened launcher settings");
                        return;
                    }
                    if should_copy_on_enter(&result) {
                        if let Err(error) = copy_text_to_clipboard(&result.title) {
                            status.set_text(&render_status_text(QueryState::Error(format!(
                                "copy failed: {error}"
                            ))));
                        } else {
                            status.set_text("Copied to clipboard");
                            hide_window(&window, &ui_settings.borrow());
                            entry.set_text("");
                        }
                        return;
                    }
                    if let Err(error) = execute(&socket_path, &result.id) {
                        status.set_text(&render_status_text(QueryState::Error(format!(
                            "execute failed: {error}"
                        ))));
                    } else {
                        status.set_text(&render_status_text(QueryState::Executed));
                        hide_window(&window, &ui_settings.borrow());
                        entry.set_text("");
                    }
                } else {
                    status.set_text(&render_status_text(QueryState::Empty));
                }
            });
        }

        {
            let list = list.clone();
            let window = window.clone();
            let app = app.clone();
            let ui_settings = ui_settings.clone();
            let socket_path = socket_path.clone();
            let controller = gtk::EventControllerKey::new();
            controller.connect_key_pressed(move |_, key, _, state| {
                let is_ctrl = state.contains(gtk::gdk::ModifierType::CONTROL_MASK);
                let is_super = state.contains(gtk::gdk::ModifierType::SUPER_MASK)
                    || state.contains(gtk::gdk::ModifierType::META_MASK);
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
                        hide_window(&window, &ui_settings.borrow());
                        true.into()
                    }
                    gtk::gdk::Key::comma if is_ctrl || is_super => {
                        open_settings_window(&app, &window, ui_settings.clone(), &socket_path);
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
            let app = app.clone();
            let ui_settings = ui_settings.clone();
            let socket_path_for_settings = socket_path.clone();
            list.connect_row_activated(move |_, row| {
                let index = row.index() as usize;
                if let Some(result) = results.borrow().get(index).cloned() {
                    if result.id == "hop-launcher-settings" {
                        open_settings_window(
                            &app,
                            &window,
                            ui_settings.clone(),
                            &socket_path_for_settings,
                        );
                        status.set_text("Opened launcher settings");
                        return;
                    }
                    if should_copy_on_enter(&result) {
                        if let Err(error) = copy_text_to_clipboard(&result.title) {
                            status.set_text(&render_status_text(QueryState::Error(format!(
                                "copy failed: {error}"
                            ))));
                        } else {
                            status.set_text("Copied to clipboard");
                            hide_window(&window, &ui_settings.borrow());
                            entry.set_text("");
                        }
                        return;
                    }
                    if let Err(error) = execute(&socket_path, &result.id) {
                        status.set_text(&render_status_text(QueryState::Error(format!(
                            "execute failed: {error}"
                        ))));
                    } else {
                        status.set_text(&render_status_text(QueryState::Executed));
                        hide_window(&window, &ui_settings.borrow());
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
            &ui_settings.borrow(),
            "",
        );

        if start_visible_on_launch() {
            present_window(&window, &entry, &ui_settings.borrow());
        } else {
            hide_window(&window, &ui_settings.borrow());
        }
    });

    app.run();
}

#[cfg(feature = "gtk_ui")]
fn toggle_window(
    window: &adw::ApplicationWindow,
    entry: &gtk::Entry,
    settings: &LauncherUiSettings,
) {
    if window.is_visible() {
        hide_window(window, settings);
    } else {
        present_window(window, entry, settings);
    }
}

#[cfg(feature = "gtk_ui")]
fn present_window(window: &adw::ApplicationWindow, entry: &gtk::Entry, settings: &LauncherUiSettings) {
    let target_opacity = settings.overlay_opacity_percent as f64 / 100.0;
    if settings.animations_enabled {
        window.set_opacity(0.0);
        window.present();
        animate_window_opacity(window.clone(), 0.0, target_opacity, settings.open_animation_ms);
    } else {
        window.set_opacity(target_opacity);
        window.present();
    }
    entry.grab_focus();
    entry.set_position(-1);
}

#[cfg(feature = "gtk_ui")]
fn hide_window(window: &adw::ApplicationWindow, settings: &LauncherUiSettings) {
    let target_opacity = settings.overlay_opacity_percent as f64 / 100.0;
    if !window.is_visible() {
        window.set_opacity(target_opacity);
        return;
    }
    if settings.animations_enabled {
        let from = window.opacity();
        let duration = settings.close_animation_ms;
        animate_window_opacity(window.clone(), from, 0.0, duration);
        let window_clone = window.clone();
        gtk::glib::timeout_add_local_once(Duration::from_millis(duration.max(1) as u64), move || {
            window_clone.hide();
            window_clone.set_opacity(target_opacity);
        });
    } else {
        window.hide();
        window.set_opacity(target_opacity);
    }
}

#[cfg(feature = "gtk_ui")]
fn animate_window_opacity(
    window: adw::ApplicationWindow,
    from: f64,
    to: f64,
    duration_ms: i32,
) {
    if duration_ms <= 1 {
        window.set_opacity(to);
        return;
    }
    let duration = duration_ms as f64;
    let started = Instant::now();
    gtk::glib::timeout_add_local(Duration::from_millis(16), move || {
        let elapsed = started.elapsed().as_millis() as f64;
        let progress = (elapsed / duration).clamp(0.0, 1.0);
        let current = from + (to - from) * progress;
        window.set_opacity(current);
        if progress >= 1.0 {
            gtk::glib::ControlFlow::Break
        } else {
            gtk::glib::ControlFlow::Continue
        }
    });
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
  background: linear-gradient(160deg, rgba(20, 26, 34, 0.88), rgba(17, 21, 30, 0.84));
}

.hop-launcher-settings-button {
  min-width: 32px;
  min-height: 32px;
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
  background: alpha(@view_bg_color, 0.70);
}

.hop-launcher-list row {
  margin: 1px 4px;
  border-radius: 10px;
  background: alpha(@view_bg_color, 0.42);
  transition: 130ms ease;
}

.hop-launcher-list row:hover {
  background: alpha(@view_bg_color, 0.56);
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

.hop-launcher-window.hop-density-compact .hop-launcher-list row {
  margin: 0 3px;
}

.hop-launcher-window.hop-density-compact .hop-launcher-row-body {
  min-height: 36px;
  margin-top: 2px;
  margin-bottom: 2px;
}

.hop-launcher-window.hop-density-compact .hop-launcher-subtitle {
  font-size: 0.82em;
}

.hop-launcher-window.hop-density-comfortable .hop-launcher-list row {
  margin: 3px 5px;
}

.hop-launcher-window.hop-density-comfortable .hop-launcher-row-body {
  min-height: 52px;
  margin-top: 9px;
  margin-bottom: 9px;
}

.hop-launcher-window.hop-density-comfortable .hop-launcher-subtitle {
  font-size: 0.98em;
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

.hop-settings-status {
  margin-top: 4px;
}

.hop-settings-status-error {
  color: @error_color;
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
    socket_path: &str,
) {
    let prefs = adw::PreferencesWindow::builder()
        .application(app)
        .title("Hop Launcher Settings")
        .default_width(560)
        .default_height(420)
        .transient_for(parent)
        .modal(true)
        .build();
    prefs.set_search_enabled(true);

    let page = adw::PreferencesPage::new();
    let appearance = adw::PreferencesGroup::builder()
        .title("Appearance")
        .description("Tune launcher translucency and window chrome.")
        .build();
    let behavior = adw::PreferencesGroup::builder()
        .title("Behavior")
        .description("Result density and interaction defaults.")
        .build();
    let providers = adw::PreferencesGroup::builder()
        .title("Providers")
        .description("Toggle result categories shown by the launcher.")
        .build();
    let ranking = adw::PreferencesGroup::builder()
        .title("Ranking")
        .description("Adjust provider weights and fuzzy threshold.")
        .build();
    let advanced = adw::PreferencesGroup::builder()
        .title("Advanced")
        .description("Parity controls for smart-provider behavior.")
        .build();
    let profile = adw::PreferencesGroup::builder()
        .title("Profile")
        .description("Manage launcher settings profile.")
        .build();
    let feedback = adw::PreferencesGroup::builder()
        .title("Status")
        .description("Settings save/sync feedback.")
        .build();
    let settings_status = gtk::Label::builder()
        .label("Ready")
        .xalign(0.0)
        .build();
    settings_status.add_css_class("dim-label");
    settings_status.add_css_class("hop-settings-status");
    feedback.add(&settings_status);

    let opacity_row = adw::ActionRow::builder()
        .title("Launcher translucency (%)")
        .subtitle("Higher values are less transparent.")
        .build();
    let opacity_adjustment = gtk::Adjustment::new(
        settings.borrow().overlay_opacity_percent as f64,
        80.0,
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
        let socket_path = socket_path.to_string();
        let settings_status = settings_status.clone();
        opacity_spin.connect_value_changed(move |spin| {
            let mut next = settings.borrow().clone();
            next.overlay_opacity_percent = spin.value_as_int().clamp(80, 100);
            parent.set_opacity(next.overlay_opacity_percent as f64 / 100.0);
            if let Err(error) = save_ui_settings(&next) {
                set_settings_feedback(
                    &settings_status,
                    &format!("Save failed: {error}"),
                    true,
                );
                return;
            }
            if let Err(error) = config_set(
                &socket_path,
                "ui.overlay_opacity_percent",
                serde_json::json!(next.overlay_opacity_percent),
            ) {
                set_settings_feedback(
                    &settings_status,
                    &format!("Sync failed: {error}"),
                    true,
                );
                return;
            }
            set_settings_feedback(&settings_status, "Saved launcher translucency", false);
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
        let socket_path = socket_path.to_string();
        let settings_status = settings_status.clone();
        frame_switch.connect_active_notify(move |toggle| {
            let mut next = settings.borrow().clone();
            next.frameless_window = toggle.is_active();
            parent.set_decorated(!next.frameless_window);
            if let Err(error) = save_ui_settings(&next) {
                set_settings_feedback(
                    &settings_status,
                    &format!("Save failed: {error}"),
                    true,
                );
                return;
            }
            if let Err(error) = config_set(
                &socket_path,
                "ui.frameless_window",
                serde_json::json!(next.frameless_window),
            ) {
                set_settings_feedback(
                    &settings_status,
                    &format!("Sync failed: {error}"),
                    true,
                );
                return;
            }
            set_settings_feedback(&settings_status, "Saved frame preference", false);
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
        let socket_path = socket_path.to_string();
        let settings_status = settings_status.clone();
        results_spin.connect_value_changed(move |spin| {
            let mut next = settings.borrow().clone();
            next.max_results = spin.value_as_int().clamp(4, 24) as u32;
            if let Err(error) = save_ui_settings(&next) {
                set_settings_feedback(
                    &settings_status,
                    &format!("Save failed: {error}"),
                    true,
                );
                return;
            }
            if let Err(error) = config_set(
                &socket_path,
                "ui.max_results",
                serde_json::json!(next.max_results),
            ) {
                set_settings_feedback(
                    &settings_status,
                    &format!("Sync failed: {error}"),
                    true,
                );
                return;
            }
            set_settings_feedback(&settings_status, "Saved max results", false);
            *settings.borrow_mut() = next;
        });
    }
    behavior.add(&results_row);

    let animations_row = adw::ActionRow::builder()
        .title("Animations enabled")
        .subtitle("Animate launcher open and close transitions.")
        .build();
    let animations_switch = gtk::Switch::builder()
        .active(settings.borrow().animations_enabled)
        .valign(gtk::Align::Center)
        .build();
    animations_row.add_suffix(&animations_switch);
    animations_row.set_activatable_widget(Some(&animations_switch));
    {
        let settings = settings.clone();
        let socket_path = socket_path.to_string();
        let settings_status = settings_status.clone();
        animations_switch.connect_active_notify(move |widget| {
            let mut next = settings.borrow().clone();
            next.animations_enabled = widget.is_active();
            if let Err(error) = save_ui_settings(&next) {
                set_settings_feedback(
                    &settings_status,
                    &format!("Save failed: {error}"),
                    true,
                );
                return;
            }
            if let Err(error) =
                config_set(&socket_path, "ui.animations_enabled", serde_json::json!(next.animations_enabled))
            {
                set_settings_feedback(
                    &settings_status,
                    &format!("Sync failed: {error}"),
                    true,
                );
                return;
            }
            set_settings_feedback(&settings_status, "Saved animations setting", false);
            *settings.borrow_mut() = next;
        });
    }
    behavior.add(&animations_row);

    add_integer_spin_row(
        &behavior,
        "Debounce (ms)",
        "Delay before triggering query refresh while typing.",
        settings.borrow().debounce_ms,
        0,
        500,
        "ui.debounce_ms",
        settings.clone(),
        socket_path,
        &settings_status,
        |state, value| state.debounce_ms = value,
    );
    let density_row = adw::ActionRow::builder()
        .title("Layout density")
        .subtitle("Adjust row spacing and compactness in results.")
        .build();
    let density = gtk::DropDown::from_strings(&["Compact", "Default", "Comfortable"]);
    density.set_selected(density_mode_to_index(&settings.borrow().density_mode));
    density.set_valign(gtk::Align::Center);
    density_row.add_suffix(&density);
    density_row.set_activatable_widget(Some(&density));
    {
        let settings = settings.clone();
        let socket_path = socket_path.to_string();
        let parent = parent.clone();
        let settings_status = settings_status.clone();
        density.connect_selected_notify(move |widget| {
            let mut next = settings.borrow().clone();
            next.density_mode = density_index_to_mode(widget.selected());
            apply_density_class(&parent, &next.density_mode);
            if let Err(error) = save_ui_settings(&next) {
                set_settings_feedback(
                    &settings_status,
                    &format!("Save failed: {error}"),
                    true,
                );
                return;
            }
            if let Err(error) =
                config_set(&socket_path, "ui.density_mode", serde_json::json!(next.density_mode))
            {
                set_settings_feedback(
                    &settings_status,
                    &format!("Sync failed: {error}"),
                    true,
                );
                return;
            }
            set_settings_feedback(&settings_status, "Saved density mode", false);
            *settings.borrow_mut() = next;
        });
    }
    behavior.add(&density_row);
    add_integer_spin_row(
        &behavior,
        "Open animation (ms)",
        "Duration for launcher open transition.",
        settings.borrow().open_animation_ms,
        1,
        500,
        "ui.open_animation_ms",
        settings.clone(),
        socket_path,
        &settings_status,
        |state, value| state.open_animation_ms = value,
    );
    add_integer_spin_row(
        &behavior,
        "Close animation (ms)",
        "Duration for launcher close transition.",
        settings.borrow().close_animation_ms,
        1,
        500,
        "ui.close_animation_ms",
        settings.clone(),
        socket_path,
        &settings_status,
        |state, value| state.close_animation_ms = value,
    );

    add_provider_switch_row(
        &providers,
        "Apps",
        "Installed application results.",
        settings.borrow().feature_apps_enabled,
        "features.apps",
        settings.clone(),
        socket_path,
        &settings_status,
        |state, value| state.feature_apps_enabled = value,
    );
    add_provider_switch_row(
        &providers,
        "Windows",
        "Open window results.",
        settings.borrow().feature_windows_enabled,
        "features.windows",
        settings.clone(),
        socket_path,
        &settings_status,
        |state, value| state.feature_windows_enabled = value,
    );
    add_provider_switch_row(
        &providers,
        "Files",
        "File search results.",
        settings.borrow().feature_files_enabled,
        "features.files",
        settings.clone(),
        socket_path,
        &settings_status,
        |state, value| state.feature_files_enabled = value,
    );
    add_provider_switch_row(
        &providers,
        "Recents",
        "Recent document results.",
        settings.borrow().feature_recents_enabled,
        "features.recents",
        settings.clone(),
        socket_path,
        &settings_status,
        |state, value| state.feature_recents_enabled = value,
    );
    add_provider_switch_row(
        &providers,
        "Settings",
        "System and launcher settings results.",
        settings.borrow().feature_settings_enabled,
        "features.settings",
        settings.clone(),
        socket_path,
        &settings_status,
        |state, value| state.feature_settings_enabled = value,
    );
    add_provider_switch_row(
        &providers,
        "Utilities",
        "Weather, timezone, emoji, calculator, and currency results.",
        settings.borrow().feature_utility_enabled,
        "features.utility",
        settings.clone(),
        socket_path,
        &settings_status,
        |state, value| state.feature_utility_enabled = value,
    );
    let indexed_row = adw::ActionRow::builder()
        .title("Indexed folders")
        .subtitle("Comma-separated folders used for file indexing/search.")
        .build();
    let indexed_entry = gtk::Entry::builder()
        .hexpand(true)
        .placeholder_text("/home/user/Documents, /home/user/Projects")
        .text(settings.borrow().indexed_folders.join(", "))
        .build();
    indexed_entry.set_valign(gtk::Align::Center);
    indexed_row.add_suffix(&indexed_entry);
    indexed_row.set_activatable_widget(Some(&indexed_entry));
    {
        let settings = settings.clone();
        let socket_path = socket_path.to_string();
        let settings_status = settings_status.clone();
        indexed_entry.connect_changed(move |entry| {
            let raw = entry.text();
            let folders = raw
                .split(',')
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect::<Vec<String>>();
            if let Err(error) = validate_indexed_folders(&folders) {
                set_settings_feedback(&settings_status, &error, true);
                return;
            }
            let mut next = settings.borrow().clone();
            next.indexed_folders = folders;
            if let Err(error) = save_ui_settings(&next) {
                set_settings_feedback(
                    &settings_status,
                    &format!("Save failed: {error}"),
                    true,
                );
                return;
            }
            if let Err(error) = config_set(
                &socket_path,
                "search.indexed_folders",
                serde_json::json!(next.indexed_folders),
            ) {
                set_settings_feedback(
                    &settings_status,
                    &format!("Sync failed: {error}"),
                    true,
                );
                return;
            }
            set_settings_feedback(&settings_status, "Saved indexed folders", false);
            *settings.borrow_mut() = next;
        });
    }
    providers.add(&indexed_row);

    add_integer_spin_row(
        &ranking,
        "Window weight",
        "Additional score applied to window matches.",
        settings.borrow().weight_windows,
        -200,
        200,
        "ranking.weight_windows",
        settings.clone(),
        socket_path,
        &settings_status,
        |state, value| state.weight_windows = value,
    );
    add_integer_spin_row(
        &ranking,
        "App weight",
        "Additional score applied to app matches.",
        settings.borrow().weight_apps,
        -200,
        200,
        "ranking.weight_apps",
        settings.clone(),
        socket_path,
        &settings_status,
        |state, value| state.weight_apps = value,
    );
    add_integer_spin_row(
        &ranking,
        "Recent weight",
        "Additional score applied to recent file matches.",
        settings.borrow().weight_recents,
        -200,
        200,
        "ranking.weight_recents",
        settings.clone(),
        socket_path,
        &settings_status,
        |state, value| state.weight_recents = value,
    );
    add_integer_spin_row(
        &ranking,
        "File weight",
        "Additional score applied to file matches.",
        settings.borrow().weight_files,
        -200,
        200,
        "ranking.weight_files",
        settings.clone(),
        socket_path,
        &settings_status,
        |state, value| state.weight_files = value,
    );
    add_integer_spin_row(
        &ranking,
        "Emoji weight",
        "Additional score applied to emoji matches.",
        settings.borrow().weight_emoji,
        -200,
        200,
        "ranking.weight_emoji",
        settings.clone(),
        socket_path,
        &settings_status,
        |state, value| state.weight_emoji = value,
    );
    add_integer_spin_row(
        &ranking,
        "Utility weight",
        "Additional score applied to utility matches.",
        settings.borrow().weight_utility,
        -200,
        200,
        "ranking.weight_utility",
        settings.clone(),
        socket_path,
        &settings_status,
        |state, value| state.weight_utility = value,
    );
    add_integer_spin_row(
        &ranking,
        "Min fuzzy score",
        "Minimum score required for non-empty search results.",
        settings.borrow().min_fuzzy_score,
        0,
        400,
        "ranking.min_fuzzy_score",
        settings.clone(),
        socket_path,
        &settings_status,
        |state, value| state.min_fuzzy_score = value,
    );

    page.add(&appearance);
    page.add(&behavior);
    page.add(&providers);
    page.add(&ranking);
    add_provider_switch_row(
        &advanced,
        "Learning enabled",
        "Enable usage-based learning signals for ranking.",
        settings.borrow().learning_enabled,
        "learning.enabled",
        settings.clone(),
        socket_path,
        &settings_status,
        |state, value| state.learning_enabled = value,
    );
    add_provider_switch_row(
        &advanced,
        "Currency refresh",
        "Allow online refresh of exchange rates when available.",
        settings.borrow().currency_refresh_enabled,
        "currency.refresh_enabled",
        settings.clone(),
        socket_path,
        &settings_status,
        |state, value| state.currency_refresh_enabled = value,
    );
    add_integer_spin_row(
        &advanced,
        "Currency TTL (hours)",
        "Hours before cached currency rates are considered stale.",
        settings.borrow().currency_rate_ttl_hours,
        1,
        168,
        "currency.rate_ttl_hours",
        settings.clone(),
        socket_path,
        &settings_status,
        |state, value| state.currency_rate_ttl_hours = value,
    );
    add_provider_switch_row(
        &advanced,
        "Web search enabled",
        "Expose web-search actions for non-empty queries.",
        settings.borrow().web_search_enabled,
        "web_search.enabled",
        settings.clone(),
        socket_path,
        &settings_status,
        |state, value| state.web_search_enabled = value,
    );
    add_integer_spin_row(
        &advanced,
        "Web search max actions",
        "Maximum number of web-search providers shown per query.",
        settings.borrow().web_search_max_actions,
        1,
        10,
        "web_search.max_actions",
        settings.clone(),
        socket_path,
        &settings_status,
        |state, value| state.web_search_max_actions = value,
    );
    let reset_row = adw::ActionRow::builder()
        .title("Reset to defaults")
        .subtitle("Restore all launcher settings to default values.")
        .build();
    let export_row = adw::ActionRow::builder()
        .title("Export profile")
        .subtitle("Save launcher settings to a JSON file.")
        .build();
    let export_button = gtk::Button::builder()
        .label("Export")
        .valign(gtk::Align::Center)
        .build();
    export_row.add_suffix(&export_button);
    export_row.set_activatable_widget(Some(&export_button));
    {
        let settings = settings.clone();
        let settings_status = settings_status.clone();
        let prefs = prefs.clone();
        export_button.connect_clicked(move |_| {
            let settings = settings.clone();
            let settings_status = settings_status.clone();
            select_json_file(
                &prefs,
                "Export Hop Launcher Settings",
                "Save",
                Some("hop-launcher-settings.json"),
                move |path| {
                    let payload = serde_json::json!({
                        "overlay_opacity_percent": settings.borrow().overlay_opacity_percent,
                        "max_results": settings.borrow().max_results,
                        "frameless_window": settings.borrow().frameless_window,
                        "feature_apps_enabled": settings.borrow().feature_apps_enabled,
                        "feature_windows_enabled": settings.borrow().feature_windows_enabled,
                        "feature_files_enabled": settings.borrow().feature_files_enabled,
                        "feature_recents_enabled": settings.borrow().feature_recents_enabled,
                        "feature_settings_enabled": settings.borrow().feature_settings_enabled,
                        "feature_utility_enabled": settings.borrow().feature_utility_enabled,
                        "weight_windows": settings.borrow().weight_windows,
                        "weight_apps": settings.borrow().weight_apps,
                        "weight_recents": settings.borrow().weight_recents,
                        "weight_files": settings.borrow().weight_files,
                        "weight_emoji": settings.borrow().weight_emoji,
                        "weight_utility": settings.borrow().weight_utility,
                        "min_fuzzy_score": settings.borrow().min_fuzzy_score,
                        "animations_enabled": settings.borrow().animations_enabled,
                        "open_animation_ms": settings.borrow().open_animation_ms,
                        "close_animation_ms": settings.borrow().close_animation_ms,
                        "debounce_ms": settings.borrow().debounce_ms,
                        "density_mode": settings.borrow().density_mode,
                        "indexed_folders": settings.borrow().indexed_folders,
                        "learning_enabled": settings.borrow().learning_enabled,
                        "currency_refresh_enabled": settings.borrow().currency_refresh_enabled,
                        "currency_rate_ttl_hours": settings.borrow().currency_rate_ttl_hours,
                        "web_search_enabled": settings.borrow().web_search_enabled,
                        "web_search_max_actions": settings.borrow().web_search_max_actions
                    });
                    let encoded = match serde_json::to_string_pretty(&payload) {
                        Ok(value) => value,
                        Err(error) => {
                            set_settings_feedback(
                                &settings_status,
                                &format!("Export encode failed: {error}"),
                                true,
                            );
                            return;
                        }
                    };
                    if let Err(error) = std::fs::write(&path, encoded) {
                        set_settings_feedback(
                            &settings_status,
                            &format!("Export write failed: {error}"),
                            true,
                        );
                        return;
                    }
                    set_settings_feedback(
                        &settings_status,
                        &format!("Exported profile to {}", path.display()),
                        false,
                    );
                },
            );
        });
    }
    let import_row = adw::ActionRow::builder()
        .title("Import profile")
        .subtitle("Load launcher settings from a JSON file.")
        .build();
    let import_button = gtk::Button::builder()
        .label("Import")
        .valign(gtk::Align::Center)
        .build();
    import_row.add_suffix(&import_button);
    import_row.set_activatable_widget(Some(&import_button));
    {
        let settings = settings.clone();
        let socket_path = socket_path.to_string();
        let settings_status = settings_status.clone();
        let parent = parent.clone();
        let prefs = prefs.clone();
        let app = app.clone();
        import_button.connect_clicked(move |_| {
            let settings = settings.clone();
            let socket_path = socket_path.clone();
            let settings_status = settings_status.clone();
            let parent = parent.clone();
            let prefs_for_dialog = prefs.clone();
            let prefs_for_close = prefs.clone();
            let app = app.clone();
            open_json_file(&prefs_for_dialog, "Import Hop Launcher Settings", move |path| {
                let imported = match load_settings_from_path(&path) {
                    Ok(value) => value,
                    Err(error) => {
                        set_settings_feedback(
                            &settings_status,
                            &format!("Import failed: {error}"),
                            true,
                        );
                        return;
                    }
                };
                parent.set_opacity(imported.overlay_opacity_percent as f64 / 100.0);
                parent.set_decorated(!imported.frameless_window);
                apply_density_class(&parent, &imported.density_mode);
                if let Err(error) = save_ui_settings(&imported) {
                    set_settings_feedback(
                        &settings_status,
                        &format!("Import save failed: {error}"),
                        true,
                    );
                    return;
                }
                sync_settings_to_hopd(&socket_path, &imported);
                *settings.borrow_mut() = imported;
                set_settings_feedback(
                    &settings_status,
                    &format!("Imported profile from {}", path.display()),
                    false,
                );
                prefs_for_close.close();
                open_settings_window(&app, &parent, settings.clone(), &socket_path);
            });
        });
    }
    let reset_button = gtk::Button::builder()
        .label("Reset")
        .valign(gtk::Align::Center)
        .build();
    reset_button.add_css_class("destructive-action");
    reset_row.add_suffix(&reset_button);
    reset_row.set_activatable_widget(Some(&reset_button));
    {
        let settings = settings.clone();
        let socket_path = socket_path.to_string();
        let settings_status = settings_status.clone();
        let parent = parent.clone();
        let prefs = prefs.clone();
        let app = app.clone();
        reset_button.connect_clicked(move |_| {
            let next = LauncherUiSettings::default();
            parent.set_opacity(next.overlay_opacity_percent as f64 / 100.0);
            parent.set_decorated(!next.frameless_window);
            apply_density_class(&parent, &next.density_mode);
            if let Err(error) = save_ui_settings(&next) {
                set_settings_feedback(&settings_status, &format!("Reset save failed: {error}"), true);
                return;
            }
            sync_settings_to_hopd(&socket_path, &next);
            *settings.borrow_mut() = next;
            set_settings_feedback(
                &settings_status,
                "Settings reset to defaults",
                false,
            );
            prefs.close();
            open_settings_window(&app, &parent, settings.clone(), &socket_path);
        });
    }
    profile.add(&import_row);
    profile.add(&export_row);
    profile.add(&reset_row);
    page.add(&advanced);
    page.add(&profile);
    page.add(&feedback);
    prefs.add(&page);
    prefs.present();
}

#[cfg(feature = "gtk_ui")]
fn add_provider_switch_row(
    group: &adw::PreferencesGroup,
    title: &str,
    subtitle: &str,
    initial_state: bool,
    hopd_key: &str,
    settings: Rc<RefCell<LauncherUiSettings>>,
    socket_path: &str,
    status_label: &gtk::Label,
    apply_value: fn(&mut LauncherUiSettings, bool),
) {
    let row = adw::ActionRow::builder()
        .title(title)
        .subtitle(subtitle)
        .build();
    let toggle = gtk::Switch::builder()
        .active(initial_state)
        .valign(gtk::Align::Center)
        .build();
    row.add_suffix(&toggle);
    row.set_activatable_widget(Some(&toggle));
    {
        let settings = settings.clone();
        let socket_path = socket_path.to_string();
        let hopd_key = hopd_key.to_string();
        let status_label = status_label.clone();
        toggle.connect_active_notify(move |widget| {
            let mut next = settings.borrow().clone();
            apply_value(&mut next, widget.is_active());
            if let Err(error) = save_ui_settings(&next) {
                set_settings_feedback(&status_label, &format!("Save failed: {error}"), true);
                return;
            }
            if let Err(error) = config_set(&socket_path, &hopd_key, serde_json::json!(widget.is_active())) {
                set_settings_feedback(&status_label, &format!("Sync failed: {error}"), true);
                return;
            }
            set_settings_feedback(&status_label, "Saved setting", false);
            *settings.borrow_mut() = next;
        });
    }
    group.add(&row);
}

#[cfg(feature = "gtk_ui")]
fn add_integer_spin_row(
    group: &adw::PreferencesGroup,
    title: &str,
    subtitle: &str,
    initial_value: i32,
    min: i32,
    max: i32,
    hopd_key: &str,
    settings: Rc<RefCell<LauncherUiSettings>>,
    socket_path: &str,
    status_label: &gtk::Label,
    apply_value: fn(&mut LauncherUiSettings, i32),
) {
    let row = adw::ActionRow::builder()
        .title(title)
        .subtitle(subtitle)
        .build();
    let adjustment = gtk::Adjustment::new(
        initial_value as f64,
        min as f64,
        max as f64,
        1.0,
        5.0,
        0.0,
    );
    let spin = gtk::SpinButton::new(Some(&adjustment), 1.0, 0);
    spin.set_valign(gtk::Align::Center);
    row.add_suffix(&spin);
    row.set_activatable_widget(Some(&spin));
    {
        let settings = settings.clone();
        let socket_path = socket_path.to_string();
        let hopd_key = hopd_key.to_string();
        let status_label = status_label.clone();
        spin.connect_value_changed(move |widget| {
            let mut next = settings.borrow().clone();
            let value = widget.value_as_int().clamp(min, max);
            apply_value(&mut next, value);
            if let Err(error) = save_ui_settings(&next) {
                set_settings_feedback(&status_label, &format!("Save failed: {error}"), true);
                return;
            }
            if let Err(error) = config_set(&socket_path, &hopd_key, serde_json::json!(value)) {
                set_settings_feedback(&status_label, &format!("Sync failed: {error}"), true);
                return;
            }
            set_settings_feedback(&status_label, "Saved setting", false);
            *settings.borrow_mut() = next;
        });
    }
    group.add(&row);
}

#[cfg(feature = "gtk_ui")]
fn select_json_file(
    parent: &adw::PreferencesWindow,
    title: &str,
    accept_label: &str,
    initial_name: Option<&str>,
    on_selected: impl Fn(std::path::PathBuf) + 'static,
) {
    let dialog = gtk::FileChooserNative::builder()
        .title(title)
        .transient_for(parent)
        .accept_label(accept_label)
        .modal(true)
        .build();
    dialog.set_action(gtk::FileChooserAction::Save);
    if let Some(name) = initial_name {
        dialog.set_current_name(name);
    }
    let filter = gtk::FileFilter::new();
    filter.set_name(Some("JSON files"));
    filter.add_pattern("*.json");
    dialog.add_filter(&filter);
    dialog.connect_response(move |chooser, response| {
        if response == gtk::ResponseType::Accept {
            if let Some(file) = chooser.file() {
                if let Some(path) = file.path() {
                    on_selected(path);
                }
            }
        }
    });
    dialog.show();
}

#[cfg(feature = "gtk_ui")]
fn open_json_file(
    parent: &adw::PreferencesWindow,
    title: &str,
    on_selected: impl Fn(std::path::PathBuf) + 'static,
) {
    let dialog = gtk::FileChooserNative::builder()
        .title(title)
        .transient_for(parent)
        .accept_label("Open")
        .modal(true)
        .build();
    dialog.set_action(gtk::FileChooserAction::Open);
    let filter = gtk::FileFilter::new();
    filter.set_name(Some("JSON files"));
    filter.add_pattern("*.json");
    dialog.add_filter(&filter);
    dialog.connect_response(move |chooser, response| {
        if response == gtk::ResponseType::Accept {
            if let Some(file) = chooser.file() {
                if let Some(path) = file.path() {
                    on_selected(path);
                }
            }
        }
    });
    dialog.show();
}

#[cfg(feature = "gtk_ui")]
fn load_settings_from_path(path: &std::path::Path) -> Result<LauncherUiSettings, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|error| format!("read file failed: {error}"))?;
    let json = serde_json::from_str::<serde_json::Value>(&raw)
        .map_err(|error| format!("parse json failed: {error}"))?;
    let default = LauncherUiSettings::default();
    let merged = serde_json::json!({
        "overlay_opacity_percent": json.get("overlay_opacity_percent").cloned().unwrap_or(serde_json::json!(default.overlay_opacity_percent)),
        "max_results": json.get("max_results").cloned().unwrap_or(serde_json::json!(default.max_results)),
        "frameless_window": json.get("frameless_window").cloned().unwrap_or(serde_json::json!(default.frameless_window)),
        "feature_apps_enabled": json.get("feature_apps_enabled").cloned().unwrap_or(serde_json::json!(default.feature_apps_enabled)),
        "feature_windows_enabled": json.get("feature_windows_enabled").cloned().unwrap_or(serde_json::json!(default.feature_windows_enabled)),
        "feature_files_enabled": json.get("feature_files_enabled").cloned().unwrap_or(serde_json::json!(default.feature_files_enabled)),
        "feature_recents_enabled": json.get("feature_recents_enabled").cloned().unwrap_or(serde_json::json!(default.feature_recents_enabled)),
        "feature_settings_enabled": json.get("feature_settings_enabled").cloned().unwrap_or(serde_json::json!(default.feature_settings_enabled)),
        "feature_utility_enabled": json.get("feature_utility_enabled").cloned().unwrap_or(serde_json::json!(default.feature_utility_enabled)),
        "weight_windows": json.get("weight_windows").cloned().unwrap_or(serde_json::json!(default.weight_windows)),
        "weight_apps": json.get("weight_apps").cloned().unwrap_or(serde_json::json!(default.weight_apps)),
        "weight_recents": json.get("weight_recents").cloned().unwrap_or(serde_json::json!(default.weight_recents)),
        "weight_files": json.get("weight_files").cloned().unwrap_or(serde_json::json!(default.weight_files)),
        "weight_emoji": json.get("weight_emoji").cloned().unwrap_or(serde_json::json!(default.weight_emoji)),
        "weight_utility": json.get("weight_utility").cloned().unwrap_or(serde_json::json!(default.weight_utility)),
        "min_fuzzy_score": json.get("min_fuzzy_score").cloned().unwrap_or(serde_json::json!(default.min_fuzzy_score)),
        "animations_enabled": json.get("animations_enabled").cloned().unwrap_or(serde_json::json!(default.animations_enabled)),
        "open_animation_ms": json.get("open_animation_ms").cloned().unwrap_or(serde_json::json!(default.open_animation_ms)),
        "close_animation_ms": json.get("close_animation_ms").cloned().unwrap_or(serde_json::json!(default.close_animation_ms)),
        "debounce_ms": json.get("debounce_ms").cloned().unwrap_or(serde_json::json!(default.debounce_ms)),
        "density_mode": json.get("density_mode").cloned().unwrap_or(serde_json::json!(default.density_mode)),
        "indexed_folders": json.get("indexed_folders").cloned().unwrap_or(serde_json::json!(default.indexed_folders)),
        "learning_enabled": json.get("learning_enabled").cloned().unwrap_or(serde_json::json!(default.learning_enabled)),
        "currency_refresh_enabled": json.get("currency_refresh_enabled").cloned().unwrap_or(serde_json::json!(default.currency_refresh_enabled)),
        "currency_rate_ttl_hours": json.get("currency_rate_ttl_hours").cloned().unwrap_or(serde_json::json!(default.currency_rate_ttl_hours)),
        "web_search_enabled": json.get("web_search_enabled").cloned().unwrap_or(serde_json::json!(default.web_search_enabled)),
        "web_search_max_actions": json.get("web_search_max_actions").cloned().unwrap_or(serde_json::json!(default.web_search_max_actions))
    });
    let encoded = serde_json::to_string(&merged).map_err(|error| format!("encode failed: {error}"))?;
    let temp_path = settings_file_path().ok_or_else(|| "missing config dir".to_string())?;
    let parent = temp_path
        .parent()
        .ok_or_else(|| "missing config dir".to_string())?;
    std::fs::create_dir_all(parent).map_err(|error| format!("create settings dir failed: {error}"))?;
    let probe_path = parent.join(".import-probe.json");
    std::fs::write(&probe_path, encoded).map_err(|error| format!("write import probe failed: {error}"))?;
    let settings = load_ui_settings_from_path(&probe_path);
    let _ = std::fs::remove_file(&probe_path);
    Ok(settings)
}

#[cfg(feature = "gtk_ui")]
fn load_ui_settings_from_path(path: &std::path::Path) -> LauncherUiSettings {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return LauncherUiSettings::default();
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return LauncherUiSettings::default();
    };
    let _ = json;
    // Reuse existing loader by temporarily overriding the default path behavior through direct parse.
    // This keeps normalization behavior in one place.
    // Small helper: write to default path parser not needed; parse manually through copy of loader logic.
    let default = LauncherUiSettings::default();
    let overlay = json
        .get("overlay_opacity_percent")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(80, 100) as i32)
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
    let feature_apps_enabled = json
        .get("feature_apps_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default.feature_apps_enabled);
    let feature_windows_enabled = json
        .get("feature_windows_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default.feature_windows_enabled);
    let feature_files_enabled = json
        .get("feature_files_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default.feature_files_enabled);
    let feature_recents_enabled = json
        .get("feature_recents_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default.feature_recents_enabled);
    let feature_settings_enabled = json
        .get("feature_settings_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default.feature_settings_enabled);
    let feature_utility_enabled = json
        .get("feature_utility_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default.feature_utility_enabled);
    let weight_windows = json
        .get("weight_windows")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(-200, 200) as i32)
        .unwrap_or(default.weight_windows);
    let weight_apps = json
        .get("weight_apps")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(-200, 200) as i32)
        .unwrap_or(default.weight_apps);
    let weight_recents = json
        .get("weight_recents")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(-200, 200) as i32)
        .unwrap_or(default.weight_recents);
    let weight_files = json
        .get("weight_files")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(-200, 200) as i32)
        .unwrap_or(default.weight_files);
    let weight_emoji = json
        .get("weight_emoji")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(-200, 200) as i32)
        .unwrap_or(default.weight_emoji);
    let weight_utility = json
        .get("weight_utility")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(-200, 200) as i32)
        .unwrap_or(default.weight_utility);
    let min_fuzzy_score = json
        .get("min_fuzzy_score")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(0, 400) as i32)
        .unwrap_or(default.min_fuzzy_score);
    let animations_enabled = json
        .get("animations_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default.animations_enabled);
    let open_animation_ms = json
        .get("open_animation_ms")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(1, 500) as i32)
        .unwrap_or(default.open_animation_ms);
    let close_animation_ms = json
        .get("close_animation_ms")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(1, 500) as i32)
        .unwrap_or(default.close_animation_ms);
    let debounce_ms = json
        .get("debounce_ms")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(0, 500) as i32)
        .unwrap_or(default.debounce_ms);
    let density_mode = json
        .get("density_mode")
        .and_then(serde_json::Value::as_str)
        .map(sanitize_density_mode)
        .unwrap_or(default.density_mode);
    let indexed_folders = json
        .get("indexed_folders")
        .and_then(serde_json::Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(serde_json::Value::as_str)
                .map(|path| path.trim().to_string())
                .filter(|path| !path.is_empty())
                .collect::<Vec<String>>()
        })
        .unwrap_or(default.indexed_folders);
    let learning_enabled = json
        .get("learning_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default.learning_enabled);
    let currency_refresh_enabled = json
        .get("currency_refresh_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default.currency_refresh_enabled);
    let currency_rate_ttl_hours = json
        .get("currency_rate_ttl_hours")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(1, 168) as i32)
        .unwrap_or(default.currency_rate_ttl_hours);
    let web_search_enabled = json
        .get("web_search_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default.web_search_enabled);
    let web_search_max_actions = json
        .get("web_search_max_actions")
        .and_then(serde_json::Value::as_i64)
        .map(|v| v.clamp(1, 10) as i32)
        .unwrap_or(default.web_search_max_actions);

    LauncherUiSettings {
        overlay_opacity_percent: overlay,
        max_results,
        frameless_window: frameless,
        feature_apps_enabled,
        feature_windows_enabled,
        feature_files_enabled,
        feature_recents_enabled,
        feature_settings_enabled,
        feature_utility_enabled,
        weight_windows,
        weight_apps,
        weight_recents,
        weight_files,
        weight_emoji,
        weight_utility,
        min_fuzzy_score,
        animations_enabled,
        open_animation_ms,
        close_animation_ms,
        debounce_ms,
        density_mode,
        indexed_folders,
        learning_enabled,
        currency_refresh_enabled,
        currency_rate_ttl_hours,
        web_search_enabled,
        web_search_max_actions,
    }
}

#[cfg(feature = "gtk_ui")]
fn should_copy_on_enter(row: &LauncherResult) -> bool {
    matches!(
        row.kind.as_str(),
        "utility" | "emoji" | "calculator" | "currency" | "weather" | "timezone"
    ) && !row.title.trim().is_empty()
}

#[cfg(feature = "gtk_ui")]
fn copy_text_to_clipboard(text: &str) -> Result<(), String> {
    let value = text.trim();
    if value.is_empty() {
        return Err("empty text".to_string());
    }
    let Some(display) = gtk::gdk::Display::default() else {
        return Err("display unavailable".to_string());
    };
    display.clipboard().set_text(value);
    Ok(())
}

#[cfg(feature = "gtk_ui")]
fn refresh_results(
    list: &gtk::ListBox,
    status: &gtk::Label,
    results: &Rc<RefCell<Vec<LauncherResult>>>,
    socket_path: &str,
    max_results: u32,
    ui_settings: &LauncherUiSettings,
    query: &str,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    status.set_text(&render_status_text(QueryState::Searching));
    match search(socket_path, query, max_results) {
        Ok(rows) => {
            let rows = filter_results_by_settings(rows, ui_settings);
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
            status.set_text(&render_status_text(if rows.is_empty() {
                QueryState::Empty
            } else {
                QueryState::Results { count: rows.len() }
            }));
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
fn is_kind_enabled(kind: &str, settings: &LauncherUiSettings) -> bool {
    match kind {
        "app" => settings.feature_apps_enabled,
        "window" => settings.feature_windows_enabled,
        "file" => settings.feature_files_enabled,
        "recent" => settings.feature_recents_enabled,
        "setting" => settings.feature_settings_enabled,
        "weather" | "timezone" | "emoji" | "utility" | "calculator" | "currency" => {
            settings.feature_utility_enabled
        }
        _ => true,
    }
}

#[cfg(feature = "gtk_ui")]
fn filter_results_by_settings(
    rows: Vec<LauncherResult>,
    settings: &LauncherUiSettings,
) -> Vec<LauncherResult> {
    rows.into_iter()
        .filter(|row| is_kind_enabled(&row.kind, settings))
        .collect()
}

#[cfg(feature = "gtk_ui")]
fn build_result_icon(row: &LauncherResult) -> gtk::Image {
    if row.kind == "app" {
        if let Some(desktop_id) = row.id.strip_prefix("app:") {
            if let Some(app_info) = gio::DesktopAppInfo::new(desktop_id) {
                if let Some(icon) = app_info.icon() {
                    return gtk::Image::from_gicon(&icon);
                }
            }
        }
    }

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
        "utility" | "weather" | "timezone" | "emoji" | "calculator" | "currency" => "Copy",
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
