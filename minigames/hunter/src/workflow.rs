use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use common::curator_core::{ActorPrompt, CuratorCommandsExt, CuratorPayload, CuratorStep, PromptChoice};
use common::path::PathRenderable;
use laserlogic::{from_ttf_text, LaserTextOptions};
use common::scene::{SceneEntity, SceneSetup};
use serde::{Deserialize, Serialize};

use crate::components::HunterTitleAnnouncement;
use crate::events::SpawnHunterTargetEvent;
use crate::types::HunterTarget;

/// Dynamic payload describing Hunter game phase changes
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum HunterPhasePayload {
    /// Trigger title announcement display
    TitleAnnouncement,
    /// Start a new wave of targets
    SpawnWave { wave: u32, target_count: u32 },
    /// Signal round conclusion
    RoundFinished { final_score: u32 },
}

impl CuratorPayload for HunterPhasePayload {
    fn event_tag() -> &'static str {
        "hunter_phase"
    }
}

/// Builds a declarative round workflow for Hunter: Title -> 3 Waves -> Replay Prompt
pub fn build_hunter_workflow(session_id: Uuid) -> CuratorStep {
    CuratorStep::sequence(vec![
        // 1. Show HUNTER title announcement for 3.5 seconds
        CuratorStep::emit(&HunterPhasePayload::TitleAnnouncement),
        CuratorStep::delay(3.5),

        // 2. 3 Waves of balloon targets
        CuratorStep::repeat(
            3,
            CuratorStep::sequence(vec![
                CuratorStep::emit(&HunterPhasePayload::SpawnWave {
                    wave: 1,
                    target_count: 5,
                }),
                CuratorStep::delay(12.0),
            ]),
        ),

        // 3. Round finished - show prompt on terminals/gamepads
        CuratorStep::emit(&HunterPhasePayload::RoundFinished { final_score: 0 }),
        CuratorStep::prompt(
            ActorPrompt::new("Hunter Over", "Round Complete! Play another round?")
                .with_session(session_id)
                .with_choices(vec![
                    PromptChoice::new("replay", "Play Again"),
                    PromptChoice::new("menu", "Back to Menu"),
                ])
                .with_timeout(15.0, "menu"),
        ),
    ])
}

/// Helper to render the center vector title announcement
fn spawn_title_announcement(
    commands: &mut Commands,
    scene_query: &Query<Entity, With<SceneEntity>>,
    scene_setup: &Res<SceneSetup>,
    existing_titles: &Query<Entity, With<HunterTitleAnnouncement>>,
) {
    for entity in existing_titles.iter() {
        commands.entity(entity).despawn();
    }

    let scene_dim = scene_setup.scene.scene_dimension;
    let height_cap = scene_dim.y as f32 * 0.55;
    let num_chars = 6usize;
    let char_width_ratio = 0.65_f32;
    let letter_spacing = 0.08_f32;
    let total_width_per_unit = num_chars as f32 * (char_width_ratio + letter_spacing);
    let width_cap = (scene_dim.x as f32 * 0.85) / total_width_per_unit;
    let text_height = height_cap.min(width_cap).clamp(0.8, 3.5);

    let options = LaserTextOptions {
        origin: Vec2::ZERO,
        height: text_height,
        color: Color::srgb(1.0, 0.95, 0.1),
        center_on_origin: true,
        ..Default::default()
    };

    const FALLBACK_FONT_BYTES: &[u8] = include_bytes!("../../../assets/fonts/centurygothic.ttf");
    let maybe_title_path = from_ttf_text(FALLBACK_FONT_BYTES, "HUNTER", &options).ok();

    if let Some(title_path) = maybe_title_path {
        let child_entity = commands
            .spawn((
                Transform::default(),
                GlobalTransform::default(),
                Visibility::default(),
                HunterTitleAnnouncement {
                    timer: Timer::from_seconds(3.0, TimerMode::Once),
                },
                title_path,
                PathRenderable::default(),
            ))
            .id();

        if let Ok(scene_entity) = scene_query.single() {
            commands.entity(scene_entity).add_child(child_entity);
        }
    }
}

/// System that handles HunterPhasePayload events emitted by the in-memory workflow
pub fn handle_hunter_phase_events(
    mut commands: Commands,
    mut envelopes: MessageReader<common::curator_core::CuratorEventEnvelope>,
    scene_query: Query<Entity, With<SceneEntity>>,
    scene_setup: Res<SceneSetup>,
    existing_titles: Query<Entity, With<HunterTitleAnnouncement>>,
    mut spawn_target_writer: MessageWriter<SpawnHunterTargetEvent>,
    mut next_server_state: ResMut<NextState<common::state::ServerState>>,
    mut prompt_responses: MessageReader<common::curator_core::ActorPromptResponseEvent>,
) {
    // 1. Handle incoming phase envelopes
    for env in envelopes.read() {
        if env.is::<HunterPhasePayload>() {
            if let Ok(phase) = env.downcast::<HunterPhasePayload>() {
                match phase {
                    HunterPhasePayload::TitleAnnouncement => {
                        info!("[HunterWorkflow] Showing Title Announcement");
                        spawn_title_announcement(&mut commands, &scene_query, &scene_setup, &existing_titles);
                    }
                    HunterPhasePayload::SpawnWave { wave, target_count } => {
                        info!("[HunterWorkflow] Spawning Wave {} with {} targets", wave, target_count);
                        let half_w = scene_setup.scene.scene_dimension.x as f32 / 2.0;
                        let bottom_y = -(scene_setup.scene.scene_dimension.y as f32 / 2.0);

                        for i in 0..target_count {
                            let fraction = (i as f32 + 0.5) / (target_count as f32);
                            let x = -half_w * 0.8 + fraction * (half_w * 1.6);
                            spawn_target_writer.write(SpawnHunterTargetEvent {
                                target: HunterTarget::Baloon(0.12, Color::srgb(1.0, 0.2, 0.2)),
                                position: Vec3::new(x, bottom_y + 0.1, 0.0),
                            });
                        }
                    }
                    HunterPhasePayload::RoundFinished { final_score } => {
                        info!("[HunterWorkflow] Round finished with score: {}", final_score);
                    }
                }
            }
        }
    }

    // 2. Handle replay prompt response
    for resp in prompt_responses.read() {
        if resp.response.selected_choice_id == "replay" {
            info!("[HunterWorkflow] Replay chosen! Restarting Hunter workflow");
            let session_id = resp.response.prompt_id;
            commands.spawn_workflow_with_session(session_id, build_hunter_workflow(session_id));
        } else if resp.response.selected_choice_id == "menu" {
            info!("[HunterWorkflow] Menu chosen! Returning to ServerState::Menu");
            next_server_state.set(common::state::ServerState::Menu);
        }
    }
}
