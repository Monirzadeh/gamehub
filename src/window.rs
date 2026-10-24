use adw::prelude::*;
use gtk::glib;
use gtk::gio;

use crate::ui::pages;

/// One entry in the sidebar: (stack name, page title, icon).
const NAV_ITEMS: &[(&str, &str, &str)] = &[
    ("games", "Games", "applications-games-symbolic"),
    ("dashboard", "Dashboard", "view-grid-symbolic"),
    ("launchers", "Launchers", "applications-games"),
    ("proton", "Proton Tools", "media-playback-start"),
    ("compatibility", "Compatibility", "emblem-ok-symbolic"),
    ("steam-session", "Steam Session", "computer"),
    ("tools", "Tools", "applications-utilities"),
    ("settings", "Settings", "preferences-system"),
];

/// Wrap a page widget in its own header bar so every page in the content
/// stack looks like a full standalone view.
fn page_container(title: &str, widget: &impl IsA<gtk::Widget>) -> adw::ToolbarView {
    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::builder()
        .title_widget(&adw::WindowTitle::new(title, ""))
        .build();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(widget));
    toolbar
}

fn sidebar_page() -> (adw::NavigationPage, gtk::ListBox, adw::HeaderBar) {
    let list = gtk::ListBox::builder()
        .css_classes(["navigation-sidebar"])
        .selection_mode(gtk::SelectionMode::Single)
        .build();

    for (name, title, icon) in NAV_ITEMS {
        let row = gtk::ListBoxRow::builder().activatable(true).build();
        row.set_widget_name(name);
        let box_ = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(10)
            .margin_end(10)
            .build();
        let image = gtk::Image::from_icon_name(icon);
        image.set_pixel_size(16);
        box_.append(&image);
        box_.append(&gtk::Label::new(Some(title)));
        row.set_child(Some(&box_));
        list.append(&row);
    }
    list.select_row(list.first_child().and_downcast::<gtk::ListBoxRow>().as_ref());

    let scrolled = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .child(&list)
        .build();

    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::builder()
        .title_widget(&adw::WindowTitle::new("Parch Gamehub", "Linux Gaming Hub"))
        .build();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&scrolled));

    (
        adw::NavigationPage::builder()
            .child(&toolbar)
            .title("Sidebar")
            .build(),
        list,
        header,
    )
}

pub fn create(app: &adw::Application) -> adw::ApplicationWindow {
    let content_stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .transition_duration(150)
        .vexpand(true)
        .build();

    // The Games page manages its own header bars (library + pushed detail),
    // every other page gets a generic one.
    let games = pages::games::new();
    content_stack.add_named(&games, Some("games"));

    let dashboard = pages::dashboard::new(Some(content_stack.clone()));
    content_stack.add_named(&page_container("Dashboard", &dashboard), Some("dashboard"));
    let launchers = pages::launchers::new();
    content_stack.add_named(&page_container("Launchers", &launchers), Some("launchers"));
    let proton = pages::proton::new();
    content_stack.add_named(&page_container("Proton Tools", &proton), Some("proton"));
    let compat = pages::compat::new();
    content_stack.add_named(&page_container("Compatibility", &compat), Some("compatibility"));
    let steam_session = pages::steam_session::new();
    content_stack.add_named(&page_container("Steam Session", &steam_session), Some("steam-session"));
    let tools = pages::tools::new();
    content_stack.add_named(&page_container("Tools", &tools), Some("tools"));
    let settings = pages::settings::new();
    content_stack.add_named(&page_container("Settings", &settings), Some("settings"));

    let (sidebar, sidebar_list, sidebar_header) = sidebar_page();
    let content = adw::NavigationPage::builder()
        .child(&content_stack)
        .title("Gamehub")
        .build();

    let split = adw::NavigationSplitView::builder()
        .sidebar(&sidebar)
        .content(&content)
        .build();

    let split_for_nav = split.clone();
    let stack_for_sidebar = content_stack.clone();
    sidebar_list.connect_row_activated(move |_, row| {
        stack_for_sidebar.set_visible_child_name(&row.widget_name());
        // On narrow widths the sidebar is a pushed page — go back to content.
        split_for_nav.set_show_content(true);
    });

    // --- Keyboard shortcuts: Ctrl+1..8 switches pages, Ctrl+W closes ----------
    let actions = gio::SimpleActionGroup::new();
    for (idx, (name, _, _)) in NAV_ITEMS.iter().enumerate() {
        let action = gio::SimpleAction::new(&format!("goto-{}", name), None);
        let stack = content_stack.clone();
        let list = sidebar_list.clone();
        let split_cb = split.clone();
        let target = name.to_string();
        action.connect_activate(move |_, _| {
            stack.set_visible_child_name(&target);
            // Keep the sidebar selection in sync with the shortcut.
            let mut child = list.first_child();
            while let Some(widget) = child {
                child = widget.next_sibling();
                if let Some(row) = widget.downcast_ref::<gtk::ListBoxRow>() {
                    if row.widget_name() == target.as_str() {
                        list.select_row(Some(row));
                    }
                }
            }
            split_cb.set_show_content(true);
        });
        actions.add_action(&action);
        app.set_accels_for_action(
            &format!("win.goto-{}", name),
            &[&format!("<primary>{}", idx + 1)],
        );
    }

    let close_action = gio::SimpleAction::new("close", None);
    actions.add_action(&close_action);
    app.set_accels_for_action("win.close", &["<primary>w"]);

    let menu_button = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text("Main Menu")
        .build();

    let menu = gio::Menu::new();
    menu.append(Some("About Gamehub"), Some("app.about"));
    menu.append(Some("Keyboard Shortcuts"), Some("app.shortcuts"));
    menu_button.set_menu_model(Some(&menu));

    // The menu button lives in the sidebar header so it is always reachable.
    sidebar_header.pack_end(&menu_button);

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .default_width(1000)
        .default_height(700)
        .title("Parch Linux Gamehub")
        .content(&split)
        .build();

    window.insert_action_group("win", Some(&actions));

    let win_weak = window.downgrade();
    close_action.connect_activate(move |_, _| {
        if let Some(win) = win_weak.upgrade() {
            win.close();
        }
    });

    if let Ok(cond) = adw::BreakpointCondition::parse("max-width: 640px") {
        let breakpoint = adw::Breakpoint::new(cond);
        breakpoint.add_setter(&split, "collapsed", Some(&true.to_value()));
        window.add_breakpoint(breakpoint);
    }

    let app_weak = app.downgrade();
    window.connect_close_request(move |_| {
        if let Some(app) = app_weak.upgrade() {
            app.quit();
        }
        glib::Propagation::Proceed
    });

    window
}
