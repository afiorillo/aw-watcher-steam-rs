//! Minimal ActivityWatch REST client.
//!
//! This replaces the `aw-client-rust` crate, whose only published release
//! (0.1.0) pulls an ancient `reqwest 0.10` stack (hyper 0.13, h2 0.2, tokio 0.2,
//! net2, tokio-tls) carrying numerous RustSec advisories. We only need two
//! endpoints — create-bucket and heartbeat — so we implement them directly on
//! the `reqwest 0.13` blocking client the watcher already uses.
//!
//! API reference: <https://docs.activitywatch.net/en/latest/api/rest.html>

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde::{Serialize, Serializer};
use serde_json::{Map, Value, json};

/// An ActivityWatch event: a timestamped, durationed blob of JSON data.
/// Matches the server's wire format (RFC3339 timestamp, duration as float
/// seconds).
#[derive(Debug, Clone, Serialize)]
pub struct Event {
    #[serde(serialize_with = "serialize_timestamp")]
    pub timestamp: DateTime<Utc>,
    #[serde(serialize_with = "serialize_duration_secs")]
    pub duration: Duration,
    pub data: Map<String, Value>,
}

fn serialize_timestamp<S: Serializer>(t: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&t.to_rfc3339_opts(SecondsFormat::Millis, true))
}

fn serialize_duration_secs<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_f64(d.num_milliseconds() as f64 / 1000.0)
}

/// A blocking client for a local aw-server.
#[derive(Debug)]
pub struct AwClient {
    base_url: String,
    http: reqwest::blocking::Client,
    /// The watcher's client name (used as the bucket's `client` field).
    pub name: String,
    /// This machine's hostname (used in bucket names and the `hostname` field).
    pub hostname: String,
}

impl AwClient {
    /// Create a client pointing at `http://{host}:{port}`. The hostname is
    /// resolved via `sysinfo` (already a dependency), falling back to
    /// `"unknown"`.
    pub fn new(host: &str, port: u16, name: &str) -> Self {
        let hostname = sysinfo::System::host_name().unwrap_or_else(|| "unknown".to_string());
        AwClient {
            base_url: format!("http://{host}:{port}"),
            http: reqwest::blocking::Client::new(),
            name: name.to_string(),
            hostname,
        }
    }

    /// Create a bucket (idempotent). The server returns `304 Not Modified` if it
    /// already exists; we only surface transport errors, not HTTP status, so
    /// re-creating an existing bucket is a no-op.
    pub fn create_bucket(&self, bucket_id: &str, bucket_type: &str) -> reqwest::Result<()> {
        let url = format!("{}/api/0/buckets/{}", self.base_url, bucket_id);
        let body = json!({
            "id": bucket_id,
            "client": self.name,
            "type": bucket_type,
            "hostname": self.hostname,
            "data": {},
            "metadata": {},
        });
        self.http.post(&url).json(&body).send()?;
        Ok(())
    }

    /// Send a heartbeat. The server merges consecutive heartbeats with identical
    /// `data` that fall within `pulsetime` seconds of each other into a single
    /// event.
    pub fn heartbeat(&self, bucket_id: &str, event: &Event, pulsetime: f64) -> reqwest::Result<()> {
        let url = format!(
            "{}/api/0/buckets/{}/heartbeat?pulsetime={}",
            self.base_url, bucket_id, pulsetime
        );
        self.http.post(&url).json(event).send()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_serializes_to_aw_wire_format() {
        let mut data = Map::new();
        data.insert("currently-playing-game".into(), json!("Portal"));
        let event = Event {
            timestamp: DateTime::parse_from_rfc3339("2026-06-14T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            duration: Duration::milliseconds(1500),
            data,
        };
        let v: Value = serde_json::to_value(&event).unwrap();
        assert_eq!(v["timestamp"], json!("2026-06-14T12:00:00.000Z"));
        assert_eq!(v["duration"], json!(1.5)); // float seconds
        assert_eq!(v["data"]["currently-playing-game"], json!("Portal"));
    }
}
