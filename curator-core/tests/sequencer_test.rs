//! Frame-exact tests for the curator workflow sequencer.
//!
//! The timestep is 1/64 s, which is exactly representable in `f32`, so subtracting
//! it from delays such as 0.25 s never leaves rounding residue and frame counts are exact.
//! Frame 1 is the startup frame and has `dt == 0` (see `test_harness`).

use std::time::Duration;

use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use curator_core::*;
use test_harness::TestApp;

const DT: Duration = Duration::from_nanos(15_625_000);
const DT_SECS: f32 = 1.0 / 64.0;

fn curator_app() -> TestApp {
    let mut t = TestApp::with_timestep(DT);
    t.add_plugins(CuratorCorePlugin);
    t.capture::<CuratorEventEnvelope>()
        .capture::<WorkflowCompletedEvent>()
        .capture::<ActorPromptRequestEvent>()
        .capture::<ActorPromptTimeoutEvent>();
    t
}

fn spawn(t: &mut TestApp, step: CuratorStep) -> Entity {
    t.world_mut().spawn(WorkflowInstance::new(step, None)).id()
}

fn status(t: &TestApp, e: Entity) -> WorkflowStatus {
    t.world().get::<WorkflowInstance>(e).unwrap().status
}

fn tags(t: &mut TestApp) -> Vec<String> {
    t.drain::<CuratorEventEnvelope>().into_iter().map(|e| e.tag).collect()
}

fn respond(t: &mut TestApp, prompt_id: Uuid, choice: &str) {
    t.send(ActorPromptResponseEvent {
        response: ActorPromptResponse {
            prompt_id,
            actor_id: None,
            selected_choice_id: choice.into(),
            custom_payload: None,
        },
    });
}

#[test]
fn delay_completes_on_exact_frame() {
    let mut t = curator_app();
    let wf = spawn(&mut t, CuratorStep::delay(0.25)); // 16 frames of 1/64 s

    t.step(); // startup, dt = 0
    t.step_n(15);
    assert_eq!(status(&t, wf), WorkflowStatus::Running, "0.234375 s elapsed");

    t.step();
    assert_eq!(status(&t, wf), WorkflowStatus::Completed);

    let done = t.captured::<WorkflowCompletedEvent>();
    assert_eq!(done.len(), 1);
    assert_eq!(done[0].entity, wf);
    assert_eq!(done[0].status, WorkflowStatus::Completed);
    assert_eq!(done[0].duration_secs, 0.25);
}

#[test]
fn completed_workflow_is_not_ticked_again() {
    let mut t = curator_app();
    spawn(&mut t, CuratorStep::emit_raw("once", serde_json::json!({})));

    t.step_n(10);

    assert_eq!(tags(&mut t), vec!["once"]);
    assert_eq!(t.captured::<WorkflowCompletedEvent>().len(), 1);
}

#[test]
fn sequence_runs_zero_time_steps_in_one_frame_and_waits_on_delays() {
    let mut t = curator_app();
    let wf = spawn(
        &mut t,
        CuratorStep::sequence(vec![
            CuratorStep::emit_raw("a", serde_json::json!(1)),
            CuratorStep::emit_raw("b", serde_json::json!(2)),
            CuratorStep::delay(4.0 * DT_SECS),
            CuratorStep::emit_raw("c", serde_json::json!(3)),
        ]),
    );

    t.step();
    assert_eq!(tags(&mut t), vec!["a", "b"]);

    t.step_n(3);
    assert!(tags(&mut t).is_empty());
    assert_eq!(status(&t, wf), WorkflowStatus::Running);

    t.step();
    assert_eq!(tags(&mut t), vec!["c"]);
    assert_eq!(status(&t, wf), WorkflowStatus::Completed);
}

#[test]
fn parallel_completes_with_its_longest_child() {
    let mut t = curator_app();
    let wf = spawn(
        &mut t,
        CuratorStep::parallel(vec![
            CuratorStep::delay(8.0 * DT_SECS),
            CuratorStep::delay(16.0 * DT_SECS),
            CuratorStep::emit_raw("instant", serde_json::json!(null)),
        ]),
    );

    t.step();
    assert_eq!(tags(&mut t), vec!["instant"]);

    t.step_n(15);
    assert_eq!(status(&t, wf), WorkflowStatus::Running);
    t.step();
    assert_eq!(status(&t, wf), WorkflowStatus::Completed);
}

#[test]
fn repeat_runs_child_exact_number_of_times() {
    let mut t = curator_app();
    let wf = spawn(
        &mut t,
        CuratorStep::repeat(
            3,
            CuratorStep::sequence(vec![
                CuratorStep::emit_raw("tick", serde_json::json!(null)),
                CuratorStep::delay(4.0 * DT_SECS),
            ]),
        ),
    );

    let mut tick_frames = Vec::new();
    while status(&t, wf) == WorkflowStatus::Running {
        t.step();
        if !tags(&mut t).is_empty() {
            tick_frames.push(t.frames());
        }
        assert!(t.frames() < 100, "repeat never finished");
    }

    // The first delay starts on the dt = 0 startup frame and so spans 4 stepped frames.
    // Later iterations start mid-frame and consume that frame's dt immediately,
    // so each subsequent iteration takes 3 frames.
    assert_eq!(tick_frames, vec![1, 5, 8]);
    assert_eq!(t.frames(), 11);
}

