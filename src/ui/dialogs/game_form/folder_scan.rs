use std::path::{Path, PathBuf};
use std::rc::Rc;

use adw::prelude::*;
use gtk::glib;

use super::FormState;

/// "Folder Import" section: pick a game folder and gamehub lists the
/// executables it finds, ready to be used as the game's executable.
pub(super) fn group(parent: &gtk::Window, form: &Rc<FormState>) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder()
        .title("Folder Import")
        .description("Pick a game folder and gamehub lists the executables it finds.")
        .build();

    let scan_row = adw::ActionRow::builder()
        .title("Scan a Folder")
        .subtitle("Searches two levels deep for .exe files")
        .build();
    let scan_btn = gtk::Button::builder()
        .label("Choose Folder…")
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .build();
    scan_row.add_suffix(&scan_btn);
    group.add(&scan_row);

    let scan_results = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .visible(false)
        .build();
    group.add(&scan_results);

    let form = form.clone();
    let parent = parent.clone();
    scan_btn.connect_clicked(move |_| {
        let chooser = gtk::FileDialog::builder()
            .title("Choose a game folder to scan")
            .build();
        let form = form.clone();
        let scan_results = scan_results.clone();
        let scan_row = scan_row.clone();
        chooser.select_folder(Some(&parent), None::<&gio::Cancellable>, move |result| {
            let Ok(file) = result else { return };
            let Some(dir) = file.path() else { return };
            scan_row.set_subtitle(&format!("Scanning {}…", dir.display()));
            let (tx, rx) = std::sync::mpsc::channel::<Vec<PathBuf>>();
            std::thread::spawn(move || {
                let _ = tx.send(scan_exe_candidates(&dir));
            });
            glib::timeout_add_local(std::time::Duration::from_millis(80), move || {
                let Ok(candidates) = rx.try_recv() else {
                    return glib::ControlFlow::Continue;
                };
                populate_scan_results(&scan_results, &scan_row, &form, &candidates);
                glib::ControlFlow::Break
            });
        });
    });

    group
}

fn populate_scan_results(
    scan_results: &gtk::ListBox,
    scan_row: &adw::ActionRow,
    form: &Rc<FormState>,
    candidates: &[PathBuf],
) {
    while let Some(child) = scan_results.first_child() {
        scan_results.remove(&child);
    }
    if candidates.is_empty() {
        scan_row.set_subtitle("No .exe files found in that folder");
        scan_results.set_visible(false);
        return;
    }
    scan_row.set_subtitle(&format!(
        "{} candidate(s) found — click to use",
        candidates.len()
    ));
    for path in candidates.iter().take(12) {
        let row = adw::ActionRow::builder()
            .title(path.file_name().and_then(|n| n.to_str()).unwrap_or("?"))
            .subtitle(path.to_string_lossy())
            .activatable(true)
            .build();
        let path_str = path.to_string_lossy().to_string();
        let form = form.clone();
        row.connect_activated(move |_| {
            form.exe_label.set_label(&path_str);
            *form.exe_path.borrow_mut() = path_str.clone();
            if let Some(parent_dir) = Path::new(&path_str).parent() {
                let dir_str = parent_dir.to_string_lossy().to_string();
                form.workdir_label.set_label(&dir_str);
                *form.workdir.borrow_mut() = Some(dir_str);
            }
            if form.name_row.text().trim().is_empty() {
                if let Some(parent_dir) = Path::new(&path_str).parent() {
                    if let Some(folder) = parent_dir.file_name().and_then(|n| n.to_str()) {
                        form.name_row.set_text(folder);
                    }
                }
            }
        });
        scan_results.append(&row);
    }
    scan_results.set_visible(true);
}

/// Collect .exe files up to two directory levels below `dir`, pushing likely
/// game launchers ahead of uninstallers, setup tools and crash handlers.
fn scan_exe_candidates(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<(u8, PathBuf)> = Vec::new();
    collect_exe_files(dir, 0, &mut found);

    let junk = [
        "uninstall", "unins", "setup", "install", "redist", "vcredist", "dxsetup",
        "crash", "unity", "dotnet", "easyanticheat", "update", "patcher", "launcher",
    ];
    found.sort_by_key(|(depth, p)| {
        let name = p
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        let junk_penalty = junk.iter().any(|j| name.contains(j)) as u8;
        // Prefer non-junk root-level executables, then alphabetical order.
        (junk_penalty, *depth, name)
    });
    found.into_iter().map(|(_, p)| p).collect()
}

fn collect_exe_files(dir: &Path, depth: u8, out: &mut Vec<(u8, PathBuf)>) {
    if depth > 2 || out.len() >= 40 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_exe_files(&path, depth + 1, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("exe"))
            .unwrap_or(false)
        {
            out.push((depth, path));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_exe_candidates_prefers_game_exe() {
        let dir = std::env::temp_dir().join("gamehub-scan-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("bin")).unwrap();
        std::fs::write(dir.join("MyGame.exe"), b"").unwrap();
        std::fs::write(dir.join("Uninstall.exe"), b"").unwrap();
        std::fs::write(dir.join("bin").join("helper.exe"), b"").unwrap();
        std::fs::write(dir.join("readme.txt"), b"").unwrap();

        let found = scan_exe_candidates(&dir);
        assert_eq!(found.len(), 3);
        assert!(found[0].ends_with("MyGame.exe"));
        assert!(found[1].ends_with("Uninstall.exe") || found[1].ends_with("helper.exe"));
        // Nested binaries rank after the root-level ones.
        assert!(found[2].ends_with("helper.exe") || found[2].ends_with("Uninstall.exe"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
