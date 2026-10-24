use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A user-added Windows game that runs through Proton via umu-launcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEntry {
    pub id: String,
    pub name: String,
    pub exe_path: String,
    pub workdir: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub proton_version: Option<String>,
    #[serde(default)]
    pub steam_appid: Option<u64>,
    #[serde(default)]
    pub use_gamemode: bool,
    #[serde(default)]
    pub last_played: Option<u64>,
    #[serde(default)]
    pub playtime_secs: u64,
}

impl GameEntry {
    pub fn slugify(name: &str) -> String {
        let slug: String = name
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c
                } else if c.is_whitespace() || matches!(c, '-' | '_' | '.' | ':') {
                    '-'
                } else {
                    '\0'
                }
            })
            .filter(|c| *c != '\0')
            .collect();
        let collapsed: Vec<&str> = slug.split('-').filter(|s| !s.is_empty()).collect();
        let slug = collapsed.join("-");
        if slug.is_empty() {
            "game".to_string()
        } else {
            slug.chars().take(48).collect()
        }
    }

    pub fn prefix_dir(&self) -> PathBuf {
        data_dir().join("prefixes").join(&self.id)
    }

    pub fn exe_dir(&self) -> Option<PathBuf> {
        std::path::Path::new(&self.exe_path)
            .parent()
            .map(|p| p.to_path_buf())
    }
}

pub fn data_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".local/share/gamehub")
}

/// JSON-backed store of the user's game library at ~/.local/share/gamehub/games.json.
pub struct GameLibrary {
    games: Vec<GameEntry>,
    path: PathBuf,
}

impl GameLibrary {
    pub fn load() -> Self {
        let path = data_dir().join("games.json");
        let games = std::fs::read_to_string(&path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default();
        Self { games, path }
    }

    pub fn save(&self) {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let tmp = self.path.with_extension("json.tmp");
        if let Ok(content) = serde_json::to_string_pretty(&self.games) {
            if std::fs::write(&tmp, content).is_ok() {
                let _ = std::fs::rename(&tmp, &self.path);
            }
        }
    }

    pub fn games(&self) -> &[GameEntry] {
        &self.games
    }

    pub fn get(&self, id: &str) -> Option<&GameEntry> {
        self.games.iter().find(|g| g.id == id)
    }

    /// An id based on the slug that is not yet taken (foo, foo-2, foo-3...).
    pub fn unique_id(&self, base: &str) -> String {
        let base = GameEntry::slugify(base);
        let mut id = base.clone();
        let mut n = 2;
        while self.games.iter().any(|g| g.id == id) {
            id = format!("{}-{}", base, n);
            n += 1;
        }
        id
    }

    pub fn add(&mut self, entry: GameEntry) {
        self.games.push(entry);
        self.save();
    }

    pub fn update(&mut self, entry: GameEntry) {
        if let Some(slot) = self.games.iter_mut().find(|g| g.id == entry.id) {
            *slot = entry;
            self.save();
        }
    }

    pub fn remove(&mut self, id: &str) -> Option<GameEntry> {
        let idx = self.games.iter().position(|g| g.id == id)?;
        let entry = self.games.remove(idx);
        self.save();
        Some(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(GameEntry::slugify("Cyberpunk 2077"), "cyberpunk-2077");
        assert_eq!(GameEntry::slugify("Elden Ring: Nightreign"), "elden-ring-nightreign");
        assert_eq!(GameEntry::slugify("  "), "game");
        assert_eq!(GameEntry::slugify("لعبة"), "game");
    }

    #[test]
    fn test_unique_id() {
        let mut lib = GameLibrary {
            games: vec![],
            path: PathBuf::from("/tmp/never-used-games.json"),
        };
        assert_eq!(lib.unique_id("My Game"), "my-game");
        lib.add(GameEntry {
            id: "my-game".into(),
            name: "My Game".into(),
            exe_path: String::new(),
            workdir: None,
            args: vec![],
            proton_version: None,
            steam_appid: None,
            use_gamemode: false,
            last_played: None,
            playtime_secs: 0,
        });
        assert_eq!(lib.unique_id("My Game"), "my-game-2");
    }
}
