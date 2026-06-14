//! Local process observation.
//!
//! Privacy note: we ask `sysinfo` to refresh **only** each process's executable
//! path (no args, no environment, no cwd, no window titles), and we use that
//! path solely to test whether it lives inside a known Steam library folder.
//! Non-matching processes are discarded immediately and never stored, logged,
//! or transmitted.

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use crate::steam::{GameCache, GameInfo};

/// Holds a reusable [`System`] so repeated scans don't reallocate. Refresh is
/// scoped to executable paths only.
pub struct ProcessScanner {
    system: System,
}

impl Default for ProcessScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessScanner {
    pub fn new() -> Self {
        ProcessScanner {
            system: System::new(),
        }
    }

    /// Refresh the process list (exe paths only) and return the first running
    /// game found in `cache`. Returns `None` when nothing from a Steam library
    /// is running.
    pub fn current_game(&mut self, cache: &GameCache) -> Option<GameInfo> {
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true, // drop dead processes from the persistent System
            ProcessRefreshKind::nothing().with_exe(UpdateKind::Always),
        );

        for process in self.system.processes().values() {
            // `exe()` can be `None` (permissions, kernel threads, exited
            // processes) — skip rather than fail.
            if let Some(exe) = process.exe()
                && let Some(info) = cache.match_exe(exe)
            {
                return Some(info.clone());
            }
        }
        None
    }
}
