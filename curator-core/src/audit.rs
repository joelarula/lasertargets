use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// A recorded event in the curator audit log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEvent {
    SessionStarted {
        session_id: Uuid,
        timestamp_ms: u64,
    },
    WorkflowSpawned {
        workflow_id: Uuid,
        session_id: Option<Uuid>,
        timestamp_ms: u64,
    },
    WorkflowCompleted {
        workflow_id: Uuid,
        duration_secs: f32,
        timestamp_ms: u64,
    },
    StepTransition {
        workflow_id: Uuid,
        step_description: String,
        timestamp_ms: u64,
    },
    PromptDispatched {
        prompt_id: Uuid,
        title: String,
        choices: Vec<String>,
        timestamp_ms: u64,
    },
    PromptResolved {
        prompt_id: Uuid,
        selected_choice_id: String,
        actor_id: Option<Uuid>,
        timestamp_ms: u64,
    },
    PayloadDispatched {
        tag: String,
        payload: serde_json::Value,
        session_id: Option<Uuid>,
        timestamp_ms: u64,
    },
}

/// In-memory ring-buffer audit store for game development and testing.
#[derive(Resource, Debug, Clone)]
pub struct AuditLog {
    max_capacity: usize,
    entries: VecDeque<AuditEvent>,
}

impl Default for AuditLog {
    fn default() -> Self {
        Self {
            max_capacity: 1000,
            entries: VecDeque::with_capacity(1000),
        }
    }
}

impl AuditLog {
    pub fn new(capacity: usize) -> Self {
        Self {
            max_capacity: capacity,
            entries: VecDeque::with_capacity(capacity),
        }
    }

    pub fn record(&mut self, event: AuditEvent) {
        if self.entries.len() >= self.max_capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(event);
    }

    pub fn entries(&self) -> &VecDeque<AuditEvent> {
        &self.entries
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

