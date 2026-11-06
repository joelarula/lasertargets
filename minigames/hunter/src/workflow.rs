use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use common::curator_core::{
    ActorPrompt, ActorPromptResponseEvent, CuratorCommandsExt, CuratorEventEnvelope, CuratorPayload,
    CuratorStep, PromptChoice,
};
use common::state::ServerState;
use serde::{Deserialize, Serialize};

use crate::audit::{HunterAuditEvent, HunterGameStats};
use crate::events::SpawnHunterTargetEvent;
use common::scene::Billboard;
use crate::systems::render::spawn_title;
use crate::types::HunterTarget;

const WAVES: u32 = 3;
const TARGETS_PER_WAVE: u32 = 5;
const WAVE_SECS: f32 = 12.0;
const WAVE_BALLOON: HunterTarget = HunterTarget::Balloon(0.12, Color::srgb(1.0, 0.2, 0.2));

/// Hunter round phases, emitted by the workflow
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum HunterPhasePayload {
    TitleAnnouncement,
    SpawnWave { wave: u32, target_count: u32 },
    RoundFinished,
}

impl CuratorPayload for HunterPhasePayload {
    fn event_tag() -> &'static str {
        "hunter_phase"
    }
}

/// Title → waves of balloons → replay prompt
pub fn build_hunter_workflow(session_id: Uuid) -> CuratorStep {
    let mut steps = vec![
        CuratorStep::emit(&HunterPhasePayload::TitleAnnouncement),
        CuratorStep::delay(3.5),
    ];
    for wave in 1..=WAVES {
        steps.push(CuratorStep::emit(&HunterPhasePayload::SpawnWave { wave, target_count: TARGETS_PER_WAVE }));
        steps.push(CuratorStep::delay(WAVE_SECS));
    }
    steps.push(CuratorStep::emit(&HunterPhasePayload::RoundFinished));
    steps.push(CuratorStep::prompt(
        ActorPrompt::new("Hunter Over", "Round Complete! Play another round?")
            .with_session(session_id)
            .with_choices(vec![
                PromptChoice::new("replay", "Play Again"),
                PromptChoice::new("menu", "Back to Menu"),
            ])
            .with_timeout(15.0, "menu"),
    ));
    CuratorStep::sequence(steps)
}

/// Acts on the workflow's phases (logging each) and on the replay prompt's answer
pub fn handle_hunter_phase_events(
    mut commands: Commands,
    mut envelopes: MessageReader<CuratorEventEnvelope>,
    mut responses: MessageReader<ActorPromptResponseEvent>,
    billboard: Billboard,
    mut stats: Option<ResMut<HunterGameStats>>,
    time: Res<Time>,
    mut spawns: MessageWriter<SpawnHunterTargetEvent>,
    mut next_server_state: ResMut<NextState<ServerState>>,
) {
    let now = time.elapsed_secs_f64();
    let phases = envelopes
        .read()
        .filter(|env| env.is::<HunterPhasePayload>())
        .filter_map(|env| env.downcast::<HunterPhasePayload>().ok());
    for phase in phases {
        info!("[HunterWorkflow] {phase:?}");
        match phase {
            HunterPhasePayload::TitleAnnouncement => spawn_title(&mut commands, &billboard),
            HunterPhasePayload::SpawnWave { wave, target_count } => {
                if let Some(stats) = stats.as_mut() {
                    stats.record(now, HunterAuditEvent::Wave { wave, targets: target_count });
                }
                // Evenly along the bottom edge, inside the middle 80%
                let half = billboard.half_size();
                for i in 0..target_count {
                    let x = half.x * 0.8 * (2.0 * (i as f32 + 0.5) / target_count as f32 - 1.0);
                    spawns.write(SpawnHunterTargetEvent {
                        target: WAVE_BALLOON,
                        position: billboard.to_world(Vec3::new(x, 0.1 - half.y, 0.0)),
                    });
                }
            }
            HunterPhasePayload::RoundFinished => {
                if let Some(stats) = stats.as_mut() {
                    stats.record(now, HunterAuditEvent::RoundFinished);
                    info!("[HunterWorkflow] Round finished: {}", stats.tally);
                }
            }
        }
    }

    // Only a running round answers the replay prompt
    let Some(session_id) = stats.as_ref().map(|s| s.session_id) else { return };
    for response in responses.read() {
        match response.response.selected_choice_id.as_str() {
            "replay" => {
                info!("[HunterWorkflow] Replay chosen, restarting the workflow");
                commands.spawn_workflow_with_session(session_id, build_hunter_workflow(session_id));
            }
            "menu" => next_server_state.set(ServerState::Menu),
            _ => {}
        }
    }
}
