#[cfg(feature = "gtk_ui")]
use std::cell::RefCell;
#[cfg(feature = "gtk_ui")]
use std::rc::Rc;

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
    default_hopd_socket_path, execute, search, start_visible_on_launch, toggle_accelerator,
    LauncherResult,
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
                if window.is_visible() {
                    window.hide();
                } else {
                    window.present();
                }
            });
        }
        app.add_action(&toggle);
        app.set_accels_for_action("app.toggle", &[toggle_accelerator()]);

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
