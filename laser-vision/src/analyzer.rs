use bevy::prelude::Vec2;

use crate::capture::RawFrame;

/// Something an analyzer found in a frame: a laser dot, a person, a marker ...
#[derive(Debug, Clone, PartialEq)]
pub struct Detection {
    /// What was found (`laser`, `person`, `hand` ...); tracks and reactions are keyed by it
    pub label: String,
    /// Centre in normalised camera coordinates: (0, 0) top-left, (1, 1) bottom-right
    pub norm_pos: Vec2,
    /// Width and height in normalised camera coordinates; zero for point-like detections
    pub size: Vec2,
    /// 0..1
    pub confidence: f32,
}

/// Everything the analyzers produced for one frame.
#[derive(Debug, Clone, Default)]
pub struct AnalysisOutput {
    pub detections: Vec<Detection>,
    /// Frame-wide values, 0..1, published to pipelines as `vision.<name>` (`motion`, `brightness` ...)
    pub signals: Vec<(String, f32)>,
}

impl AnalysisOutput {
    pub fn clear(&mut self) {
        self.detections.clear();
        self.signals.clear();
    }

    pub fn detect(&mut self, detection: Detection) {
        self.detections.push(detection);
    }

    pub fn signal(&mut self, name: impl Into<String>, value: f32) {
        self.signals.push((name.into(), value.clamp(0.0, 1.0)));
    }
}

/// One pluggable step of frame analysis: a classic algorithm (spot finder, motion) or a model
/// (object detector). It reads the frame and adds detections and/or signals to `out`.
///
/// Analyzers run on the vision worker thread, never on the Bevy main thread, so they may be slow;
/// the engine always hands them the newest frame.
pub trait VisionAnalyzer: Send + 'static {
    /// Name used in logs
    fn name(&self) -> &str;
    fn analyze(&mut self, frame: &RawFrame, out: &mut AnalysisOutput);
}

/// How much the picture changed since the previous frame, published as signal `motion`
/// (0 = still, 1 = `full_scale` of the mean luminance change or more).
pub struct MotionAnalyzer {
    /// Mean per-pixel luminance change (0..255) that reads as full motion
    pub full_scale: f32,
    /// Only every `stride`-th pixel in each direction is compared
    pub stride: usize,
    previous: Vec<u8>,
}

impl Default for MotionAnalyzer {
    fn default() -> Self {
        Self { full_scale: 24.0, stride: 4, previous: Vec::new() }
    }
}

impl VisionAnalyzer for MotionAnalyzer {
    fn name(&self) -> &str {
        "motion"
    }

    fn analyze(&mut self, frame: &RawFrame, out: &mut AnalysisOutput) {
        let (w, h) = (frame.width as usize, frame.height as usize);
        let stride = self.stride.max(1);
        let luma: Vec<u8> = (0..h)
            .step_by(stride)
            .flat_map(|y| (0..w).step_by(stride).map(move |x| (y * w + x) * 3))
            .filter_map(|i| frame.rgb_data.get(i..i + 3))
            .map(|p| (0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32) as u8)
            .collect();

        if self.previous.len() == luma.len() && !luma.is_empty() {
            let total: u64 = luma.iter().zip(&self.previous).map(|(a, b)| a.abs_diff(*b) as u64).sum();
            let mean = total as f32 / luma.len() as f32;
            out.signal("motion", mean / self.full_scale);
        }
        self.previous = luma;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(value: u8) -> RawFrame {
        RawFrame { timestamp_ms: 0, width: 16, height: 16, rgb_data: vec![value; 16 * 16 * 3] }
    }

    #[test]
    fn motion_is_zero_when_still_and_full_on_a_big_change() {
        let mut motion = MotionAnalyzer::default();
        let mut out = AnalysisOutput::default();

        motion.analyze(&frame(10), &mut out);
        assert!(out.signals.is_empty(), "no previous frame to compare with yet");

        motion.analyze(&frame(10), &mut out);
        assert_eq!(out.signals, vec![("motion".to_string(), 0.0)]);

        out.clear();
        motion.analyze(&frame(200), &mut out);
        assert_eq!(out.signals, vec![("motion".to_string(), 1.0)]);
    }
}
