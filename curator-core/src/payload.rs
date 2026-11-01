use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// Trait implemented by typed game payloads to be sent over the dynamic curator envelope.
pub trait CuratorPayload: Serialize + DeserializeOwned + Send + Sync + 'static {
    /// Unique identifier for this payload type (e.g. "hunter:target_hit", "game:ready_check").
    fn event_tag() -> &'static str;

    fn to_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }

    fn from_value(value: &serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value.clone())
    }
}

/// Generic in-memory and over-the-wire dynamic event envelope.
#[derive(Message, Event, Debug, Clone, Serialize, Deserialize)]
pub struct CuratorEventEnvelope {
    pub tag: String,
    pub payload: serde_json::Value,
    pub session_id: Option<Uuid>,
    pub source_entity: Option<Entity>,
    pub timestamp_ms: u64,
}

impl CuratorEventEnvelope {
    pub fn new(tag: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            tag: tag.into(),
            payload,
            session_id: None,
            source_entity: None,
            timestamp_ms: 0,
        }
    }

    pub fn with_session(mut self, session_id: Uuid) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn from_payload<P: CuratorPayload>(payload: &P, session_id: Option<Uuid>) -> Self {
        Self {
            tag: P::event_tag().to_string(),
            payload: payload.to_value(),
            session_id,
            source_entity: None,
            timestamp_ms: 0,
        }
    }

    pub fn downcast<P: CuratorPayload>(&self) -> Result<P, serde_json::Error> {
        if self.tag != P::event_tag() {
            return Err(serde::de::Error::custom(format!(
                "Tag mismatch: expected '{}', got '{}'",
                P::event_tag(),
                self.tag
            )));
        }
        P::from_value(&self.payload)
    }

    pub fn is<P: CuratorPayload>(&self) -> bool {
        self.tag == P::event_tag()
    }
}

