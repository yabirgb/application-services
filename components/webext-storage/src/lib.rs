/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#![allow(unknown_lints)]
#![warn(rust_2018_idioms)]

mod api;
pub mod db;
pub mod error;
pub mod repo;
mod schema;
mod sync;

use std::marker::PhantomData;

/// A minimal ServerTimestamp - ideally we'd use the one in sync15_traits, but
/// that's tricky. Not sure whether to close server_timestamp.rs or not?
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Default)]
struct ServerTimestamp(pub i64);

impl ServerTimestamp {
    pub fn from_float_seconds(ts: f64) -> Self {
        let rf = (ts * 1000.0).round();
        if !rf.is_finite() || rf < 0.0 || rf >= i64::max_value() as f64 {
            log::error!("Illegal timestamp: {}", ts);
            ServerTimestamp(0)
        } else {
            ServerTimestamp(rf as i64)
        }
    }

    /// Get the milliseconds for the timestamp.
    #[inline]
    pub fn as_millis(self) -> i64 {
        self.0
    }
}

impl serde::ser::Serialize for ServerTimestamp {
    fn serialize<S: serde::ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f64(self.0 as f64 / 1000.0)
    }
}

struct TimestampVisitor(PhantomData<ServerTimestamp>);

impl<'de> serde::de::Visitor<'de> for TimestampVisitor {
    type Value = ServerTimestamp;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a floating point number")
    }

    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
        Ok(ServerTimestamp::from_float_seconds(value))
    }
}

impl<'de> serde::de::Deserialize<'de> for ServerTimestamp {
    fn deserialize<D: serde::de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_f64(TimestampVisitor(PhantomData))
    }
}

// This is what we roughly expect the "bridge" used by desktop to do.
// It's primarily here to avoid dead-code warnings (but I don't want to disable
// those warning, as stuff that remains after this is suspect!)
pub fn delme_demo_usage() -> error::Result<()> {
    use serde_json::json;

    let repo = repo::Repo::new("webext-storage.db")?;
    repo.set("ext-id", json!({}))?;
    repo.get("ext-id", json!({}))?;
    repo.remove("ext-id", json!({}))?;
    repo.clear("ext-id")?;
    Ok(())
}
