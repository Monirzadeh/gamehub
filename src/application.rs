use adw::prelude::*;
use gtk::gio;

use crate::window;

pub fn run() {
    let _ = adw::init();
    let style_manager = adw::StyleManager::default();
    style_manager.set_color_scheme(adw::ColorScheme::Default);

    let app = adw::Application::builder()
        .application_id("com.parchlinux.gamehub")
        .resource_base_path("/com/parchlinux/gamehub")
        .build();

    app.connect_startup(|app| {
        setup_actions(app);
    });

    app.connect_activate(|app| {
        let win = window::create(app);
        win.present();
    });

    app.run();
}

fn setup_actions(app: &adw::Application) {
    let quit_action = gio::SimpleAction::new("quit", None);
    let app_weak = app.downgrade();
    quit_action.connect_activate(move |_, _| {
        if let Some(app) = app_weak.upgrade() {
            app.quit();
        }
    });
    app.add_action(&quit_action);

    let about_action = gio::SimpleAction::new("about", None);
    let app_weak = app.downgrade();
    about_action.connect_activate(move |_, _| {
        if let Some(app) = app_weak.upgrade() {
            show_about(&app);
        }
    });
    app.add_action(&about_action);

    let shortcuts_action = gio::SimpleAction::new("shortcuts", None);
    let app_weak = app.downgrade();
    shortcuts_action.connect_activate(move |_, _| {
        if let Some(app) = app_weak.upgrade() {
            show_shortcuts(&app);
        }
    });
    app.add_action(&shortcuts_action);

    app.set_accels_for_action("app.quit", &["<primary>q"]);
    app.set_accels_for_action("app.shortcuts", &["<primary>question"]);
}

fn show_about(app: &adw::Application) {
    let about = adw::AboutDialog::builder()
        .application_name("Parch Linux Gamehub")
        .application_icon("com.parchlinux.gamehub")
        .version(env!("CARGO_PKG_VERSION"))
        .developers(["Parch Linux"])
        .website("https://parchlinux.com")
        .copyright("© 2026 Parch Linux")
        .license_type(gtk::License::Agpl30)
        .build();
    about.present(app.active_window().as_ref());
}

fn show_shortcuts(app: &adw::Application) {
    let builder = gtk::Builder::from_string(
        r#"<interface>
  <object class="GtkShortcutsWindow" id="shortcuts">
    <property name="modal">true</property>
    <child>
      <object class="GtkShortcutsSection">
        <property name="section-name">general</property>
        <property name="title">General</property>
        <child>
          <object class="GtkShortcutsGroup">
            <property name="title">Application</property>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Quit</property>
                <property name="accelerator">&lt;primary&gt;q</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Close Window</property>
                <property name="accelerator">&lt;primary&gt;w</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Toggle Sidebar</property>
                <property name="accelerator">&lt;primary&gt;slash</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Keyboard Shortcuts</property>
                <property name="accelerator">&lt;primary&gt;question</property>
              </object>
            </child>
          </object>
        </child>
        <child>
          <object class="GtkShortcutsGroup">
            <property name="title">Go To</property>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Games</property>
                <property name="accelerator">&lt;primary&gt;1</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Dashboard</property>
                <property name="accelerator">&lt;primary&gt;2</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Launchers</property>
                <property name="accelerator">&lt;primary&gt;3</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Proton Tools</property>
                <property name="accelerator">&lt;primary&gt;4</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Compatibility</property>
                <property name="accelerator">&lt;primary&gt;5</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Steam Session</property>
                <property name="accelerator">&lt;primary&gt;6</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Tools</property>
                <property name="accelerator">&lt;primary&gt;7</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Settings</property>
                <property name="accelerator">&lt;primary&gt;8</property>
              </object>
            </child>
          </object>
        </child>
      </object>
    </child>
    <child>
      <object class="GtkShortcutsSection">
        <property name="title">Games</property>
        <child>
          <object class="GtkShortcutsGroup">
            <property name="title">Library</property>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Add Game</property>
                <property name="accelerator">&lt;primary&gt;n</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Search Games</property>
                <property name="accelerator">&lt;primary&gt;f</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Refresh Library</property>
                <property name="accelerator">F5</property>
              </object>
            </child>
            <child>
              <object class="GtkShortcutsShortcut">
                <property name="title">Close Search / Go Back</property>
                <property name="accelerator">Escape</property>
              </object>
            </child>
          </object>
        </child>
      </object>
    </child>
  </object>
</interface>"#,
    );
    let shortcuts: gtk::ShortcutsWindow = builder
        .object("shortcuts")
        .expect("Missing shortcuts window");
    shortcuts.set_transient_for(app.active_window().as_ref());
    shortcuts.present();
}
