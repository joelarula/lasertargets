use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Target audience for an actor prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActorTarget {
    /// Broadcast to all connected terminals/actors
    All,
    /// Targeted at a specific actor UUID
    Actor(Uuid),
    /// Targeted at any actor with the given role (e.g. "player", "spectator", "controller")
    Role(String),
}

/// A choice option offered in an actor prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptChoice {
    pub id: String,
    pub label: String,
}

impl PromptChoice {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

/// Standardized interaction prompt requesting a decision or input from an actor/terminal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorPrompt {
    pub prompt_id: Uuid,
    pub session_id: Option<Uuid>,
    pub target: ActorTarget,
    pub title: String,
    pub message: String,
    pub choices: Vec<PromptChoice>,
    pub timeout_secs: Option<f32>,
    pub default_choice_id: Option<String>,
}

impl ActorPrompt {
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            prompt_id: Uuid::new_v4(),
            session_id: None,
            target: ActorTarget::All,
            title: title.into(),
            message: message.into(),
            choices: Vec::new(),
            timeout_secs: None,
            default_choice_id: None,
        }
    }

    pub fn with_choices(mut self, choices: Vec<PromptChoice>) -> Self {
        self.choices = choices;
        self
    }

    pub fn with_timeout(mut self, secs: f32, default_choice_id: impl Into<String>) -> Self {
        self.timeout_secs = Some(secs);
        self.default_choice_id = Some(default_choice_id.into());
        self
    }

    pub fn with_target(mut self, target: ActorTarget) -> Self {
        self.target = target;
        self
    }

    pub fn with_session(mut self, session_id: Uuid) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Helper for standard [Yes, No] confirmation prompt.
    pub fn confirm(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(title, message).with_choices(vec![
            PromptChoice::new("yes", "Yes"),
            PromptChoice::new("no", "No"),
        ])
    }
}

/// Incoming answer or response to an active ActorPrompt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorPromptResponse {
    pub prompt_id: Uuid,
    pub actor_id: Option<Uuid>,
    pub selected_choice_id: String,
    pub custom_payload: Option<serde_json::Value>,
}

/// Emitted when an engine workflow or system requests actor input.
#[derive(Message, Event, Debug, Clone, Serialize, Deserialize)]
pub struct ActorPromptRequestEvent {
    pub prompt: ActorPrompt,
}

/// Emitted by any input source (network message, gamepad, keyboard) to resolve an active prompt.
#[derive(Message, Event, Debug, Clone, Serialize, Deserialize)]
pub struct ActorPromptResponseEvent {
    pub response: ActorPromptResponse,
}

/// Emitted when a prompt times out without a response.
#[derive(Message, Event, Debug, Clone, Serialize, Deserialize)]
pub struct ActorPromptTimeoutEvent {
    pub prompt_id: Uuid,
    pub default_choice_id: Option<String>,
}

