use crate::audit::{AuditEvent, AuditLog};
use crate::payload::CuratorEventEnvelope;
use crate::prompt::{ActorPrompt, ActorPromptRequestEvent, ActorPromptResponseEvent, ActorPromptTimeoutEvent};
use crate::step::CuratorStep;
use bevy::asset::uuid::Uuid;
use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Status of an active in-memory workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowStatus {
    Running,
    Completed,
    Failed,
}

/// Progress outcome of ticking an execution node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepProgress {
    Running,
    Completed,
    Failed(String),
}

/// Active execution state node mirroring the declarative CuratorStep AST.
#[derive(Debug, Clone)]
pub enum ExecutionNode {
    Delay {
        remaining: f32,
    },
    EmitPayload {
        tag: String,
        payload: serde_json::Value,
        emitted: bool,
    },
    Prompt {
        prompt: ActorPrompt,
        remaining: Option<f32>,
        requested: bool,
        resolved_choice: Option<String>,
    },
    WaitForEvent {
        tag: String,
        remaining: Option<f32>,
        received: bool,
    },
    Sequence {
        steps: Vec<CuratorStep>,
        current_idx: usize,
        current_node: Option<Box<ExecutionNode>>,
    },
    Parallel {
        nodes: Vec<ExecutionNode>,
    },
    Repeat {
        count: usize,
        current_iteration: usize,
        step: CuratorStep,
        current_node: Option<Box<ExecutionNode>>,
    },
    CustomAction {
        name: String,
        payload: serde_json::Value,
        executed: bool,
    },
}

impl ExecutionNode {
    pub fn from_step(step: &CuratorStep) -> Self {
        match step {
            CuratorStep::Delay(secs) => Self::Delay { remaining: *secs },
            CuratorStep::EmitPayload { tag, payload } => Self::EmitPayload {
                tag: tag.clone(),
                payload: payload.clone(),
                emitted: false,
            },
            CuratorStep::Prompt(prompt) => Self::Prompt {
                remaining: prompt.timeout_secs,
                prompt: prompt.clone(),
                requested: false,
                resolved_choice: None,
            },
            CuratorStep::WaitForEvent { tag, timeout_secs } => Self::WaitForEvent {
                tag: tag.clone(),
                remaining: *timeout_secs,
                received: false,
            },
            CuratorStep::Sequence(steps) => Self::Sequence {
                steps: steps.clone(),
                current_idx: 0,
                current_node: None,
            },
            CuratorStep::Parallel(steps) => Self::Parallel {
                nodes: steps.iter().map(Self::from_step).collect(),
            },
            CuratorStep::Repeat { count, step } => Self::Repeat {
                count: *count,
                current_iteration: 0,
                step: (**step).clone(),
                current_node: None,
            },
            CuratorStep::CustomAction { name, payload } => Self::CustomAction {
                name: name.clone(),
                payload: payload.clone(),
                executed: false,
            },
        }
    }

