mod detail;
mod grid;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use gtk::glib;

use crate::modules::game_library::GameLibrary;
use crate::ui::dialogs::game_form;

pub struct PageState {
    pub(super) library: Rc<RefCell<GameLibrary>>,
    pub(super) tiers: Rc<RefCell<HashMap<u64, String>>>,
    pub(super) covers: Rc<RefCell<HashMap<u64, PathBuf>>>,
    pub(super) fetching: Rc<RefCell<HashSet<u64>>>,
    pub(super) preflight: Rc<RefCell<HashMap<String, Vec<crate::modules::protondb::CheckRow>>>>,
    pub(super) current_detail: Rc<RefCell<Option<String>>>,
    pub(super) nav_view: adw::NavigationView,
    pub(super) detail_page: adw::NavigationPage,
    pub(super) detail_toolbar: adw::ToolbarView,
    pub(super) detail_title: adw::WindowTitle,
    pub(super) flowbox: gtk::FlowBox,
    pub(super) empty_box: gtk::Box,
    pub(super) search_entry: gtk::SearchEntry,
    pub(super) toast: adw::ToastOverlay,
    pub(super) covers_dir: PathBuf, 
}

pub(super) type State = Rc<PageState>;

pub fn new(sidebar_toggle: &gtk::ToggleButton) -> (gtk::Box, gtk::FlowBox, adw::ButtonContent) {
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();

    let toast_overlay = adw::ToastOverlay::new();
    let nav_view = adw::NavigationView::new();
    toast_overlay.set_child(Some(&nav_view));
    root.append(&toast_overlay);

    // =========================================================================
    // Library page (grid)
    // =========================================================================
    let search_entry = gtk::SearchEntry::builder()
        .placeholder_text("Filter games…")
        .hexpand(true)
        .build();

    let search_bar = gtk::SearchBar::builder().show_close_button(true).build();
    search_bar.set_child(Some(&search_entry));

    let search_toggle = gtk::ToggleButton::builder()
        .icon_name("system-search-symbolic")
        .tooltip_text("Search games")
        .build();
    search_toggle
        .bind_property("active", &search_bar, "search-mode-enabled")
        .bidirectional()
        .sync_create()
        .build();

    let add_content = adw::ButtonContent::builder()
        .icon_name("list-add-symbolic")
        .label("Add Game")
        .build();
        
    let add_btn = gtk::Button::builder()
        .css_classes(["suggested-action"])
        .tooltip_text("Add Game (Ctrl+N)")
        .child(&add_content)
        .build();

    let header = adw::HeaderBar::builder()
        .title_widget(&adw::WindowTitle::new("Games", "Your Windows game library"))
        .build();
    header.pack_start(sidebar_toggle);
    header.pack_end(&add_btn);
    header.pack_end(&search_toggle);

    let library_toolbar = adw::ToolbarView::new();
    library_toolbar.add_top_bar(&header);
    library_toolbar.add_top_bar(&search_bar);

    let clamp = adw::Clamp::builder()
        .maximum_size(1000)
        .tightening_threshold(800)
        .build();

    let grid_content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .margin_top(20)
        .margin_bottom(24)
        .margin_start(12)
        .margin_end(12)
        .build();

    let flowbox = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .activate_on_single_click(true)
        .min_children_per_line(2)
        .max_children_per_line(5)
        .column_spacing(14)
        .row_spacing(14)
        .hexpand(true)
        .build();
    grid_content.append(&flowbox);

    let empty_status = adw::StatusPage::builder()
        .icon_name("applications-games-symbolic")
        .title("No games yet")
        .description("Add a Windows game from your disk and gamehub will run it with Proton.")
        .build();
    let empty_add = gtk::Button::builder()
        .label("Add Your First Game")
        .css_classes(["suggested-action", "pill"])
        .halign(gtk::Align::Center)
        .build();
    let empty_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .valign(gtk::Align::Center)
        .vexpand(true)
        .visible(false)
        .build();
    empty_box.append(&empty_status);
    empty_box.append(&empty_add);
    grid_content.append(&empty_box);

    clamp.set_child(Some(&grid_content));
    let scrolled = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .child(&clamp)
        .build();
    library_toolbar.set_content(Some(&scrolled));

    let library_page = adw::NavigationPage::builder()
        .child(&library_toolbar)
        .title("Games")
        .tag("library")
        .build();
    nav_view.add(&library_page);

    // =========================================================================
    // Detail page (content is rebuilt per game by detail::populate_detail)
    // =========================================================================
    let detail_title = adw::WindowTitle::new("Game", "");
    let detail_header = adw::HeaderBar::builder().title_widget(&detail_title).build();
    let detail_toolbar = adw::ToolbarView::new();
    detail_toolbar.add_top_bar(&detail_header);
    let detail_page = adw::NavigationPage::builder()
        .child(&detail_toolbar)
        .title("Game")
        .tag("detail")
        .build();

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let covers_dir = PathBuf::from(home).join(".local/share/gamehub/covers");
    let _ = std::fs::create_dir_all(&covers_dir);

    let state: State = Rc::new(PageState {
        library: Rc::new(RefCell::new(GameLibrary::load())),
        tiers: Rc::new(RefCell::new(HashMap::new())),
        covers: Rc::new(RefCell::new(HashMap::new())),
        fetching: Rc::new(RefCell::new(HashSet::new())),
        preflight: Rc::new(RefCell::new(HashMap::new())),
        current_detail: Rc::new(RefCell::new(None)),
        nav_view: nav_view.clone(),
        detail_page: detail_page.clone(),
        detail_toolbar: detail_toolbar.clone(),
        detail_title: detail_title.clone(),
        flowbox: flowbox.clone(),
        empty_box: empty_box.clone(),
        search_entry: search_entry.clone(),
        toast: toast_overlay.clone(),
        covers_dir,
    });

    // --- Wiring ---------------------------------------------------------------

    let state_add = state.clone();
    add_btn.connect_clicked(move |btn| {
        if let Some(window) = btn.root().and_downcast_ref::<gtk::Window>() {
            open_add_dialog(&state_add, window);
        }
    });

    let state_empty = state.clone();
    empty_add.connect_clicked(move |btn| {
        if let Some(window) = btn.root().and_downcast_ref::<gtk::Window>() {
            open_add_dialog(&state_empty, window);
        }
    });

    {
        let state = state.clone();
        let entry = search_entry.clone();
        search_entry.connect_search_changed(move |_| {
            let _ = entry;
            grid::rebuild_grid(&state);
        });
    }

    {
        let state = state.clone();
        flowbox.connect_child_activated(move |_, child| {
            let id = child.widget_name().to_string();
            detail::populate_detail(&state, &id);
        });
    }

    // Returning from a detail page (back button or gesture) refreshes the grid
    // so running indicators and covers are up to date.
    {
        let state = state.clone();
        nav_view.connect_popped(move |_, _page| {
            *state.current_detail.borrow_mut() = None;
            grid::rebuild_grid(&state);
        });
    }

    add_shortcuts(&state, &root, &search_bar, &search_entry);

    grid::rebuild_grid(&state);
    ensure_meta(&state);

    (root, flowbox, add_content)
}

