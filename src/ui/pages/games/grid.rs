use adw::prelude::*;

use super::{tier_badge, State};
use crate::modules::game_library::GameEntry;
use crate::modules::game_runner::GameRunner;

pub(super) fn rebuild_grid(state: &State) {
    while let Some(child) = state.flowbox.first_child() {
        state.flowbox.remove(&child);
    }

    let filter = state.search_entry.text().to_lowercase();
    let games: Vec<GameEntry> = {
        let lib = state.library.borrow();
        lib.games()
            .iter()
            .filter(|g| filter.is_empty() || g.name.to_lowercase().contains(&filter))
            .cloned()
            .collect()
    };

    let empty = games.is_empty();
    state.flowbox.set_visible(!empty);
    state.empty_box.set_visible(empty);

    for game in games {
        let child = gtk::FlowBoxChild::new();
        child.set_widget_name(&game.id);
        child.set_child(Some(&build_game_card(state, &game)));
        state.flowbox.append(&child);
    }
}

fn build_game_card(state: &State, game: &GameEntry) -> gtk::Box {
    let card = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(0)
        .css_classes(["card"])
        .hexpand(true)
        .build();

    match game
        .steam_appid
        .and_then(|id| state.covers.borrow().get(&id).cloned())
    {
        Some(path) => {
            let pic = gtk::Picture::for_filename(path);
            pic.set_content_fit(gtk::ContentFit::Cover);
            pic.set_can_shrink(true);
            pic.set_size_request(-1, 100);
            pic.set_hexpand(true);
            card.append(&pic);
        }
        None => {
            let icon_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .hexpand(true)
                .height_request(100)
                .build();
            let icon = gtk::Image::from_icon_name("applications-games-symbolic");
            icon.set_pixel_size(40);
            icon.add_css_class("dim-label");
            icon_box.append(&icon);
            card.append(&icon_box);
        }
    }

    let footer = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .margin_top(10)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    let name = gtk::Label::builder()
        .label(&game.name)
        .css_classes(["heading"])
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .halign(gtk::Align::Start)
        .build();
    footer.append(&name);

    let meta = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .build();

    if GameRunner::is_running(&game.id) {
        let running = gtk::Label::builder()
            .label("● Running")
            .css_classes(["success", "caption", "heading"])
            .halign(gtk::Align::Start)
            .valign(gtk::Align::Center)
            .hexpand(true)
            .build();
        meta.append(&running);
    } else {
        let filler = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        filler.set_hexpand(true);
        meta.append(&filler);
    }

    let tier = game
        .steam_appid
        .and_then(|id| state.tiers.borrow().get(&id).cloned());
    let (badge_text, badge_class) = match &tier {
        Some(t) => tier_badge(t),
        None if game.steam_appid.is_some() => ("…", "card"),
        None => ("NO DATA", "card"),
    };
    let badge = gtk::Label::builder()
        .label(badge_text)
        .css_classes([badge_class, "pill", "caption"])
        .valign(gtk::Align::Center)
        .build();
    meta.append(&badge);

    footer.append(&meta);
    card.append(&footer);
    card
}
