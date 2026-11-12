use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::glib;

use crate::modules::proton_manager::{CompatTool, ProtonManager, ReleaseInfo};

struct UiState {
    tools: Vec<CompatTool>,
    all_releases: Vec<Vec<ReleaseInfo>>,
    selected_tool: usize,
    selected_version: usize,
}

fn shell_escape(s: &str) -> String {
    s.replace('\'', "'\\''")
}

pub fn new() -> gtk::Box {
    let tools = ProtonManager::available_tools();
    let tool_count = tools.len();

    let toast_overlay = adw::ToastOverlay::new();

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build();

    let clamp = adw::Clamp::builder()
        .maximum_size(800)
        .tightening_threshold(600)
        .build();

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(24)
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(12)
        .margin_end(12)
        .build();

    // GNOME HIG: use AdwStatusPage for loading state
    let loading_page = adw::StatusPage::builder()
        .icon_name("content-loading-symbolic")
        .title("Loading Compatibility Tools")
        .description("Fetching available releases from GitHub…")
        .build();
    let spinner = gtk::Spinner::builder()
        .halign(gtk::Align::Center)
        .width_request(32)
        .height_request(32)
        .build();
    spinner.start();
    loading_page.set_child(Some(&spinner));
    content.append(&loading_page);

    clamp.set_child(Some(&content));
    scroll.set_child(Some(&clamp));
    toast_overlay.set_child(Some(&scroll));

    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    container.append(&toast_overlay);

    let (tx, rx) = std::sync::mpsc::channel::<(usize, Vec<ReleaseInfo>)>();
    let tools_for_thread = tools.clone();

    for (i, tool) in tools_for_thread.iter().enumerate() {
        let tx = tx.clone();
        let id = tool.id;
        let tools_ref = tools_for_thread.clone();
        std::thread::spawn(move || {
            if let Some(t) = tools_ref.iter().find(|t| t.id == id) {
                let releases = ProtonManager::list_available_releases(t);
                let _ = tx.send((i, releases));
            }
        });
    }
    drop(tx);

    let content_loading = content.clone();
    let tools_loaded = Rc::new(RefCell::new(Some(tools.clone())));
    let received_releases: Rc<RefCell<Vec<Vec<ReleaseInfo>>>> =
        Rc::new(RefCell::new(vec![vec![]; tool_count]));
    let received_count = Rc::new(RefCell::new(0usize));
    let toast_clone = toast_overlay.clone();

    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        while let Ok((i, rels)) = rx.try_recv() {
            received_releases.borrow_mut()[i] = rels;
            *received_count.borrow_mut() += 1;
        }

        if *received_count.borrow() >= tool_count {
            while let Some(child) = content_loading.first_child() {
                content_loading.remove(&child);
            }
            if let Some(t) = tools_loaded.borrow_mut().take() {
                let all = received_releases.borrow().clone();
                build_page(&content_loading, t, all, &toast_clone);
            }
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });

    container
}

