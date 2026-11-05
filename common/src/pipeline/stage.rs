use super::signals::SignalView;
use crate::path::UniversalPath;

/// Whether a stage's output depends only on its input and parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageKind {
    /// Cached; rerun only when the input or the stage changes
    Static,
    /// Rerun every frame (reads time or signals)
    Dynamic,
}

/// What a stage or source may read besides its input.
pub struct StageCtx<'a> {
    pub delta_secs: f32,
    pub elapsed_secs: f32,
    pub signals: SignalView<'a>,
}

/// One pluggable step of a pipeline.
///
/// Pure: reads `input`, writes the complete result into `out`. `out` holds an older result whose
/// allocations can be reused (`out.clone_from(input)` then modify is the usual pattern), so stages
/// do not allocate in steady state. Stages never touch the ECS world.
pub trait PathStage: Send + Sync + 'static {
    /// Report `Dynamic` when any parameter is bound to a signal or the stage reads time
    fn kind(&self) -> StageKind;
    fn apply(&mut self, input: &UniversalPath, ctx: &StageCtx, out: &mut UniversalPath);
}

/// Generator of a pipeline's base geometry (shape, fill, text, morph).
pub trait PathSource: Send + Sync + 'static {
    fn kind(&self) -> StageKind;
    /// Writes the complete base path into `out`, reusing its allocations
    fn generate(&mut self, ctx: &StageCtx, out: &mut UniversalPath);
}

/// Applies `f` to every point of `out` after copying `input` into it.
pub fn map_points(input: &UniversalPath, out: &mut UniversalPath, mut f: impl FnMut(&mut crate::path::PathPoint)) {
    out.clone_from(input);
    out.segments.iter_mut().flat_map(|s| s.points.iter_mut()).for_each(|p| f(p));
}
