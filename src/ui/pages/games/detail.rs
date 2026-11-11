use adw::prelude::*;

use super::{
    ensure_meta, format_last_played, format_playtime, now_secs, tier_badge, toast, State,
};
use crate::modules::game_library::GameEntry;
use crate::modules::game_runner::GameRunner;
use crate::modules::pacman::PacmanManager;
use crate::modules::protondb::CheckVerdict;

fn preflight_icon(verdict: CheckVerdict) -> (&'static str, &'static str) {
    match verdict {
        CheckVerdict::Pass => ("emblem-ok-symbolic", "success"),
        CheckVerdict::Fail => ("dialog-error-symbolic", "error"),
        CheckVerdict::Unknown => ("dialog-information-symbolic", "dim-label"),
    }
}

/// (Re)build the detail page for a game and push it onto the navigation view
/// if it is not the page already being shown.
pub(super) fn populate_detail(state: &State, id: &str) {
    let game = match state.library.borrow().get(id).cloned() {
        Some(g) => g,
        None => return,
    };
    *state.current_detail.borrow_mut() = Some(game.id.clone());

    let page = build_detail_page(state, &game);

    state.detail_title.set_title(&game.name);
    state.detail_title.set_subtitle(&match game.steam_appid {
        Some(appid) => format!("Steam AppID {}", appid),
        None => "Local game".to_string(),
    });
    state.detail_toolbar.set_content(Some(&page));

    let already_showing = state
        .nav_view
        .visible_page_tag()
        .map(|t| t == "detail")
        .unwrap_or(false);
    if !already_showing {
        state.nav_view.push(&state.detail_page);
    }
}

fn build_detail_page(state: &State, game: &GameEntry) -> gtk::ScrolledWindow {
    let clamp = adw::Clamp::builder()
        .maximum_size(720)
        .tightening_threshold(600)
        .build();

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(18)
        .margin_top(18)
        .margin_bottom(24)
        .margin_start(12)
        .margin_end(12)
        .build();

    content.append(&build_header_card(state, game));
    content.append(&build_launch_group(state, game));
    content.append(&build_checks_group(state, game));
    content.append(&build_manage_group(state, game));

    clamp.set_child(Some(&content));
    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .child(&clamp)
        .build()
}

fn build_header_card(state: &State, game: &GameEntry) -> gtk::Box {
    let card = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(0)
        .css_classes(["card"])
        .build();

    if let Some(path) = game
        .steam_appid
        .and_then(|appid| state.covers.borrow().get(&appid).cloned())
    {
        let pic = gtk::Picture::for_filename(path);
        pic.set_content_fit(gtk::ContentFit::Cover);
        pic.set_size_request(460, 200);
        card.append(&pic);
    }

    let meta = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin_start(16)
        .margin_end(16)
        .margin_top(16)
        .margin_bottom(16)
        .build();

    let title = gtk::Label::builder()
        .label(&game.name)
        .css_classes(["title-2"])
        .halign(gtk::Align::Start)
        .wrap(true)
        .build();
    meta.append(&title);

    let badge_row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .build();

    let tier = game
        .steam_appid
        .and_then(|appid| state.tiers.borrow().get(&appid).cloned());
    let (badge_text, badge_class, tier_desc) = match &tier {
        Some(t) => {
            let (text, class) = tier_badge(t);
            let desc = match t.as_str() {
                "platinum" => "Runs flawlessly out of the box.",
                "gold" => "Runs perfectly after minor tweaks.",
                "silver" => "Runs with minor issues.",
                "bronze" => "Runs but crashes or glitches frequently.",
                "borked" => "Does not run or is unplayable.",
                _ => "No community reports yet.",
            };
            (text, class, desc.to_string())
        }
        None => (
            "NO DATA",
            "card",
            if game.steam_appid.is_some() {
                "Fetching ProtonDB rating…".to_string()
            } else {
                "No Steam match — edit the game to match it for a rating.".to_string()
            },
        ),
    };
    let badge = gtk::Label::builder()
        .label(format!("ProtonDB: {}", badge_text))
        .css_classes([badge_class, "pill", "heading"])
        .halign(gtk::Align::Start)
        .build();
    badge_row.append(&badge);
    meta.append(&badge_row);

    let tier_desc_label = gtk::Label::builder()
        .label(&tier_desc)
        .css_classes(["dim-label"])
        .halign(gtk::Align::Start)
        .wrap(true)
        .build();
    meta.append(&tier_desc_label);

    let proton_version = game
        .proton_version
        .clone()
        .unwrap_or_else(|| "Automatic".to_string());
    let info_label = gtk::Label::builder()
        .label(format!(
            "Proton: {} • Last played: {} • Playtime: {}",
            proton_version,
            game.last_played
                .map(format_last_played)
                .unwrap_or_else(|| "Never".to_string()),
            format_playtime(game.playtime_secs),
        ))
        .css_classes(["dim-label", "caption"])
        .halign(gtk::Align::Start)
        .wrap(true)
        .build();
    meta.append(&info_label);
    card.append(&meta);
    card
}

