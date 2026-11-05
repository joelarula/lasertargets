use bevy::prelude::Vec2;

use crate::analyzer::{AnalysisOutput, VisionAnalyzer};
use crate::capture::{FrameSource, RawFrame};
use crate::homography::Homography;
use crate::tracker::{LostTrack, Tracker, TrackerConfig, TrackId};

/// A tracked object in one frame, in camera and projector coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct VisionObject {
    pub track: TrackId,
    /// First frame this object is reported in
    pub appeared: bool,
    pub label: String,
    /// Centre in normalised camera coordinates: (0, 0) top-left, (1, 1) bottom-right
    pub camera_pos: Vec2,
    /// Centre in projector coordinates (`common::scene::projector_ndc`): -1..1, +Y down
    pub galvo_pos: Vec2,
    /// Top-left and bottom-right corners of the object's box in projector coordinates
    pub galvo_bounds: [Vec2; 2],
    pub confidence: f32,
}

/// Result of analysing one camera frame.
#[derive(Debug, Clone)]
pub struct VisionFrame {
    pub timestamp_ms: u64,
    pub objects: Vec<VisionObject>,
    pub lost: Vec<LostTrack>,
    /// Frame-wide analyzer signals, 0..1
    pub signals: Vec<(String, f32)>,
    pub raw_frame: RawFrame,
}

/// Camera space → projector space for an uncalibrated rig: the camera sees exactly what the
/// projector covers, same orientation. Both spaces have +Y down.
fn default_homography() -> Homography {
    let camera = [Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(1.0, 1.0), Vec2::new(0.0, 1.0)];
    let galvo = [Vec2::new(-1.0, -1.0), Vec2::new(1.0, -1.0), Vec2::new(1.0, 1.0), Vec2::new(-1.0, 1.0)];
    Homography::from_four_points(&camera, &galvo).unwrap_or_else(Homography::identity)
}

/// Frame intake → analyzers → tracking → projector coordinates.
pub struct VisionEngine {
    frame_source: Box<dyn FrameSource>,
    analyzers: Vec<Box<dyn VisionAnalyzer>>,
    tracker: Tracker,
    homography: Homography,
    calibration_quad: Option<[Vec2; 4]>,
    scratch: AnalysisOutput,
}

impl VisionEngine {
    pub fn new(frame_source: Box<dyn FrameSource>, analyzers: Vec<Box<dyn VisionAnalyzer>>) -> Self {
        Self {
            frame_source,
            analyzers,
            tracker: Tracker::default(),
            homography: default_homography(),
            calibration_quad: None,
            scratch: AnalysisOutput::default(),
        }
    }

    pub fn with_tracker(mut self, config: TrackerConfig) -> Self {
        self.tracker = Tracker::new(config);
        self
    }

    pub fn add_analyzer(&mut self, analyzer: Box<dyn VisionAnalyzer>) {
        self.analyzers.push(analyzer);
    }

    pub fn homography(&self) -> &Homography {
        &self.homography
    }

    /// Camera corners of the calibration quad, if calibrated
    pub fn calibration_quad(&self) -> Option<[Vec2; 4]> {
        self.calibration_quad
    }

    /// Maps 4 normalised camera points onto 4 projector points, ordered top-left, top-right,
    /// bottom-right, bottom-left. Returns false (and keeps the old mapping) for a degenerate quad.
    pub fn set_calibration_quad(&mut self, camera_pts: [Vec2; 4], galvo_pts: [Vec2; 4]) -> bool {
        match Homography::from_four_points(&camera_pts, &galvo_pts) {
            Some(h) => {
                self.homography = h;
                self.calibration_quad = Some(camera_pts);
                true
            }
            None => false,
        }
    }

    pub fn reset_calibration(&mut self) {
        self.homography = default_homography();
        self.calibration_quad = None;
    }

    /// Takes the next camera frame and runs every analyzer and the tracker over it. `None` when
    /// the source has no frame right now.
    pub fn process_next_frame(&mut self) -> Option<VisionFrame> {
        let raw = self.frame_source.next_frame()?;

        self.scratch.clear();
        for analyzer in &mut self.analyzers {
            analyzer.analyze(&raw, &mut self.scratch);
        }

        let update = self.tracker.update(&self.scratch.detections);
        let objects = update
            .seen
            .into_iter()
            .map(|seen| {
                let d = seen.detection;
                let half = d.size * 0.5;
                VisionObject {
                    track: seen.track,
                    appeared: seen.appeared,
                    galvo_pos: self.homography.transform_point(d.norm_pos),
                    galvo_bounds: [
                        self.homography.transform_point(d.norm_pos - half),
                        self.homography.transform_point(d.norm_pos + half),
                    ],
                    camera_pos: d.norm_pos,
                    label: d.label,
                    confidence: d.confidence,
                }
            })
            .collect();

        Some(VisionFrame {
            timestamp_ms: raw.timestamp_ms,
            objects,
            lost: update.lost,
            signals: std::mem::take(&mut self.scratch.signals),
            raw_frame: raw,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::MotionAnalyzer;
    use crate::capture::MockFrameSource;
    use crate::detector::SpotAnalyzer;

    fn engine() -> VisionEngine {
        let analyzers: Vec<Box<dyn VisionAnalyzer>> = vec![Box::new(SpotAnalyzer::default()), Box::new(MotionAnalyzer::default())];
        VisionEngine::new(Box::new(MockFrameSource::new(320, 240, 240)), analyzers)
    }

    #[test]
    fn mock_dot_becomes_one_track_with_signals() {
        let mut engine = engine();
        let frames: Vec<VisionFrame> = (0..4).filter_map(|_| engine.process_next_frame()).collect();

        let objects: Vec<&VisionObject> = frames.iter().flat_map(|f| &f.objects).collect();
        assert!(!objects.is_empty(), "the mock laser dot is tracked");
        assert!(objects.iter().all(|o| o.track == objects[0].track && o.label == "laser"));
        assert_eq!(objects.iter().filter(|o| o.appeared).count(), 1, "appears exactly once");
        assert!(frames.iter().skip(1).all(|f| f.signals.iter().any(|(name, _)| name == "motion")));
    }

    #[test]
    fn default_mapping_matches_projector_orientation() {
        let h = engine().homography;
        // Camera top-left is the projector's top-left, which is -1, -1 with +Y down
        assert!(h.transform_point(Vec2::new(0.0, 0.0)).distance(Vec2::new(-1.0, -1.0)) < 1e-4);
        assert!(h.transform_point(Vec2::new(1.0, 1.0)).distance(Vec2::new(1.0, 1.0)) < 1e-4);
        assert!(h.transform_point(Vec2::new(0.5, 0.5)).length() < 1e-4);
    }

    #[test]
    fn calibration_quad_replaces_the_mapping() {
        let mut engine = engine();
        let camera = [Vec2::new(0.1, 0.1), Vec2::new(0.9, 0.1), Vec2::new(0.9, 0.9), Vec2::new(0.1, 0.9)];
        let galvo = [Vec2::new(-0.8, -0.8), Vec2::new(0.8, -0.8), Vec2::new(0.8, 0.8), Vec2::new(-0.8, 0.8)];

        assert!(engine.set_calibration_quad(camera, galvo));
        assert_eq!(engine.calibration_quad(), Some(camera));
        assert!(engine.homography().transform_point(Vec2::new(0.1, 0.1)).distance(Vec2::new(-0.8, -0.8)) < 1e-3);

        engine.reset_calibration();
        assert_eq!(engine.calibration_quad(), None);
    }
}