    /// Try resolving an active prompt inside this node tree.
    pub fn resolve_prompt(&mut self, prompt_id: Uuid, choice_id: &str) -> bool {
        match self {
            Self::Prompt { prompt, resolved_choice, .. } => {
                if prompt.prompt_id == prompt_id {
                    *resolved_choice = Some(choice_id.to_string());
                    return true;
                }
                false
            }
            Self::Sequence { current_node, .. } => {
                if let Some(node) = current_node {
                    node.resolve_prompt(prompt_id, choice_id)
                } else {
                    false
                }
            }
            Self::Parallel { nodes } => {
                let mut resolved = false;
                for node in nodes {
                    if node.resolve_prompt(prompt_id, choice_id) {
                        resolved = true;
                    }
                }
                resolved
            }
            Self::Repeat { current_node, .. } => {
                if let Some(node) = current_node {
                    node.resolve_prompt(prompt_id, choice_id)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Try resolving an active WaitForEvent inside this node tree.
    pub fn resolve_event(&mut self, tag: &str) -> bool {
        match self {
            Self::WaitForEvent { tag: expected_tag, received, .. } => {
                if expected_tag == tag {
                    *received = true;
                    return true;
                }
                false
            }
            Self::Sequence { current_node, .. } => {
                if let Some(node) = current_node {
                    node.resolve_event(tag)
                } else {
                    false
                }
            }
            Self::Parallel { nodes } => {
                let mut resolved = false;
                for node in nodes {
                    if node.resolve_event(tag) {
                        resolved = true;
                    }
                }
                resolved
            }
            Self::Repeat { current_node, .. } => {
                if let Some(node) = current_node {
                    node.resolve_event(tag)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Advance this node by `dt` seconds.
    pub fn tick(
        &mut self,
        dt: f32,
        prompt_writer: &mut MessageWriter<ActorPromptRequestEvent>,
        timeout_writer: &mut MessageWriter<ActorPromptTimeoutEvent>,
        envelope_writer: &mut MessageWriter<CuratorEventEnvelope>,
        session_id: Option<Uuid>,
        audit_log: &mut AuditLog,
        workflow_id: Uuid,
    ) -> StepProgress {
        match self {
            Self::Delay { remaining } => {
                *remaining -= dt;
                if *remaining <= 0.0 {
                    StepProgress::Completed
                } else {
                    StepProgress::Running
                }
            }
            Self::EmitPayload { tag, payload, emitted } => {
                if !*emitted {
                    *emitted = true;
                    envelope_writer.write(CuratorEventEnvelope {
                        tag: tag.clone(),
                        payload: payload.clone(),
                        session_id,
                        source_entity: None,
                        timestamp_ms: 0,
                    });
                    audit_log.record(AuditEvent::PayloadDispatched {
                        tag: tag.clone(),
                        payload: payload.clone(),
                        session_id,
                        timestamp_ms: 0,
                    });
                }
                StepProgress::Completed
            }
            Self::Prompt {
                prompt,
                remaining,
                requested,
                resolved_choice,
            } => {
                if !*requested {
                    *requested = true;
                    let mut p = prompt.clone();
                    if p.session_id.is_none() {
                        p.session_id = session_id;
                    }
                    prompt_writer.write(ActorPromptRequestEvent { prompt: p.clone() });
                    audit_log.record(AuditEvent::PromptDispatched {
                        prompt_id: p.prompt_id,
                        title: p.title.clone(),
                        choices: p.choices.iter().map(|c| c.id.clone()).collect(),
                        timestamp_ms: 0,
                    });
                }

                if resolved_choice.is_some() {
                    return StepProgress::Completed;
                }

                if let Some(rem) = remaining {
                    *rem -= dt;
                    if *rem <= 0.0 {
                        let default_choice = prompt.default_choice_id.clone().unwrap_or_else(|| "timeout".into());
                        *resolved_choice = Some(default_choice.clone());
                        timeout_writer.write(ActorPromptTimeoutEvent {
                            prompt_id: prompt.prompt_id,
                            default_choice_id: prompt.default_choice_id.clone(),
                        });
                        audit_log.record(AuditEvent::PromptResolved {
                            prompt_id: prompt.prompt_id,
                            selected_choice_id: default_choice,
                            actor_id: None,
                            timestamp_ms: 0,
                        });
                        return StepProgress::Completed;
                    }
                }

                StepProgress::Running
            }
            Self::WaitForEvent { tag: _, remaining, received } => {
                if *received {
                    return StepProgress::Completed;
                }
                if let Some(rem) = remaining {
                    *rem -= dt;
                    if *rem <= 0.0 {
                        return StepProgress::Completed;
                    }
                }
                StepProgress::Running
            }
            Self::Sequence {
                steps,
                current_idx,
                current_node,
            } => {
                loop {
                    if *current_idx >= steps.len() {
                        return StepProgress::Completed;
                    }

                    if current_node.is_none() {
                        *current_node = Some(Box::new(Self::from_step(&steps[*current_idx])));
                        audit_log.record(AuditEvent::StepTransition {
                            workflow_id,
                            step_description: format!("Sequence step [{}/{}]", *current_idx + 1, steps.len()),
                            timestamp_ms: 0,
                        });
                    }

                    if let Some(node) = current_node {
                        let res = node.tick(
                            dt,
                            prompt_writer,
                            timeout_writer,
                            envelope_writer,
                            session_id,
                            audit_log,
                            workflow_id,
                        );
                        match res {
                            StepProgress::Running => return StepProgress::Running,
                            StepProgress::Failed(err) => return StepProgress::Failed(err),
                            StepProgress::Completed => {
                                *current_idx += 1;
                                *current_node = None;
                            }
                        }
                    }
                }
            }
            Self::Parallel { nodes } => {
                let mut all_completed = true;
                for node in nodes.iter_mut() {
                    let res = node.tick(
                        dt,
                        prompt_writer,
                        timeout_writer,
                        envelope_writer,
                        session_id,
                        audit_log,
                        workflow_id,
                    );
                    match res {
                        StepProgress::Running => all_completed = false,
                        StepProgress::Failed(err) => return StepProgress::Failed(err),
                        StepProgress::Completed => {}
                    }
                }
                if all_completed {
                    StepProgress::Completed
                } else {
                    StepProgress::Running
                }
            }
            Self::Repeat {
                count,
                current_iteration,
                step,
                current_node,
            } => {
                loop {
                    if *count > 0 && *current_iteration >= *count {
                        return StepProgress::Completed;
                    }

                    if current_node.is_none() {
                        *current_node = Some(Box::new(Self::from_step(step)));
                        audit_log.record(AuditEvent::StepTransition {
                            workflow_id,
                            step_description: format!("Repeat iteration [{}]", *current_iteration + 1),
                            timestamp_ms: 0,
                        });
                    }

                    if let Some(node) = current_node {
                        let res = node.tick(
                            dt,
                            prompt_writer,
                            timeout_writer,
                            envelope_writer,
                            session_id,
                            audit_log,
                            workflow_id,
                        );
                        match res {
                            StepProgress::Running => return StepProgress::Running,
                            StepProgress::Failed(err) => return StepProgress::Failed(err),
                            StepProgress::Completed => {
                                *current_iteration += 1;
                                *current_node = None;
                            }
                        }
                    }
                }
            }
            Self::CustomAction { name, payload: _, executed } => {
                if !*executed {
                    *executed = true;
                    audit_log.record(AuditEvent::StepTransition {
                        workflow_id,
                        step_description: format!("CustomAction: {}", name),
                        timestamp_ms: 0,
                    });
                }
                StepProgress::Completed
            }
        }
    }
}

/// Component attached to a workflow entity running an in-memory CuratorStep sequence.
#[derive(Component, Debug)]
pub struct WorkflowInstance {
    pub workflow_id: Uuid,
    pub session_id: Option<Uuid>,
    pub root_node: ExecutionNode,
    pub status: WorkflowStatus,
    pub elapsed_secs: f32,
}

impl WorkflowInstance {
    pub fn new(step: CuratorStep, session_id: Option<Uuid>) -> Self {
        Self {
            workflow_id: Uuid::new_v4(),
            session_id,
            root_node: ExecutionNode::from_step(&step),
            status: WorkflowStatus::Running,
            elapsed_secs: 0.0,
        }
    }
}

/// Emitted when a workflow finishes execution.
#[derive(Message, Event, Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCompletedEvent {
    pub workflow_id: Uuid,
    pub session_id: Option<Uuid>,
    pub entity: Entity,
    pub duration_secs: f32,
    pub status: WorkflowStatus,
}

/// System to handle incoming ActorPrompt responses and resolve active prompts in workflows.
pub fn resolve_workflow_prompts_system(
    mut prompt_responses: MessageReader<ActorPromptResponseEvent>,
    mut workflows: Query<&mut WorkflowInstance>,
    mut audit_log: ResMut<AuditLog>,
) {
    for resp_event in prompt_responses.read() {
        let resp = &resp_event.response;
        for mut wf in workflows.iter_mut() {
            if wf.status != WorkflowStatus::Running {
                continue;
            }
            if wf.root_node.resolve_prompt(resp.prompt_id, &resp.selected_choice_id) {
                audit_log.record(AuditEvent::PromptResolved {
                    prompt_id: resp.prompt_id,
                    selected_choice_id: resp.selected_choice_id.clone(),
                    actor_id: resp.actor_id,
                    timestamp_ms: 0,
                });
            }
        }
    }
}

/// System to handle incoming dynamic event envelopes and resolve WaitForEvent steps.
pub fn resolve_workflow_events_system(
    mut envelopes: MessageReader<CuratorEventEnvelope>,
    mut workflows: Query<&mut WorkflowInstance>,
) {
    for env in envelopes.read() {
        for mut wf in workflows.iter_mut() {
            if wf.status != WorkflowStatus::Running {
                continue;
            }
            wf.root_node.resolve_event(&env.tag);
        }
    }
}

/// Main system that advances all running in-memory workflows each frame.
pub fn advance_workflows_system(
    time: Res<Time>,
    mut workflows: Query<(Entity, &mut WorkflowInstance)>,
    mut prompt_writer: MessageWriter<ActorPromptRequestEvent>,
    mut timeout_writer: MessageWriter<ActorPromptTimeoutEvent>,
    mut envelope_writer: MessageWriter<CuratorEventEnvelope>,
    mut completed_writer: MessageWriter<WorkflowCompletedEvent>,
    mut audit_log: ResMut<AuditLog>,
) {
    let dt = time.delta_secs();

    for (entity, mut wf) in workflows.iter_mut() {
        if wf.status != WorkflowStatus::Running {
            continue;
        }

        wf.elapsed_secs += dt;
        let wf_id = wf.workflow_id;
        let session_id = wf.session_id;

        let res = wf.root_node.tick(
            dt,
            &mut prompt_writer,
            &mut timeout_writer,
            &mut envelope_writer,
            session_id,
            &mut audit_log,
            wf_id,
        );

        match res {
            StepProgress::Running => {}
            StepProgress::Completed => {
                wf.status = WorkflowStatus::Completed;
                completed_writer.write(WorkflowCompletedEvent {
                    workflow_id: wf_id,
                    session_id,
                    entity,
                    duration_secs: wf.elapsed_secs,
                    status: WorkflowStatus::Completed,
                });
                audit_log.record(AuditEvent::WorkflowCompleted {
                    workflow_id: wf_id,
                    duration_secs: wf.elapsed_secs,
                    timestamp_ms: 0,
                });
            }
            StepProgress::Failed(err) => {
                wf.status = WorkflowStatus::Failed;
                log::error!("Workflow {:?} failed: {}", wf_id, err);
                completed_writer.write(WorkflowCompletedEvent {
                    workflow_id: wf_id,
                    session_id,
                    entity,
                    duration_secs: wf.elapsed_secs,
                    status: WorkflowStatus::Failed,
                });
            }
        }
    }
}

/// Extension trait on Commands to easily spawn workflows.
pub trait CuratorCommandsExt {
    fn spawn_workflow(&mut self, step: CuratorStep) -> EntityCommands<'_>;
    fn spawn_workflow_with_session(&mut self, session_id: Uuid, step: CuratorStep) -> EntityCommands<'_>;
}

impl<'w, 's> CuratorCommandsExt for Commands<'w, 's> {
    fn spawn_workflow(&mut self, step: CuratorStep) -> EntityCommands<'_> {
        self.spawn(WorkflowInstance::new(step, None))
    }

    fn spawn_workflow_with_session(&mut self, session_id: Uuid, step: CuratorStep) -> EntityCommands<'_> {
        self.spawn(WorkflowInstance::new(step, Some(session_id)))
    }
}
