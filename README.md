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

> **Known limitation — Proton / Windows games on Linux.** Native games (Linux, and
> all games on Windows/macOS) are detected reliably. Windows games run through
> **Proton** on Linux/Steam Deck are not: the game's `.exe` runs under the Proton/
> wine loader, so the OS reports the loader's path rather than a path inside the
> game's folder, and we intentionally don't inspect process command lines (see the
> privacy section). To track Proton titles, enable the optional **Steam Web API
> layer** below and set `disable_process_scan = true` — the API then reports your
> own current game regardless of how it's launched. (When process scanning is left
> on, the API layer only tracks friends, to avoid two sources fighting over the
> game bucket.)

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

Requires a running ActivityWatch server.

The watcher runs with no configuration at all. It logs one line when a game starts,
stops, or changes (at `info` level); per-tick heartbeats are logged at `debug`.
Verbosity is controlled by the `RUST_LOG` environment variable (e.g.
`RUST_LOG=debug`), or the `log_level` config key (see below).

### With Cargo

```sh
cargo install --path .          # or: cargo build --release
aw-watcher-steam-rs
```

A minimal local-only binary (no Steam Web API / friends support, fewer dependencies):

```sh
cargo build --release --no-default-features
```

### With Nix (flake)

This repo is a flake, so you can run or install it reproducibly without a Rust
toolchain on your host:

```sh
# Run it directly:
nix run github:clawde/aw-watcher-steam-rs

# Or install into your profile:
nix profile install github:clawde/aw-watcher-steam-rs
```

To run it as a managed service, use the provided modules (they define a systemd
**user** service — the watcher must run as your user to see your processes/Steam).

**home-manager** (recommended):

```nix
{
  inputs.aw-watcher-steam-rs.url = "github:clawde/aw-watcher-steam-rs";

  # in your home-manager configuration:
  imports = [ inputs.aw-watcher-steam-rs.homeManagerModules.default ];
  services.aw-watcher-steam-rs = {
    enable = true;
    # logLevel = "debug";   # optional, sets RUST_LOG
  };
}
```

**NixOS** (installs the package and a per-user service):

```nix
{
  imports = [ inputs.aw-watcher-steam-rs.nixosModules.default ];
  services.aw-watcher-steam-rs.enable = true;
}
```

The flake also exposes `packages.default`, an `overlays.default`, and a dev shell
(`nix develop`).

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
poll_interval_seconds = 15

# Log verbosity: "error", "warn", "info", "debug", "trace".
# RUST_LOG overrides this if set.
log_level = "info"

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

## Running as a service (systemd, non-Nix)

The watcher should run as a **systemd user service** so it runs in your session and
can see your processes and Steam. A ready-to-edit unit is in
[`packaging/systemd/aw-watcher-steam-rs.service`](packaging/systemd/aw-watcher-steam-rs.service):

```sh
mkdir -p ~/.config/systemd/user
cp packaging/systemd/aw-watcher-steam-rs.service ~/.config/systemd/user/
# Edit ExecStart in the copied file to point at your binary
# (e.g. ~/.cargo/bin/aw-watcher-steam-rs).

systemctl --user daemon-reload
systemctl --user enable --now aw-watcher-steam-rs.service

# Follow logs:
journalctl --user -u aw-watcher-steam-rs -f

# Optional: keep it running without an active login session:
loginctl enable-linger "$USER"
```

(Nix users should use the home-manager / NixOS modules above instead.)

## Development

The dev environment is defined by the flake (`flake.nix`) and provides the Rust
toolchain plus `openssl`/`pkg-config` (needed by reqwest's TLS for the Steam Web
API layer). Enter it with `nix develop` (or via direnv — the repo's `.envrc` uses
`use flake`):

```sh
nix develop -c cargo test
nix develop -c cargo test --no-default-features   # local-only build
nix develop -c cargo clippy --all-targets
nix develop -c cargo fmt --check
```

## Contributing

Contributions — including AI/LLM-assisted ones — are welcome. Note that **most of
this codebase was written by Claude** (Anthropic's AI coding agent) and that review
is best-effort. See [CONTRIBUTING.md](CONTRIBUTING.md) before opening a PR, and
[SECURITY.md](SECURITY.md) for the threat model and how to report vulnerabilities.

## License

Licensed under the [GNU Affero General Public License v3.0 or later](LICENSE)
(AGPL-3.0-or-later).
