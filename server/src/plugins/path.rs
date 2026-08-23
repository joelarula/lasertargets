use bevy::prelude::*;
use common::path::{
    AbstractPathData, BroadcastScenePaths, LoopMode, PathAnimation, PathModulator, PathPoint,
    PathRenderable, UniversalPath, VectorFrame,
};
use common::scene::SceneEntity;

#[derive(Resource, Default)]
pub struct VectorFrameCounter(pub u64);

pub struct PathNetworkPlugin;

impl Plugin for PathNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VectorFrameCounter>()
            .add_message::<BroadcastScenePaths>()
            .add_systems(Update, (animate_vector_paths, update_path_modulators))
            .add_systems(PostUpdate, broadcast_combined_abstract_scene_paths);
    }
}

/// Animate UniversalPath components based on active PathAnimation keyframe timing
fn animate_vector_paths(
    time: Res<Time>,
    mut query: Query<(&mut UniversalPath, &mut PathAnimation)>,
) {
    let delta = time.delta_secs();

    for (mut path, mut anim) in query.iter_mut() {
        anim.elapsed_secs += delta;

        let raw_t = (anim.elapsed_secs / anim.duration_secs).max(0.0);
        let normalized_t = match anim.loop_mode {
            LoopMode::Once => raw_t.min(1.0),
            LoopMode::Repeat => raw_t.fract(),
            LoopMode::PingPong => {
                let cycle = (raw_t as u32) % 2;
                let rem = raw_t.fract();
                if cycle == 0 { rem } else { 1.0 - rem }
            }
        };

        let eased_t = anim.easing.apply(normalized_t);

        // Interpolate point-by-point between keyframe_a and keyframe_b segments
        if !anim.keyframe_a.segments.is_empty() && !anim.keyframe_b.segments.is_empty() {
            let seg_a = &anim.keyframe_a.segments[0];
            let seg_b = &anim.keyframe_b.segments[0];

            if !seg_a.points.is_empty() && !seg_b.points.is_empty() {
                let max_len = seg_a.points.len().max(seg_b.points.len());
                let mut interpolated_points = Vec::with_capacity(max_len);

                for i in 0..max_len {
                    let pa = &seg_a.points[i % seg_a.points.len()];
                    let pb = &seg_b.points[i % seg_b.points.len()];

                    let x = pa.x + (pb.x - pa.x) * eased_t;
                    let y = pa.y + (pb.y - pa.y) * eased_t;
                    let r = (pa.r as f32 + (pb.r as f32 - pa.r as f32) * eased_t) as u8;
                    let g = (pa.g as f32 + (pb.g as f32 - pa.g as f32) * eased_t) as u8;
                    let b = (pa.b as f32 + (pb.b as f32 - pa.b as f32) * eased_t) as u8;

                    interpolated_points.push(PathPoint::new(x, y, r, g, b, pa.dwell));
                }

                path.segments[0].points = interpolated_points;
            }
        }
    }
}

/// System evaluating PathModulators (LFO, Trigger, Ratio, Manual) through EasingFunction to drive UniversalPath
fn update_path_modulators(
    time: Res<Time>,
    mut query: Query<(&mut UniversalPath, &mut PathModulator)>,
) {
    let elapsed = time.elapsed_secs();

    for (mut path, mut modulator) in query.iter_mut() {
        let raw_t = modulator.source.sample(elapsed);
        let eased_t = modulator.easing.apply(raw_t);
        modulator.current_t = eased_t;

        if !modulator.keyframe_a.segments.is_empty() && !modulator.keyframe_b.segments.is_empty() {
            let seg_a = &modulator.keyframe_a.segments[0];
            let seg_b = &modulator.keyframe_b.segments[0];

            if !seg_a.points.is_empty() && !seg_b.points.is_empty() {
                let max_len = seg_a.points.len().max(seg_b.points.len());
                let mut interpolated_points = Vec::with_capacity(max_len);

                for i in 0..max_len {
                    let pa = &seg_a.points[i % seg_a.points.len()];
                    let pb = &seg_b.points[i % seg_b.points.len()];

                    let x = pa.x + (pb.x - pa.x) * eased_t;
                    let y = pa.y + (pb.y - pa.y) * eased_t;
                    let r = (pa.r as f32 + (pb.r as f32 - pa.r as f32) * eased_t) as u8;
                    let g = (pa.g as f32 + (pb.g as f32 - pa.g as f32) * eased_t) as u8;
                    let b = (pa.b as f32 + (pb.b as f32 - pa.b as f32) * eased_t) as u8;

                    interpolated_points.push(PathPoint::new(x, y, r, g, b, pa.dwell));
                }

                if !path.segments.is_empty() {
                    path.segments[0].points = interpolated_points;
                }
            }
        }
    }
}

/// Aggregate all active abstract scene paths into a single VectorFrame payload every tick
fn broadcast_combined_abstract_scene_paths(
    paths_query: Query<
        (&UniversalPath, &Transform, Option<&ChildOf>),
        With<PathRenderable>,
    >,
    scene_query: Query<&Transform, With<SceneEntity>>,
    mut frame_counter: ResMut<VectorFrameCounter>,
    mut path_writer: MessageWriter<BroadcastScenePaths>,
) {
    let scene_transform = scene_query.single().ok();
    let mut abstract_paths = Vec::new();

    for (path, transform, parent) in paths_query.iter() {
        let position = if let Some(parent) = parent {
            if let Ok(p_transform) = scene_query.get(parent.parent()) {
                p_transform.transform_point(transform.translation)
            } else {
                transform.translation
            }
        } else if let Some(scene_transform) = scene_transform {
            scene_transform.transform_point(transform.translation)
        } else {
            transform.translation
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
