//! Optional Steam Web API layer (compiled only with the `steam-api` feature).
//!
//! Enabled when a `[steam_api]` section is present in config. Polls the Steam
//! Web API for the user's own current game and, optionally, friends' current
//! games — the one thing local process detection cannot see, since friends play
//! on other machines.
//!
//! Official documentation:
//! - Steam Web API overview: <https://steamcommunity.com/dev>
//! - Steamworks Web API reference: <https://partner.steamgames.com/doc/webapi>
//! - Community wiki (unofficial but thorough): <https://developer.valvesoftware.com/wiki/Steam_Web_API>
//! - Get an API key: <https://steamcommunity.com/dev/apikey>

use std::error::Error;

use serde::Deserialize;

use crate::config::SteamApiConfig;
use crate::steam::GameInfo;

const BASE: &str = "https://api.steampowered.com";

/// A friend currently in a game: `(persona name, game name)`.
pub type FriendPlaying = (String, String);

/// Client for the [Steam Web API](https://partner.steamgames.com/doc/webapi).
///
/// Uses two `ISteamUser` endpoints:
/// - [`GetPlayerSummaries` (v2)](https://partner.steamgames.com/doc/webapi/ISteamUser#GetPlayerSummaries)
///   — current persona/game for one or more SteamIDs.
/// - [`GetFriendList` (v1)](https://partner.steamgames.com/doc/webapi/ISteamUser#GetFriendList)
///   — the configured user's friend SteamIDs.
pub struct SteamApiClient {
    cfg: SteamApiConfig,
    http: reqwest::blocking::Client,
}

#[derive(Deserialize)]
struct FriendListResponse {
    friendslist: Option<FriendList>,
}
#[derive(Deserialize)]
struct FriendList {
    friends: Vec<Friend>,
}
#[derive(Deserialize)]
struct Friend {
    steamid: String,
}

#[derive(Deserialize)]
struct SummariesResponse {
    response: Summaries,
}
#[derive(Deserialize)]
struct Summaries {
    players: Vec<PlayerSummary>,
}
#[derive(Deserialize)]
struct PlayerSummary {
    #[serde(default)]
    personaname: String,
    /// Present only while in a game; the numeric app id as a string.
    gameid: Option<String>,
    /// Human-readable game name while in a game.
    gameextrainfo: Option<String>,
}

impl SteamApiClient {
    pub fn new(cfg: SteamApiConfig) -> Self {
        SteamApiClient {
            cfg,
            http: reqwest::blocking::Client::new(),
        }
    }

    /// The user's own currently-playing game, if any.
    pub fn self_current_game(&self) -> Result<Option<GameInfo>, Box<dyn Error>> {
        let summaries = self.player_summaries(std::slice::from_ref(&self.cfg.steam_id))?;
        Ok(summaries.into_iter().find_map(|p| to_game_info(&p)))
    }

    /// Friends who are currently in a game, as `(persona, game)` pairs.
    pub fn friends_playing(&self) -> Result<Vec<FriendPlaying>, Box<dyn Error>> {
        let ids = self.friend_ids()?;
        let mut playing = Vec::new();
        // GetPlayerSummaries accepts up to 100 ids per call.
        for chunk in ids.chunks(100) {
            for p in self.player_summaries(chunk)? {
                if let Some(game) = p.gameextrainfo.clone() {
                    playing.push((p.personaname.clone(), game));
                }
            }
        }
        Ok(playing)
    }

    /// Fetch the configured user's friend SteamIDs via
    /// [`ISteamUser/GetFriendList` (v1)](https://partner.steamgames.com/doc/webapi/ISteamUser#GetFriendList).
    /// Returns an empty list if the profile's friends list is private.
    fn friend_ids(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let url = format!(
            "{BASE}/ISteamUser/GetFriendList/v1/?key={}&steamid={}&relationship=friend",
            self.cfg.api_key, self.cfg.steam_id
        );
        let resp: FriendListResponse = self.http.get(&url).send()?.error_for_status()?.json()?;
        Ok(resp
            .friendslist
            .map(|fl| fl.friends.into_iter().map(|f| f.steamid).collect())
            .unwrap_or_default())
    }

    /// Fetch player summaries via
    /// [`ISteamUser/GetPlayerSummaries` (v2)](https://partner.steamgames.com/doc/webapi/ISteamUser#GetPlayerSummaries).
    /// Accepts up to 100 SteamIDs per call. `gameextrainfo`/`gameid` are present
    /// only while a player is in a game and their profile exposes it.
    fn player_summaries(&self, ids: &[String]) -> Result<Vec<PlayerSummary>, Box<dyn Error>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let url = format!(
            "{BASE}/ISteamUser/GetPlayerSummaries/v2/?key={}&steamids={}",
            self.cfg.api_key,
            ids.join(",")
        );
        let resp: SummariesResponse = self.http.get(&url).send()?.error_for_status()?.json()?;
        Ok(resp.response.players)
    }
}

/// Convert a player summary into a [`GameInfo`] if they're in a game. The
/// install dir is unknown from the API, so it's left empty.
fn to_game_info(p: &PlayerSummary) -> Option<GameInfo> {
    let name = p.gameextrainfo.clone()?;
    let app_id = p.gameid.as_ref().and_then(|g| g.parse().ok()).unwrap_or(0);
    Some(GameInfo {
        app_id,
        name,
        install_dir: String::new(),
    })
}