#[test]
fn prompt_times_out_to_default_choice_on_exact_frame() {
    let mut t = curator_app();
    let prompt = ActorPrompt::confirm("Replay?", "Play again?").with_timeout(0.5, "no");
    let prompt_id = prompt.prompt_id;
    let wf = spawn(&mut t, CuratorStep::prompt(prompt));

    t.step();
    let requests = t.drain::<ActorPromptRequestEvent>();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].prompt.prompt_id, prompt_id);

    t.step_n(31);
    assert!(t.captured::<ActorPromptTimeoutEvent>().is_empty());
    assert_eq!(status(&t, wf), WorkflowStatus::Running);

    t.step(); // 32 * 1/64 = 0.5 s
    let timeouts = t.captured::<ActorPromptTimeoutEvent>();
    assert_eq!(timeouts.len(), 1);
    assert_eq!(timeouts[0].default_choice_id.as_deref(), Some("no"));
    assert_eq!(status(&t, wf), WorkflowStatus::Completed);

    let resolved: Vec<_> = t
        .resource::<AuditLog>()
        .entries()
        .iter()
        .filter_map(|e| match e {
            AuditEvent::PromptResolved { selected_choice_id, .. } => Some(selected_choice_id.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(resolved, vec!["no"]);
}

#[test]
fn prompt_response_completes_workflow_in_the_frame_it_is_read() {
    let mut t = curator_app();
    let prompt = ActorPrompt::confirm("Ready?", "Start game?");
    let prompt_id = prompt.prompt_id;
    let wf = spawn(&mut t, CuratorStep::prompt(prompt));

    t.step_n(5);
    assert_eq!(status(&t, wf), WorkflowStatus::Running);

    respond(&mut t, Uuid::new_v4(), "yes"); // unrelated prompt
    t.step();
    assert_eq!(status(&t, wf), WorkflowStatus::Running);

    respond(&mut t, prompt_id, "yes");
    t.step();
    assert_eq!(status(&t, wf), WorkflowStatus::Completed);
}

#[test]
fn wait_for_event_ignores_other_tags_and_resolves_on_match() {
    let mut t = curator_app();
    let wf = spawn(
        &mut t,
        CuratorStep::sequence(vec![
            CuratorStep::wait_event("target:hit"),
            CuratorStep::emit_raw("score", serde_json::json!(10)),
        ]),
    );

    t.step_n(3);
    t.send(CuratorEventEnvelope::from_raw("target:miss"));
    t.step();
    assert_eq!(status(&t, wf), WorkflowStatus::Running);

    t.send(CuratorEventEnvelope::from_raw("target:hit"));
    t.step();
    assert_eq!(status(&t, wf), WorkflowStatus::Completed);
    let emitted = tags(&mut t);
    assert_eq!(emitted.last().map(String::as_str), Some("score"));
}

#[test]
fn wait_for_event_timeout_completes_after_exact_duration() {
    let mut t = curator_app();
    let wf = spawn(&mut t, CuratorStep::wait_event_timeout("never", 10.0 * DT_SECS));

    t.step();
    t.step_n(9);
    assert_eq!(status(&t, wf), WorkflowStatus::Running);
    t.step();
    assert_eq!(status(&t, wf), WorkflowStatus::Completed);
}

/// Same input must yield the same audit trace, frame by frame.
#[test]
fn identical_runs_produce_identical_traces() {
    fn run() -> Vec<(u32, String)> {
        let mut t = curator_app();
        let prompt = ActorPrompt::confirm("Go?", "Go?")
            .with_timeout(0.25, "yes");
        spawn(
            &mut t,
            CuratorStep::sequence(vec![
                CuratorStep::emit_raw("start", serde_json::json!(null)),
                CuratorStep::parallel(vec![
                    CuratorStep::prompt(prompt),
                    CuratorStep::repeat(2, CuratorStep::delay(3.0 * DT_SECS)),
                ]),
                CuratorStep::emit_raw("end", serde_json::json!(null)),
            ]),
        );

        let mut trace = Vec::new();
        let mut seen = 0;
        for _ in 0..40 {
            t.step();
            let log = t.resource::<AuditLog>().entries();
            for entry in log.iter().skip(seen) {
                trace.push((t.frames(), describe(entry)));
            }
            seen = log.len();
        }
        trace
    }

    let first = run();
    assert!(!first.is_empty());
    assert_eq!(first, run());
}

/// Audit entry without random ids, for comparing runs.
fn describe(e: &AuditEvent) -> String {
    match e {
        AuditEvent::StepTransition { step_description, .. } => format!("step {step_description}"),
        AuditEvent::PayloadDispatched { tag, .. } => format!("payload {tag}"),
        AuditEvent::PromptDispatched { title, .. } => format!("prompt {title}"),
        AuditEvent::PromptResolved { selected_choice_id, .. } => format!("resolved {selected_choice_id}"),
        AuditEvent::WorkflowCompleted { duration_secs, .. } => format!("completed {duration_secs}"),
        other => format!("{other:?}"),
    }
}

trait FromRaw {
    fn from_raw(tag: &str) -> Self;
}

impl FromRaw for CuratorEventEnvelope {
    fn from_raw(tag: &str) -> Self {
        CuratorEventEnvelope {
            tag: tag.into(),
            payload: serde_json::Value::Null,
            session_id: None,
            source_entity: None,
            timestamp_ms: 0,
        }
    }
}
