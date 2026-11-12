use adw::prelude::*;
use gtk::glib;
use gtk::gio;

use crate::config::AppConfig;
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

/// Build a toggle button bound to the desktop split view's `show-sidebar` property.
/// Hidden automatically on small screens (`<= 640px`).
fn sidebar_toggle_button(
    desktop_split: &adw::OverlaySplitView,
    mobile_split: &adw::NavigationSplitView,
) -> gtk::ToggleButton {
    let btn = gtk::ToggleButton::builder()
        .icon_name("sidebar-show-symbolic")
        .tooltip_text("Toggle Sidebar (Ctrl+/)")
        .build();
    desktop_split
        .bind_property("show-sidebar", &btn, "active")
        .bidirectional()
        .sync_create()
        .build();
    mobile_split
        .bind_property("collapsed", &btn, "visible")
        .invert_boolean()
        .sync_create()
        .build();
    btn
}

/// Wrap a page widget in its own header bar so every page in the content
/// stack looks like a full standalone view.
fn page_container(title: &str, widget: &impl IsA<gtk::Widget>, desktop_split: &adw::OverlaySplitView, mobile_split: &adw::NavigationSplitView,) -> adw::ToolbarView {
    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::builder()
        .title_widget(&adw::WindowTitle::new(title, ""))
        .build();
    header.pack_start(&sidebar_toggle_button(desktop_split, mobile_split));
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(widget));
    toolbar
}

fn sidebar_page() -> (gtk::Widget, gtk::ListBox, adw::HeaderBar) {
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

    (toolbar.upcast(), list, header)
}