/// Page-local shortcuts: Ctrl+N add game, Ctrl+F search, F5 refresh,
/// Escape closes search or pops the detail page.
fn add_shortcuts(
    state: &State,
    root: &gtk::Box,
    search_bar: &gtk::SearchBar,
    search_entry: &gtk::SearchEntry,
) {
    let controller = gtk::ShortcutController::new();
    controller.set_scope(gtk::ShortcutScope::Local);

    let state_n = state.clone();
    controller.add_shortcut(gtk::Shortcut::new(
        Some(gtk::KeyvalTrigger::new(
            gtk::gdk::Key::n,
            gtk::gdk::ModifierType::CONTROL_MASK,
        )),
        Some(gtk::CallbackAction::new(move |widget, _| {
            if let Some(window) = widget.root().and_downcast_ref::<gtk::Window>() {
                open_add_dialog(&state_n, window);
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        })),
    ));

    let search_bar_f = search_bar.clone();
    let search_entry_f = search_entry.clone();
    controller.add_shortcut(gtk::Shortcut::new(
        Some(gtk::KeyvalTrigger::new(
            gtk::gdk::Key::f,
            gtk::gdk::ModifierType::CONTROL_MASK,
        )),
        Some(gtk::CallbackAction::new(move |_, _| {
            search_bar_f.set_search_mode(true);
            search_entry_f.grab_focus();
            glib::Propagation::Stop
        })),
    ));

    let state_r = state.clone();
    controller.add_shortcut(gtk::Shortcut::new(
        Some(gtk::KeyvalTrigger::new(
            gtk::gdk::Key::F5,
            gtk::gdk::ModifierType::empty(),
        )),
        Some(gtk::CallbackAction::new(move |_, _| {
            grid::rebuild_grid(&state_r);
            ensure_meta(&state_r);
            toast(&state_r, "Library refreshed");
            glib::Propagation::Stop
        })),
    ));

    let state_e = state.clone();
    let search_bar_e = search_bar.clone();
    controller.add_shortcut(gtk::Shortcut::new(
        Some(gtk::KeyvalTrigger::new(
            gtk::gdk::Key::Escape,
            gtk::gdk::ModifierType::empty(),
        )),
        Some(gtk::CallbackAction::new(move |_, _| {
            if search_bar_e.is_search_mode() {
                search_bar_e.set_search_mode(false);
                return glib::Propagation::Stop;
            }
            if state_e
                .nav_view
                .visible_page_tag()
                .map(|t| t == "detail")
                .unwrap_or(false)
            {
                state_e.nav_view.pop();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        })),
    ));

    root.add_controller(controller);
}

/// Open the Add Game dialog and refresh everything after a game is saved.
pub(super) fn open_add_dialog(state: &State, window: &gtk::Window) {
    let state = state.clone();
    game_form::show(
        window,
        state.library.clone(),
        None,
        Box::new(move |entry: crate::modules::game_library::GameEntry| {
            toast(&state, &format!("Added {} to your library", entry.name));
            grid::rebuild_grid(&state);
            ensure_meta(&state);
        }),
    );
}

pub(super) fn toast(state: &State, msg: &str) {
    state.toast.add_toast(adw::Toast::new(msg));
}

pub(super) fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub(super) fn format_playtime(secs: u64) -> String {
    if secs < 60 {
        return format!("{} sec", secs);
    }
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 {
        format!("{}h {}m", h, m)
    } else {
        format!("{}m", m)
    }
}

pub(super) fn format_last_played(secs: u64) -> String {
    glib::DateTime::from_unix_local(secs as i64)
        .ok()
        .and_then(|dt| dt.format("%Y-%m-%d %H:%M").ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

pub(super) fn tier_badge(tier: &str) -> (&'static str, &'static str) {
    match tier {
        "platinum" => ("PLATINUM", "success"),
        "gold" => ("GOLD", "accent"),
        "silver" => ("SILVER", "card"),
        "bronze" => ("BRONZE", "warning"),
        "borked" => ("BORKED", "destructive-action"),
        _ => ("NO DATA", "card"),
    }
}

/// Fetch ProtonDB tiers and cover art for every matched game that is missing
/// either, then refresh whatever is on screen. Guarded so each appid is only
/// in flight once (rebuilds triggered on completion do not refetch).
pub(super) fn ensure_meta(state: &State) {
    // (appid, need_tier, need_cover) decided up front so the worker thread
    // touches no shared state.
    let covers_dir = state.covers_dir.clone(); 
    let jobs: Vec<(u64, bool, bool)> = {
        let lib = state.library.borrow();
        let fetching = state.fetching.borrow();
        let tiers = state.tiers.borrow();
        let covers = state.covers.borrow();
        lib.games()
            .iter()
            .filter_map(|g| g.steam_appid)
            .filter(|id| !fetching.contains(id))
            .map(|id| {
                let need_tier = !tiers.contains_key(&id);
                
                let local_custom_path = covers_dir.join(format!("{}.png", id));
                let has_cached_cover = covers.contains_key(&id);
                let need_cover = !has_cached_cover || (has_cached_cover && !local_custom_path.is_file());

                (id, need_tier, need_cover)
            })
            .filter(|(_, tier, cover)| *tier || *cover)
            .collect()
    };
    if jobs.is_empty() {
        return;
    }

    {
        let mut fetching = state.fetching.borrow_mut();
        for (id, _, _) in &jobs {
            fetching.insert(*id);
        }
    }

    let (tx, rx) = std::sync::mpsc::channel::<Vec<(u64, Option<String>, Option<PathBuf>)>>();
    let worker_covers_dir = state.covers_dir.clone();

    std::thread::spawn(move || {
        let mut results = Vec::new();

        for (id, need_tier, need_cover) in jobs {
            let tier = if need_tier {
                crate::modules::protondb::ProtonDbClient::get_protondb_summary(id)
                    .map(|s| s.tier)
            } else {
                None
            };
            let cover = if need_cover {
                let local_custom_path = worker_covers_dir.join(format!("{}.png", id));
                if local_custom_path.is_file() {
                    Some(local_custom_path) 
                } else {
                    let online_res = crate::modules::protondb::ProtonDbClient::get_cover_image(id);
                    if online_res.is_some() {
                        online_res
                    } else {
                        let default_fallback = covers_dir.join("default.png");
                        if default_fallback.is_file() {
                            Some(default_fallback)
                        } else {
                            None 
                        }
                    }
                }
            } else {
                None
            };
            results.push((id, tier, cover));
        }
        let _ = tx.send(results);
    });

    let state = state.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(120), move || {
        let Ok(results) = rx.try_recv() else {
            return glib::ControlFlow::Continue;
        };
        let mut refreshed_ids: HashSet<u64> = HashSet::new();
        for (id, tier, cover) in results {
            if let Some(t) = tier {
                state.tiers.borrow_mut().insert(id, t);
            }
            if let Some(c) = cover {
                state.covers.borrow_mut().insert(id, c);
            } else {
                state.covers.borrow_mut().remove(&id); 
            }
            state.fetching.borrow_mut().remove(&id);
            refreshed_ids.insert(id);
        }
        grid::rebuild_grid(&state);

        // Refresh the open detail page if its game was among the fetched ones.
        let detail_id = state.current_detail.borrow().clone();
        if let Some(id) = detail_id {
            let appid = state.library.borrow().get(&id).and_then(|g| g.steam_appid);
            if let Some(appid) = appid {
                if refreshed_ids.contains(&appid) {
                    detail::populate_detail(&state, &id);
                }
            }
        }
        glib::ControlFlow::Break
    });
}