fn build_page(
    content: &gtk::Box,
    tools: Vec<CompatTool>,
    all_releases: Vec<Vec<ReleaseInfo>>,
    toast_overlay: &adw::ToastOverlay,
) {
    while let Some(child) = content.first_child() {
        content.remove(&child);
    }

    let state = Rc::new(RefCell::new(UiState {
        tools,
        all_releases,
        selected_tool: 0,
        selected_version: 0,
    }));

    // --- Tool Selector Group (GNOME HIG: AdwComboRow) ---
    let selector_group = adw::PreferencesGroup::builder()
        .title("Compatibility Tool")
        .description("Select and manage Proton and Wine compatibility layers")
        .build();

    let tool_store = gtk::StringList::new(&[]);
    for t in &state.borrow().tools {
        tool_store.append(t.name);
    }

    let tool_row = adw::ComboRow::builder()
        .title("Tool")
        .subtitle("Choose a compatibility tool to manage")
        .model(&tool_store)
        .selected(0)
        .build();
    selector_group.add(&tool_row);

    // Description row for selected tool
    let desc_row = adw::ActionRow::builder()
        .title("Description")
        .subtitle(
            state
                .borrow()
                .tools
                .first()
                .map(|t| t.description)
                .unwrap_or(""),
        )
        .subtitle_lines(0)
        .build();
    desc_row.add_prefix(&gtk::Image::from_icon_name("dialog-information-symbolic"));
    selector_group.add(&desc_row);

    content.append(&selector_group);

    // --- Available Versions Group ---
    let version_group = adw::PreferencesGroup::builder()
        .title("Available Versions")
        .description("Versions available for download and installation")
        .build();

    // Header suffix: Refresh button (GNOME HIG: pill button in header)
    let refresh_btn = gtk::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text("Refresh releases from GitHub")
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();
    version_group.set_header_suffix(Some(&refresh_btn));

    let version_list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();

    version_group.add(&version_list);
    content.append(&version_group);

    // --- Installed Versions Group ---
    let installed_group = adw::PreferencesGroup::builder()
        .title("Installed Versions")
        .description("Versions currently installed on your system")
        .build();

    let installed_list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    installed_group.add(&installed_list);
    content.append(&installed_group);

    // --- Release Notes Group (GNOME HIG: ExpanderRow) ---
    let notes_group = adw::PreferencesGroup::builder()
        .title("Release Notes")
        .build();

    let notes_expander = adw::ExpanderRow::builder()
        .title("View Release Notes")
        .subtitle("Details for the selected version")
        .show_enable_switch(false)
        .build();

    let notes_label = gtk::Label::builder()
        .wrap(true)
        .xalign(0.0)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(12)
        .margin_end(12)
        .selectable(true)
        .css_classes(["body"])
        .build();

    let notes_row = adw::PreferencesRow::builder().build();
    notes_row.set_child(Some(&notes_label));
    notes_expander.add_row(&notes_row);
    notes_group.add(&notes_expander);
    content.append(&notes_group);

    // --- Action Buttons (GNOME HIG: pill buttons with proper spacing) ---
    let action_group = adw::PreferencesGroup::builder().build();

    let install_btn = gtk::Button::builder()
        .label("Install")
        .tooltip_text("Install Selected")
        .css_classes(["suggested-action", "pill"])
        .hexpand(true)
        .build();
    let remove_btn = gtk::Button::builder()
        .label("Remove")
        .tooltip_text("Remove Selected")
        .css_classes(["destructive-action", "pill"])
        .hexpand(true)
        .sensitive(false)
        .build();

    let btn_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .halign(gtk::Align::Center)
        .homogeneous(true)
        .margin_top(6)
        .margin_bottom(6)
        .build();
    btn_box.append(&install_btn);
    btn_box.append(&remove_btn);

    action_group.add(&btn_box);
    content.append(&action_group);

    // --- Populate initial view ---
    populate_view(
        &state,
        &version_list,
        &installed_list,
        &notes_label,
        &notes_expander,
        &desc_row,
        &remove_btn,
        0,
    );

    // --- Tool selector handler ---
    let state_s = state.clone();
    let vl = version_list.clone();
    let il = installed_list.clone();
    let nl = notes_label.clone();
    let ne = notes_expander.clone();
    let dr = desc_row.clone();
    let rb = remove_btn.clone();
    tool_row.connect_selected_notify(move |dd| {
        let idx = dd.selected() as usize;
        state_s.borrow_mut().selected_tool = idx;
        state_s.borrow_mut().selected_version = 0;
        populate_view(&state_s, &vl, &il, &nl, &ne, &dr, &rb, idx);
    });

    // --- Version list selection handler ---
    let state_sel = state.clone();
    let nl_sel = notes_label.clone();
    let ne_sel = notes_expander.clone();
    version_list.connect_row_selected(move |_, row| {
        if let Some(row) = row {
            let idx = row.index() as usize;
            state_sel.borrow_mut().selected_version = idx;
            let s = state_sel.borrow();
            let rels = &s.all_releases;
            let tool_idx = s.selected_tool;
            if let Some(release) = rels[tool_idx].get(idx) {
                ne_sel.set_subtitle(&release.version);
                if release.body.is_empty() {
                    nl_sel.set_label("No release notes available.");
                } else {
                    nl_sel.set_label(&release.body);
                }
            }
        }
    });

    // --- Install button ---
    let state_i = state.clone();
    let toast_i = toast_overlay.clone();
    let vl_inst = version_list.clone();
    let il_inst = installed_list.clone();
    let nl_inst = notes_label.clone();
    let ne_inst = notes_expander.clone();
    let dr_inst = desc_row.clone();
    let rb_inst = remove_btn.clone();
    install_btn.connect_clicked(move |btn| {
        let s = state_i.borrow();
        let tool_idx = s.selected_tool;
        let ver_idx = s.selected_version;
        let rels = &s.all_releases;
        if let Some(release) = rels[tool_idx].get(ver_idx) {
            if let Some(root) = btn.root() {
                if let Some(window) = root.downcast_ref::<gtk::Window>() {
                    let install_dirs = ProtonManager::steam_install_dirs();
                    let install_dir = install_dirs.first().cloned().unwrap_or_else(|| {
                        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                        std::path::PathBuf::from(&home).join(".steam/steam/compatibilitytools.d")
                    });
                    
                    let tar_flag = if release.asset_name.ends_with(".zst") {
                        "--zstd"
                    } else if release.asset_name.ends_with(".xz") {
                        "--xz"
                    } else {
                        "-z"
                    };

                    let cmd = format!(
                        "mkdir -p '{install_dir}' && \
                         cd /tmp && \
                         curl -fsSL -o '{asset}' '{url}' && \
                         tar {tar_flag} -xf '{asset}' -C '{install_dir}' && \
                         rm -f '{asset}'",
                        install_dir = shell_escape(&install_dir.to_string_lossy()),
                        asset = shell_escape(&release.asset_name),
                        url = shell_escape(&release.download_url),
                        tar_flag = tar_flag
                    );
                    let tool_name = s.tools[tool_idx].name;
                    let title = format!("Install {} — {}", tool_name, release.version);
                    let state_cb = state_i.clone();
                    let vl_cb = vl_inst.clone();
                    let il_cb = il_inst.clone();
                    let nl_cb = nl_inst.clone();
                    let ne_cb = ne_inst.clone();
                    let dr_cb = dr_inst.clone();
                    let rb_cb = rb_inst.clone();
                    crate::ui::dialogs::terminal::show(
                        window,
                        &title,
                        &cmd,
                        Some(Box::new(move |success| {
                            if success {
                                let idx = state_cb.borrow().selected_tool;
                                populate_view(&state_cb, &vl_cb, &il_cb, &nl_cb, &ne_cb, &dr_cb, &rb_cb, idx);
                            }
                        })),
                    );
                }
            }
        } else {
            let toast = adw::Toast::new("No version selected to install");
            toast_i.add_toast(toast);
        }
    });

    // --- Remove button ---
    let state_rm = state.clone();
    let toast_rm = toast_overlay.clone();
    remove_btn.connect_clicked(move |_| {
        let s = state_rm.borrow();
        let tool_idx = s.selected_tool;
        let tool = &s.tools[tool_idx];
        let installed = ProtonManager::list_installed(tool);
        if installed.is_empty() {
            let toast = adw::Toast::new("No installed versions to remove");
            toast_rm.add_toast(toast);
        }
        // Removal is handled per-row via the row delete buttons
    });

    // --- Refresh button ---
    let state_r = state.clone();
    let vl_r = version_list.clone();
    let il_r = installed_list.clone();
    let nl_r = notes_label.clone();
    let ne_r = notes_expander.clone();
    let dr_r = desc_row.clone();
    let rb_r = remove_btn.clone();
    let toast_r = toast_overlay.clone();
    refresh_btn.connect_clicked(move |btn| {
        btn.set_sensitive(false);
        let (tx, rx) = std::sync::mpsc::channel::<(usize, Vec<ReleaseInfo>)>();
        let tools_ref = state_r.borrow().tools.clone();
        let tool_count = tools_ref.len();

        for (i, tool) in tools_ref.iter().enumerate() {
            let tx = tx.clone();
            let id = tool.id;
            let t_ref = tools_ref.clone();
            std::thread::spawn(move || {
                if let Some(t) = t_ref.iter().find(|t| t.id == id) {
                    let rels = ProtonManager::list_available_releases(t);
                    let _ = tx.send((i, rels));
                }
            });
        }
        drop(tx);

        let state_r2 = state_r.clone();
        let vl_r2 = vl_r.clone();
        let il_r2 = il_r.clone();
        let nl_r2 = nl_r.clone();
        let ne_r2 = ne_r.clone();
        let dr_r2 = dr_r.clone();
        let rb_r2 = rb_r.clone();
        let refresh_received = Rc::new(RefCell::new(0usize));
        let btn_clone = btn.clone();
        let toast_r2 = toast_r.clone();

        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            while let Ok((i, rels)) = rx.try_recv() {
                state_r2.borrow_mut().all_releases[i] = rels;
                *refresh_received.borrow_mut() += 1;
            }

            if *refresh_received.borrow() >= tool_count {
                let idx = state_r2.borrow().selected_tool;
                populate_view(&state_r2, &vl_r2, &il_r2, &nl_r2, &ne_r2, &dr_r2, &rb_r2, idx);
                btn_clone.set_sensitive(true);
                let toast = adw::Toast::new("Releases refreshed");
                toast_r2.add_toast(toast);
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    });
}

#[allow(clippy::too_many_arguments)]
fn populate_view(
    state: &Rc<RefCell<UiState>>,
    version_list: &gtk::ListBox,
    installed_list: &gtk::ListBox,
    notes_label: &gtk::Label,
    notes_expander: &adw::ExpanderRow,
    desc_row: &adw::ActionRow,
    remove_btn: &gtk::Button,
    tool_idx: usize,
) {
    let s = state.borrow();
    let rels = &s.all_releases;
    let tool = &s.tools[tool_idx];
    let releases = &rels[tool_idx];

    // Update tool description
    desc_row.set_subtitle(tool.description);

    // Clear lists
    while let Some(child) = version_list.first_child() {
        version_list.remove(&child);
    }
    while let Some(child) = installed_list.first_child() {
        installed_list.remove(&child);
    }

    // Populate available versions
    if releases.is_empty() {
        let row = adw::ActionRow::builder()
            .title("No releases available")
            .subtitle("Check your internet connection or try again later (GitHub may be rate-limiting requests)")
            .build();
        row.add_prefix(&gtk::Image::from_icon_name("dialog-warning-symbolic"));
        version_list.append(&row);
    } else {
        for r in releases {
            let row = adw::ActionRow::builder()
                .title(&r.version)
                .subtitle(&r.asset_name)
                .title_lines(0)
                .subtitle_lines(0)
                .activatable(true)
                .build();
            row.add_prefix(&gtk::Image::from_icon_name("package-x-generic-symbolic"));
            let size_label = gtk::Label::builder()
                .label("↓")
                .valign(gtk::Align::Center)
                .css_classes(["dim-label"])
                .build();
            row.add_suffix(&size_label);
            version_list.append(&row);
        }
    }

    // Populate installed versions
    let installed = ProtonManager::list_installed(tool);
    if installed.is_empty() {
        let row = adw::ActionRow::builder()
            .title("No versions installed")
            .subtitle("Install a version from the list above")
            .build();
        row.add_prefix(&gtk::Image::from_icon_name("folder-symbolic"));
        installed_list.append(&row);
        remove_btn.set_sensitive(false);
    } else {
        remove_btn.set_sensitive(true);
        for v in &installed {
            let row = adw::ActionRow::builder()
                .title(&v.version)
                .subtitle(&*v.path.to_string_lossy())
                .title_lines(0)
                .subtitle_lines(0)
                .build();
            row.add_prefix(&gtk::Image::from_icon_name("emblem-ok-symbolic"));

            // Per-row remove button (GNOME HIG pattern)
            let del_btn = gtk::Button::builder()
                .icon_name("user-trash-symbolic")
                .valign(gtk::Align::Center)
                .css_classes(["flat"])
                .tooltip_text("Remove this version")
                .build();
            let path = v.path.clone();
            let state_del = state.clone();
            let vl_del = version_list.clone();
            let il_del = installed_list.clone();
            let nl_del = notes_label.clone();
            let ne_del = notes_expander.clone();
            let dr_del = desc_row.clone();
            let rb_del = remove_btn.clone();
            del_btn.connect_clicked(move |btn| {
                if let Some(root) = btn.root() {
                    if let Some(window) = root.downcast_ref::<gtk::Window>() {
                        let cmd = format!("rm -rf '{}'", shell_escape(&path.to_string_lossy()));
                        let title = format!("Remove {}", path.display());
                        let state_cb = state_del.clone();
                        let vl_cb = vl_del.clone();
                        let il_cb = il_del.clone();
                        let nl_cb = nl_del.clone();
                        let ne_cb = ne_del.clone();
                        let dr_cb = dr_del.clone();
                        let rb_cb = rb_del.clone();
                        crate::ui::dialogs::terminal::show(
                            window,
                            &title,
                            &cmd,
                            Some(Box::new(move |success| {
                                if success {
                                    let idx = state_cb.borrow().selected_tool;
                                    populate_view(&state_cb, &vl_cb, &il_cb, &nl_cb, &ne_cb, &dr_cb, &rb_cb, idx);
                                }
                            })),
                        );
                    }
                }
            });
            row.add_suffix(&del_btn);
            installed_list.append(&row);
        }
    }

    // Show first version release notes
    if let Some(first) = releases.first() {
        notes_expander.set_subtitle(&first.version);
        if first.body.is_empty() {
            notes_label.set_label("No release notes available.");
        } else {
            notes_label.set_label(&first.body);
        }
    } else {
        notes_expander.set_subtitle("No version selected");
        notes_label.set_label("Select a version to view its release notes.");
    }
}

