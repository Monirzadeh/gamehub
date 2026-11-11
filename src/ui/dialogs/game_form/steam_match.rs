use std::rc::Rc;

use adw::prelude::*;
use gtk::glib;

use super::FormState;
use crate::modules::protondb::{ProtonDbClient, SteamGameMatch};

/// "Steam Match (Optional)" section: match the game to its Steam AppID to
/// enable cover art and the ProtonDB rating.
pub(super) fn group(form: &Rc<FormState>) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder()
        .title("Steam Match (Optional)")
        .description("Match the game to its Steam AppID for cover art and ProtonDB rating.")
        .build();

    let search_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .build();
    let search_entry = gtk::SearchEntry::builder()
        .placeholder_text("Search Steam for this game…")
        .hexpand(true)
        .build();
    let search_btn = gtk::Button::builder()
        .label("Search")
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .build();
    search_box.append(&search_entry);
    search_box.append(&search_btn);
    group.add(&search_box);

    let match_row = adw::ActionRow::builder().visible(false).build();
    let match_clear = gtk::Button::builder()
        .label("Clear")
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .build();
    match_row.add_suffix(&match_clear);
    group.add(&match_row);

    let custom_art_row = adw::ActionRow::builder()
        .title("Custom Cover Art")
        .subtitle("Select an image from disk to use as this game's cover")
        .visible(false)
        .build();
    let custom_art_browse = gtk::Button::builder()
        .label("Browse…")
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .build();
    let custom_art_remove = gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .tooltip_text("Remove custom cover art")
        .css_classes(["flat", "destructive-action"])
        .valign(gtk::Align::Center)
        .visible(false)
        .build();
    custom_art_row.add_suffix(&custom_art_browse);
    custom_art_row.add_suffix(&custom_art_remove);
    group.add(&custom_art_row);

    let search_results = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .visible(false)
        .build();
    group.add(&search_results);

    // Show the existing match when editing.
    let covers_dir = form.covers_dir.clone();
    set_match_display(&match_row, &custom_art_row, &custom_art_remove, &covers_dir, form.matched_appid.borrow().as_ref().copied(), None);

    {
        let form = form.clone();
        let custom_art_browse_clone = custom_art_browse.clone();
        let remove_btn_clone = custom_art_remove.clone();
        
        custom_art_browse.connect_clicked(move |_| {
            let form_inner = form.clone();
            let remove_btn_inner = remove_btn_clone.clone();
            
            let Some(appid) = form_inner.matched_appid.borrow().as_ref().copied() else {
                return;
            };

            let Some(root_window) = custom_art_browse_clone.root().and_then(|r| r.downcast::<gtk::Window>().ok()) else {
                return;
            };

            let chooser = gtk::FileDialog::builder()
                .title("Select Custom Cover Art Image")
                .accept_label("Use Image")
                .build();

            let filter = gtk::FileFilter::new();
            filter.set_name(Some("Images (*.png, *.jpg)"));
            filter.add_mime_type("image/png");
            filter.add_mime_type("image/jpeg");
            let filters = gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&filter);
            chooser.set_filters(Some(&filters));

            let window_for_closure = root_window.clone();

            let browse_covers_dir = form.covers_dir.clone();
            
            chooser.open(Some(&root_window), gio::Cancellable::NONE, move |result| {
                if let Ok(file) = result {
                    if let Some(src_path) = file.path() {
                        let target_path = browse_covers_dir.join(format!("{}.png", appid));

                        let _ = std::fs::create_dir_all(&browse_covers_dir);
                        if std::fs::copy(&src_path, &target_path).is_ok() {
                            remove_btn_inner.set_visible(true);
                            if let Some(toast_overlay) = window_for_closure.child().and_then(|c| c.downcast::<adw::ToastOverlay>().ok()) {
                                toast_overlay.add_toast(adw::Toast::new("Custom cover art applied successfully!"));
                            }
                        }
                    }
                }
            });
        });
    }
    // --- Connect Custom Art Remove Button Click Event ---
    {
        let form = form.clone();
        let custom_art_remove_clone = custom_art_remove.clone();
        let remove_covers_dir = form.covers_dir.clone();
        custom_art_remove.connect_clicked(move |_| {
            if let Some(appid) = form.matched_appid.borrow().as_ref().copied() {
                let target_path = remove_covers_dir.join(format!("{}.png", appid));

                if target_path.is_file() {
                    let _ = std::fs::remove_file(target_path);
                }

                custom_art_remove_clone.set_visible(false);

                if let Some(root_window) = custom_art_remove_clone.root().and_then(|r| r.downcast::<gtk::Window>().ok()) {
                    if let Some(toast_overlay) = root_window.child().and_then(|c| c.downcast::<adw::ToastOverlay>().ok()) {
                        // Toast message updated to explicitly let the user know we're falling back [1]
                        toast_overlay.add_toast(adw::Toast::new("Custom cover art removed! Online cover will reload on Save."));
                    }
                }
            }
        });
    }
    let search_entry_trig = search_entry.clone();
    let search_btn_trig = search_btn.clone();
    let search_results_trig = search_results.clone();
    let match_row_trig = match_row.clone();
    let custom_art_row_trig = custom_art_row.clone();
    let custom_art_remove_trig = custom_art_remove.clone();
    let form_trig = form.clone();
    let search_trigger = move || {
        let query = search_entry_trig.text().trim().to_string();
        if query.is_empty() {
            return;
        }
        search_btn_trig.set_sensitive(false);
        let (tx, rx) = std::sync::mpsc::channel::<Vec<SteamGameMatch>>();
        std::thread::spawn(move || {
            let _ = tx.send(ProtonDbClient::search_steam_games(&query));
        });

        let search_results = search_results_trig.clone();
        let match_row = match_row_trig.clone();
        let custom_art_row = custom_art_row_trig.clone();
        let custom_art_remove = custom_art_remove_trig.clone();
        let search_btn = search_btn_trig.clone();
        let form = form_trig.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(80), move || {
            let Ok(matches) = rx.try_recv() else {
                return glib::ControlFlow::Continue;
            };
            search_btn.set_sensitive(true);
            populate_results(&search_results, &match_row, &custom_art_row, &custom_art_remove, &form, &matches);
            glib::ControlFlow::Break
        });
    };

    let t = search_trigger.clone();
    search_btn.connect_clicked(move |_| t());
    let t = search_trigger.clone();
    search_entry.connect_activate(move |_| t());

    {
        let form = form.clone();
        let match_row = match_row.clone();
        let custom_art_row = custom_art_row.clone();
        let custom_art_remove = custom_art_remove.clone();
        let clear_covers_dir = form.covers_dir.clone();
        match_clear.connect_clicked(move |_| {
            *form.matched_appid.borrow_mut() = None;
            set_match_display(&match_row, &custom_art_row, &custom_art_remove, &clear_covers_dir, None, None);
        });
    }

    group
}

