//! aw-watcher-steam-rs — an ActivityWatch watcher for Steam activity.
//!
//! Layer 1 (default, zero-config, offline): detect the game you're playing by
//! observing processes whose executable lives inside a Steam library folder.
//! Layer 2 (optional, needs a Steam Web API key): also report your own and your
//! friends' current games.

mod aw;
mod config;
mod process;
mod report;
mod steam;

#[cfg(feature = "steam-api")]
mod api;

use std::error::Error;
use std::thread;
use std::time::Duration;

use clap::{Parser, Subcommand};

use crate::aw::AwClient;

#[derive(Parser)]
#[command(name = "aw-watcher-steam-rs", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Manage the configuration file.
    Config {
        /// Write a commented starter config to the default path.
        #[arg(long)]
        generate: bool,
        /// Print the config file path and exit.
        #[arg(long)]
        path: bool,
        /// Overwrite an existing config when used with --generate.
        #[arg(long)]
        force: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Some(Command::Config {
            generate,
            path,
            force,
        }) => {
            init_logger(None);
            run_config(generate, path, force)
        }
        // run_watcher initializes logging itself once it has read the config, so
        // the file's `log_level` can take effect.
        None => run_watcher(),
    };

    if let Err(e) = result {
        // Use eprintln rather than log so a failure before/around logger setup
        // (e.g. an unparseable config) is still reported.
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

/// Initialize logging. Precedence: `RUST_LOG` env var > config `log_level` >
/// `"info"`. Safe to call once.
fn init_logger(config_level: Option<&str>) {
    let default = config_level.unwrap_or("info").to_string();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(default)).init();
}

/// Handle the `config` subcommand.
fn run_config(generate: bool, path: bool, force: bool) -> Result<(), Box<dyn Error>> {
    // With no flags, behave like --path (just show where config lives).
    if path || !generate {
        match config::config_path() {
            Some(p) => println!("{}", p.display()),
            None => return Err("could not determine a config directory".into()),
        }
    }
    if generate {
        let written = config::generate(force)?;
        log::info!("wrote starter config to {}", written.display());
        println!("Generated config at {}", written.display());
    }
    Ok(())
}

/// Run the watcher loop. Never returns under normal operation.
fn run_watcher() -> Result<(), Box<dyn Error>> {
    let cfg = config::load()?;
    init_logger(cfg.log_level.as_deref());

    let client = AwClient::new(&cfg.server.host, cfg.server.port, "aw-watcher-steam");
    let reporter = report::Reporter::new(&client)?;
    log::info!(
        "reporting to aw-server at {}:{} (host {})",
        cfg.server.host,
        cfg.server.port,
        client.hostname
    );

    let poll = cfg.poll_interval_seconds.max(1);
    let pulsetime = poll as f64 + 1.0;
    // Rebuild the game cache roughly every 5 minutes to pick up new installs.
    let cache_refresh_iters = (300 / poll).max(1);

    if cfg.disable_process_scan {
        log::info!("local process scanning disabled by config");
    }

    let mut api_layer = setup_api_layer(&cfg, &reporter)?;

    let mut scanner = process::ProcessScanner::new();
    let mut cache = steam::GameCache::build();
    if cache.is_empty() && !cfg.disable_process_scan {
        log::warn!(
            "no installed Steam games found — local detection will report nothing until a game is installed (is Steam installed and have you launched it?)"
        );
    }
    let mut current_game: Option<steam::GameInfo> = None;
    let mut iter: u64 = 0;

    loop {
        if iter != 0 && iter.is_multiple_of(cache_refresh_iters) {
            cache = steam::GameCache::build();
        }

        if !cfg.disable_process_scan {
            let game = scanner.current_game(&cache);
            log_game_change(&mut current_game, game.as_ref());
            if let Err(e) = reporter.heartbeat_game(game.as_ref(), "local", pulsetime) {
                log::warn!("failed to send game heartbeat: {e}");
            }
        }

        poll_api_layer(&mut api_layer, &cfg, &reporter);

        iter = iter.wrapping_add(1);
        thread::sleep(Duration::from_secs(poll));
    }
}