pub fn create(app: &adw::Application) -> adw::ApplicationWindow {
    let content_stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .transition_duration(150)
        .hhomogeneous(false)
        .vexpand(true)
        .build();

    let config = std::rc::Rc::new(AppConfig::new());
    let (sidebar, sidebar_list, sidebar_header) = sidebar_page();

    // Desktop view: collapsible sidebar on big screens (> 720px)
    let desktop_split = adw::OverlaySplitView::builder()
        .sidebar(&sidebar)
        .content(&content_stack)
        .show_sidebar(config.show_sidebar())
        .collapsed(false)
        .build();

    // Mobile view: exact original NavigationSplitView on small screens (<= 640px)
    let mobile_sidebar_page = adw::NavigationPage::builder()
        .child(&gtk::Box::new(gtk::Orientation::Vertical, 0))
        .title("Sidebar")
        .build();
    let mobile_content_page = adw::NavigationPage::builder()
        .child(&gtk::Box::new(gtk::Orientation::Vertical, 0))
        .title("Gamehub")
        .build();
    let mobile_split = adw::NavigationSplitView::builder()
        .sidebar(&mobile_sidebar_page)
        .content(&mobile_content_page)
        .build();

    let root_stack = adw::ViewStack::builder()
        .hhomogeneous(false)
        .build();
    root_stack.add_named(&desktop_split, Some("desktop"));
    root_stack.add_named(&mobile_split, Some("mobile"));
    root_stack.set_visible_child_name("desktop");

    // Move sidebar & content cleanly between desktop and mobile layouts at 640px
    {
        let config = config.clone();
        let sidebar = sidebar.clone();
        let content_stack = content_stack.clone();
        let desktop_split = desktop_split.clone();
        let mobile_sidebar_page = mobile_sidebar_page.clone();
        let mobile_content_page = mobile_content_page.clone();
        let root_stack = root_stack.clone();
        mobile_split.connect_collapsed_notify(move |split| {
            if split.is_collapsed() {
                desktop_split.set_collapsed(true);
                desktop_split.set_sidebar(gtk::Widget::NONE);
                desktop_split.set_content(gtk::Widget::NONE);
                mobile_sidebar_page.set_child(Some(&sidebar));
                mobile_content_page.set_child(Some(&content_stack));
                split.set_show_content(true);
                root_stack.set_visible_child_name("mobile");
            } else {
                mobile_sidebar_page.set_child(gtk::Widget::NONE);
                mobile_content_page.set_child(gtk::Widget::NONE);
                desktop_split.set_collapsed(false);
                desktop_split.set_sidebar(Some(&sidebar));
                desktop_split.set_content(Some(&content_stack));
                desktop_split.set_show_sidebar(config.show_sidebar());
                root_stack.set_visible_child_name("desktop");
            }
        });
    }

    {
        let config = config.clone();
        let mobile_split = mobile_split.clone();
        desktop_split.connect_show_sidebar_notify(move |split| {
            if !mobile_split.is_collapsed() {
                config.set_show_sidebar(split.shows_sidebar());
            }
        });
    }

    // The Games page manages its own header bars (library + pushed detail),
    // every other page gets a generic one.
    let (games, games_flowbox, games_add_content) =
        pages::games::new(&sidebar_toggle_button(&desktop_split, &mobile_split));
    content_stack.add_named(&games, Some("games"));

    let dashboard = pages::dashboard::new(Some(content_stack.clone()));
    content_stack.add_named(&page_container("Dashboard", &dashboard, &desktop_split, &mobile_split),Some("dashboard"),);
    let (launchers, launchers_flowbox) = pages::launchers::new();
    content_stack.add_named(&page_container("Launchers", &launchers, &desktop_split, &mobile_split),Some("launchers"),);
    let proton = pages::proton::new();
    content_stack.add_named(&page_container("Proton Tools", &proton, &desktop_split, &mobile_split),Some("proton"),);
    let compat = pages::compat::new();
    content_stack.add_named(&page_container("Compatibility", &compat, &desktop_split, &mobile_split),Some("compatibility"),);
    let steam_session = pages::steam_session::new();
    content_stack.add_named(&page_container("Steam Session", &steam_session, &desktop_split, &mobile_split),Some("steam-session"),);
    let tools = pages::tools::new();
    content_stack.add_named(&page_container("Tools", &tools, &desktop_split, &mobile_split),Some("tools"),);
    let settings = pages::settings::new();content_stack.add_named(&page_container("Settings", &settings, &desktop_split, &mobile_split),Some("settings"),);

    let mobile_split_for_nav = mobile_split.clone();
    let stack_for_sidebar = content_stack.clone();
    sidebar_list.connect_row_activated(move |_, row| {
        stack_for_sidebar.set_visible_child_name(&row.widget_name());
        // On narrow widths the sidebar is a pushed page — go back to content.
        mobile_split_for_nav.set_show_content(true);
    });

    // --- Keyboard shortcuts: Ctrl+1..8 switches pages, Ctrl+/ toggles sidebar, Ctrl+W closes
    let actions = gio::SimpleActionGroup::new();
    for (idx, (name, _, _)) in NAV_ITEMS.iter().enumerate() {
        let action = gio::SimpleAction::new(&format!("goto-{}", name), None);
        let stack = content_stack.clone();
        let list = sidebar_list.clone();
        let mobile_cb = mobile_split.clone();
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
            mobile_cb.set_show_content(true);
        });
        actions.add_action(&action);
        app.set_accels_for_action(
            &format!("win.goto-{}", name),
            &[&format!("<primary>{}", idx + 1)],
        );
    }

    let toggle_sidebar_action = gio::SimpleAction::new("toggle-sidebar", None);
    let desktop_toggle = desktop_split.clone();
    let mobile_toggle = mobile_split.clone();
    toggle_sidebar_action.connect_activate(move |_, _| {
        if !mobile_toggle.is_collapsed() {
            desktop_toggle.set_show_sidebar(!desktop_toggle.shows_sidebar());
        }
    });
    actions.add_action(&toggle_sidebar_action);
    app.set_accels_for_action("win.toggle-sidebar", &["<primary>slash"]);

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
        .content(&root_stack)
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
        breakpoint.add_setter(&mobile_split, "collapsed", Some(&true.to_value()));
        window.add_breakpoint(breakpoint);
    }
   
    if let Ok(cond) = adw::BreakpointCondition::parse("max-width: 490px") {
        let breakpoint = adw::Breakpoint::new(cond);
        breakpoint.add_setter(&mobile_split, "collapsed", Some(&true.to_value()));
        breakpoint.add_setter(
            &games_flowbox,
            "min-children-per-line",
            Some(&1u32.to_value()),
        );
        breakpoint.add_setter(
            &games_flowbox,
            "max-children-per-line",
            Some(&1u32.to_value()),
        );
        breakpoint.add_setter(
            &launchers_flowbox,
            "min-children-per-line",
            Some(&1u32.to_value()),
        );
        breakpoint.add_setter(
            &launchers_flowbox,
            "max-children-per-line",
            Some(&1u32.to_value()),
        );
        breakpoint.add_setter(&games_add_content, "label", Some(&"".to_value()));
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