fn populate_results(
    search_results: &gtk::ListBox,
    match_row: &adw::ActionRow,
    custom_art_row: &adw::ActionRow,
    custom_art_remove: &gtk::Button,
    form: &Rc<FormState>,
    matches: &[SteamGameMatch],
) {
    while let Some(child) = search_results.first_child() {
        search_results.remove(&child);
    }
    for m in matches.iter().take(6) {
        let row = adw::ActionRow::builder()
            .title(&m.name)
            .subtitle(format!("AppID {}", m.id))
            .activatable(true)
            .build();
        let id = m.id;
        let name = m.name.clone();
        let form = form.clone();
        let row_results = search_results.clone();
        let match_row = match_row.clone();
        let custom_art_row = custom_art_row.clone();
        let custom_art_remove = custom_art_remove.clone();
        let populate_covers_dir = form.covers_dir.clone();
        row.connect_activated(move |_| {
            *form.matched_appid.borrow_mut() = Some(id);
            set_match_display(&match_row, &custom_art_row, &custom_art_remove, &populate_covers_dir, Some(id), Some(&name));
            row_results.set_visible(false);
        });

        search_results.append(&row);
    }
    search_results.set_visible(!matches.is_empty());
}

fn set_match_display(match_row: &adw::ActionRow,custom_art_row: &adw::ActionRow, custom_art_remove: &gtk::Button, covers_dir: &std::path::Path, appid: Option<u64>, name: Option<&str>) {
    match (appid, name) {
        (Some(id), Some(n)) => {
            match_row.set_title(&format!("Matched: {}", n));
            match_row.set_subtitle(&format!("Steam AppID {}", id));
            match_row.set_visible(true);
            custom_art_row.set_visible(true);
            let local_custom_path = covers_dir.join(format!("{}.png", id));
            custom_art_remove.set_visible(local_custom_path.is_file());
        }
        (Some(id), None) => {
            match_row.set_title(&format!("Matched AppID {}", id));
            match_row.set_subtitle("Cover art and ProtonDB rating enabled");
            match_row.set_visible(true);
            custom_art_row.set_visible(true);

            let local_custom_path = covers_dir.join(format!("{}.png", id));
            custom_art_remove.set_visible(local_custom_path.is_file());
        }
        (None, _) => {
            match_row.set_visible(false);
            custom_art_row.set_visible(false);
            custom_art_remove.set_visible(false);
        }
    }
}
