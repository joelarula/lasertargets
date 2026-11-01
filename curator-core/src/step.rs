use crate::payload::CuratorPayload;
use crate::prompt::ActorPrompt;
use serde::{Deserialize, Serialize};

/// Lightweight, in-memory workflow AST step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CuratorStep {
    /// Wait for a duration (in seconds)
    Delay(f32),

    /// Broadcast a dynamic event envelope to the ECS (and connected clients)
    EmitPayload {
        tag: String,
        payload: serde_json::Value,
    },

    /// Request input from an actor/terminal and wait for response or timeout
    Prompt(ActorPrompt),

    /// Wait until a CuratorEventEnvelope with matching tag is received
    WaitForEvent {
        tag: String,
        timeout_secs: Option<f32>,
    },

    /// Execute steps sequentially in order
    Sequence(Vec<CuratorStep>),

    /// Execute steps concurrently; completes when all child steps complete
    Parallel(Vec<CuratorStep>),

    /// Repeat a child step N times (count = 0 means loop indefinitely until interrupted)
    Repeat {
        count: usize,
        step: Box<CuratorStep>,
    },

    /// Custom game action
    CustomAction {
        name: String,
        payload: serde_json::Value,
    },
}

impl CuratorStep {
    pub fn delay(secs: f32) -> Self {
        Self::Delay(secs)
    }

    pub fn emit<P: CuratorPayload>(payload: &P) -> Self {
        Self::EmitPayload {
            tag: P::event_tag().to_string(),
            payload: payload.to_value(),
        }
    }

    pub fn emit_raw(tag: impl Into<String>, payload: serde_json::Value) -> Self {
        Self::EmitPayload {
            tag: tag.into(),
            payload,
        }
    }

    pub fn prompt(prompt: ActorPrompt) -> Self {
        Self::Prompt(prompt)
    }

    pub fn wait_event(tag: impl Into<String>) -> Self {
        Self::WaitForEvent {
            tag: tag.into(),
            timeout_secs: None,
        }
    }

    pub fn wait_event_timeout(tag: impl Into<String>, timeout_secs: f32) -> Self {
        Self::WaitForEvent {
            tag: tag.into(),
            timeout_secs: Some(timeout_secs),
        }
    }

    pub fn sequence(steps: Vec<CuratorStep>) -> Self {
        Self::Sequence(steps)
    }

    pub fn parallel(steps: Vec<CuratorStep>) -> Self {
        Self::Parallel(steps)
    }

    pub fn repeat(count: usize, step: CuratorStep) -> Self {
        Self::Repeat {
            count,
            step: Box::new(step),
        }
    }
}

