mod folder_scan;
mod steam_match;

use std::cell::RefCell;
use std::path::Path;
use std::path::PathBuf; 
use std::rc::Rc;

use adw::prelude::*;

use crate::modules::game_library::{GameEntry, GameLibrary};
use crate::modules::game_runner::GameRunner;

/// Widgets and state shared between the form sections. Fields are updated by
/// the executable/working-directory pickers, the folder scanner and the Steam
/// matcher, then read once on Save.
pub(super) struct FormState {
    pub(super) exe_path: Rc<RefCell<String>>,
    pub(super) workdir: Rc<RefCell<Option<String>>>,
    pub(super) matched_appid: Rc<RefCell<Option<u64>>>,
    pub(super) name_row: adw::EntryRow,
    pub(super) exe_label: gtk::Label,
    pub(super) workdir_label: gtk::Label,
    pub(super) covers_dir: PathBuf, 
}

/// Add/edit dialog for a game entry: executable picker, folder scan for exe
/// candidates, Proton version, GameMode toggle, and optional Steam match
/// (drives cover art and ProtonDB rating).
pub fn show(
    parent: &gtk::Window,
    library: Rc<RefCell<GameLibrary>>,
    existing: Option<GameEntry>,
    on_saved: Box<dyn FnOnce(GameEntry) + 'static>,
) {
    let is_edit = existing.is_some();
    let dialog = adw::Dialog::builder()
        .title(if is_edit { "Edit Game" } else { "Add Game" })
        .content_width(600)
        .content_height(740)
        .build();

    let toolbar = adw::ToolbarView::new();

    let header = adw::HeaderBar::builder()
        .title_widget(&adw::WindowTitle::new(
            if is_edit { "Edit Game" } else { "Add Game" },
            "",
        ))
        .build();

    let cancel_btn = gtk::Button::with_label("Cancel");
    let dlg_cancel = dialog.clone();
    cancel_btn.connect_clicked(move |_| dlg_cancel.force_close());

    let save_btn = gtk::Button::builder().label("Save").build();
    save_btn.add_css_class("suggested-action");

    header.pack_start(&cancel_btn);
    header.pack_end(&save_btn);
    toolbar.add_top_bar(&header);

    let scrolled = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build();

    let page = adw::PreferencesPage::new();

    // --- Shared form state ----------------------------------------------------
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let covers_dir = PathBuf::from(home).join(".local/share/gamehub/covers");
    let _ = std::fs::create_dir_all(&covers_dir);
    let form = Rc::new(FormState {
        exe_path: Rc::new(RefCell::new(
            existing.as_ref().map(|g| g.exe_path.clone()).unwrap_or_default(),
        )),
        workdir: Rc::new(RefCell::new(existing.as_ref().and_then(|g| g.workdir.clone()))),
        matched_appid: Rc::new(RefCell::new(existing.as_ref().and_then(|g| g.steam_appid))),
        name_row: adw::EntryRow::builder()
            .title("Game Name")
            .text(existing.as_ref().map(|g| g.name.as_str()).unwrap_or(""))
            .build(),
        exe_label: gtk::Label::builder()
            .ellipsize(gtk::pango::EllipsizeMode::Start)
            .css_classes(["dim-label"])
            .build(),
        workdir_label: gtk::Label::builder()
            .ellipsize(gtk::pango::EllipsizeMode::Start)
            .css_classes(["dim-label"])
            .build(),
        covers_dir,
    });
    form.exe_label.set_label(&form.exe_path.borrow());
    form.workdir_label.set_label(
        &form
            .workdir
            .borrow()
            .clone()
            .unwrap_or_else(|| "Automatic".to_string()),
    );

    // --- Game details ---------------------------------------------------------
    let details = details_group(parent, &form, existing.as_ref());
    page.add(&details.group);

    // --- Folder import ----------------------------------------------------------
    page.add(&folder_scan::group(parent, &form));

    // --- Steam match ------------------------------------------------------------
    page.add(&steam_match::group(&form));

    scrolled.set_child(Some(&page));
    let toast_overlay = adw::ToastOverlay::new();
    toast_overlay.set_child(Some(&scrolled));
    toolbar.set_content(Some(&toast_overlay));
    dialog.set_child(Some(&toolbar));
    dialog.present(Some(parent));

    // --- Save ---------------------------------------------------------------------
    let on_saved_cell = RefCell::new(Some(on_saved));
    {
        let dialog = dialog.clone();
        let form = form.clone();
        let library = library.clone();
        let args_row = details.args_row;
        let proton_row = details.proton_row;
        let proton_names = details.proton_names;
        let gamemode_row = details.gamemode_row;
        save_btn.connect_clicked(move |_| {
            let name = form.name_row.text().trim().to_string();
            let exe = form.exe_path.borrow().clone();
            if name.is_empty() {
                form.name_row.add_css_class("error");
                form.name_row.set_title("Game Name (required)");
                return;
            }
            if exe.is_empty() || !Path::new(&exe).is_file() {
                toast_overlay.add_toast(adw::Toast::new(
                    "Pick a valid game executable first",
                ));
                return;
            }

            let proton_version = match proton_row.selected() {
                0 => None,
                idx => proton_names.get(idx as usize).cloned(),
            };

            let entry = GameEntry {
                id: match &existing {
                    Some(e) => e.id.clone(),
                    None => library.borrow().unique_id(&name),
                },
                name,
                exe_path: exe,
                workdir: form.workdir.borrow().clone(),
                args: args_row
                    .text()
                    .split_whitespace()
                    .map(String::from)
                    .collect(),
                proton_version,
                steam_appid: form.matched_appid.borrow().as_ref().copied(),
                use_gamemode: gamemode_row.property::<bool>("active"),
                last_played: existing.as_ref().and_then(|e| e.last_played),
                playtime_secs: existing.as_ref().map(|e| e.playtime_secs).unwrap_or(0),
            };

            let was_edit = library.borrow().get(&entry.id).is_some();
            if was_edit {
                library.borrow_mut().update(entry.clone());
            } else {
                library.borrow_mut().add(entry.clone());
            }
            dialog.force_close();
            if let Some(cb) = on_saved_cell.borrow_mut().take() {
                cb(entry);
            }
        });
    }
}