/// Log game start/stop/switch transitions at `info` level. The steady-state
/// per-tick heartbeat stays at `debug` (see [`report::Reporter::heartbeat_game`]),
/// so default-verbosity logs show one line per session change, not per poll.
fn log_game_change(current: &mut Option<steam::GameInfo>, detected: Option<&steam::GameInfo>) {
    let changed = current.as_ref().map(|c| c.app_id) != detected.map(|d| d.app_id);
    if !changed {
        return;
    }
    match (current.as_ref(), detected) {
        (None, Some(now)) => log::info!("now playing: {} (appid {})", now.name, now.app_id),
        (Some(prev), Some(now)) => log::info!(
            "now playing: {} (appid {}) [was: {}]",
            now.name,
            now.app_id,
            prev.name
        ),
        (Some(prev), None) => log::info!("stopped playing: {} (appid {})", prev.name, prev.app_id),
        (None, None) => {}
    }
    *current = detected.cloned();
}

// --- Layer 2: Steam Web API (self + friends), compiled in only with `steam-api` ---

#[cfg(feature = "steam-api")]
struct ApiLayer {
    client: api::SteamApiClient,
    poll_interval: Duration,
    pulsetime: f64,
    track_friends: bool,
    last_poll: std::time::Instant,
}

#[cfg(feature = "steam-api")]
fn setup_api_layer(
    cfg: &config::Config,
    reporter: &report::Reporter,
) -> Result<Option<ApiLayer>, Box<dyn Error>> {
    let Some(api_cfg) = cfg.steam_api.clone() else {
        if cfg.disable_process_scan {
            log::warn!(
                "process scanning is disabled and no [steam_api] is configured — nothing will be reported"
            );
        }
        return Ok(None);
    };
    if api_cfg.track_friends {
        reporter.ensure_friends_bucket()?;
    }
    let poll_interval = Duration::from_secs(api_cfg.poll_interval_seconds.max(1));
    let pulsetime = api_cfg.poll_interval_seconds as f64 + 5.0;
    let track_friends = api_cfg.track_friends;
    log::info!("Steam Web API layer enabled (friends: {track_friends})");
    Ok(Some(ApiLayer {
        client: api::SteamApiClient::new(api_cfg),
        poll_interval,
        pulsetime,
        track_friends,
        // Back-date so the first loop iteration polls immediately.
        last_poll: std::time::Instant::now() - poll_interval,
    }))
}

#[cfg(feature = "steam-api")]
fn poll_api_layer(layer: &mut Option<ApiLayer>, cfg: &config::Config, reporter: &report::Reporter) {
    let Some(layer) = layer.as_mut() else { return };
    if layer.last_poll.elapsed() < layer.poll_interval {
        return;
    }
    layer.last_poll = std::time::Instant::now();

    // Own game via API only when local scanning is off (otherwise Layer 1 owns
    // the game bucket and we'd flap between two sources).
    if cfg.disable_process_scan {
        match layer.client.self_current_game() {
            Ok(game) => {
                if let Err(e) = reporter.heartbeat_game(game.as_ref(), "api", layer.pulsetime) {
                    log::warn!("failed to send API game heartbeat: {e}");
                }
            }
            Err(e) => log::warn!("Steam API self-summary failed: {e}"),
        }
    }

    if layer.track_friends {
        match layer.client.friends_playing() {
            Ok(playing) => {
                if let Err(e) = reporter.heartbeat_friends(&playing, layer.pulsetime) {
                    log::warn!("failed to send friends heartbeat: {e}");
                }
            }
            Err(e) => log::warn!("Steam API friends query failed: {e}"),
        }
    }
}

#[cfg(not(feature = "steam-api"))]
type ApiLayer = ();

#[cfg(not(feature = "steam-api"))]
fn setup_api_layer(
    cfg: &config::Config,
    _reporter: &report::Reporter,
) -> Result<Option<ApiLayer>, Box<dyn Error>> {
    if cfg.steam_api.is_some() {
        log::warn!(
            "[steam_api] is configured but this binary was built without the `steam-api` feature; ignoring it"
        );
    }
    if cfg.disable_process_scan {
        log::warn!(
            "process scanning is disabled and the Steam API layer is unavailable — nothing will be reported"
        );
    }
    Ok(None)
}

#[cfg(not(feature = "steam-api"))]
fn poll_api_layer(
    _layer: &mut Option<ApiLayer>,
    _cfg: &config::Config,
    _reporter: &report::Reporter,
) {
}
