//! Draws the laser paths the server broadcasts, on the scene, as gizmo lines

use bevy::prelude::*;
use common::network::NetworkMessage;
use common::path::AbstractPathData;
use common::scene::SceneEntity;
use common::state::TerminalState;

use crate::net::ServerMessage;

/// Paths sit just in front of the scene so they are not hidden by it
const PATH_DEPTH: f32 = 0.05;

/// The newest frame of paths from the server
#[derive(Resource, Default)]
struct ScenePaths {
    frame_id: u64,
    paths: Vec<AbstractPathData>,
}

pub struct PathsPlugin;

impl Plugin for PathsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScenePaths>()
            .add_systems(OnEnter(TerminalState::Connecting), |mut paths: ResMut<ScenePaths>| *paths = default())
            .add_systems(Update, (receive_paths, draw_paths).chain());
    }
}

fn receive_paths(mut messages: MessageReader<ServerMessage>, mut current: ResMut<ScenePaths>) {
    for ServerMessage(message) in messages.read() {
        // Frames can arrive out of order; keep the newest
        if let NetworkMessage::BroadcastScenePaths(frame) = message
            && (frame.frame_id > current.frame_id || current.frame_id == 0)
        {
            *current = ScenePaths { frame_id: frame.frame_id, paths: frame.paths.clone() };
        }
    }
}

fn draw_paths(mut gizmos: Gizmos, current: Res<ScenePaths>, scene: Query<&Transform, With<SceneEntity>>) {
    let scene = scene.single().map(Transform::to_matrix).unwrap_or(Mat4::IDENTITY);
    for path_data in &current.paths {
        let to_world = scene * Mat4::from_translation(path_data.position);
        for segment in &path_data.path.segments {
            for pair in segment.points.windows(2) {
                let [from, to] = pair else { continue };
                // Black points are blanked laser moves
                let blank = |p: &common::path::PathPoint| p.r == 0 && p.g == 0 && p.b == 0;
                if blank(from) || blank(to) {
                    continue;
                }
                gizmos.line(
                    to_world.transform_point3(Vec3::new(from.x, from.y, PATH_DEPTH)),
                    to_world.transform_point3(Vec3::new(to.x, to.y, PATH_DEPTH)),
                    Color::srgb_u8(from.r, from.g, from.b),
                );
            }
        }
    }
}
