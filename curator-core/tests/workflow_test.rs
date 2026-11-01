use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use curator_core::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct TestWavePayload {
    wave_number: u32,
    target_count: u32,
}

impl CuratorPayload for TestWavePayload {
    fn event_tag() -> &'static str {
        "test:spawn_wave"
    }
}

#[test]
fn test_curator_payload_envelope_downcast() {
    let payload = TestWavePayload {
        wave_number: 1,
        target_count: 5,
    };
    let session = Uuid::new_v4();
    let envelope = CuratorEventEnvelope::from_payload(&payload, Some(session));

    assert_eq!(envelope.tag, "test:spawn_wave");
    assert!(envelope.is::<TestWavePayload>());

    let downcasted = envelope.downcast::<TestWavePayload>().unwrap();
    assert_eq!(downcasted, payload);
    assert_eq!(envelope.session_id, Some(session));
}

#[test]
fn test_in_memory_workflow_sequence_and_prompt() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(CuratorCorePlugin);

    let prompt = ActorPrompt::confirm("Game Over", "Replay game?");
    let prompt_id = prompt.prompt_id;

    // Build a sequence: Emit Payload -> Prompt for Replay -> Delay
    let step = CuratorStep::sequence(vec![
        CuratorStep::emit(&TestWavePayload {
            wave_number: 1,
            target_count: 3,
        }),
        CuratorStep::prompt(prompt),
        CuratorStep::delay(1.0),
    ]);

    let session_id = Uuid::new_v4();
    let entity = app.world_mut().commands().spawn_workflow_with_session(session_id, step).id();
    app.update();

    // Step 1: Payload emitted on first update
    {
        let audit = app.world().resource::<AuditLog>();
        assert!(audit.count() > 0);
        let has_payload = audit.entries().iter().any(|e| match e {
            AuditEvent::PayloadDispatched { tag, .. } => tag == "test:spawn_wave",
            _ => false,
        });
        assert!(has_payload, "Expected test:spawn_wave in audit log");
    }

    // Step 2: Now waiting on ActorPrompt
    {
        let wf = app.world().get::<WorkflowInstance>(entity).unwrap();
        assert_eq!(wf.status, WorkflowStatus::Running);
    }

    // Resolve the prompt by sending an ActorPromptResponseEvent
    {
        let mut writer = app.world_mut().resource_mut::<bevy::ecs::message::Messages<ActorPromptResponseEvent>>();
        writer.write(ActorPromptResponseEvent {
            response: ActorPromptResponse {
                prompt_id,
                actor_id: None,
                selected_choice_id: "yes".into(),
                custom_payload: None,
            },
        });
    }

    // Run updates to process prompt response and advance
    app.update();

    // Verify audit log has the resolved prompt
    {
        let audit = app.world().resource::<AuditLog>();
        let prompt_resolved = audit.entries().iter().any(|e| match e {
            AuditEvent::PromptResolved { selected_choice_id, .. } => selected_choice_id == "yes",
            _ => false,
        });
        assert!(prompt_resolved, "Expected PromptResolved in audit log");
    }
}
