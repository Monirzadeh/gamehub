use std::collections::HashMap;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

use crate::modules::game_library::GameEntry;
use crate::modules::launcher_manager::apply_locale;
use crate::modules::proton_manager::{CompatTool, InstalledVersion, ProtonManager};
use crate::modules::protondb::{CheckRow, CheckVerdict, ProtonDbClient};

/// Pids of running games keyed by game id (the child is its own process group
/// leader, so a negative pid targets the whole game tree).
fn running_registry() -> &'static Mutex<HashMap<String, i32>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, i32>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub struct GameRunner;

impl GameRunner {
    pub fn is_running(id: &str) -> bool {
        running_registry().lock().unwrap().contains_key(id)
    }

    pub fn which(binary: &str) -> bool {
        Command::new("which")
            .arg(binary)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub fn umu_available() -> bool {
        Self::which("umu-run")
    }

    pub fn gamemode_available() -> bool {
        Self::which("gamemoderun")
    }

    pub fn winetricks_available() -> bool {
        Self::which("winetricks")
    }

    /// Only actual Proton builds (GE-Proton, Proton-CachyOS, GE-Proton-RTSP),
    /// not DXVK/VKD3D which share the tool catalog but are not runnable targets.
    pub fn proton_tools() -> Vec<CompatTool> {
        ProtonManager::available_tools()
            .into_iter()
            .filter(|t| t.install_subdir == "compatibilitytools.d")
            .collect()
    }

    pub fn list_installed_protons() -> Vec<InstalledVersion> {
        let mut seen = Vec::new();
        let mut result = Vec::new();
        for tool in Self::proton_tools() {
            for inst in ProtonManager::list_installed(&tool) {
                if seen.contains(&inst.version) {
                    continue;
                }
                seen.push(inst.version.clone());
                result.push(inst);
            }
        }
        // umu's own managed builds (downloaded on first automatic launch).
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        if let Ok(entries) = std::fs::read_dir(
            PathBuf::from(&home).join(".local/share/umu"),
        ) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                if path.is_dir() && name.starts_with("UMU-Proton") && !seen.contains(&name) {
                    seen.push(name.clone());
                    result.push(InstalledVersion { version: name, path });
                }
            }
        }
        result
    }

    /// Resolve the Proton build to use: the named version if present,
    /// otherwise the first installed build (most recently scanned).
    pub fn resolve_proton(version: Option<&str>) -> Option<InstalledVersion> {
        let mut installed = Self::list_installed_protons();
        match version {
            Some(v) => match installed.iter().position(|p| p.version == v) {
                Some(i) => Some(installed.swap_remove(i)),
                None => installed.into_iter().next(),
            },
            None => installed.into_iter().next(),
        }
    }

    pub fn log_path(id: &str) -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home)
            .join(".cache/parch-gamehub/game-logs")
            .join(format!("{}.log", id))
    }

    pub fn launch(
        game: &GameEntry,
        on_exit: Box<dyn FnOnce(u64, bool) + 'static>,
    ) -> Result<(), String> {
        if Self::is_running(&game.id) {
            return Err("Game is already running".to_string());
        }
        if !Self::umu_available() {
            return Err(
                "umu-run was not found. Install umu-launcher (pacman -S umu-launcher) and try again."
                    .to_string(),
            );
        }
        // None means "Automatic": leave PROTONPATH unset so umu picks (and
        // downloads, if needed) its own UMU-Proton.
        let proton = Self::resolve_proton(game.proton_version.as_deref());

        let exe_path = PathBuf::from(&game.exe_path);
        if !exe_path.is_file() {
            return Err(format!("Executable not found: {}", game.exe_path));
        }

        let prefix_dir = game.prefix_dir();
        std::fs::create_dir_all(&prefix_dir)
            .map_err(|e| format!("Could not create prefix directory: {}", e))?;

        let log_path = Self::log_path(&game.id);
        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let log_file = std::fs::File::create(&log_path)
            .map_err(|e| format!("Could not open log file: {}", e))?;

        let mut cmd = if game.use_gamemode && Self::gamemode_available() {
            let mut c = Command::new("gamemoderun");
            c.arg("umu-run");
            c
        } else {
            Command::new("umu-run")
        };
        cmd.arg(&exe_path)
            .args(&game.args)
            .stdout(Stdio::from(log_file.try_clone().map_err(|e| e.to_string())?))
            .stderr(Stdio::from(log_file))
            .env("WINEPREFIX", &prefix_dir);
        if let Some(ref proton) = proton {
            cmd.env("PROTONPATH", &proton.path);
        }
        cmd.env(
            "GAMEID",
            game.steam_appid
                .map(|id| format!("umu-{}", id))
                .unwrap_or_else(|| "umu-0".to_string()),
        )
        .env("PROTON_VERB", "waitforexitandrun")
        .process_group(0);

        match game.workdir.as_deref().map(PathBuf::from) {
            Some(dir) if dir.is_dir() => {
                cmd.current_dir(dir);
            }
            _ => {
                if let Some(parent) = exe_path.parent() {
                    cmd.current_dir(parent);
                }
            }
        }
        apply_locale(&mut cmd);

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to start umu-run: {}", e))?;
        let pid = child.id() as i32;
        running_registry()
            .lock()
            .unwrap()
            .insert(game.id.clone(), pid);

        let (tx, rx) = std::sync::mpsc::channel::<(u64, bool)>();
        let game_id = game.id.clone();
        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            let ok = child.wait().map(|s| s.success()).unwrap_or(false);
            let secs = started.elapsed().as_secs();
            running_registry().lock().unwrap().remove(&game_id);
            let _ = tx.send((secs, ok));
        });

        // Deliver the exit on the main loop so the callback can touch widgets.
        let mut on_exit = Some(on_exit);
        glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
            match rx.try_recv() {
                Ok((secs, ok)) => {
                    if let Some(cb) = on_exit.take() {
                        cb(secs, ok);
                    }
                    glib::ControlFlow::Break
                }
                Err(_) => glib::ControlFlow::Continue,
            }
        });

        Ok(())
    }

    /// SIGTERM the game's whole process group (umu-run + game + wine).
    pub fn force_stop(id: &str) -> bool {
        let pid = running_registry().lock().unwrap().get(id).copied();
        match pid {
            Some(pid) => Command::new("kill")
                .args(["-TERM", "--", &format!("-{}", pid)])
                .status()
                .map(|s| s.success())
                .unwrap_or(false),
            None => false,
        }
    }

    fn which_detail(binary: &str) -> Option<String> {
        let output = Command::new("which").arg(binary).output().ok()?;
        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            None
        }
    }

    /// Pre-launch verification of everything a game needs to run.
    pub fn run_preflight(game: &GameEntry) -> Vec<CheckRow> {
        let mut rows = Vec::new();

        match Self::which_detail("umu-run") {
            Some(path) => rows.push(CheckRow {
                item: "umu-launcher",
                verdict: CheckVerdict::Pass,
                detail: format!("umu-run found at {}", path),
            }),
            None => rows.push(CheckRow {
                item: "umu-launcher",
                verdict: CheckVerdict::Fail,
                detail: "umu-run not found — install it with: pkexec pacman -S umu-launcher"
                    .to_string(),
            }),
        }

        match Self::resolve_proton(game.proton_version.as_deref()) {
            Some(proton) => rows.push(CheckRow {
                item: "Proton build",
                verdict: CheckVerdict::Pass,
                detail: format!("{} ({})", proton.version, proton.path.display()),
            }),
            None => rows.push(CheckRow {
                item: "Proton build",
                verdict: CheckVerdict::Pass,
                detail: "Automatic — umu-launcher will use its own UMU-Proton (downloaded on first launch)"
                    .to_string(),
            }),
        }

        if PathBuf::from(&game.exe_path).is_file() {
            rows.push(CheckRow {
                item: "Game executable",
                verdict: CheckVerdict::Pass,
                detail: game.exe_path.clone(),
            });
        } else {
            rows.push(CheckRow {
                item: "Game executable",
                verdict: CheckVerdict::Fail,
                detail: format!("File not found: {}", game.exe_path),
            });
        }

        match &game.workdir {
            Some(dir) => {
                let verdict = if PathBuf::from(dir).is_dir() {
                    CheckVerdict::Pass
                } else {
                    CheckVerdict::Fail
                };
                rows.push(CheckRow {
                    item: "Working directory",
                    verdict,
                    detail: dir.clone(),
                });
            }
            None => rows.push(CheckRow {
                item: "Working directory",
                verdict: CheckVerdict::Pass,
                detail: "Not set — the executable's folder will be used".to_string(),
            }),
        }

        let free_disk = ProtonDbClient::get_free_disk_gb();
        rows.push(CheckRow {
            item: "Storage",
            verdict: if free_disk >= 10.0 {
                CheckVerdict::Pass
            } else {
                CheckVerdict::Fail
            },
            detail: format!(
                "{:.1} GB free on home filesystem (prefix and shader cache need a few GB)",
                free_disk
            ),
        });

        let prefix = game.prefix_dir();
        rows.push(CheckRow {
            item: "Wine prefix",
            verdict: CheckVerdict::Pass,
            detail: if prefix.exists() {
                format!("Ready at {}", prefix.display())
            } else {
                format!("Will be created at {}", prefix.display())
            },
        });

        if game.use_gamemode {
            rows.push(CheckRow {
                item: "GameMode",
                verdict: if Self::gamemode_available() {
                    CheckVerdict::Pass
                } else {
                    CheckVerdict::Fail
                },
                detail: if Self::gamemode_available() {
                    "gamemoderun found — performance optimizations active on launch".to_string()
                } else {
                    "gamemoderun not found — install the gamemode package or disable the toggle"
                        .to_string()
                },
            });
        }

        rows
    }

    pub fn preflight_passed(rows: &[CheckRow]) -> bool {
        !rows
            .iter()
            .any(|r| matches!(r.verdict, CheckVerdict::Fail))
    }

    /// Shell command that opens the winetricks GUI inside the game's Proton
    /// environment, for use with the terminal dialog.
    pub fn winetricks_command(game: &GameEntry) -> Result<String, String> {
        let prefix = game.prefix_dir();
        let proton_part = match Self::resolve_proton(game.proton_version.as_deref()) {
            Some(proton) => format!("PROTONPATH='{}' ", shell_escape(&proton.path.to_string_lossy())),
            None => String::new(),
        };
        Ok(format!(
            "WINEPREFIX='{prefix}' {proton_part}GAMEID=umu-0 umu-run winetricks",
            prefix = shell_escape(&prefix.to_string_lossy()),
        ))
    }
}

/// Escape a value for interpolation into a single-quoted shell string.
pub fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_escape() {
        assert_eq!(shell_escape("/home/user/game dir"), "'/home/user/game dir'");
        assert_eq!(shell_escape("it's"), r"'it'\''s'");
    }

    #[test]
    fn test_preflight_missing_exe() {
        let game = GameEntry {
            id: "test-game".into(),
            name: "Test Game".into(),
            exe_path: "/nonexistent/path/game.exe".into(),
            workdir: None,
            args: vec![],
            proton_version: None,
            steam_appid: None,
            use_gamemode: false,
            last_played: None,
            playtime_secs: 0,
        };
        let rows = GameRunner::run_preflight(&game);
        let exe_row = rows.iter().find(|r| r.item == "Game executable").unwrap();
        assert!(matches!(exe_row.verdict, CheckVerdict::Fail));
        assert!(!GameRunner::preflight_passed(&rows));
    }
}
