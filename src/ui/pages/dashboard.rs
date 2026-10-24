use std::path::PathBuf;

use adw::prelude::*;
use gtk::glib;

use crate::config::AppConfig;
use crate::modules::launcher_manager::apply_locale;
use crate::modules::system_info::{InstalledLauncher, SystemInfo, SystemStatus};

fn cached_icon_path(id: &str) -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let path = PathBuf::from(home).join(format!(".local/share/gamehub/icons/{}.png", id));
    if path.exists() { Some(path) } else { None }
}

pub fn new(view_stack: Option<gtk::Stack>) -> gtk::Box {
    let system_info = SystemInfo::new();

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

    // Welcome banner
    let config = AppConfig::new();
    let banner_dismissed = config.welcome_banner_dismissed();

    let banner = adw::Banner::builder()
        .title("Welcome to Parch Linux Gamehub")
        .button_label("Get Started")
        .revealed(!banner_dismissed)
        .build();

    let banner_toast = toast_overlay.clone();
    banner.connect_button_clicked(move |b| {
        b.set_revealed(false);
        let cfg = AppConfig::new();
        cfg.set_welcome_banner_dismissed(true);
        let toast = adw::Toast::new("Let's optimize your gaming setup!");
        banner_toast.add_toast(toast);
    });

    let banner_weak = banner.downgrade();
    glib::timeout_add_seconds_local(5, move || {
        if let Some(banner) = banner_weak.upgrade() {
            if banner.is_revealed() {
                banner.set_revealed(false);
                let cfg = AppConfig::new();
                cfg.set_welcome_banner_dismissed(true);
            }
        }
        glib::ControlFlow::Break
    });

    content.append(&banner);

    // System Status Group
    let status_group = adw::PreferencesGroup::builder()
        .title("System Status")
        .description("Current gaming environment status")
        .build();

    let kernel_row = adw::ActionRow::builder()
        .title("Linux Kernel")
        .subtitle("Detecting...")
        .build();
    kernel_row.add_prefix(&gtk::Image::from_icon_name("computer"));
    let kernel_btn = gtk::Button::builder()
        .label("Kernel Manager")
        .css_classes(["flat", "compact"])
        .valign(gtk::Align::Center)
        .tooltip_text("Open Parch Kernel Manager")
        .build();
    kernel_btn.connect_clicked(|_| {
        let mut child = std::process::Command::new("parch-kernel-manager");
        child.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
        apply_locale(&mut child);
        if let Ok(mut c) = child.spawn() {
            std::thread::spawn(move || { let _ = c.wait(); });
        }
    });
    kernel_row.add_suffix(&kernel_btn);
    status_group.add(&kernel_row);

    let gpu_row = adw::ActionRow::builder()
        .title("Graphics Card")
        .subtitle("Detecting...")
        .build();
    gpu_row.add_prefix(&gtk::Image::from_icon_name("video-display"));
    status_group.add(&gpu_row);

    let driver_row = adw::ActionRow::builder()
        .title("Graphics Driver")
        .subtitle("Detecting...")
        .build();
    driver_row.add_prefix(&gtk::Image::from_icon_name("preferences-system"));
    let driver_btn = gtk::Button::builder()
        .label("Driver Manager")
        .css_classes(["flat", "compact"])
        .valign(gtk::Align::Center)
        .tooltip_text("Open Parch Driver Manager")
        .build();
    driver_btn.connect_clicked(|_| {
        let mut child = std::process::Command::new("parch-driver-manager");
        child.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
        apply_locale(&mut child);
        if let Ok(mut c) = child.spawn() {
            std::thread::spawn(move || { let _ = c.wait(); });
        }
    });
    driver_row.add_suffix(&driver_btn);
    status_group.add(&driver_row);

    let vulkan_row = adw::ActionRow::builder()
        .title("Vulkan Support")
        .subtitle("Checking...")
        .build();
    vulkan_row.add_prefix(&gtk::Image::from_icon_name("video-display"));
    status_group.add(&vulkan_row);

    let chaotic_row = adw::ActionRow::builder()
        .title("Chaotic AUR")
        .subtitle("Checking...")
        .build();
    chaotic_row.add_prefix(&gtk::Image::from_icon_name("package"));
    status_group.add(&chaotic_row);

    content.append(&status_group);

    // Quick Actions Group
    let quick_group = adw::PreferencesGroup::builder()
        .title("Quick Actions")
        .build();

    let games_row = adw::ActionRow::builder()
        .title("My Games")
        .subtitle("Add and launch your Windows games with Proton")
        .activatable(true)
        .build();
    games_row.add_prefix(&gtk::Image::from_icon_name("applications-games-symbolic"));
    games_row.add_suffix(&gtk::Image::from_icon_name("go-next"));
    if let Some(ref vs) = view_stack {
        let vs_games = vs.clone();
        games_row.connect_activated(move |_| {
            vs_games.set_visible_child_name("games");
        });
    }
    quick_group.add(&games_row);

    let install_row = adw::ActionRow::builder()
        .title("Install Game Launchers")
        .subtitle("Steam, Lutris, Heroic, RetroArch, Prism and more")
        .activatable(true)
        .build();
    install_row.add_prefix(&gtk::Image::from_icon_name("list-add"));
    install_row.add_suffix(&gtk::Image::from_icon_name("go-next"));
    if let Some(ref vs) = view_stack {
        let vs_install = vs.clone();
        install_row.connect_activated(move |_| {
            vs_install.set_visible_child_name("launchers");
        });
    }
    quick_group.add(&install_row);

    let protondb_row = adw::ActionRow::builder()
        .title("ProtonDB Compatibility Check")
        .subtitle("Check game compatibility and verify system requirements")
        .activatable(true)
        .build();
    protondb_row.add_prefix(&gtk::Image::from_icon_name("emblem-ok-symbolic"));
    protondb_row.add_suffix(&gtk::Image::from_icon_name("go-next"));
    if let Some(ref vs) = view_stack {
        let vs_proton = vs.clone();
        protondb_row.connect_activated(move |_| {
            vs_proton.set_visible_child_name("compatibility");
        });
    }
    quick_group.add(&protondb_row);

    let kernel_quick_row = adw::ActionRow::builder()
        .title("Parch Kernel Manager")
        .subtitle("Install and switch gaming kernels (Zen, CachyOS)")
        .activatable(true)
        .build();
    kernel_quick_row.add_prefix(&gtk::Image::from_icon_name("computer"));
    kernel_quick_row.add_suffix(&gtk::Image::from_icon_name("go-next"));
    kernel_quick_row.connect_activated(move |_| {
        let mut child = std::process::Command::new("parch-kernel-manager");
        child.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
        apply_locale(&mut child);
        if let Ok(mut c) = child.spawn() {
            std::thread::spawn(move || { let _ = c.wait(); });
        }
    });
    quick_group.add(&kernel_quick_row);

    let driver_quick_row = adw::ActionRow::builder()
        .title("Parch Driver Manager")
        .subtitle("Detect and configure GPU drivers (RADV, AMDVLK, NVIDIA)")
        .activatable(true)
        .build();
    driver_quick_row.add_prefix(&gtk::Image::from_icon_name("preferences-system"));
    driver_quick_row.add_suffix(&gtk::Image::from_icon_name("go-next"));
    driver_quick_row.connect_activated(move |_| {
        let mut child = std::process::Command::new("parch-driver-manager");
        child.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
        apply_locale(&mut child);
        if let Ok(mut c) = child.spawn() {
            std::thread::spawn(move || { let _ = c.wait(); });
        }
    });
    quick_group.add(&driver_quick_row);

    let optimize_row = adw::ActionRow::builder()
        .title("Gaming Optimizations and Tools")
        .subtitle("Configure GameMode, MangoHud, CoreCtrl, and tools")
        .activatable(true)
        .build();
    optimize_row.add_prefix(&gtk::Image::from_icon_name("emblem-system"));
    optimize_row.add_suffix(&gtk::Image::from_icon_name("go-next"));
    if let Some(ref vs) = view_stack {
        let vs_opt = vs.clone();
        optimize_row.connect_activated(move |_| {
            vs_opt.set_visible_child_name("tools");
        });
    }
    quick_group.add(&optimize_row);

    content.append(&quick_group);

    // Installed Launchers Group
    let launchers_group = adw::PreferencesGroup::builder()
        .title("Installed Launchers")
        .build();

    let launchers_status = adw::StatusPage::builder()
        .icon_name("applications-games")
        .title("Checking installed launchers...")
        .description("Please wait")
        .build();
    launchers_group.add(&launchers_status);

    let launchers_box = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .visible(false)
        .build();
    launchers_group.add(&launchers_box);

    content.append(&launchers_group);

    clamp.set_child(Some(&content));
    scroll.set_child(Some(&clamp));
    toast_overlay.set_child(Some(&scroll));

    // Load system info asynchronously using channel
    let toast_overlay_clone = toast_overlay.clone();
    let (tx, rx) = std::sync::mpsc::channel::<(SystemStatus, Vec<InstalledLauncher>)>();

    std::thread::spawn(move || {
        let status = system_info.gather_all();
        let launchers = system_info.get_installed_launchers();
        let _ = tx.send((status, launchers));
    });

    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        let Ok((status, launchers)) = rx.try_recv() else {
            return glib::ControlFlow::Continue;
        };
        kernel_row.set_subtitle(&status.kernel);
        gpu_row.set_subtitle(&status.gpu);
        driver_row.set_subtitle(&status.driver);

        if status.vulkan_supported {
            vulkan_row.set_subtitle("Available");
        } else {
            vulkan_row.set_subtitle("Not detected");
        }

        if status.chaotic_aur_installed {
            chaotic_row.set_subtitle("Installed");
        } else {
            chaotic_row.set_subtitle("Not installed");
        }

        launchers_status.set_visible(false);
        launchers_box.set_visible(true);

        if launchers.is_empty() {
            let empty_row = adw::ActionRow::builder()
                .title("No launchers installed")
                .subtitle("Install some from the Launchers page")
                .build();
            launchers_box.append(&empty_row);
        } else {
            for launcher in launchers {
                let name = launcher.name.unwrap_or_else(|| "Unknown".to_string());
                let cmd = launcher.command.clone();

                let prefix_icon: gtk::Widget = if let Some(path) = cached_icon_path(&launcher.id) {
                    gtk::Image::from_file(path).upcast()
                } else {
                    gtk::Image::from_icon_name("applications-games").upcast()
                };

                let row = adw::ActionRow::builder()
                    .title(&name)
                    .subtitle("Click to launch")
                    .activatable(true)
                    .build();
                row.add_prefix(&prefix_icon);

                let launch_btn = gtk::Button::builder()
                    .label("Launch")
                    .css_classes(["suggested-action", "compact"])
                    .valign(gtk::Align::Center)
                    .build();
                let cmd_btn = cmd.clone();
                let toast_click = toast_overlay_clone.clone();
                launch_btn.connect_clicked(move |_| {
                    let mut child = std::process::Command::new(&cmd_btn);
                    child.stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null());
                    apply_locale(&mut child);
                    if let Ok(mut c) = child.spawn() {
                        std::thread::spawn(move || { let _ = c.wait(); });
                    }
                    let t = adw::Toast::new(&format!("Launching {}...", cmd_btn));
                    toast_click.add_toast(t);
                });
                row.add_suffix(&launch_btn);

                let toast_act = toast_overlay_clone.clone();
                let cmd_row = cmd.clone();
                row.connect_activated(move |_| {
                    let mut child = std::process::Command::new(&cmd_row);
                    child.stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null());
                    apply_locale(&mut child);
                    if let Ok(mut c) = child.spawn() {
                        std::thread::spawn(move || { let _ = c.wait(); });
                    }
                    let t = adw::Toast::new(&format!("Launching {}...", cmd_row));
                    toast_act.add_toast(t);
                });
                launchers_box.append(&row);
            }
        }

        glib::ControlFlow::Break
    });

    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    container.append(&toast_overlay);
    container
}