/// Widgets the Save handler reads back from the Game Details section.
struct DetailsWidgets {
    group: adw::PreferencesGroup,
    args_row: adw::EntryRow,
    proton_row: adw::ComboRow,
    proton_names: Vec<String>,
    gamemode_row: adw::SwitchRow,
}

/// Name, executable, working directory, arguments, Proton version and
/// GameMode, plus the executable/working-directory pickers.
fn details_group(
    parent: &gtk::Window,
    form: &Rc<FormState>,
    existing: Option<&GameEntry>,
) -> DetailsWidgets {
    let group = adw::PreferencesGroup::builder()
        .title("Game Details")
        .description("Point gamehub at the game's executable; it will run through Proton.")
        .build();

    group.add(&form.name_row);

    let exe_row = adw::ActionRow::builder().title("Executable").build();
    exe_row.add_suffix(&form.exe_label);
    let exe_browse = gtk::Button::builder()
        .label("Browse…")
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .build();
    exe_row.add_suffix(&exe_browse);
    group.add(&exe_row);

    let workdir_row = adw::ActionRow::builder()
        .title("Working Directory")
        .subtitle("Usually the executable's folder")
        .build();
    workdir_row.add_suffix(&form.workdir_label);
    let workdir_browse = gtk::Button::builder()
        .label("Browse…")
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .build();
    workdir_row.add_suffix(&workdir_browse);
    group.add(&workdir_row);

    let args_row = adw::EntryRow::builder()
        .title("Launch Arguments")
        .text(existing.map(|g| g.args.join(" ")).unwrap_or_default())
        .build();
    group.add(&args_row);

    let protons = GameRunner::list_installed_protons();
    let mut proton_names = vec!["Automatic (umu-managed if none installed)".to_string()];
    proton_names.extend(protons.iter().map(|p| p.version.clone()));
    let proton_model = gtk::StringList::new(
        &proton_names.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
    );
    let proton_row = adw::ComboRow::builder()
        .title("Proton Version")
        .model(&proton_model)
        .build();
    if let Some(version) = existing.and_then(|g| g.proton_version.as_deref()) {
        if let Some(idx) = proton_names.iter().position(|n| n == version) {
            proton_row.set_selected(idx as u32);
        }
    }
    if protons.is_empty() {
        proton_row.set_subtitle("No local builds — Automatic lets umu-launcher manage UMU-Proton");
    }
    group.add(&proton_row);

    let gamemode_row = adw::SwitchRow::builder()
        .title("GameMode")
        .subtitle("Request performance mode while the game runs (needs gamemoderun)")
        .active(existing.map(|g| g.use_gamemode).unwrap_or(false))
        .build();
    group.add(&gamemode_row);

    // --- Executable picker ------------------------------------------------------
    {
        let form = form.clone();
        let parent = parent.clone();
        exe_browse.connect_clicked(move |_| {
            let chooser = gtk::FileDialog::builder()
                .title("Select game executable")
                .build();
            let form = form.clone();
            chooser.open(Some(&parent), None::<&gio::Cancellable>, move |result| {
                if let Ok(file) = result {
                    if let Some(file) = file.path() {
                        let path_str = file.to_string_lossy().to_string();
                        if form.name_row.text().trim().is_empty() {
                            if let Some(stem) = file.file_stem().and_then(|s| s.to_str()) {
                                form.name_row.set_text(stem);
                            }
                        }
                        if form.workdir.borrow().is_none() {
                            if let Some(parent_dir) = file.parent() {
                                let dir_str = parent_dir.to_string_lossy().to_string();
                                form.workdir_label.set_label(&dir_str);
                                *form.workdir.borrow_mut() = Some(dir_str);
                            }
                        }
                        form.exe_label.set_label(&path_str);
                        *form.exe_path.borrow_mut() = path_str;
                    }
                }
            });
        });
    }

    // --- Working directory picker -------------------------------------------------
    {
        let form = form.clone();
        let parent = parent.clone();
        workdir_browse.connect_clicked(move |_| {
            let chooser = gtk::FileDialog::builder()
                .title("Select working directory")
                .build();
            let form = form.clone();
            chooser.select_folder(Some(&parent), None::<&gio::Cancellable>, move |result| {
                if let Ok(file) = result {
                    if let Some(dir) = file.path() {
                        let dir_str = dir.to_string_lossy().to_string();
                        form.workdir_label.set_label(&dir_str);
                        *form.workdir.borrow_mut() = Some(dir_str);
                    }
                }
            });
        });
    }

    DetailsWidgets {
        group,
        args_row,
        proton_row,
        proton_names,
        gamemode_row,
    }
}
