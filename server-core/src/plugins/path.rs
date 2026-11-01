use bevy::prelude::*;
use common::path::{
    AbstractPathData, BroadcastScenePaths, ModifierContext, MorphStrategyRegistry,
    PathModifierPipeline, PathModifierRegistry, PathModulator, PathRenderable, UniversalPath,
    VectorFrame,
};

#[derive(Resource, Default)]
pub struct VectorFrameCounter(pub u64);

pub struct PathNetworkPlugin;

impl Plugin for PathNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VectorFrameCounter>()
            .init_resource::<MorphStrategyRegistry>()
            .init_resource::<PathModifierRegistry>()
            .add_message::<BroadcastScenePaths>()
            .add_systems(
                Update,
                (evaluate_path_modulators, apply_path_modifier_pipelines).chain(),
            )
            .add_systems(PostUpdate, broadcast_combined_abstract_scene_paths);
    }
}

/// Evaluates active PathModulator signal sources, easing, and executes pluggable morph strategies in-place
fn evaluate_path_modulators(
    time: Res<Time>,
    mut query: Query<(&mut UniversalPath, &mut PathModulator)>,
) {
    let delta = time.delta_secs();
    let elapsed = time.elapsed_secs();

    for (mut path, mut modulator) in query.iter_mut() {
        let raw_t = modulator.source.advance_and_sample(delta, elapsed);
        let eased_t = modulator.easing.apply(raw_t);
        modulator.current_t = eased_t;

        let PathModulator {
            keyframe_a,
            keyframe_b,
            strategy,
            ..
        } = &mut *modulator;

        strategy.morph(keyframe_a, keyframe_b, eased_t, &mut path);
    }
}

/// Evaluates pluggable modifier pipelines (transformers, oscillators, color shaders) in-place
fn apply_path_modifier_pipelines(
    time: Res<Time>,
    mut query: Query<(&mut UniversalPath, &mut PathModifierPipeline, Option<&PathModulator>)>,
) {
    let delta_secs = time.delta_secs();
    let elapsed_secs = time.elapsed_secs();

    for (mut path, mut pipeline, modulator) in query.iter_mut() {
        if !pipeline.enabled || pipeline.modifiers.is_empty() {
            continue;
        }

        let modulation_t = modulator.map(|m| m.current_t).unwrap_or(1.0);
        let ctx = ModifierContext {
            delta_secs,
            elapsed_secs,
            modulation_t,
            audio_spectral: None,
        };

        pipeline.apply(&mut path, &ctx);
    }
}

/// Aggregate all active abstract scene paths into a single VectorFrame payload.
/// Broadcasts on change or periodic heartbeat (every ~30 frames) to conserve network bandwidth.
fn broadcast_combined_abstract_scene_paths(
    paths_query: Query<
        (&UniversalPath, Option<&GlobalTransform>, Option<&Transform>),
        With<PathRenderable>,
    >,
    changed_query: Query<
        (),
        (
            With<PathRenderable>,
            Or<(Changed<UniversalPath>, Changed<GlobalTransform>, Changed<Transform>)>,
        ),
    >,
    mut last_path_count: Local<usize>,
    mut frame_counter: ResMut<VectorFrameCounter>,
    mut path_writer: MessageWriter<BroadcastScenePaths>,
) {
    let current_path_count = paths_query.iter().count();
    let count_changed = current_path_count != *last_path_count;
    *last_path_count = current_path_count;

    let has_modifications = !changed_query.is_empty();

    // Broadcast on changes or every 30 frames (~0.5s at 60Hz) as a heartbeat
    let is_heartbeat = frame_counter.0 % 30 == 0;

    if !has_modifications && !count_changed && !is_heartbeat {
        return;
    }

    let mut abstract_paths = Vec::with_capacity(current_path_count);

    for (path, global_transform, local_transform) in paths_query.iter() {
        let position = if let Some(global) = global_transform {
            global.translation()
        } else if let Some(local) = local_transform {
            local.translation
        } else {
            Vec3::ZERO
        };

        abstract_paths.push(AbstractPathData {
            path: path.clone(),
            position,
        });
    }

    frame_counter.0 = frame_counter.0.wrapping_add(1);
    let frame = VectorFrame::new(frame_counter.0, abstract_paths);

    path_writer.write(BroadcastScenePaths { frame });
}
