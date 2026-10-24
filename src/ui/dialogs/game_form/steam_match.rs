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

    let search_results = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .visible(false)
        .build();
    group.add(&search_results);

    // Show the existing match when editing.
    set_match_display(&match_row, form.matched_appid.borrow().as_ref().copied(), None);

    let search_entry_trig = search_entry.clone();
    let search_btn_trig = search_btn.clone();
    let search_results_trig = search_results.clone();
    let match_row_trig = match_row.clone();
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
        let search_btn = search_btn_trig.clone();
        let form = form_trig.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(80), move || {
            let Ok(matches) = rx.try_recv() else {
                return glib::ControlFlow::Continue;
            };
            search_btn.set_sensitive(true);
            populate_results(&search_results, &match_row, &form, &matches);
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
        match_clear.connect_clicked(move |_| {
            *form.matched_appid.borrow_mut() = None;
            set_match_display(&match_row, None, None);
        });
    }

    group
}

fn populate_results(
    search_results: &gtk::ListBox,
    match_row: &adw::ActionRow,
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
        row.connect_activated(move |_| {
            *form.matched_appid.borrow_mut() = Some(id);
            set_match_display(&match_row, Some(id), Some(&name));
            row_results.set_visible(false);
        });
        search_results.append(&row);
    }
    search_results.set_visible(!matches.is_empty());
}

fn set_match_display(match_row: &adw::ActionRow, appid: Option<u64>, name: Option<&str>) {
    match (appid, name) {
        (Some(id), Some(n)) => {
            match_row.set_title(&format!("Matched: {}", n));
            match_row.set_subtitle(&format!("Steam AppID {}", id));
            match_row.set_visible(true);
        }
        (Some(id), None) => {
            match_row.set_title(&format!("Matched AppID {}", id));
            match_row.set_subtitle("Cover art and ProtonDB rating enabled");
            match_row.set_visible(true);
        }
        (None, _) => match_row.set_visible(false),
    }
}