fn build_launch_group(state: &State, game: &GameEntry) -> adw::PreferencesGroup {
    let launch_group = adw::PreferencesGroup::builder()
        .title("Launch")
        .description("Runs through Proton via umu-launcher")
        .build();

    let action_row = adw::ActionRow::new();
    if GameRunner::is_running(&game.id) {
        let running_pill = gtk::Label::builder()
            .label("● Running")
            .css_classes(["success", "pill", "heading"])
            .valign(gtk::Align::Center)
            .build();
        action_row.add_prefix(&running_pill);

        let stop_btn = gtk::Button::builder()
            .label("Force Stop")
            .css_classes(["destructive-action", "pill"])
            .valign(gtk::Align::Center)
            .build();
        {
            let state = state.clone();
            let game_id = game.id.clone();
            let game_name = game.name.clone();
            stop_btn.connect_clicked(move |_| {
                if GameRunner::force_stop(&game_id) {
                    toast(&state, &format!("Stopping {}…", game_name));
                } else {
                    toast(&state, "Could not stop the game process");
                }
            });
        }
        action_row.add_suffix(&stop_btn);
    } else {
        let launch_btn = gtk::Button::builder()
            .css_classes(["suggested-action", "pill"])
            .valign(gtk::Align::Center)
            .build();
        launch_btn.set_child(Some(
            &adw::ButtonContent::builder()
                .icon_name("media-playback-start-symbolic")
                .label("Launch")
                .build(),
        ));
        {
            let state = state.clone();
            let game = game.clone();
            launch_btn.connect_clicked(move |btn| {
                btn.set_sensitive(false);
                launch_game(state.clone(), game.clone());
            });
        }
        action_row.add_suffix(&launch_btn);
    }
    launch_group.add(&action_row);
    launch_group
}

fn build_checks_group(state: &State, game: &GameEntry) -> adw::PreferencesGroup {
    let checks_group = adw::PreferencesGroup::builder()
        .title("Pre-Launch Checks")
        .description("Verified before every launch")
        .build();

    let checks_list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    match state.preflight.borrow().get(&game.id).cloned() {
        Some(rows) => {
            for check in rows {
                let (icon_name, icon_class) = preflight_icon(check.verdict);
                let row = adw::ActionRow::builder()
                    .title(check.item)
                    .subtitle(&check.detail)
                    .build();
                let icon = gtk::Image::from_icon_name(icon_name);
                icon.set_css_classes(&[icon_class]);
                row.add_prefix(&icon);
                checks_list.append(&row);
            }
        }
        None => {
            let placeholder = adw::ActionRow::builder()
                .title("Checking environment…")
                .build();
            let spinner = gtk::Spinner::new();
            spinner.start();
            placeholder.add_prefix(&spinner);
            checks_list.append(&placeholder);
            kick_preflight(state, game.clone());
        }
    }

    let recheck_row = adw::ActionRow::builder()
        .title("Re-check")
        .subtitle("Run the environment checks again")
        .build();
    let recheck_btn = gtk::Button::builder()
        .label("Check")
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .build();
    {
        let state = state.clone();
        let game = game.clone();
        recheck_btn.connect_clicked(move |_| {
            state.preflight.borrow_mut().remove(&game.id);
            populate_detail(&state, &game.id);
        });
    }
    recheck_row.add_suffix(&recheck_btn);
    checks_list.append(&recheck_row);

    checks_group.add(&checks_list);
    checks_group
}

