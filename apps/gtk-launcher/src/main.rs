#[cfg(feature = "gtk_ui")]
use std::cell::RefCell;
#[cfg(feature = "gtk_ui")]
use std::collections::{HashMap, HashSet};
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
use std::sync::OnceLock;
#[cfg(feature = "gtk_ui")]
use std::thread;
#[cfg(feature = "gtk_ui")]
use std::time::Duration;
#[cfg(feature = "gtk_ui")]
use std::time::Instant;
#[cfg(feature = "gtk_ui")]
use std::path::{Path, PathBuf};
#[cfg(feature = "gtk_ui")]
use std::process::Command as ProcessCommand;

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
    config_set, default_hopd_socket_path, execute, learning_record, learning_reset, parse_control_request, render_status_text, search,
    settings_accelerators, start_visible_on_launch, toggle_accelerator, ControlMethod, LauncherResult, QueryState,
};

fn main() {
    run();
}

#[cfg(feature = "gtk_ui")]
#[derive(Clone, Debug)]
struct LauncherUiSettings {
    overlay_opacity_percent: i32,
    blur_strength_percent: i32,
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
    advanced_mode_enabled: bool,
    currency_refresh_enabled: bool,
    currency_rate_ttl_hours: i32,
    web_search_enabled: bool,
    web_search_max_actions: i32,
    web_search_services_json: String,
    global_shortcut: String,
}

