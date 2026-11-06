use bevy::prelude::*;
use common::path::{PathRenderable, UniversalPath};

use crate::rules::{Grid, SnakeState};
use crate::types::{GEM_HALF_SIZE, SEGMENT_RADIUS};

/// Everything the snake game spawns; removed together when the game stops
#[derive(Component, Debug, Clone, Default)]
pub struct SnakeEntity;

/// A drawn piece of the game. Its base shape and position follow [`SnakeState`]; its pipeline
/// ([`SnakePart::pipeline`]) turns that into what the laser draws.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
#[require(SnakeEntity, Transform, Visibility, PathRenderable)]
pub enum SnakePart {
    /// Circle on the head cell
    Head,
    /// Polyline through every cell, split where it wraps around an edge
    Body,
    /// Diamond on the gem cell
    Gem,
}

impl SnakePart {
    pub const ALL: [SnakePart; 3] = [SnakePart::Head, SnakePart::Body, SnakePart::Gem];

    pub fn pipeline(self) -> &'static str {
        match self {
            Self::Head => "snake/head",
            Self::Body => "snake/body",
            Self::Gem => "snake/gem",
        }
    }

    /// Base shape (entity-local) and entity position for the current state
    pub fn shape(self, state: &SnakeState) -> (UniversalPath, Vec3) {
        match self {
            Self::Head => (
                UniversalPath::circle(Vec2::ZERO, SEGMENT_RADIUS, state.head_color),
                state.grid.to_local(state.head()),
            ),
            Self::Body => (body_path(state), Vec3::ZERO),
            Self::Gem => match state.gem {
                Some(gem) => (
                    UniversalPath::diamond(Vec2::ZERO, GEM_HALF_SIZE, gem.color),
                    state.grid.to_local(gem.cell),
                ),
                None => (UniversalPath::new(), Vec3::ZERO),
            },
        }
    }
}

/// One point per body cell. Where the body wraps around an edge a new sub-path starts, so the
/// laser doesn't draw a line across the field.
fn body_path(state: &SnakeState) -> UniversalPath {
    let mut path = UniversalPath::new();
    let mut segment = common::path::PathSegment::empty();
    let mut previous: Option<IVec2> = None;
    for &cell in &state.body {
        if previous.is_some_and(|p| Grid::wraps_between(p, cell)) && !segment.points.is_empty() {
            path.add_segment(std::mem::replace(
                &mut segment,
                common::path::PathSegment::empty(),
            ));
        }
        let at = state.grid.to_local(cell);
        segment.push(at.x, at.y, state.body_color, 1);
        previous = Some(cell);
    }
    if !segment.points.is_empty() {
        path.add_segment(segment);
    }
    path
}
