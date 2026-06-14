//! Building ActivityWatch events and sending them to aw-server.
//!
//! Event schema (own-game bucket, type `currently-playing-game`, matching the
//! original aw-watcher-steam for dashboard compatibility, plus extras):
//! ```json
//! { "currently-playing-game": "<name>", "game-id": "<app_id>",
//!   "appid": <app_id>, "install-dir": "<installdir>", "source": "local|api" }
//! ```

use chrono::Utc;
use serde_json::{Map, Value, json};

use crate::aw::{AwClient, Event};

use crate::steam::GameInfo;

pub const GAME_BUCKET_TYPE: &str = "currently-playing-game";
#[cfg(feature = "steam-api")]
pub const FRIENDS_BUCKET_TYPE: &str = "steam-friends-activity";

/// Bucket name for the user's own currently-playing game.
pub fn game_bucket_name(hostname: &str) -> String {
    format!("aw-watcher-steam_{hostname}")
}

/// Bucket name for friends' activity (Steam Web API layer only).
#[cfg(feature = "steam-api")]
pub fn friends_bucket_name(hostname: &str) -> String {
    format!("aw-watcher-steam-friends_{hostname}")
}

/// Build the event payload for a currently-playing game. `source` is `"local"`
/// (process detection) or `"api"` (Steam Web API).
pub fn game_event(info: &GameInfo, source: &str) -> Event {
    let mut data = Map::new();
    data.insert(
        "currently-playing-game".to_string(),
        Value::String(info.name.clone()),
    );
    // Keep `game-id` a string to match the original watcher's schema; also
    // expose a numeric `appid` and the install dir for richer queries.
    data.insert(
        "game-id".to_string(),
        Value::String(info.app_id.to_string()),
    );
    data.insert("appid".to_string(), json!(info.app_id));
    data.insert(
        "install-dir".to_string(),
        Value::String(info.install_dir.clone()),
    );
    data.insert("source".to_string(), Value::String(source.to_string()));
    event_now(data)
}

/// Build a friends snapshot event: one `friend-<persona>-playing` key per
/// online friend currently in a game.
#[cfg(feature = "steam-api")]
pub fn friends_event(playing: &[(String, String)]) -> Event {
    let mut data = Map::new();
    for (persona, game) in playing {
        data.insert(
            format!("friend-{persona}-playing"),
            Value::String(game.clone()),
        );
    }
    event_now(data)
}

/// An event starting now with zero duration; the server extends it via
/// heartbeat merging.
fn event_now(data: Map<String, Value>) -> Event {
    Event {
        timestamp: Utc::now(),
        duration: chrono::Duration::zero(),
        data,
    }
}

/// Thin wrapper that owns the bucket names and forwards heartbeats to aw-server.
pub struct Reporter<'a> {
    client: &'a AwClient,
    game_bucket: String,
    #[cfg(feature = "steam-api")]
    friends_bucket: String,
}

impl<'a> Reporter<'a> {
    /// Create buckets (idempotent) and return a reporter. Bucket names embed the
    /// hostname the client resolved.
    pub fn new(client: &'a AwClient) -> Result<Self, Box<dyn std::error::Error>> {
        let game_bucket = game_bucket_name(&client.hostname);
        client.create_bucket(&game_bucket, GAME_BUCKET_TYPE)?;
        Ok(Reporter {
            client,
            game_bucket,
            #[cfg(feature = "steam-api")]
            friends_bucket: friends_bucket_name(&client.hostname),
        })
    }

    /// Ensure the friends bucket exists (only needed when the API layer is on).
    #[cfg(feature = "steam-api")]
    pub fn ensure_friends_bucket(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.client
            .create_bucket(&self.friends_bucket, FRIENDS_BUCKET_TYPE)?;
        Ok(())
    }

    /// Heartbeat the currently-playing game. Pass `None` to report nothing,
    /// letting the open event expire after `pulsetime`.
    pub fn heartbeat_game(
        &self,
        game: Option<&GameInfo>,
        source: &str,
        pulsetime: f64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(info) = game {
            let event = game_event(info, source);
            self.client
                .heartbeat(&self.game_bucket, &event, pulsetime)?;
            log::debug!("heartbeat: playing {} ({})", info.name, info.app_id);
        }
        Ok(())
    }

    /// Heartbeat a friends snapshot.
    #[cfg(feature = "steam-api")]
    pub fn heartbeat_friends(
        &self,
        playing: &[(String, String)],
        pulsetime: f64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let event = friends_event(playing);
        self.client
            .heartbeat(&self.friends_bucket, &event, pulsetime)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> GameInfo {
        GameInfo {
            app_id: 400,
            name: "Portal".to_string(),
            install_dir: "Portal".to_string(),
        }
    }

    #[test]
    fn bucket_names_follow_convention() {
        assert_eq!(game_bucket_name("HOST"), "aw-watcher-steam_HOST");
    }

    #[cfg(feature = "steam-api")]
    #[test]
    fn friends_bucket_name_follows_convention() {
        assert_eq!(friends_bucket_name("HOST"), "aw-watcher-steam-friends_HOST");
    }

    #[test]
    fn game_event_has_compatible_and_extra_fields() {
        let e = game_event(&sample(), "local");
        assert_eq!(e.data["currently-playing-game"], json!("Portal"));
        assert_eq!(e.data["game-id"], json!("400")); // string, original schema
        assert_eq!(e.data["appid"], json!(400)); // numeric extra
        assert_eq!(e.data["install-dir"], json!("Portal"));
        assert_eq!(e.data["source"], json!("local"));
    }

    #[cfg(feature = "steam-api")]
    #[test]
    fn friends_event_keys_per_persona() {
        let e = friends_event(&[
            ("Alice".to_string(), "Dota 2".to_string()),
            ("Bob".to_string(), "CS2".to_string()),
        ]);
        assert_eq!(e.data["friend-Alice-playing"], json!("Dota 2"));
        assert_eq!(e.data["friend-Bob-playing"], json!("CS2"));
    }
}
