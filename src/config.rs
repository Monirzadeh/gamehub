use std::cell::RefCell;

use gio::prelude::SettingsExt;
use gio::Settings;

const SETTINGS_SCHEMA: &str = "com.parchlinux.gamehub";

pub struct AppConfig {
    settings: RefCell<Settings>,
}

impl AppConfig {
    pub fn new() -> Self {
        let settings = Settings::new(SETTINGS_SCHEMA);
        Self {
            settings: RefCell::new(settings),
        }
    }

    pub fn auto_update(&self) -> bool {
        self.settings.borrow().boolean("auto-update")
    }

    pub fn set_auto_update(&self, value: bool) {
        self.settings.borrow().set_boolean("auto-update", value).ok();
    }

    pub fn show_notifications(&self) -> bool {
        self.settings.borrow().boolean("show-notifications")
    }

    pub fn set_show_notifications(&self, value: bool) {
        self.settings.borrow().set_boolean("show-notifications", value).ok();
    }

    pub fn performance_cpu_governor(&self) -> bool {
        self.settings.borrow().boolean("performance-cpu-governor")
    }

    pub fn set_performance_cpu_governor(&self, value: bool) {
        self.settings.borrow().set_boolean("performance-cpu-governor", value).ok();
    }

    pub fn gaming_kernel_parameters(&self) -> bool {
        self.settings.borrow().boolean("gaming-kernel-parameters")
    }

    pub fn set_gaming_kernel_parameters(&self, value: bool) {
        self.settings.borrow().set_boolean("gaming-kernel-parameters", value).ok();
    }

    pub fn esync_fsync(&self) -> bool {
        self.settings.borrow().boolean("esync-fsync")
    }

    pub fn set_esync_fsync(&self, value: bool) {
        self.settings.borrow().set_boolean("esync-fsync", value).ok();
    }

    pub fn welcome_banner_dismissed(&self) -> bool {
        self.settings.borrow().boolean("welcome-banner-dismissed")
    }

    pub fn set_welcome_banner_dismissed(&self, value: bool) {
        self.settings.borrow().set_boolean("welcome-banner-dismissed", value).ok();
    }

    pub fn show_sidebar(&self) -> bool {
        self.settings.borrow().boolean("show-sidebar")
    }

    pub fn set_show_sidebar(&self, value: bool) {
        self.settings.borrow().set_boolean("show-sidebar", value).ok();
    }
}