fn build_manage_group(state: &State, game: &GameEntry) -> adw::PreferencesGroup {
    let manage_group = adw::PreferencesGroup::builder().title("Manage").build();

    let manage_list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    // Run Winetricks
    let winetricks_row = adw::ActionRow::builder()
        .title("Run Winetricks")
        .subtitle("Install Windows components (vcrun, dotnet, d3d…) into this game's prefix")
        .activatable(true)
        .build();
    winetricks_row.add_prefix(&gtk::Image::from_icon_name("package-x-generic-symbolic"));
    {
        let state = state.clone();
        let game = game.clone();
        winetricks_row.connect_activated(move |row| {
            if let Some(window) = row.root().and_downcast_ref::<gtk::Window>() {
                run_winetricks(&state, window, &game);
            }
        });
    }
    manage_list.append(&winetricks_row);

    // View log
    let log_row = adw::ActionRow::builder()
        .title("View Launch Log")
        .subtitle("Proton and game output from the last launch")
        .activatable(true)
        .build();
    log_row.add_prefix(&gtk::Image::from_icon_name("text-x-generic-symbolic"));
    {
        let game = game.clone();
        log_row.connect_activated(move |row| {
            if let Some(window) = row.root().and_downcast_ref::<gtk::Window>() {
                show_log_dialog(window, &game);
            }
        });
    }
    manage_list.append(&log_row);

    // Open game folder
    let game_folder_row = adw::ActionRow::builder()
        .title("Open Game Folder")
        .subtitle(game.workdir.clone().unwrap_or_else(|| {
            game.exe_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default()
        }))
        .activatable(true)
        .build();
    game_folder_row.add_prefix(&gtk::Image::from_icon_name("folder-open-symbolic"));
    {
        let game = game.clone();
        game_folder_row.connect_activated(move |_| {
            let dir = game
                .workdir
                .clone()
                .or_else(|| game.exe_dir().map(|p| p.to_string_lossy().to_string()));
            if let Some(dir) = dir {
                let _ = std::process::Command::new("xdg-open").arg(&dir).spawn();
            }
        });
    }
    manage_list.append(&game_folder_row);

    // Open prefix
    let prefix_row = adw::ActionRow::builder()
        .title("Open Wine Prefix")
        .subtitle(game.prefix_dir().to_string_lossy().to_string())
        .activatable(true)
        .build();
    prefix_row.add_prefix(&gtk::Image::from_icon_name("folder-templates-symbolic"));
    {
        let prefix = game.prefix_dir();
        prefix_row.connect_activated(move |_| {
            let _ = std::process::Command::new("xdg-open")
                .arg(prefix.to_string_lossy().to_string())
                .spawn();
        });
    }
    manage_list.append(&prefix_row);

    // Edit
    let edit_row = adw::ActionRow::builder()
        .title("Edit Game")
        .subtitle("Executable, Proton version, arguments, Steam match")
        .activatable(true)
        .build();
    edit_row.add_prefix(&gtk::Image::from_icon_name("document-edit-symbolic"));
    {
        let state = state.clone();
        let game = game.clone();
        edit_row.connect_activated(move |row| {
            if let Some(window) = row.root().and_downcast_ref::<gtk::Window>() {
                let state_cb = state.clone();
                let game_cb = game.clone();
                let game_id = game.id.clone();
                open_edit_dialog(
                    &state,
                    window,
                    game_cb,
                    Box::new(move |entry| {
                        state_cb.preflight.borrow_mut().remove(&game_id);
                        super::grid::rebuild_grid(&state_cb);
                        populate_detail(&state_cb, &entry.id);
                        ensure_meta(&state_cb);
                    }),
                );
            }
        });
    }
    manage_list.append(&edit_row);

    // Remove
    let remove_row = adw::ActionRow::builder()
        .title("Remove Game")
        .subtitle("Remove from the library, optionally deleting its prefix")
        .activatable(true)
        .build();
    remove_row.add_prefix(&gtk::Image::from_icon_name("user-trash-symbolic"));
    {
        let state = state.clone();
        let game = game.clone();
        remove_row.connect_activated(move |row| {
            if let Some(window) = row.root().and_downcast_ref::<gtk::Window>() {
                confirm_remove(&state, window, &game);
            }
        });
    }
    manage_list.append(&remove_row);

    manage_group.add(&manage_list);
    manage_group
}

