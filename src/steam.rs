//! Steam library discovery and the local game-id cache.
//!
//! Wraps [`steamlocate`] to enumerate installed apps and build a cache mapping
//! each app's absolute install directory to its `(app_id, name)`. The matching
//! logic ([`GameCache::match_exe`]) is kept pure so it can be unit-tested with
//! fixture paths, without a real Steam install.

use std::path::{Path, PathBuf};

/// Identifying info for an installed Steam app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameInfo {
    pub app_id: u32,
    pub name: String,
    /// The `installdir` folder name (e.g. `"GarrysMod"`).
    pub install_dir: String,
}

/// Maps absolute install directories to the games installed there. Entries are
/// kept sorted by descending path length so the first prefix match is the most
/// specific one (longest-prefix wins when directories nest).
#[derive(Debug, Default)]
pub struct GameCache {
    entries: Vec<(PathBuf, GameInfo)>,
}

impl GameCache {
    /// Build a cache from explicit entries. Used by [`build`](Self::build) and
    /// directly by tests.
    pub fn from_entries(mut entries: Vec<(PathBuf, GameInfo)>) -> Self {
        entries.sort_by_key(|(path, _)| std::cmp::Reverse(path.as_os_str().len()));
        GameCache { entries }
    }

    /// Discover Steam libraries and build the cache. Returns an empty cache
    /// (not an error) when Steam can't be located, so the watcher keeps running
    /// and can retry on the next refresh.
    pub fn build() -> Self {
        let mut entries = Vec::new();
        match steamlocate::SteamDir::locate() {
            Ok(steam_dir) => match steam_dir.libraries() {
                Ok(libraries) => {
                    for library in libraries.flatten() {
                        for app in library.apps().flatten() {
                            let dir = library.resolve_app_dir(&app);
                            let name = app.name.clone().unwrap_or_else(|| app.install_dir.clone());
                            entries.push((
                                dir,
                                GameInfo {
                                    app_id: app.app_id,
                                    name,
                                    install_dir: app.install_dir,
                                },
                            ));
                        }
                    }
                }
                Err(e) => log::warn!("could not read Steam libraries: {e}"),
            },
            Err(e) => log::warn!("could not locate Steam installation: {e}"),
        }
        log::info!("game cache built with {} installed app(s)", entries.len());
        Self::from_entries(entries)
    }

    /// Whether the cache is empty (e.g. Steam not found or no games installed).
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return the game whose install directory is a prefix of `exe`, if any.
    /// Longest-prefix wins. This is the only thing we do with a process path.
    pub fn match_exe(&self, exe: &Path) -> Option<&GameInfo> {
        self.entries
            .iter()
            .find(|(dir, _)| exe.starts_with(dir))
            .map(|(_, info)| info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(id: u32, name: &str, dir: &str) -> GameInfo {
        GameInfo {
            app_id: id,
            name: name.to_string(),
            install_dir: dir.to_string(),
        }
    }

    fn sample_cache() -> GameCache {
        GameCache::from_entries(vec![
            (
                PathBuf::from("/steam/steamapps/common/Half-Life"),
                game(70, "Half-Life", "Half-Life"),
            ),
            (
                PathBuf::from("/steam/steamapps/common/Portal"),
                game(400, "Portal", "Portal"),
            ),
        ])
    }

    #[test]
    fn matches_exe_inside_install_dir() {
        let cache = sample_cache();
        let hit = cache
            .match_exe(Path::new("/steam/steamapps/common/Portal/portal.exe"))
            .expect("should match Portal");
        assert_eq!(hit.app_id, 400);
        assert_eq!(hit.name, "Portal");
    }

    #[test]
    fn no_match_for_non_steam_process() {
        let cache = sample_cache();
        assert!(cache.match_exe(Path::new("/usr/bin/firefox")).is_none());
    }

    #[test]
    fn longest_prefix_wins_when_dirs_nest() {
        // A library nested inside another library's tree must resolve to the
        // more specific (longer) path.
        let cache = GameCache::from_entries(vec![
            (PathBuf::from("/games"), game(1, "Outer", "Outer")),
            (
                PathBuf::from("/games/steamapps/common/Inner"),
                game(2, "Inner", "Inner"),
            ),
        ]);
        let hit = cache
            .match_exe(Path::new("/games/steamapps/common/Inner/run.exe"))
            .unwrap();
        assert_eq!(hit.app_id, 2);
    }
}
