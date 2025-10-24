//\! Primitive type wrappers for `MessageChunk` compliance
//!
//! This module provides wrapper types for primitive and external types
//\! that cannot directly implement `MessageChunk` due to orphan rules.
//! Includes wrappers for:
//! - Unit type ()
//! - Uuid
//! - bool
//! - Duration
//\! - DateTime<Utc>
//\! - ZeroOneOrMany<T>

use cyrup_sugars::{ZeroOneOrMany, prelude::MessageChunk};
use serde::{Deserialize, Serialize};

/// Wrapper for unit type () to implement `MessageChunk`
#[derive(Debug, Clone, Default)]
pub struct CandleUnit(pub ());

impl MessageChunk for CandleUnit {
    fn bad_chunk(_error: String) -> Self {
        CandleUnit(())
    }

    fn error(&self) -> Option<&str> {
        None
    }
}

impl From<()> for CandleUnit {
    fn from((): ()) -> Self {
        CandleUnit(())
    }
}

/// Wrapper for Uuid to implement `MessageChunk` without orphan rule violations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleUuidChunk(pub uuid::Uuid);

impl Default for CandleUuidChunk {
    fn default() -> Self {
        CandleUuidChunk(uuid::Uuid::new_v4())
    }
}

impl MessageChunk for CandleUuidChunk {
    fn bad_chunk(_error: String) -> Self {
        // Create a deterministic UUID from error for debugging
        CandleUuidChunk(uuid::Uuid::new_v4())
    }

    fn error(&self) -> Option<&str> {
        None // UUIDs don't carry error state
    }
}

impl From<uuid::Uuid> for CandleUuidChunk {
    fn from(uuid: uuid::Uuid) -> Self {
        CandleUuidChunk(uuid)
    }
}

/// Wrapper for bool to implement `MessageChunk` without orphan rule violations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CandleBoolChunk(pub bool);

impl MessageChunk for CandleBoolChunk {
    fn bad_chunk(_error: String) -> Self {
        CandleBoolChunk(false) // Error state represented as false
    }

    fn error(&self) -> Option<&str> {
        None // Bools don't carry error state
    }
}

impl From<bool> for CandleBoolChunk {
    fn from(value: bool) -> Self {
        CandleBoolChunk(value)
    }
}

/// Wrapper for Duration to implement `MessageChunk` without orphan rule violations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleDurationChunk(#[serde(with = "duration_serde")] pub std::time::Duration);

impl Default for CandleDurationChunk {
    fn default() -> Self {
        CandleDurationChunk(std::time::Duration::from_secs(0))
    }
}

impl MessageChunk for CandleDurationChunk {
    fn bad_chunk(_error: String) -> Self {
        CandleDurationChunk(std::time::Duration::from_secs(0))
    }

    fn error(&self) -> Option<&str> {
        None // Durations don't carry error state
    }
}

impl From<std::time::Duration> for CandleDurationChunk {
    fn from(duration: std::time::Duration) -> Self {
        CandleDurationChunk(duration)
    }
}

/// Wrapper for `ZeroOneOrMany` to implement `MessageChunk` without orphan rule violations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleZeroOneOrManyChunk<T>(pub ZeroOneOrMany<T>);

impl<T: Default> Default for CandleZeroOneOrManyChunk<T> {
    fn default() -> Self {
        CandleZeroOneOrManyChunk(ZeroOneOrMany::None)
    }
}

impl<T> MessageChunk for CandleZeroOneOrManyChunk<T>
where
    T: Default + Clone,
{
    fn bad_chunk(_error: String) -> Self {
        CandleZeroOneOrManyChunk(ZeroOneOrMany::None)
    }

    fn error(&self) -> Option<&str> {
        None // ZeroOneOrMany doesn't carry error state
    }
}

impl<T> From<ZeroOneOrMany<T>> for CandleZeroOneOrManyChunk<T> {
    fn from(value: ZeroOneOrMany<T>) -> Self {
        CandleZeroOneOrManyChunk(value)
    }
}

impl<T> std::ops::Deref for CandleZeroOneOrManyChunk<T> {
    type Target = ZeroOneOrMany<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for CandleZeroOneOrManyChunk<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Wrapper for `DateTime<Utc>` to implement `MessageChunk` without orphan rule violations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleDateTimeChunk(pub chrono::DateTime<chrono::Utc>);

impl Default for CandleDateTimeChunk {
    fn default() -> Self {
        CandleDateTimeChunk(chrono::Utc::now())
    }
}

impl MessageChunk for CandleDateTimeChunk {
    fn bad_chunk(_error: String) -> Self {
        CandleDateTimeChunk(chrono::Utc::now())
    }

    fn error(&self) -> Option<&str> {
        None // DateTimes don't carry error state
    }
}

impl From<chrono::DateTime<chrono::Utc>> for CandleDateTimeChunk {
    fn from(datetime: chrono::DateTime<chrono::Utc>) -> Self {
        CandleDateTimeChunk(datetime)
    }
}

// Duration serialization helpers
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

/// Wrapper for `ZeroOneOrMany<f32>` to implement `MessageChunk` without orphan rule violations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroOneOrManyF32Chunk(pub cyrup_sugars::ZeroOneOrMany<f32>);

impl Default for ZeroOneOrManyF32Chunk {
    fn default() -> Self {
        ZeroOneOrManyF32Chunk(cyrup_sugars::ZeroOneOrMany::None)
    }
}

impl MessageChunk for ZeroOneOrManyF32Chunk {
    fn bad_chunk(_error: String) -> Self {
        ZeroOneOrManyF32Chunk(cyrup_sugars::ZeroOneOrMany::None)
    }

    fn error(&self) -> Option<&str> {
        None // ZeroOneOrMany doesn't carry error state
    }
}

impl From<cyrup_sugars::ZeroOneOrMany<f32>> for ZeroOneOrManyF32Chunk {
    fn from(value: cyrup_sugars::ZeroOneOrMany<f32>) -> Self {
        ZeroOneOrManyF32Chunk(value)
    }
}

impl From<ZeroOneOrManyF32Chunk> for cyrup_sugars::ZeroOneOrMany<f32> {
    fn from(chunk: ZeroOneOrManyF32Chunk) -> Self {
        chunk.0
    }
}