/// Open the Edit Game dialog for an existing entry.
pub(super) fn open_edit_dialog(
    state: &State,
    window: &gtk::Window,
    game: GameEntry,
    on_saved: Box<dyn FnOnce(GameEntry) + 'static>, // No underscore needed, we are using it now!
) {
    let state_clone = state.clone();
    crate::ui::dialogs::game_form::show(
        window,
        state.library.clone(),
        Some(game),
        Box::new(move |entry| {
            // 1. Invalidate cache: Remove AppID from memory so it is forced to check disk again
            if let Some(appid) = entry.steam_appid {
                state_clone.covers.borrow_mut().remove(&appid);
            }

            // 2. Call the original on_saved closure that was passed into this function
            on_saved(entry);
        }),
    );
}


/// Background preflight for the detail page; result is cached and the detail
/// view is refreshed if it is still the one on screen.
fn kick_preflight(state: &State, game: GameEntry) {
    let (tx, rx) = std::sync::mpsc::channel::<Vec<crate::modules::protondb::CheckRow>>();
    let game_id = game.id.clone();
    std::thread::spawn(move || {
        let _ = tx.send(GameRunner::run_preflight(&game));
    });
    let state = state.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(120), move || {
        let Ok(rows) = rx.try_recv() else {
            return glib::ControlFlow::Continue;
        };
        state.preflight.borrow_mut().insert(game_id.clone(), rows);
        if state.current_detail.borrow().as_deref() == Some(game_id.as_str()) {
            populate_detail(&state, &game_id);
        }
        glib::ControlFlow::Break
    });
}

/// Preflight in the background, then launch or surface the failures.
fn launch_game(state: State, game: GameEntry) {
    let (tx, rx) = std::sync::mpsc::channel::<Vec<crate::modules::protondb::CheckRow>>();
    let game_for_thread = game.clone();
    std::thread::spawn(move || {
        let _ = tx.send(GameRunner::run_preflight(&game_for_thread));
    });

    glib::timeout_add_local(std::time::Duration::from_millis(120), move || {
        let Ok(rows) = rx.try_recv() else {
            return glib::ControlFlow::Continue;
        };
        state
            .preflight
            .borrow_mut()
            .insert(game.id.clone(), rows.clone());

        if GameRunner::preflight_passed(&rows) {
            let state_cb = state.clone();
            let game_id = game.id.clone();
            let game_name = game.name.clone();
            let result = GameRunner::launch(
                &game,
                Box::new(move |secs, ok| {
                    let mut updated = state_cb.library.borrow().get(&game_id).cloned();
                    if let Some(ref mut entry) = updated {
                        entry.last_played = Some(now_secs());
                        entry.playtime_secs += secs;
                    }
                    if let Some(entry) = updated {
                        state_cb.library.borrow_mut().update(entry);
                    }
                    state_cb.preflight.borrow_mut().remove(&game_id);
                    let msg = if ok {
                        format!("{} exited", game_name)
                    } else {
                        format!("{} exited with an error — check the launch log", game_name)
                    };
                    toast(&state_cb, &msg);
                    super::grid::rebuild_grid(&state_cb);
                    if state_cb.current_detail.borrow().as_deref() == Some(game_id.as_str()) {
                        populate_detail(&state_cb, &game_id);
                    }
                }),
            );
            match result {
                Ok(()) => {
                    toast(&state, &format!("Launching {} with Proton…", game.name));
                    populate_detail(&state, &game.id);
                }
                Err(e) => {
                    toast(&state, &e);
                    populate_detail(&state, &game.id);
                }
            }
        } else {
            toast(&state, "Pre-launch checks failed — fix the highlighted rows");
            populate_detail(&state, &game.id);
        }
        glib::ControlFlow::Break
    });
}

