use adw::prelude::*;
use gtk::glib;

use crate::modules::launcher_manager::{apply_locale, LauncherInfo, LauncherManager};
use crate::ui::dialogs::terminal;

pub fn new() -> (gtk::Box, gtk::FlowBox) {
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build();

    let clamp = adw::Clamp::builder()
        .maximum_size(1000)
        .tightening_threshold(600)
        .build();

    let page_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .build();

    let search_entry = gtk::SearchEntry::builder()
        .placeholder_text("Search launchers and engines (Steam, RetroArch, Prism, Heroic)...")
        .margin_top(16)
        .margin_start(16)
        .margin_end(16)
        .margin_bottom(4)
        .build();

    let flowbox = gtk::FlowBox::builder()
        .max_children_per_line(6)
        .min_children_per_line(2)
        .selection_mode(gtk::SelectionMode::None)
        .homogeneous(true)
        .column_spacing(12)
        .row_spacing(12)
        .margin_top(12)
        .margin_bottom(24)
        .margin_start(12)
        .margin_end(12)
        .build();

    let fb = flowbox.clone();
    let se = search_entry.clone();
    search_entry.connect_search_changed(move |entry| {
        let text = entry.text().to_string();
        populate_flowbox(&fb, &text, &se);
    });

    populate_flowbox(&flowbox, "", &search_entry);

    page_box.append(&search_entry);
    page_box.append(&flowbox);

    clamp.set_child(Some(&page_box));
    scroll.set_child(Some(&clamp));

    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    container.append(&scroll);
    (container, flowbox)
}

fn populate_flowbox(flowbox: &gtk::FlowBox, filter: &str, search_entry: &gtk::SearchEntry) {
    while let Some(child) = flowbox.first_child() {
        flowbox.remove(&child);
    }

    let manager = LauncherManager::new();
    let flowbox_ref = flowbox.clone();
    let filter_lower = filter.trim().to_lowercase();
    let search_ref = search_entry.clone();

    for info in manager.all_launchers() {
        if !filter_lower.is_empty() {
            let name_matches = info.name.to_lowercase().contains(&filter_lower);
            let desc_matches = info.description.to_lowercase().contains(&filter_lower);
            if !name_matches && !desc_matches {
                continue;
            }
        }

        let installed = manager.is_installed(&info.id);
        let fb = flowbox_ref.clone();
        let se = search_ref.clone();
        let card = create_launcher_card(info, installed, move || {
            let text = se.text().to_string();
            populate_flowbox(&fb, &text, &se);
        });
        flowbox.append(&card);
    }
}

fn create_launcher_card<F: Fn() + Clone + 'static>(
    info: LauncherInfo,
    installed: bool,
    on_refresh: F,
) -> gtk::Box {
    let manager = LauncherManager::new();
    let icon_path = manager.existing_icon_path(&info.id);

    let icon: gtk::Image = if let Some(ref path) = icon_path {
        gtk::Image::builder()
            .file(path.to_string_lossy().as_ref())
            .pixel_size(64)
            .build()
    } else {
        let img = gtk::Image::builder()
            .icon_name("applications-games")
            .pixel_size(64)
            .build();
        let (tx, rx) = std::sync::mpsc::channel::<std::path::PathBuf>();
        let id = info.id.clone();
        let icon_url = info.icon_url.to_string();
        std::thread::spawn(move || {
            let mgr = LauncherManager::new();
            if let Some(path) = mgr.download_icon_by_url(&id, &icon_url) {
                let _ = tx.send(path);
            }
        });
        let img_clone = img.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            if let Ok(path) = rx.try_recv() {
                img_clone.set_from_file(Some(&path));
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
        img
    };

    let name_label = gtk::Label::builder()
        .label(info.name)
        .css_classes(["heading"])
        .wrap(true)
        .build();

    let desc_label = gtk::Label::builder()
        .label(info.description)
        .css_classes(["caption"])
        .wrap(true)
        .lines(2)
        .build();

    let status_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .halign(gtk::Align::Center)
        .build();

    let status_label = gtk::Label::builder()
        .label(if installed {
            "Installed"
        } else {
            "Not installed"
        })
        .css_classes(["caption"])
        .build();
    status_box.append(&status_label);

    if info.chaotic_aur {
        let chaotic_badge = gtk::Label::builder()
            .label("Chaotic-AUR")
            .css_classes(["caption", "accent"])
            .build();
        status_box.append(&chaotic_badge);
    }

    let action_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .halign(gtk::Align::Center)
        .spacing(6)
        .build();

    let pkg_list: Vec<String> = info.packages.iter().map(|s| s.to_string()).collect();

    if installed {
        let launch_btn = gtk::Button::builder()
            .label("Launch")
            .css_classes(["suggested-action", "compact"])
            .build();
        let cmd = info.command.to_string();
        launch_btn.connect_clicked(move |_| {
            let mut child = std::process::Command::new(&cmd);
            child.stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
            apply_locale(&mut child);
            if let Ok(mut c) = child.spawn() {
                std::thread::spawn(move || {
                    let _ = c.wait();
                });
            }
        });
        action_box.append(&launch_btn);

        let remove_btn = gtk::Button::builder()
            .label("Remove")
            .css_classes(["destructive-action", "compact"])
            .build();
        let remove_cmd = crate::modules::pacman::PacmanManager::remove_command(&pkg_list);
        let remove_title = info.name.to_string();
        let refresh = on_refresh.clone();
        remove_btn.connect_clicked(move |btn| {
            if let Some(root) = btn.root() {
                if let Some(window) = root.downcast_ref::<gtk::Window>() {
                    let ref_cb = refresh.clone();
                    terminal::show(
                        window,
                        &remove_title,
                        &remove_cmd,
                        Some(Box::new(move |success| {
                            if success {
                                ref_cb();
                            }
                        })),
                    );
                }
            }
        });
        action_box.append(&remove_btn);
    } else {
        let install_btn = gtk::Button::builder()
            .label("Install")
            .css_classes(["suggested-action", "compact"])
            .build();
        let install_cmd = crate::modules::pacman::PacmanManager::install_command(&pkg_list);
        let install_title = info.name.to_string();
        let refresh = on_refresh.clone();
        let is_chaotic = info.chaotic_aur;
        let item_name = info.name.to_string();
        install_btn.connect_clicked(move |btn| {
            if let Some(root) = btn.root() {
                if let Some(window) = root.downcast_ref::<gtk::Window>() {
                    if is_chaotic && !crate::modules::pacman::PacmanManager::new().check_chaotic_aur() {
                        crate::ui::dialogs::chaotic::prompt_chaotic_aur(window, &item_name);
                        return;
                    }
                    let ref_cb = refresh.clone();
                    terminal::show(
                        window,
                        &install_title,
                        &install_cmd,
                        Some(Box::new(move |success| {
                            if success {
                                ref_cb();
                            }
                        })),
                    );
                }
            }
        });
        action_box.append(&install_btn);
    }

    let inner = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(12)
        .margin_end(12)
        .valign(gtk::Align::Start)
        .build();

    inner.append(&icon);
    inner.append(&name_label);
    inner.append(&desc_label);
    inner.append(&status_box);
    inner.append(&action_box);

    let card = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .css_classes(["card"])
        .build();
    card.append(&inner);
    card
}
