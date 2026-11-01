pub mod audit;
pub mod payload;
pub mod prompt;
pub mod sequencer;
pub mod step;

pub use audit::{AuditEvent, AuditLog};
pub use payload::{CuratorEventEnvelope, CuratorPayload};
pub use prompt::{
    ActorPrompt, ActorPromptRequestEvent, ActorPromptResponse, ActorPromptResponseEvent,
    ActorPromptTimeoutEvent, ActorTarget, PromptChoice,
};
pub use sequencer::{
    advance_workflows_system, resolve_workflow_events_system, resolve_workflow_prompts_system,
    CuratorCommandsExt, ExecutionNode, StepProgress, WorkflowCompletedEvent, WorkflowInstance,
    WorkflowStatus,
};
pub use step::CuratorStep;

use bevy::prelude::*;

/// Bevy plugin providing in-memory workflow execution, actor prompts, dynamic event envelopes, and audit tracking.
pub struct CuratorCorePlugin;

impl Plugin for CuratorCorePlugin {
    fn build(&self, app: &mut App) {
        // Register messages / events
        app.add_message::<CuratorEventEnvelope>();
        app.add_message::<ActorPromptRequestEvent>();
        app.add_message::<ActorPromptResponseEvent>();
        app.add_message::<ActorPromptTimeoutEvent>();
        app.add_message::<WorkflowCompletedEvent>();

        // Register in-memory audit log resource
        app.init_resource::<AuditLog>();

        // Register execution systems
        app.add_systems(
            Update,
            (
                resolve_workflow_prompts_system,
                resolve_workflow_events_system,
                advance_workflows_system,
            )
                .chain(),
        );
    }
}