#[cfg(feature = "gtk_ui")]
impl Default for LauncherUiSettings {
    fn default() -> Self {
        Self {
            overlay_opacity_percent: 100,
            blur_strength_percent: 0,
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
            advanced_mode_enabled: false,
            currency_refresh_enabled: true,
            currency_rate_ttl_hours: 12,
            web_search_enabled: true,
            web_search_max_actions: 3,
            web_search_services_json: default_web_search_services_json(),
            global_shortcut: "<Super>space".to_string(),
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
fn sanitize_blur_strength_percent(raw: i64) -> i32 {
    raw.clamp(0, 100) as i32
}

#[cfg(feature = "gtk_ui")]
fn legacy_blur_mode_to_percent(raw: &str) -> i32 {
    match raw {
        "off" => 0,
        "strong" => 70,
        "soft" => 35,
        _ => 0,
    }
}

#[cfg(feature = "gtk_ui")]
fn sanitize_shortcut(raw: &str) -> String {
    let value = raw.trim();
    if value.is_empty() {
        "<Super>space".to_string()
    } else {
        value.to_string()
    }
}

#[cfg(feature = "gtk_ui")]
fn resolve_hop_hotkeyd_program() -> String {
    if let Ok(home) = std::env::var("HOME") {
        let candidate = format!("{home}/.local/bin/hop-hotkeyd");
        if Path::new(&candidate).exists() {
            return candidate;
        }
    }
    "hop-hotkeyd".to_string()
}

#[cfg(feature = "gtk_ui")]
fn build_shortcut_apply_steps(shortcut: &str, control_socket: &str) -> [Vec<String>; 2] {
    [
        vec![
            "config".to_string(),
            "set".to_string(),
            "--shortcut".to_string(),
            shortcut.to_string(),
        ],
        vec![
            "setup-shortcut".to_string(),
            "--socket".to_string(),
            control_socket.to_string(),
        ],
    ]
}

#[cfg(feature = "gtk_ui")]
fn apply_shortcut_via_hotkeyd(shortcut: &str, control_socket: &str) -> Result<(), String> {
    let program = resolve_hop_hotkeyd_program();
    let steps = build_shortcut_apply_steps(shortcut, control_socket);

    // Step 1: config set must report applied=true.
    let config_set = ProcessCommand::new(&program)
        .args(&steps[0])
        .output()
        .map_err(|error| format!("Shortcut apply failed: {error}"))?;
    if !config_set.status.success() {
        let stderr = String::from_utf8_lossy(&config_set.stderr).trim().to_string();
        let detail = if stderr.is_empty() {
            "config set exited non-zero".to_string()
        } else {
            stderr
        };
        return Err(format!("Shortcut apply failed: {detail}"));
    }
    let config_payload: serde_json::Value = serde_json::from_slice(&config_set.stdout)
        .map_err(|error| format!("Shortcut apply failed: invalid config set response ({error})"))?;
    let applied = config_payload
        .get("applied")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if !applied {
        let warning = config_payload
            .get("warnings")
            .and_then(serde_json::Value::as_array)
            .and_then(|arr| arr.first())
            .and_then(serde_json::Value::as_str)
            .unwrap_or("shortcut rejected by hotkeyd");
        return Err(format!("Shortcut apply failed: {warning}"));
    }

    // Step 2: compositor setup wiring.
    let setup = ProcessCommand::new(&program)
        .args(&steps[1])
        .output()
        .map_err(|error| format!("Shortcut apply failed: {error}"))?;
    if !setup.status.success() {
        let stderr = String::from_utf8_lossy(&setup.stderr).trim().to_string();
        let detail = if stderr.is_empty() {
            "setup-shortcut exited non-zero".to_string()
        } else {
            stderr
        };
        return Err(format!("Shortcut apply failed: {detail}"));
    }

    Ok(())
}

#[cfg(feature = "gtk_ui")]
fn normalize_accel_value(raw: &str) -> String {
    raw.trim()
        .trim_matches('\'')
        .trim_matches('"')
        .replace(' ', "")
        .to_ascii_lowercase()
}

#[cfg(feature = "gtk_ui")]
fn extract_gsettings_path_items(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\'' {
            continue;
        }
        let mut item = String::new();
        for next in chars.by_ref() {
            if next == '\'' {
                break;
            }
            item.push(next);
        }
        if !item.trim().is_empty() {
            out.push(item);
        }
    }
    out
}

#[cfg(feature = "gtk_ui")]
fn parse_gsettings_recursive_conflict_line(line: &str, target: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut parts = trimmed.split_whitespace();
    let schema = parts.next()?;
    let key = parts.next()?;
    let value = parts.collect::<Vec<_>>().join(" ");
    let normalized_target = normalize_accel_value(target);
    let normalized_value = normalize_accel_value(&value);
    if normalized_target.is_empty() || !normalized_value.contains(&normalized_target) {
        return None;
    }
    Some(format!("{schema} {key}"))
}

#[cfg(feature = "gtk_ui")]
fn run_gsettings(args: &[&str]) -> Result<String, String> {
    let output = ProcessCommand::new("gsettings")
        .args(args)
        .output()
        .map_err(|error| format!("gsettings failed: {error}"))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if detail.is_empty() {
            "gsettings exited non-zero".to_string()
        } else {
            detail
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(feature = "gtk_ui")]
fn detect_gnome_shortcut_conflict(shortcut: &str, control_socket: &str) -> Option<String> {
    let normalized_target = normalize_accel_value(shortcut);
    if normalized_target.is_empty() {
        return None;
    }

    if let Ok(lines) = run_gsettings(&["list-recursively", "org.gnome.desktop.wm.keybindings"]) {
        for line in lines.lines() {
            if let Some(owner) = parse_gsettings_recursive_conflict_line(line, shortcut) {
                return Some(owner);
            }
        }
    }

    let custom_paths_raw = run_gsettings(&[
        "get",
        "org.gnome.settings-daemon.plugins.media-keys",
        "custom-keybindings",
    ]).ok()?;
    let custom_paths = extract_gsettings_path_items(&custom_paths_raw);
    for path in custom_paths {
        let schema = format!(
            "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}",
            path
        );
        let binding = run_gsettings(&["get", &schema, "binding"]).ok()?;
        if normalize_accel_value(&binding) != normalized_target {
            continue;
        }
        let name = run_gsettings(&["get", &schema, "name"]).unwrap_or_else(|_| "'Custom binding'".to_string());
        let command = run_gsettings(&["get", &schema, "command"]).unwrap_or_default();
        let control_marker = format!("--socket {control_socket}");
        let is_ours = command.contains("hop-hotkeyd trigger")
            && (command.contains(&control_marker) || command.contains("hop-hotkeyd trigger"));
        if !is_ours {
            return Some(format!(
                "{} ({})",
                name.trim_matches('\''),
                command.trim_matches('\'')
            ));
        }
    }

    None
}

#[cfg(feature = "gtk_ui")]
fn detect_shortcut_conflict_owner(shortcut: &str, control_socket: &str) -> Option<String> {
    let desktop = format!(
        "{}:{}",
        std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
        std::env::var("XDG_SESSION_DESKTOP").unwrap_or_default()
    )
    .to_ascii_lowercase();
    if desktop.contains("gnome") {
        return detect_gnome_shortcut_conflict(shortcut, control_socket);
    }
    None
}

#[cfg(feature = "gtk_ui")]
fn shortcut_setup_hint() -> String {
    let desktop = format!(
        "{}:{}",
        std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
        std::env::var("XDG_SESSION_DESKTOP").unwrap_or_default()
    )
    .to_lowercase();
    if desktop.contains("gnome") {
        return "GNOME: Apply writes a custom keybinding via gsettings.".to_string();
    }
    if desktop.contains("kde") || desktop.contains("plasma") {
        return "KDE: Apply prints KGlobalAccel setup instructions if direct DBus wiring is unavailable.".to_string();
    }
    if std::env::var("SWAYSOCK").is_ok() {
        return "Sway: use `hop-hotkeyd print-bindings` and add the bindsym snippet to your sway config."
            .to_string();
    }
    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
        return "Hyprland: use `hop-hotkeyd print-bindings` and add the bind snippet to hyprland.conf."
            .to_string();
    }
    "If shortcut setup fails, run `~/.local/bin/hop-hotkeyd setup-shortcut --dry-run` for diagnostics."
        .to_string()
}

#[cfg(feature = "gtk_ui")]
fn toggle_accelerators_for_state(configured: &str, is_capturing: bool) -> Vec<String> {
    if is_capturing {
        return Vec::new();
    }
    vec![configured.to_string(), toggle_accelerator().to_string()]
}

#[cfg(feature = "gtk_ui")]
fn apply_toggle_accelerators(app: &adw::Application, configured: &str, is_capturing: bool) {
    let values = toggle_accelerators_for_state(configured, is_capturing);
    let refs = values.iter().map(|v| v.as_str()).collect::<Vec<_>>();
    app.set_accels_for_action("app.toggle", &refs);
}

#[cfg(feature = "gtk_ui")]
fn should_hide_on_focus_loss(is_active: bool, is_shown: bool) -> bool {
    is_shown && !is_active
}

#[cfg(feature = "gtk_ui")]
fn persist_global_shortcut_setting(
    settings: &Rc<RefCell<LauncherUiSettings>>,
    raw: &str,
) -> Result<String, String> {
    let value = sanitize_shortcut(raw);
    let mut next = settings.borrow().clone();
    next.global_shortcut = value.clone();
    save_ui_settings(&next)?;
    *settings.borrow_mut() = next;
    Ok(value)
}

#[cfg(feature = "gtk_ui")]
fn global_shortcut_warning_from_status_payload(payload: &serde_json::Value) -> Option<String> {
    let control_socket_reachable = payload
        .get("control_socket_reachable")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    if !control_socket_reachable {
        return Some(
            "Global shortcut not active: hop-hotkeyd cannot reach launcher control socket."
                .to_string(),
        );
    }

    let applied = payload
        .get("applied")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if !applied {
        let compositor = payload
            .get("wayland_compositor")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("current compositor");
        return Some(format!(
            "Global shortcut not active for {}. Run `hop-hotkeyd setup-shortcut --dry-run` and apply the suggested binding.",
            compositor
        ));
    }

    None
}

#[cfg(feature = "gtk_ui")]
fn probe_global_shortcut_warning(control_socket_path: &str) -> Option<String> {
    let mut cmd = ProcessCommand::new(resolve_hop_hotkeyd_program());

    let output = cmd
        .args(["status", "--socket", control_socket_path])
        .output();
    let output = match output {
        Ok(output) => output,
        Err(error) => {
            return Some(format!(
                "Global shortcut probe failed: unable to run hop-hotkeyd status ({error})"
            ));
        }
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = if stderr.is_empty() {
            "non-zero exit status".to_string()
        } else {
            stderr
        };
        return Some(format!(
            "Global shortcut probe failed: hop-hotkeyd status error ({detail})"
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let payload: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(payload) => payload,
        Err(error) => {
            return Some(format!(
                "Global shortcut probe failed: invalid status JSON ({error})"
            ));
        }
    };

    global_shortcut_warning_from_status_payload(&payload)
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
    let blur_strength_percent = json
        .get("blur_strength_percent")
        .and_then(serde_json::Value::as_i64)
        .map(sanitize_blur_strength_percent)
        .or_else(|| {
            json.get("blur_mode")
                .and_then(serde_json::Value::as_str)
                .map(legacy_blur_mode_to_percent)
        })
        .unwrap_or(default.blur_strength_percent);
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
    let advanced_mode_enabled = json
        .get("advanced_mode_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default.advanced_mode_enabled);
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
    let web_search_services_json = json
        .get("web_search_services_json")
        .and_then(serde_json::Value::as_str)
        .map(|raw| {
            let rows = parse_web_search_services_json(raw, true);
            serialize_web_search_services_json(&rows, true)
        })
        .unwrap_or(default.web_search_services_json);
    let global_shortcut = json
        .get("global_shortcut")
        .and_then(serde_json::Value::as_str)
        .map(sanitize_shortcut)
        .unwrap_or(default.global_shortcut);

    LauncherUiSettings {
        overlay_opacity_percent: overlay,
        blur_strength_percent,
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
        advanced_mode_enabled,
        currency_refresh_enabled,
        currency_rate_ttl_hours,
        web_search_enabled,
        web_search_max_actions,
        web_search_services_json,
        global_shortcut,
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
        "blur_strength_percent": settings.blur_strength_percent,
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
        "advanced_mode_enabled": settings.advanced_mode_enabled,
        "currency_refresh_enabled": settings.currency_refresh_enabled,
        "currency_rate_ttl_hours": settings.currency_rate_ttl_hours,
        "web_search_enabled": settings.web_search_enabled,
        "web_search_max_actions": settings.web_search_max_actions,
        "web_search_services_json": settings.web_search_services_json,
        "global_shortcut": settings.global_shortcut,
    });
    let encoded = serde_json::to_string_pretty(&payload)
        .map_err(|error| format!("encode settings failed: {error}"))?;
    fs::write(path, encoded).map_err(|error| format!("write settings failed: {error}"))
}

#[cfg(feature = "gtk_ui")]
fn sync_settings_to_hopd(socket_path: &str, settings: &LauncherUiSettings) {
    let values = [
        ("ui.overlay_opacity_percent", serde_json::json!(settings.overlay_opacity_percent)),
        (
            "ui.blur_strength_percent",
            serde_json::json!(settings.blur_strength_percent),
        ),
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
        ("ui.debounce_ms", serde_json::json!(settings.debounce_ms)),
        ("ui.density_mode", serde_json::json!(settings.density_mode)),
        ("search.indexed_folders", serde_json::json!(settings.indexed_folders)),
        (
            "currency.refresh_enabled",
            serde_json::json!(settings.currency_refresh_enabled),
        ),
        (
            "currency.rate_ttl_hours",
            serde_json::json!(settings.currency_rate_ttl_hours),
        ),
        ("features.web_search", serde_json::json!(settings.web_search_enabled)),
        (
            "web_search.max_actions",
            serde_json::json!(settings.web_search_max_actions),
        ),
        (
            "web_search.services_json",
            serde_json::json!(settings.web_search_services_json),
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
fn apply_blur_class(window: &adw::ApplicationWindow, strength_percent: i32) {
    for class_name in [
        "hop-blur-none",
        "hop-blur-low",
        "hop-blur-medium",
        "hop-blur-high",
    ] {
        window.remove_css_class(class_name);
    }
    let class_name = match strength_percent.clamp(0, 100) {
        0 => "hop-blur-none",
        1..=33 => "hop-blur-low",
        34..=66 => "hop-blur-medium",
        _ => "hop-blur-high",
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
fn default_web_search_services() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "id": "google",
            "name": "Google",
            "urlTemplate": "https://www.google.com/search?q=%s",
            "enabled": true,
            "keyword": "g"
        }),
        serde_json::json!({
            "id": "duckduckgo",
            "name": "DuckDuckGo",
            "urlTemplate": "https://duckduckgo.com/?q=%s",
            "enabled": true,
            "keyword": "ddg"
        }),
    ]
}

#[cfg(feature = "gtk_ui")]
fn default_web_search_services_json() -> String {
    serde_json::to_string(&default_web_search_services()).unwrap_or_else(|_| "[]".to_string())
}

#[cfg(feature = "gtk_ui")]
fn normalize_web_search_id(name: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in name.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    let normalized = out.trim_matches('-').to_string();
    if normalized.is_empty() {
        "service-custom".to_string()
    } else {
        normalized
    }
}

#[cfg(feature = "gtk_ui")]
fn validate_web_search_service_row(row: &serde_json::Value) -> Option<serde_json::Value> {
    let name = row.get("name")?.as_str()?.trim().to_string();
    if name.is_empty() {
        return None;
    }
    let template = row
        .get("urlTemplate")
        .and_then(serde_json::Value::as_str)
        .or_else(|| row.get("url").and_then(serde_json::Value::as_str))
        .or_else(|| row.get("template").and_then(serde_json::Value::as_str))
        .unwrap_or_default()
        .trim()
        .to_string();
    if template.is_empty() || !template.contains("%s") {
        return None;
    }
    let candidate = template.replace("%s", "query");
    if !candidate.starts_with("https://") {
        return None;
    }
    let id = row
        .get("id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| normalize_web_search_id(&name));
    let enabled = row
        .get("enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    let keyword = row
        .get("keyword")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    Some(serde_json::json!({
        "id": id,
        "name": name,
        "urlTemplate": template,
        "enabled": enabled,
        "keyword": keyword
    }))
}

#[cfg(feature = "gtk_ui")]
fn parse_web_search_services_json(raw: &str, fallback_to_defaults: bool) -> Vec<serde_json::Value> {
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw) else {
        return if fallback_to_defaults {
            default_web_search_services()
        } else {
            Vec::new()
        };
    };
    let Some(rows) = parsed.as_array() else {
        return if fallback_to_defaults {
            default_web_search_services()
        } else {
            Vec::new()
        };
    };
    let mut out = Vec::new();
    for row in rows {
        if let Some(valid) = validate_web_search_service_row(row) {
            out.push(valid);
        }
    }
    if out.is_empty() {
        if fallback_to_defaults {
            default_web_search_services()
        } else {
            Vec::new()
        }
    } else {
        out
    }
}

#[cfg(feature = "gtk_ui")]
fn serialize_web_search_services_json(
    rows: &[serde_json::Value],
    fallback_to_defaults: bool,
) -> String {
    let mut valid = Vec::new();
    for row in rows {
        if let Some(next) = validate_web_search_service_row(row) {
            valid.push(next);
        }
    }
    if valid.is_empty() {
        if fallback_to_defaults {
            return default_web_search_services_json();
        }
        return "[]".to_string();
    }
    serde_json::to_string(&valid).unwrap_or_else(|_| "[]".to_string())
}

#[cfg(all(feature = "gtk_ui", test))]
fn canonical_web_search_services_json(raw: &str, fallback_to_defaults: bool) -> Result<String, String> {
    let parsed = serde_json::from_str::<serde_json::Value>(raw)
        .map_err(|error| format!("Invalid web-search providers JSON: {error}"))?;
    let rows = parsed
        .as_array()
        .ok_or_else(|| "Web-search providers must be a JSON array".to_string())?;
    Ok(serialize_web_search_services_json(rows, fallback_to_defaults))
}

#[cfg(feature = "gtk_ui")]
fn persist_web_search_services_json(
    settings: &Rc<RefCell<LauncherUiSettings>>,
    socket_path: &str,
    status_label: &gtk::Label,
    rows: &[serde_json::Value],
    success_message: &str,
) -> Result<(), String> {
    let canonical = serialize_web_search_services_json(rows, false);
    let mut next = settings.borrow().clone();
    next.web_search_services_json = canonical;
    save_ui_settings(&next)?;
    config_set(
        socket_path,
        "web_search.services_json",
        serde_json::json!(next.web_search_services_json),
    )?;
    *settings.borrow_mut() = next;
    set_settings_feedback(status_label, success_message, false);
    Ok(())
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
            .default_height(260)
            .build();
        {
            let settings = ui_settings.borrow().clone();
            window.set_opacity(settings.overlay_opacity_percent as f64 / 100.0);
            window.set_decorated(!settings.frameless_window);
            apply_density_class(&window, &settings.density_mode);
            apply_blur_class(&window, settings.blur_strength_percent);
        }
        window.set_hide_on_close(true);
        window.set_modal(false);
        window.set_can_focus(true);
        window.set_focus_visible(true);
        window.add_css_class("hop-launcher-window");

        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(4)
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
        list.add_css_class("hop-launcher-list");
        let list_scroller = gtk::ScrolledWindow::builder()
            .hexpand(true)
            .max_content_height(420)
            .build();
        list_scroller.set_propagate_natural_height(true);
        list_scroller.set_vexpand(false);
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
            settings_accelerators(),
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
        let configured_shortcut = sanitize_shortcut(&ui_settings.borrow().global_shortcut);
        apply_toggle_accelerators(app, &configured_shortcut, false);

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
        {
            let ui_settings = ui_settings.clone();
            window.connect_is_active_notify(move |window| {
                if should_hide_on_focus_loss(window.is_active(), window.has_css_class("hop-shown")) {
                    hide_window(window, &ui_settings.borrow());
                }
            });
        }
        sync_settings_to_hopd(&socket_path, &ui_settings.borrow());
        {
            let status = status.clone();
            let control_socket_path = default_control_socket_path();
            let (probe_tx, probe_rx) = std::sync::mpsc::channel::<Option<String>>();
            std::thread::spawn(move || {
                let _ = probe_tx.send(probe_global_shortcut_warning(&control_socket_path));
            });
            gtk::glib::timeout_add_local(Duration::from_millis(25), move || match probe_rx.try_recv() {
                Ok(Some(warning)) => {
                    eprintln!("[hop-launcher] {}", warning);
                    status.set_text(&warning);
                    gtk::glib::ControlFlow::Break
                }
                Ok(None) => gtk::glib::ControlFlow::Break,
                Err(std::sync::mpsc::TryRecvError::Empty) => gtk::glib::ControlFlow::Continue,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    eprintln!("[hop-launcher] global shortcut probe worker disconnected");
                    gtk::glib::ControlFlow::Break
                }
            });
        }

        {
            let list = list.clone();
            let list_scroller = list_scroller.clone();
            let window = window.clone();
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
                let list_scroller = list_scroller.clone();
                let window = window.clone();
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
                            &window,
                            &list,
                            &list_scroller,
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
                            let query_text = entry.text().to_string();
                            let _ = learning_record(&socket_path, &query_text, &result.id);
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
                        let query_text = entry.text().to_string();
                        let _ = learning_record(&socket_path, &query_text, &result.id);
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
            let list_scroller = list_scroller.clone();
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
                        move_selection(&list, &list_scroller, 1);
                        true.into()
                    }
                    gtk::gdk::Key::Up => {
                        move_selection(&list, &list_scroller, -1);
                        true.into()
                    }
                    gtk::gdk::Key::j if is_ctrl => {
                        move_selection(&list, &list_scroller, 1);
                        true.into()
                    }
                    gtk::gdk::Key::k if is_ctrl => {
                        move_selection(&list, &list_scroller, -1);
                        true.into()
                    }
                    gtk::gdk::Key::Tab if is_shift => {
                        move_selection(&list, &list_scroller, -1);
                        true.into()
                    }
                    gtk::gdk::Key::ISO_Left_Tab => {
                        move_selection(&list, &list_scroller, -1);
                        true.into()
                    }
                    gtk::gdk::Key::Tab => {
                        move_selection(&list, &list_scroller, 1);
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
                            let query_text = entry.text().to_string();
                            let _ = learning_record(&socket_path, &query_text, &result.id);
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
                        let query_text = entry.text().to_string();
                        let _ = learning_record(&socket_path, &query_text, &result.id);
                        status.set_text(&render_status_text(QueryState::Executed));
                        hide_window(&window, &ui_settings.borrow());
                        entry.set_text("");
                    }
                }
            });
        }

        refresh_results(
            &window,
            &list,
            &list_scroller,
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
    if window.has_css_class("hop-shown") {
        hide_window(window, settings);
    } else {
        present_window(window, entry, settings);
    }
}

#[cfg(feature = "gtk_ui")]
fn present_window(window: &adw::ApplicationWindow, entry: &gtk::Entry, settings: &LauncherUiSettings) {
    window.add_css_class("hop-shown");
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
    window.remove_css_class("hop-shown");
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
fn launcher_css() -> &'static str {
    r#"
.hop-launcher-window {
  background: transparent;
  border: none;
  outline: none;
  box-shadow: none;
}

.hop-launcher-content {
  border-radius: 14px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  outline: none;
  box-shadow: none;
  background: rgba(20, 21, 24, 0.90);
  color: rgba(255, 255, 255, 0.92);
}

.hop-launcher-window.hop-blur-none .hop-launcher-content {
  background: rgba(20, 21, 24, 0.96);
}

.hop-launcher-window.hop-blur-low .hop-launcher-content {
  background: rgba(20, 21, 24, 0.90);
}

.hop-launcher-window.hop-blur-medium .hop-launcher-content {
  background: rgba(20, 21, 24, 0.84);
}

.hop-launcher-window.hop-blur-high .hop-launcher-content {
  background: rgba(20, 21, 24, 0.76);
}

.hop-launcher-settings-button {
  min-width: 32px;
  min-height: 32px;
}

.hop-launcher-entry {
  min-height: 40px;
  border-radius: 10px;
  border: 1px solid rgba(255, 255, 255, 0.07);
  background: rgba(255, 255, 255, 0.04);
  color: rgba(255, 255, 255, 0.92);
  padding: 0 10px;
}

.hop-launcher-status {
  margin-bottom: 2px;
}

.hop-launcher-subtitle {
  font-size: 0.92em;
}

.hop-launcher-scroll {
  border-radius: 10px;
  border: 1px solid rgba(255, 255, 255, 0.06);
  outline: none;
  box-shadow: none;
  background: rgba(13, 14, 16, 0.46);
}

.hop-launcher-list {
  padding: 2px;
}

.hop-launcher-list row {
  margin: 2px 2px;
  border-radius: 8px;
  border: 1px solid transparent;
  background: transparent;
  transition: 80ms ease;
  outline: none;
}

.hop-launcher-list row:hover {
  background: rgba(255, 255, 255, 0.05);
}

.hop-launcher-list row:selected {
  background: rgba(255, 255, 255, 0.10);
  border: 1px solid rgba(255, 255, 255, 0.12);
  outline: none;
}

.hop-launcher-action-hint {
  font-size: 0.76em;
  min-width: 44px;
  opacity: 0.0;
  transition: 120ms ease;
}

.hop-launcher-list row:hover .hop-launcher-action-hint,
.hop-launcher-list row:selected .hop-launcher-action-hint {
  opacity: 0.82;
}

.hop-launcher-row-body {
  min-height: 40px;
}

.hop-launcher-title-text {
  font-weight: 580;
}

.dim-label {
  color: rgba(255, 255, 255, 0.58);
}

.hop-launcher-window.hop-density-compact .hop-launcher-list row {
  margin: 1px 2px;
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
  margin: 3px 2px;
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
  margin-right: 2px;
}

.hop-launcher-icon-app {
  padding: 2px;
}

.hop-launcher-icon-window {
  padding: 1px;
}

.hop-settings-status {
  margin-top: 4px;
}

.hop-settings-status-error {
  color: @error_color;
}
"#
}

#[cfg(feature = "gtk_ui")]
fn install_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(launcher_css());
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

#[cfg(feature = "gtk_ui")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeightConstraintOp {
    SetMin(i32),
    SetMax(i32),
}

#[cfg(feature = "gtk_ui")]
fn scroller_height_ops(target_height: i32) -> [HeightConstraintOp; 4] {
    let target = target_height.max(1);
    [
        HeightConstraintOp::SetMin(-1),
        HeightConstraintOp::SetMax(-1),
        HeightConstraintOp::SetMin(target),
        HeightConstraintOp::SetMax(target),
    ]
}

#[cfg(feature = "gtk_ui")]
fn apply_scroller_height(list_scroller: &gtk::ScrolledWindow, target_height: i32) {
    for op in scroller_height_ops(target_height) {
        match op {
            HeightConstraintOp::SetMin(height) => list_scroller.set_min_content_height(height),
            HeightConstraintOp::SetMax(height) => list_scroller.set_max_content_height(height),
        }
    }
}

#[cfg(feature = "gtk_ui")]
fn target_height_for_natural_list_height(natural_height: i32, max_height: i32) -> i32 {
    natural_height.max(1).min(max_height.max(1))
}

#[cfg(feature = "gtk_ui")]
fn window_height_for_target_height(target_height: i32) -> i32 {
    // Non-list chrome: content margins + entry + spacing.
    (84 + target_height).clamp(120, 620)
}

#[cfg(feature = "gtk_ui")]
fn apply_window_height(window: &adw::ApplicationWindow, height: i32) {
    // Reset constraints first so the window can shrink below its previous size.
    window.set_size_request(-1, -1);
    window.set_default_size(900, height);
    window.set_size_request(900, height);
}

#[cfg(feature = "gtk_ui")]
fn scroll_value_for_row_visibility(
    current_value: f64,
    page_size: f64,
    row_top: f64,
    row_height: f64,
    lower: f64,
    upper: f64,
) -> f64 {
    if page_size <= 0.0 {
        return current_value;
    }
    let row_bottom = row_top + row_height.max(0.0);
    let visible_bottom = current_value + page_size;
    let next_value = if row_top < current_value {
        row_top
    } else if row_bottom > visible_bottom {
        row_bottom - page_size
    } else {
        current_value
    };
    let max_value = (upper - page_size).max(lower);
    next_value.clamp(lower, max_value)
}

#[cfg(feature = "gtk_ui")]
fn ensure_row_visible(list_scroller: &gtk::ScrolledWindow, row: &gtk::ListBoxRow) {
    let adjustment = list_scroller.vadjustment();
    let Some(bounds) = row.compute_bounds(list_scroller) else {
        return;
    };
    let next = scroll_value_for_row_visibility(
        adjustment.value(),
        adjustment.page_size(),
        bounds.y() as f64,
        bounds.height() as f64,
        adjustment.lower(),
        adjustment.upper(),
    );
    if (next - adjustment.value()).abs() > f64::EPSILON {
        adjustment.set_value(next);
    }
}

#[cfg(feature = "gtk_ui")]
fn move_selection(list: &gtk::ListBox, list_scroller: &gtk::ScrolledWindow, delta: i32) {
    let current = list.selected_row().map(|row| row.index()).unwrap_or(-1);
    let next = if current < 0 {
        0
    } else {
        (current + delta).max(0)
    };
    if let Some(target) = list.row_at_index(next) {
        list.select_row(Some(&target));
        ensure_row_visible(list_scroller, &target);
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
        .modal(false)
        .build();
    prefs.set_search_enabled(true);

    let page = adw::PreferencesPage::new();
    let appearance = adw::PreferencesGroup::builder()
        .title("Appearance")
        .description("Tune launcher translucency and window chrome.")
        .build();
    let search_group = adw::PreferencesGroup::builder()
        .title("Search")
        .description("Toggle result categories and search parameters.")
        .build();
    let shortcuts_group = adw::PreferencesGroup::builder()
        .title("Shortcuts")
        .description("Global shortcut configuration.")
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

    let blur_row = adw::ActionRow::builder()
        .title("Background blur (%)")
        .subtitle("0 keeps the minimal style. Increase for more glass effect.")
        .build();
    let blur_adjustment = gtk::Adjustment::new(
        settings.borrow().blur_strength_percent as f64,
        0.0,
        100.0,
        1.0,
        10.0,
        0.0,
    );
    let blur = gtk::Scale::new(gtk::Orientation::Horizontal, Some(&blur_adjustment));
    blur.set_draw_value(true);
    blur.set_digits(0);
    blur.set_valign(gtk::Align::Center);
    blur.set_hexpand(false);
    blur.set_size_request(220, -1);
    blur_row.add_suffix(&blur);
    blur_row.set_activatable_widget(Some(&blur));
    {
        let settings = settings.clone();
        let socket_path = socket_path.to_string();
        let parent = parent.clone();
        let settings_status = settings_status.clone();
        blur.connect_value_changed(move |widget| {
            let mut next = settings.borrow().clone();
            next.blur_strength_percent = widget.value().round() as i32;
            apply_blur_class(&parent, next.blur_strength_percent);
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
                "ui.blur_strength_percent",
                serde_json::json!(next.blur_strength_percent),
            )
            {
                set_settings_feedback(
                    &settings_status,
                    &format!("Sync failed: {error}"),
                    true,
                );
                return;
            }
            set_settings_feedback(&settings_status, "Saved blur strength", false);
            *settings.borrow_mut() = next;
        });
    }
    appearance.add(&blur_row);

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
    search_group.add(&results_row);

    let shortcut_row = adw::ActionRow::builder()
        .title("Global shortcut")
        .subtitle("Captured shortcut is auto-applied via hop-hotkeyd.")
        .build();
    let shortcut_value = Rc::new(RefCell::new(settings.borrow().global_shortcut.clone()));
    let shortcut_capturing = Rc::new(RefCell::new(false));
    let shortcut_button = gtk::Button::with_label(&settings.borrow().global_shortcut);
    shortcut_button.set_tooltip_text(Some("Click to record a new shortcut combination."));
    shortcut_button.set_width_request(180);
    shortcut_button.set_focusable(true);
    {
        let app = app.clone();
        let shortcut_button_for_click = shortcut_button.clone();
        let capturing = shortcut_capturing.clone();
        let shortcut_value_for_click = shortcut_value.clone();
        shortcut_button.connect_clicked(move |_| {
            let was_capturing = *capturing.borrow();
            if was_capturing {
                let restored = shortcut_value_for_click.borrow().clone();
                shortcut_button_for_click.set_label(&restored);
                *capturing.borrow_mut() = false;
                apply_toggle_accelerators(&app, &restored, false);
            } else {
                *capturing.borrow_mut() = true;
                shortcut_button_for_click.set_label("Press shortcut\u{2026}");
                apply_toggle_accelerators(&app, "", true);
            }
        });
    }
    {
        let app = app.clone();
        let shortcut_button_for_key = shortcut_button.clone();
        let capturing = shortcut_capturing.clone();
        let shortcut_value_for_key = shortcut_value.clone();
        let settings = settings.clone();
        let settings_status = settings_status.clone();
        let controller = gtk::EventControllerKey::new();
        controller.connect_key_pressed(move |_, key, _, state| {
            if !*capturing.borrow() {
                return false.into();
            }
            if key == gtk::gdk::Key::Escape {
                let restored = shortcut_value_for_key.borrow().clone();
                shortcut_button_for_key.set_label(&restored);
                *capturing.borrow_mut() = false;
                apply_toggle_accelerators(&app, &restored, false);
                return true.into();
            }
            let is_modifier_only = matches!(
                key,
                gtk::gdk::Key::Control_L
                    | gtk::gdk::Key::Control_R
                    | gtk::gdk::Key::Shift_L
                    | gtk::gdk::Key::Shift_R
                    | gtk::gdk::Key::Alt_L
                    | gtk::gdk::Key::Alt_R
                    | gtk::gdk::Key::Super_L
                    | gtk::gdk::Key::Super_R
                    | gtk::gdk::Key::Meta_L
                    | gtk::gdk::Key::Meta_R
                    | gtk::gdk::Key::ISO_Level3_Shift
                    | gtk::gdk::Key::Hyper_L
                    | gtk::gdk::Key::Hyper_R
            );
            if is_modifier_only {
                return true.into();
            }
            let mods = state
                & (gtk::gdk::ModifierType::CONTROL_MASK
                    | gtk::gdk::ModifierType::SHIFT_MASK
                    | gtk::gdk::ModifierType::ALT_MASK
                    | gtk::gdk::ModifierType::SUPER_MASK
                    | gtk::gdk::ModifierType::META_MASK);
            let has_primary_mod = mods.intersects(
                gtk::gdk::ModifierType::CONTROL_MASK
                    | gtk::gdk::ModifierType::ALT_MASK
                    | gtk::gdk::ModifierType::SUPER_MASK
                    | gtk::gdk::ModifierType::META_MASK,
            );
            if !has_primary_mod {
                return true.into();
            }
            let accel = gtk::accelerator_name(key, mods);
            let value = match persist_global_shortcut_setting(&settings, accel.as_str()) {
                Ok(value) => value,
                Err(error) => {
                    set_settings_feedback(
                        &settings_status,
                        &format!("Save failed: {error}"),
                        true,
                    );
                    *capturing.borrow_mut() = false;
                    return true.into();
                }
            };
            shortcut_button_for_key.set_label(&value);
            *shortcut_value_for_key.borrow_mut() = value.clone();
            *capturing.borrow_mut() = false;
            let restored = shortcut_value_for_key.borrow().clone();
            apply_toggle_accelerators(&app, &restored, false);
            let control_socket = default_control_socket_path();
            if let Some(owner) = detect_shortcut_conflict_owner(&value, &control_socket) {
                set_settings_feedback(
                    &settings_status,
                    &format!("Shortcut already used by {owner}. Replace it there first."),
                    true,
                );
                return true.into();
            }
            set_settings_feedback(
                &settings_status,
                "Applying global shortcut...",
                false,
            );
            let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
            let value_for_worker = value.clone();
            std::thread::spawn(move || {
                let outcome = apply_shortcut_via_hotkeyd(&value_for_worker, &control_socket);
                let _ = tx.send(outcome);
            });
            let settings_status_async = settings_status.clone();
            gtk::glib::timeout_add_local(Duration::from_millis(25), move || match rx.try_recv() {
                Ok(Ok(())) => {
                    set_settings_feedback(&settings_status_async, "Applied global shortcut", false);
                    gtk::glib::ControlFlow::Break
                }
                Ok(Err(message)) => {
                    set_settings_feedback(&settings_status_async, &message, true);
                    gtk::glib::ControlFlow::Break
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => gtk::glib::ControlFlow::Continue,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    set_settings_feedback(
                        &settings_status_async,
                        "Shortcut apply failed: worker disconnected",
                        true,
                    );
                    gtk::glib::ControlFlow::Break
                }
            });
            true.into()
        });
        shortcut_button.add_controller(controller);
    }
    let shortcut_controls = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(0)
        .build();
    shortcut_controls.append(&shortcut_button);
    shortcut_row.add_suffix(&shortcut_controls);
    let shortcut_hint_row = adw::ActionRow::builder()
        .title("Shortcut backend hint")
        .subtitle(&shortcut_setup_hint())
        .build();
    shortcut_hint_row.set_activatable(false);
    shortcuts_group.add(&shortcut_row);
    shortcuts_group.add(&shortcut_hint_row);

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
    appearance.add(&animations_row);

    let debounce_row = add_integer_spin_row(
        &search_group,
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
    appearance.add(&density_row);
    let open_anim_row = add_integer_spin_row(
        &appearance,
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
    let close_anim_row = add_integer_spin_row(
        &appearance,
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
        &search_group,
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
        &search_group,
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
        &search_group,
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
        &search_group,
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
        &search_group,
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
        &search_group,
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
    search_group.add(&indexed_row);

    let weight_windows_row = add_integer_spin_row(
        &search_group,
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
    let weight_apps_row = add_integer_spin_row(
        &search_group,
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
    let weight_recents_row = add_integer_spin_row(
        &search_group,
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
    let weight_files_row = add_integer_spin_row(
        &search_group,
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
    let weight_emoji_row = add_integer_spin_row(
        &search_group,
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
    let weight_utility_row = add_integer_spin_row(
        &search_group,
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
    let min_fuzzy_row = add_integer_spin_row(
        &search_group,
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

    // Web search enabled toggle (normal row in Search group)
    add_provider_switch_row(
        &search_group,
        "Web search enabled",
        "Expose web-search actions for non-empty queries.",
        settings.borrow().web_search_enabled,
        "web_search.enabled",
        settings.clone(),
        socket_path,
        &settings_status,
        |state, value| state.web_search_enabled = value,
    );

    // Advanced Search rows
    let currency_refresh_row = add_provider_switch_row(
        &search_group,
        "Currency refresh",
        "Allow online refresh of exchange rates when available.",
        settings.borrow().currency_refresh_enabled,
        "currency.refresh_enabled",
        settings.clone(),
        socket_path,
        &settings_status,
        |state, value| state.currency_refresh_enabled = value,
    );
    let currency_ttl_row = add_integer_spin_row(
        &search_group,
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
    let web_search_max_row = add_integer_spin_row(
        &search_group,
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
    let mut advanced_web_search_widgets: Vec<gtk::Widget> = Vec::new();
    let web_search_header = adw::ActionRow::builder()
        .title("Web search providers")
        .subtitle("Manage provider templates used for `web` and keyword-prefixed queries.")
        .build();
    search_group.add(&web_search_header);
    advanced_web_search_widgets.push(web_search_header.upcast::<gtk::Widget>());
    let current_services =
        parse_web_search_services_json(&settings.borrow().web_search_services_json, false);
    if current_services.is_empty() {
        let empty_row = adw::ActionRow::builder()
            .title("No providers configured")
            .subtitle("Add a provider to enable web actions.")
            .build();
        search_group.add(&empty_row);
        advanced_web_search_widgets.push(empty_row.upcast::<gtk::Widget>());
    }
    for (index, row) in current_services.iter().enumerate() {
        let name = row
            .get("name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Provider")
            .to_string();
        let id = row
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        let url_template = row
            .get("urlTemplate")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        let keyword = row
            .get("keyword")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        let enabled = row
            .get("enabled")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);

        let provider_row = adw::ActionRow::builder()
            .title(format!("Provider {}: {}", index + 1, name))
            .subtitle(if id.is_empty() {
                "Configure and save this provider.".to_string()
            } else {
                format!("id: {id}")
            })
            .build();
        let up_button = gtk::Button::builder()
            .label("Up")
            .valign(gtk::Align::Center)
            .sensitive(index > 0)
            .build();
        let down_button = gtk::Button::builder()
            .label("Down")
            .valign(gtk::Align::Center)
            .sensitive(index + 1 < current_services.len())
            .build();
        let remove_button = gtk::Button::builder()
            .label("Remove")
            .valign(gtk::Align::Center)
            .build();
        remove_button.add_css_class("destructive-action");
        provider_row.add_suffix(&up_button);
        provider_row.add_suffix(&down_button);
        provider_row.add_suffix(&remove_button);
        search_group.add(&provider_row);
        advanced_web_search_widgets.push(provider_row.upcast::<gtk::Widget>());

        let name_row = adw::ActionRow::builder()
            .title("Name")
            .subtitle("Provider label shown in results.")
            .build();
        let name_entry = gtk::Entry::builder().hexpand(true).text(&name).build();
        name_entry.set_valign(gtk::Align::Center);
        name_row.add_suffix(&name_entry);
        name_row.set_activatable_widget(Some(&name_entry));
        search_group.add(&name_row);
        advanced_web_search_widgets.push(name_row.upcast::<gtk::Widget>());

        let template_row = adw::ActionRow::builder()
            .title("URL template")
            .subtitle("Must be https and contain %s placeholder.")
            .build();
        let template_entry = gtk::Entry::builder()
            .hexpand(true)
            .text(&url_template)
            .build();
        template_entry.set_valign(gtk::Align::Center);
        template_row.add_suffix(&template_entry);
        template_row.set_activatable_widget(Some(&template_entry));
        search_group.add(&template_row);
        advanced_web_search_widgets.push(template_row.upcast::<gtk::Widget>());

        let keyword_row = adw::ActionRow::builder()
            .title("Keyword and enabled")
            .subtitle("Keyword enables \u{201c}&lt;keyword&gt; query\u{201d} mode.")
            .build();
        let keyword_entry = gtk::Entry::builder().width_chars(8).text(&keyword).build();
        keyword_entry.set_valign(gtk::Align::Center);
        let enabled_switch = gtk::Switch::builder()
            .active(enabled)
            .valign(gtk::Align::Center)
            .build();
        let save_button = gtk::Button::builder()
            .label("Save")
            .valign(gtk::Align::Center)
            .build();
        keyword_row.add_suffix(&keyword_entry);
        keyword_row.add_suffix(&enabled_switch);
        keyword_row.add_suffix(&save_button);
        search_group.add(&keyword_row);
        advanced_web_search_widgets.push(keyword_row.upcast::<gtk::Widget>());

        {
            let settings = settings.clone();
            let socket_path = socket_path.to_string();
            let settings_status = settings_status.clone();
            let parent = parent.clone();
            let prefs = prefs.clone();
            let app = app.clone();
            let name_entry = name_entry.clone();
            let template_entry = template_entry.clone();
            let keyword_entry = keyword_entry.clone();
            let enabled_switch = enabled_switch.clone();
            save_button.connect_clicked(move |_| {
                let mut services =
                    parse_web_search_services_json(&settings.borrow().web_search_services_json, false);
                if index >= services.len() {
                    set_settings_feedback(&settings_status, "Provider index out of range", true);
                    return;
                }
                let existing_id = services[index]
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let draft_name = name_entry.text().trim().to_string();
                let draft = serde_json::json!({
                    "id": if existing_id.is_empty() { normalize_web_search_id(&draft_name) } else { existing_id },
                    "name": draft_name,
                    "urlTemplate": template_entry.text().trim().to_string(),
                    "keyword": keyword_entry.text().trim().to_string(),
                    "enabled": enabled_switch.is_active()
                });
                let Some(validated) = validate_web_search_service_row(&draft) else {
                    set_settings_feedback(&settings_status, "Invalid provider: require name + https template with %s", true);
                    return;
                };
                services[index] = validated;
                if let Err(error) = persist_web_search_services_json(
                    &settings,
                    &socket_path,
                    &settings_status,
                    &services,
                    "Saved web-search provider",
                ) {
                    set_settings_feedback(&settings_status, &format!("Save failed: {error}"), true);
                    return;
                }
                prefs.close();
                open_settings_window(&app, &parent, settings.clone(), &socket_path);
            });
        }

        {
            let settings = settings.clone();
            let socket_path = socket_path.to_string();
            let settings_status = settings_status.clone();
            let parent = parent.clone();
            let prefs = prefs.clone();
            let app = app.clone();
            up_button.connect_clicked(move |_| {
                if index == 0 {
                    return;
                }
                let mut services =
                    parse_web_search_services_json(&settings.borrow().web_search_services_json, false);
                services.swap(index, index - 1);
                if let Err(error) = persist_web_search_services_json(
                    &settings,
                    &socket_path,
                    &settings_status,
                    &services,
                    "Moved provider up",
                ) {
                    set_settings_feedback(&settings_status, &format!("Save failed: {error}"), true);
                    return;
                }
                prefs.close();
                open_settings_window(&app, &parent, settings.clone(), &socket_path);
            });
        }

        {
            let settings = settings.clone();
            let socket_path = socket_path.to_string();
            let settings_status = settings_status.clone();
            let parent = parent.clone();
            let prefs = prefs.clone();
            let app = app.clone();
            down_button.connect_clicked(move |_| {
                let mut services =
                    parse_web_search_services_json(&settings.borrow().web_search_services_json, false);
                if index + 1 >= services.len() {
                    return;
                }
                services.swap(index, index + 1);
                if let Err(error) = persist_web_search_services_json(
                    &settings,
                    &socket_path,
                    &settings_status,
                    &services,
                    "Moved provider down",
                ) {
                    set_settings_feedback(&settings_status, &format!("Save failed: {error}"), true);
                    return;
                }
                prefs.close();
                open_settings_window(&app, &parent, settings.clone(), &socket_path);
            });
        }

        {
            let settings = settings.clone();
            let socket_path = socket_path.to_string();
            let settings_status = settings_status.clone();
            let parent = parent.clone();
            let prefs = prefs.clone();
            let app = app.clone();
            remove_button.connect_clicked(move |_| {
                let mut services =
                    parse_web_search_services_json(&settings.borrow().web_search_services_json, false);
                if index >= services.len() {
                    return;
                }
                services.remove(index);
                if let Err(error) = persist_web_search_services_json(
                    &settings,
                    &socket_path,
                    &settings_status,
                    &services,
                    "Removed provider",
                ) {
                    set_settings_feedback(&settings_status, &format!("Save failed: {error}"), true);
                    return;
                }
                prefs.close();
                open_settings_window(&app, &parent, settings.clone(), &socket_path);
            });
        }
    }

    let add_provider_row = adw::ActionRow::builder()
        .title("Add web-search provider")
        .subtitle("Appends a new enabled provider row.")
        .build();
    let add_provider_button = gtk::Button::builder()
        .label("Add")
        .valign(gtk::Align::Center)
        .build();
    add_provider_row.add_suffix(&add_provider_button);
    add_provider_row.set_activatable_widget(Some(&add_provider_button));
    {
        let settings = settings.clone();
        let socket_path = socket_path.to_string();
        let settings_status = settings_status.clone();
        let parent = parent.clone();
        let prefs = prefs.clone();
        let app = app.clone();
        add_provider_button.connect_clicked(move |_| {
            let mut services =
                parse_web_search_services_json(&settings.borrow().web_search_services_json, false);
            let next_index = services.len() + 1;
            services.push(serde_json::json!({
                "id": format!("provider-{next_index}"),
                "name": format!("Provider {next_index}"),
                "urlTemplate": "https://example.com/search?q=%s",
                "enabled": true,
                "keyword": ""
            }));
            if let Err(error) = persist_web_search_services_json(
                &settings,
                &socket_path,
                &settings_status,
                &services,
                "Added provider",
            ) {
                set_settings_feedback(&settings_status, &format!("Save failed: {error}"), true);
                return;
            }
            prefs.close();
            open_settings_window(&app, &parent, settings.clone(), &socket_path);
        });
    }
    search_group.add(&add_provider_row);
    advanced_web_search_widgets.push(add_provider_row.upcast::<gtk::Widget>());
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
                        "blur_strength_percent": settings.borrow().blur_strength_percent,
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
                        "advanced_mode_enabled": settings.borrow().advanced_mode_enabled,
                        "currency_refresh_enabled": settings.borrow().currency_refresh_enabled,
                        "currency_rate_ttl_hours": settings.borrow().currency_rate_ttl_hours,
                        "web_search_enabled": settings.borrow().web_search_enabled,
                        "web_search_max_actions": settings.borrow().web_search_max_actions,
                        "web_search_services_json": settings.borrow().web_search_services_json,
                        "global_shortcut": settings.borrow().global_shortcut
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
                apply_blur_class(&parent, imported.blur_strength_percent);
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
            apply_blur_class(&parent, next.blur_strength_percent);
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
    let reset_learning_row = adw::ActionRow::builder()
        .title("Reset learning data")
        .subtitle("Clear all launch history used for ranking. Cannot be undone.")
        .build();
    let reset_learning_button = gtk::Button::builder()
        .label("Reset")
        .valign(gtk::Align::Center)
        .build();
    reset_learning_button.add_css_class("destructive-action");
    reset_learning_row.add_suffix(&reset_learning_button);
    reset_learning_row.set_activatable_widget(Some(&reset_learning_button));
    {
        let socket_path = socket_path.to_string();
        let settings_status = settings_status.clone();
        reset_learning_button.connect_clicked(move |_| {
            if let Err(error) = learning_reset(&socket_path) {
                set_settings_feedback(&settings_status, &format!("Reset learning failed: {error}"), true);
                return;
            }
            set_settings_feedback(&settings_status, "Learning data cleared", false);
        });
    }
    profile.add(&import_row);
    profile.add(&export_row);
    profile.add(&reset_learning_row);
    profile.add(&reset_row);

    // -- Advanced mode toggle group --
    let advanced_toggle_group = adw::PreferencesGroup::builder()
        .title("Advanced")
        .description("Show or hide advanced tuning options.")
        .build();
    let advanced_toggle_row = adw::ActionRow::builder()
        .title("Advanced mode")
        .subtitle("Show ranking weights, animation timing, and web search editor.")
        .build();
    let advanced_switch = gtk::Switch::builder()
        .active(settings.borrow().advanced_mode_enabled)
        .valign(gtk::Align::Center)
        .build();
    advanced_toggle_row.add_suffix(&advanced_switch);
    advanced_toggle_row.set_activatable_widget(Some(&advanced_switch));
    advanced_toggle_group.add(&advanced_toggle_row);

    // Collect all advanced rows for visibility toggling
    let advanced_widgets: Vec<gtk::Widget> = {
        let mut widgets: Vec<gtk::Widget> = vec![
            blur_row.upcast::<gtk::Widget>(),
            animations_row.upcast::<gtk::Widget>(),
            open_anim_row.upcast::<gtk::Widget>(),
            close_anim_row.upcast::<gtk::Widget>(),
            weight_windows_row.upcast::<gtk::Widget>(),
            weight_apps_row.upcast::<gtk::Widget>(),
            weight_recents_row.upcast::<gtk::Widget>(),
            weight_files_row.upcast::<gtk::Widget>(),
            weight_emoji_row.upcast::<gtk::Widget>(),
            weight_utility_row.upcast::<gtk::Widget>(),
            min_fuzzy_row.upcast::<gtk::Widget>(),
            debounce_row.upcast::<gtk::Widget>(),
            currency_refresh_row.upcast::<gtk::Widget>(),
            currency_ttl_row.upcast::<gtk::Widget>(),
            web_search_max_row.upcast::<gtk::Widget>(),
        ];
        widgets.extend(advanced_web_search_widgets);
        widgets
    };

    // Set initial visibility
    let is_advanced = settings.borrow().advanced_mode_enabled;
    for widget in &advanced_widgets {
        widget.set_visible(is_advanced);
    }

    // Wire advanced toggle
    {
        let settings = settings.clone();
        let settings_status = settings_status.clone();
        advanced_switch.connect_active_notify(move |switch| {
            let visible = switch.is_active();
            for widget in &advanced_widgets {
                widget.set_visible(visible);
            }
            let mut next = settings.borrow().clone();
            next.advanced_mode_enabled = visible;
            if let Err(error) = save_ui_settings(&next) {
                set_settings_feedback(
                    &settings_status,
                    &format!("Save failed: {error}"),
                    true,
                );
                return;
            }
            set_settings_feedback(
                &settings_status,
                if visible { "Advanced mode enabled" } else { "Advanced mode disabled" },
                false,
            );
            *settings.borrow_mut() = next;
        });
    }

    page.add(&appearance);
    page.add(&search_group);
    page.add(&shortcuts_group);
    page.add(&advanced_toggle_group);
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
) -> adw::ActionRow {
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
    row
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
) -> adw::ActionRow {
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
    row
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
        "blur_strength_percent": json
            .get("blur_strength_percent")
            .cloned()
            .or_else(|| {
                json.get("blur_mode")
                    .and_then(serde_json::Value::as_str)
                    .map(legacy_blur_mode_to_percent)
                    .map(|value| serde_json::json!(value))
            })
            .unwrap_or(serde_json::json!(default.blur_strength_percent)),
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
        "advanced_mode_enabled": json.get("advanced_mode_enabled").cloned().unwrap_or(serde_json::json!(default.advanced_mode_enabled)),
        "currency_refresh_enabled": json.get("currency_refresh_enabled").cloned().unwrap_or(serde_json::json!(default.currency_refresh_enabled)),
        "currency_rate_ttl_hours": json.get("currency_rate_ttl_hours").cloned().unwrap_or(serde_json::json!(default.currency_rate_ttl_hours)),
        "web_search_enabled": json.get("web_search_enabled").cloned().unwrap_or(serde_json::json!(default.web_search_enabled)),
        "web_search_max_actions": json.get("web_search_max_actions").cloned().unwrap_or(serde_json::json!(default.web_search_max_actions)),
        "web_search_services_json": json.get("web_search_services_json").cloned().unwrap_or(serde_json::json!(default.web_search_services_json)),
        "global_shortcut": json.get("global_shortcut").cloned().unwrap_or(serde_json::json!(default.global_shortcut))
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
    let blur_strength_percent = json
        .get("blur_strength_percent")
        .and_then(serde_json::Value::as_i64)
        .map(sanitize_blur_strength_percent)
        .or_else(|| {
            json.get("blur_mode")
                .and_then(serde_json::Value::as_str)
                .map(legacy_blur_mode_to_percent)
        })
        .unwrap_or(default.blur_strength_percent);
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
    let advanced_mode_enabled = json
        .get("advanced_mode_enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default.advanced_mode_enabled);
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
    let web_search_services_json = json
        .get("web_search_services_json")
        .and_then(serde_json::Value::as_str)
        .map(|raw| {
            let rows = parse_web_search_services_json(raw, true);
            serialize_web_search_services_json(&rows, true)
        })
        .unwrap_or(default.web_search_services_json);
    let global_shortcut = json
        .get("global_shortcut")
        .and_then(serde_json::Value::as_str)
        .map(sanitize_shortcut)
        .unwrap_or(default.global_shortcut);

    LauncherUiSettings {
        overlay_opacity_percent: overlay,
        blur_strength_percent,
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
        advanced_mode_enabled,
        currency_refresh_enabled,
        currency_rate_ttl_hours,
        web_search_enabled,
        web_search_max_actions,
        web_search_services_json,
        global_shortcut,
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
    window: &adw::ApplicationWindow,
    list: &gtk::ListBox,
    list_scroller: &gtk::ScrolledWindow,
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
                icon.set_valign(gtk::Align::Center);
                icon.set_halign(gtk::Align::Center);
                icon.set_size_request(24, -1);
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
                let text = gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .spacing(2)
                    .hexpand(true)
                    .valign(gtk::Align::Center)
                    .build();
                text.append(&title);
                if !row.subtitle.trim().is_empty() {
                    let subtitle = gtk::Label::builder()
                        .xalign(0.0)
                        .label(&row.subtitle)
                        .build();
                    subtitle.add_css_class("dim-label");
                    subtitle.add_css_class("hop-launcher-subtitle");
                    text.append(&subtitle);
                }

                let action_hint = gtk::Label::builder()
                    .label(action_hint_for_row(row))
                    .xalign(1.0)
                    .valign(gtk::Align::Center)
                    .build();
                action_hint.add_css_class("dim-label");
                action_hint.add_css_class("hop-launcher-action-hint");

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
                body.append(&action_hint);
                let item_row = gtk::ListBoxRow::new();
                item_row.set_child(Some(&body));
                item_row.add_css_class("hop-launcher-row");
                list.append(&item_row);
            }
            if list.first_child().is_some() {
                if let Some(first) = list.row_at_index(0) {
                    list.select_row(Some(&first));
                    ensure_row_visible(list_scroller, &first);
                }
            }
            let (_, natural_height, _, _) = list.measure(gtk::Orientation::Vertical, -1);
            let target_height = target_height_for_natural_list_height(natural_height, 420);
            apply_scroller_height(list_scroller, target_height);
            apply_window_height(window, window_height_for_target_height(target_height));
            status.set_text(&render_status_text(if rows.is_empty() {
                QueryState::Empty
            } else {
                QueryState::Results { count: rows.len() }
            }));
        }
        Err(error) => {
            results.borrow_mut().clear();
            apply_scroller_height(list_scroller, 1);
            apply_window_height(window, window_height_for_target_height(1));
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
    if matches!(row.kind.as_str(), "file" | "recent") {
        if let Some(path) = result_local_path(row) {
            let file = gio::File::for_path(path);
            if let Ok(info) =
                file.query_info("standard::icon", gio::FileQueryInfoFlags::NONE, None::<&gio::Cancellable>)
            {
                if let Some(icon) = info.icon() {
                    return gtk::Image::from_gicon(&icon);
                }
            }
        }
    }
    if row.kind == "window" {
        if let Some(desktop_id) = resolve_desktop_id_for_window_hint(&row.icon) {
            if let Some(app_info) = gio::DesktopAppInfo::new(&desktop_id) {
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
fn result_local_path(row: &LauncherResult) -> Option<String> {
    let raw = if let Some(path) = row.id.strip_prefix("file:") {
        path
    } else if let Some(path) = row.id.strip_prefix("recent:") {
        path
    } else {
        return None;
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    gtk::glib::uri_unescape_string(trimmed, None::<&str>)
        .map(|value| value.to_string())
        .or_else(|| Some(trimmed.to_string()))
}

#[cfg(feature = "gtk_ui")]
fn resolve_desktop_id_for_window_hint(raw: &str) -> Option<String> {
    let hint = raw.trim();
    if hint.is_empty() {
        return None;
    }
    if hint.ends_with(".desktop") {
        return Some(hint.to_string());
    }

    let index = desktop_icon_index();
    for alias in alias_candidates(hint) {
        if let Some(found) = index.get(&alias) {
            return Some(found.clone());
        }
    }
    None
}

#[cfg(feature = "gtk_ui")]
fn desktop_icon_index() -> &'static HashMap<String, String> {
    static INDEX: OnceLock<HashMap<String, String>> = OnceLock::new();
    INDEX.get_or_init(build_desktop_icon_index)
}

#[cfg(feature = "gtk_ui")]
fn build_desktop_icon_index() -> HashMap<String, String> {
    let mut index = HashMap::new();
    let mut roots = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        roots.push(Path::new(&home).join(".local/share/applications"));
        roots.push(Path::new(&home).join(".local/share/flatpak/exports/share/applications"));
    }
    roots.push(PathBuf::from("/usr/share/applications"));
    roots.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));

    for root in roots {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("desktop") {
                continue;
            }
            let Some(desktop_id) = path
                .file_name()
                .and_then(|name| name.to_str())
                .map(ToString::to_string)
            else {
                continue;
            };
            let stem = path
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            add_desktop_aliases(&mut index, stem, &desktop_id);
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            for line in content.lines() {
                if let Some(value) = line.strip_prefix("Icon=") {
                    add_desktop_aliases(&mut index, value.trim(), &desktop_id);
                } else if let Some(value) = line.strip_prefix("StartupWMClass=") {
                    add_desktop_aliases(&mut index, value.trim(), &desktop_id);
                }
            }
        }
    }

    index
}

#[cfg(feature = "gtk_ui")]
fn add_desktop_aliases(index: &mut HashMap<String, String>, raw: &str, desktop_id: &str) {
    for alias in alias_candidates(raw) {
        index
            .entry(alias)
            .or_insert_with(|| desktop_id.to_string());
    }
}

#[cfg(feature = "gtk_ui")]
fn alias_candidates(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut push_alias = |value: String| {
        let trimmed = value.trim().trim_matches('.').trim_matches('-').to_string();
        if trimmed.is_empty() {
            return;
        }
        if seen.insert(trimmed.clone()) {
            out.push(trimmed);
        }
    };

    let lowered = raw.trim().to_lowercase();
    if lowered.is_empty() {
        return out;
    }
    let base = lowered
        .strip_suffix(".desktop")
        .unwrap_or(lowered.as_str())
        .to_string();
    push_alias(base.clone());
    push_alias(base.replace('_', "-"));
    push_alias(base.replace('.', "-").replace('_', "-"));

    let tokens = base
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    if tokens.is_empty() {
        return out;
    }

    for token in &tokens {
        push_alias((*token).to_string());
    }
    push_alias(tokens.join("-"));
    if tokens.len() >= 2 {
        push_alias(tokens[tokens.len() - 2..].join("-"));
    }
    if tokens.len() >= 3 {
        push_alias(tokens[tokens.len() - 3..].join("-"));
    }
    if ["org", "io", "com", "net", "app", "dev"].contains(&tokens[0]) && tokens.len() > 1 {
        push_alias(tokens[1..].join("-"));
    }

    out
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

#[cfg(all(test, feature = "gtk_ui"))]
mod tests {
    use super::*;

    fn write_temp_settings(json: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("hop-launcher-gtk-settings-{nanos}.json"));
        std::fs::write(&path, json).expect("write temp settings");
        path
    }

    #[test]
    fn default_settings_start_with_minimal_blur() {
        let settings = LauncherUiSettings::default();
        assert_eq!(settings.blur_strength_percent, 0);
    }

    #[test]
    fn settings_loader_migrates_legacy_blur_mode_values() {
        let path = write_temp_settings(r#"{"blur_mode":"strong"}"#);
        let settings = load_ui_settings_from_path(&path);
        let _ = std::fs::remove_file(path);
        assert_eq!(settings.blur_strength_percent, 70);
    }

    #[test]
    fn settings_loader_prefers_numeric_blur_strength_percent() {
        let path = write_temp_settings(r#"{"blur_strength_percent":22,"blur_mode":"strong"}"#);
        let settings = load_ui_settings_from_path(&path);
        let _ = std::fs::remove_file(path);
        assert_eq!(settings.blur_strength_percent, 22);
    }

    #[test]
    fn launcher_css_does_not_use_margin_end_property() {
        assert!(!launcher_css().contains("margin-end:"));
    }

    #[test]
    fn launcher_css_trims_bottom_tail_after_last_row() {
        let css = launcher_css();
        assert!(css.contains(".hop-launcher-list row {") || css.contains(".hop-launcher-list row
{"));
        assert!(css.contains("margin: 2px 2px;"));
        assert!(!css.contains(".hop-launcher-list row:last-child"));
    }

    #[test]
    fn launcher_css_adds_subtle_content_border_matching_background_tone() {
        let css = launcher_css();
        assert!(css.contains("border: 1px solid rgba(255, 255, 255, 0.08);"));
        assert!(!css.contains(
            ".hop-launcher-window.hop-blur-none .hop-launcher-content {\n  background: rgba(20, 21, 24, 0.96);\n  border-color:"
        ));
    }

    #[test]
    fn scroller_height_ops_reset_constraints_before_target() {
        assert_eq!(
            scroller_height_ops(0),
            [
                HeightConstraintOp::SetMin(-1),
                HeightConstraintOp::SetMax(-1),
                HeightConstraintOp::SetMin(1),
                HeightConstraintOp::SetMax(1),
            ]
        );
    }

    #[test]
    fn window_height_formula_uses_compact_chrome_offset() {
        assert_eq!(window_height_for_target_height(384), 468);
    }

    #[test]
    fn window_height_formula_does_not_force_chin_for_single_row() {
        assert_eq!(window_height_for_target_height(48), 132);
    }

    #[test]
    fn target_height_uses_minimum_for_empty_results() {
        assert_eq!(target_height_for_natural_list_height(0, 420), 1);
    }

    #[test]
    fn target_height_preserves_natural_height_when_within_bounds() {
        assert_eq!(target_height_for_natural_list_height(137, 420), 137);
    }

    #[test]
    fn target_height_clamps_to_max_height() {
        assert_eq!(target_height_for_natural_list_height(680, 420), 420);
    }

    #[test]
    fn scroll_target_moves_down_when_row_below_viewport() {
        let next = scroll_value_for_row_visibility(0.0, 120.0, 140.0, 40.0, 0.0, 500.0);
        assert_eq!(next, 60.0);
    }

    #[test]
    fn scroll_target_moves_up_when_row_above_viewport() {
        let next = scroll_value_for_row_visibility(100.0, 120.0, 40.0, 40.0, 0.0, 500.0);
        assert_eq!(next, 40.0);
    }

    #[test]
    fn web_search_service_validation_accepts_legacy_url_field() {
        let row = serde_json::json!({
            "name": "Kagi",
            "url": "https://kagi.com/search?q=%s",
            "enabled": true,
            "keyword": "kg"
        });
        let out = validate_web_search_service_row(&row).expect("valid web search service");
        assert_eq!(out["urlTemplate"], "https://kagi.com/search?q=%s");
        assert_eq!(out["keyword"], "kg");
    }

    #[test]
    fn web_search_service_validation_rejects_non_https_template() {
        let row = serde_json::json!({
            "name": "Bad",
            "urlTemplate": "http://example.com?q=%s"
        });
        assert!(validate_web_search_service_row(&row).is_none());
    }

    #[test]
    fn parse_web_search_services_json_falls_back_to_defaults_for_malformed_input() {
        let out = parse_web_search_services_json("not-json", true);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn serialize_web_search_services_json_returns_empty_when_all_rows_invalid_and_no_fallback() {
        let rows = vec![serde_json::json!({"name":"Bad","urlTemplate":"http://x.com?q=%s"})];
        assert_eq!(serialize_web_search_services_json(&rows, false), "[]");
    }

    #[test]
    fn canonical_web_search_services_json_rejects_non_array_payload() {
        let out = canonical_web_search_services_json("{}", false);
        assert!(out.is_err());
    }

    #[test]
    fn startup_hotkey_probe_accepts_healthy_status_payload() {
        let payload = serde_json::json!({
            "applied": true,
            "control_socket_reachable": true
        });
        assert_eq!(global_shortcut_warning_from_status_payload(&payload), None);
    }

    #[test]
    fn startup_hotkey_probe_warns_when_shortcut_not_applied() {
        let payload = serde_json::json!({
            "applied": false,
            "control_socket_reachable": true,
            "wayland_compositor": "sway"
        });
        let warning = global_shortcut_warning_from_status_payload(&payload).expect("warning");
        assert!(warning.contains("not active"));
        assert!(warning.contains("sway"));
    }

    #[test]
    fn startup_hotkey_probe_warns_when_control_socket_unreachable() {
        let payload = serde_json::json!({
            "applied": true,
            "control_socket_reachable": false
        });
        let warning = global_shortcut_warning_from_status_payload(&payload).expect("warning");
        assert!(warning.contains("control socket"));
    }

    #[test]
    fn shortcut_apply_steps_use_config_set_then_setup_shortcut() {
        let steps = build_shortcut_apply_steps("<Primary><Shift>ampersand", "/tmp/control.sock");
        assert_eq!(
            steps[0],
            vec![
                "config".to_string(),
                "set".to_string(),
                "--shortcut".to_string(),
                "<Primary><Shift>ampersand".to_string()
            ]
        );
        assert_eq!(
            steps[1],
            vec![
                "setup-shortcut".to_string(),
                "--socket".to_string(),
                "/tmp/control.sock".to_string()
            ]
        );
    }

    #[test]
    fn toggle_accelerators_are_disabled_while_capturing() {
        let active = toggle_accelerators_for_state("<Primary><Shift>ampersand", false);
        assert_eq!(
            active,
            vec![
                "<Primary><Shift>ampersand".to_string(),
                toggle_accelerator().to_string()
            ]
        );

        let during_capture = toggle_accelerators_for_state("<Primary><Shift>ampersand", true);
        assert!(during_capture.is_empty());
    }

    #[test]
    fn normalize_accel_value_strips_quotes_spaces_and_case() {
        assert_eq!(
            normalize_accel_value(" '<Super> space' "),
            "<super>space".to_string()
        );
    }

    #[test]
    fn extract_gsettings_path_items_parses_single_quoted_array() {
        let items = extract_gsettings_path_items(
            "['/org/gnome/x/', '/org/gnome/y/']",
        );
        assert_eq!(
            items,
            vec![
                "/org/gnome/x/".to_string(),
                "/org/gnome/y/".to_string()
            ]
        );
    }

    #[test]
    fn parse_gsettings_recursive_conflict_line_detects_matching_owner() {
        let line =
            "org.gnome.desktop.wm.keybindings switch-windows ['<Super>space', '<Alt>Tab']";
        let owner = parse_gsettings_recursive_conflict_line(line, "<Super>space");
        assert_eq!(
            owner,
            Some("org.gnome.desktop.wm.keybindings switch-windows".to_string())
        );
    }

    #[test]
    fn focus_loss_hides_only_when_launcher_is_shown() {
        assert!(should_hide_on_focus_loss(false, true));
        assert!(!should_hide_on_focus_loss(true, true));
        assert!(!should_hide_on_focus_loss(false, false));
        assert!(!should_hide_on_focus_loss(true, false));
    }

    #[test]
    fn shortcut_apply_rejects_unapplied_config_payload() {
        let payload = serde_json::json!({
            "applied": false,
            "warnings": ["shortcut must include a key"]
        });
        let applied = payload
            .get("applied")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        assert!(!applied);
        let warning = payload
            .get("warnings")
            .and_then(serde_json::Value::as_array)
            .and_then(|arr| arr.first())
            .and_then(serde_json::Value::as_str)
            .unwrap_or("shortcut rejected by hotkeyd");
        assert_eq!(warning, "shortcut must include a key");
    }
}

#[cfg(not(feature = "gtk_ui"))]
fn run() {
    println!("hop-launcher-gtk scaffold ready. Rebuild with --features gtk_ui to run the GTK UI.");
}
