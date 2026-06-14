# aw-watcher-steam-rs

An [ActivityWatch](https://activitywatch.net/) [watcher](https://docs.activitywatch.net/en/latest/watchers.html) for Steam activity.
Inspired by <https://github.com/Edwardsoen/aw-watcher-steam>, and written in Rust, hence the suffix.

It tracks which Steam game you're currently playing and reports it to your local
ActivityWatch server. By default it needs **no Steam API key, no account, and no
network** — it figures out what you're playing entirely from local signals.

## Features

- [x] Zero-config, no-API-key mode: detect the running game by observing processes
      whose executable lives inside a Steam library folder.
- [x] Works fully offline (e.g. Steam Offline Mode).
- [x] Cross-platform: Windows, macOS, Linux (native, Flatpak, Snap, Steam Deck).
- [x] Optional Steam Web API layer: also report your own and your **friends'**
      currently-playing games.
- [x] Optional TOML config with a `config --generate` helper.

## How it works

ActivityWatch watchers push **events** into a **bucket** on the local server
(default `http://localhost:5600`) using **heartbeats** — consecutive identical
heartbeats within a `pulsetime` window are merged into one event, so a long play
session becomes a single event rather than thousands of tiny ones.

This watcher has two layers:

### Layer 1 — local detection (default, always on)

1. It locates your Steam installation and libraries and builds a local cache mapping
   each installed game's folder (`steamapps/common/<installdir>`) to its app-id and
   name (via [`steamlocate`](https://crates.io/crates/steamlocate)).
2. Every few seconds it lists running processes and checks whether any executable
   lives inside one of those game folders (via [`sysinfo`](https://crates.io/crates/sysinfo)).
3. If so, that game is reported as currently playing.

Steam's old local "currently running" signal (`RunningAppID`) is
[no longer reliable](https://github.com/ValveSoftware/steam-for-linux/issues/9672)
(it now always reads `0`), which is why detection is process-based.

Events land in the bucket `aw-watcher-steam_<hostname>` (type
`currently-playing-game`), with this data:

```json
{
  "currently-playing-game": "Portal",
  "game-id": "400",
  "appid": 400,
  "install-dir": "Portal",
  "source": "local"
}
```

(`currently-playing-game` and `game-id` match the original aw-watcher-steam so
existing dashboards keep working; the rest are extras.)

### Layer 2 — Steam Web API (optional)

If you add a `[steam_api]` section to the config with an API key and your SteamID, the
watcher additionally polls the [Steam Web API](https://partner.steamgames.com/doc/webapi)
to report what your **friends** are currently playing (something local detection
can't see, since they play on other machines). Friends' activity goes into the
bucket `aw-watcher-steam-friends_<hostname>` (type `steam-friends-activity`) as a
snapshot event with one `friend-<persona>-playing` key per online-and-in-game friend.

Get a key at <https://steamcommunity.com/dev/apikey>. Friends tracking requires
your (and friends') game details to be visible to your account.

## Privacy

Detecting your own game requires reading the system process list, which is
sensitive. This watcher is deliberately minimal about it:

- It refreshes and reads **only each process's executable path** — never command-line
  arguments, environment variables, working directories, or window titles.
- That path is used **only** to test whether it sits inside a known Steam library
  folder. Non-matching processes are discarded immediately and are **never stored,
  logged, or transmitted**.
- The only thing that leaves the machine is the matched game's name/app-id, sent to
  your local ActivityWatch server.
- Don't want process scanning at all? Set `disable_process_scan = true` and rely
  solely on the (network) Steam Web API layer.

## Compatibility

Run the watcher as a **host-native** process (not inside a sandbox) — a host-native
binary can see games launched by sandboxed (Flatpak/Snap) Steam, but a sandboxed
watcher cannot see host processes.

| Platform           | Steam discovery                                                  | Process detection            | Notes                                             |
| ------------------ | --------------------------------------------------------------- | ---------------------------- | ------------------------------------------------- |
| Linux (native)     | `~/.steam/steam`, `~/.local/share/Steam`                        | `/proc`                      | Primary tested target.                            |
| Linux (Flatpak)    | `~/.var/app/com.valvesoftware.Steam/.../steamapps`              | host `/proc` sees game exes  | Run watcher host-native.                          |
| Linux (Snap)       | `~/snap/steam/common/.local/share/Steam`                        | host `/proc`                 | Run watcher host-native.                          |
| Steam Deck         | Flatpak-style paths                                             | `/proc`                      | Best-effort (treated as Linux + Flatpak).         |
| Windows            | registry `SteamPath` + `C:\Program Files (x86)\Steam`           | Win32                        | Reading some exe paths may need adequate rights.  |
| macOS              | `~/Library/Application Support/Steam`                           | sysinfo                      | App-bundle exes resolve under the library folder. |

If Steam can't be located on an unusual layout, please open an issue with your setup.

## Installation & usage

Requires a running ActivityWatch server. Then:

```sh
cargo build --release
./target/release/aw-watcher-steam-rs
```

The watcher runs with no configuration at all. Logging verbosity is controlled with
`RUST_LOG` (e.g. `RUST_LOG=debug`).

A minimal local-only binary (no Steam Web API / friends support, fewer dependencies)
can be built with:

```sh
cargo build --release --no-default-features
```

### Configuration (optional)

Everything has sensible defaults; a config file is only needed to change the poll
interval, point at a non-default server, or enable the Steam Web API layer.

```sh
# Print where the config file lives (or would live):
aw-watcher-steam-rs config --path

# Write a fully-commented starter config there (won't overwrite without --force):
aw-watcher-steam-rs config --generate
```

The file lives at `<config-dir>/activitywatch/aw-watcher-steam/config.toml`
(e.g. `~/.config/...` on Linux). Example:

```toml
poll_interval_seconds = 5

[server]
host = "localhost"
port = 5600

# Optional — enables friends tracking (and own-game reporting when process
# scanning is disabled). Requires network access.
[steam_api]
api_key = "YOUR_STEAM_WEB_API_KEY"
steam_id = "76561197960287930"
poll_interval_seconds = 60
track_friends = true
```

## Development

This repo uses a Nix dev shell (`shell.nix`) providing the Rust toolchain plus
`openssl`/`pkg-config` (needed by reqwest's TLS for the Steam Web API layer):

```sh
nix-shell --run 'cargo test'
nix-shell --run 'cargo clippy --all-targets'
```

## License

Licensed under the [GNU Affero General Public License v3.0 or later](LICENSE)
(AGPL-3.0-or-later).