fn run_winetricks(state: &State, window: &gtk::Window, game: &GameEntry) {
    let window = window.clone();
    let game = game.clone();
    if !GameRunner::winetricks_available() {
        let window_for_cb = window.clone();
        let confirm = adw::AlertDialog::builder()
            .heading("winetricks is not installed")
            .body("winetricks installs Windows components into the game's prefix. Install it now from the official repositories?")
            .build();
        confirm.add_response("cancel", "Cancel");
        confirm.add_response("install", "Install winetricks");
        confirm.set_response_appearance("install", adw::ResponseAppearance::Suggested);
        confirm.set_default_response(Some("cancel"));
        confirm.connect_response(None, move |_, resp| {
            if resp == "install" {
                let cmd = PacmanManager::install_command(&["winetricks".to_string()]);
                crate::ui::dialogs::terminal::show(
                    &window_for_cb,
                    "Install winetricks",
                    &cmd,
                    None,
                );
            }
        });
        confirm.present(Some(&window));
        return;
    }

    match GameRunner::winetricks_command(&game) {
        Ok(cmd) => {
            crate::ui::dialogs::terminal::show(
                &window,
                &format!("Winetricks — {}", game.name),
                &cmd,
                None,
            );
        }
        Err(e) => toast(state, &e),
    }
}

fn show_log_dialog(parent: &gtk::Window, game: &GameEntry) {
    let game = game.clone();
    let dialog = adw::Dialog::builder()
        .title(format!("Launch Log — {}", game.name))
        .content_width(780)
        .content_height(520)
        .build();

    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::builder()
        .title_widget(&adw::WindowTitle::new(&game.name, "Launch log"))
        .build();

    let refresh_btn = gtk::Button::with_label("Refresh");
    let close_btn = gtk::Button::with_label("Close");
    let dlg_close = dialog.clone();
    close_btn.connect_clicked(move |_| dlg_close.force_close());
    header.pack_start(&refresh_btn);
    header.pack_end(&close_btn);
    toolbar.add_top_bar(&header);

    let text_view = gtk::TextView::builder()
        .vexpand(true)
        .hexpand(true)
        .editable(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::None)
        .left_margin(12)
        .right_margin(12)
        .top_margin(12)
        .bottom_margin(12)
        .build();

    let load_log = {
        let buffer = text_view.buffer();
        move || {
            let path = GameRunner::log_path(&game.id);
            let content = std::fs::read_to_string(&path).unwrap_or_else(|_| {
                "No log yet — launch the game once and its output will appear here.".to_string()
            });
            buffer.set_text(&tail_text(&content, 256 * 1024));
        }
    };
    load_log();

    let scrolled = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .child(&text_view)
        .build();
    toolbar.set_content(Some(&scrolled));
    dialog.set_child(Some(&toolbar));

    let refresh = load_log.clone();
    refresh_btn.connect_clicked(move |_| refresh());

    dialog.present(Some(parent));
}

/// Keep only the last `cap` bytes worth of lines, without splitting UTF-8.
fn tail_text(content: &str, cap: usize) -> String {
    if content.len() <= cap {
        return content.to_string();
    }
    let lines: Vec<&str> = content.lines().collect();
    let mut size = 0;
    let mut start = lines.len();
    while start > 0 {
        size += lines[start - 1].len() + 1;
        if size > cap {
            break;
        }
        start -= 1;
    }
    lines[start..].join("\n")
}

fn confirm_remove(state: &State, window: &gtk::Window, game: &GameEntry) {
    let state = state.clone();
    let game = game.clone();
    let confirm = adw::AlertDialog::builder()
        .heading(format!("Remove {}?", game.name))
        .body("The game files on disk are not touched. The Wine prefix can be deleted to reclaim space.")
        .build();
    confirm.add_response("cancel", "Cancel");
    confirm.add_response("remove", "Remove Entry");
    confirm.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
    confirm.add_response("remove-prefix", "Remove + Delete Prefix");
    confirm.set_response_appearance("remove-prefix", adw::ResponseAppearance::Destructive);
    confirm.set_default_response(Some("cancel"));
    confirm.connect_response(None, move |_, resp| {
        let delete_prefix = resp == "remove-prefix";
        if resp == "remove" || delete_prefix {
            state.library.borrow_mut().remove(&game.id);
            if delete_prefix {
                let _ = std::fs::remove_dir_all(game.prefix_dir());
            }
            state.preflight.borrow_mut().remove(&game.id);
            *state.current_detail.borrow_mut() = None;
            if state
                .nav_view
                .visible_page_tag()
                .map(|t| t == "detail")
                .unwrap_or(false)
            {
                state.nav_view.pop();
            }
            super::grid::rebuild_grid(&state);
            toast(&state, &format!("Removed {}", game.name));
        }
    });
    confirm.present(Some(window));
}
