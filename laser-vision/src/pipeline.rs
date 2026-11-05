use bevy::prelude::Vec2;
use common::camera::{VisualEventKind, VisualEventMarker};

use crate::capture::{FrameSource, RawFrame};
use crate::detector::{detect_laser_spots, DetectionConfig, DetectedSpot};
use crate::homography::Homography;

/// Result of processing a single camera frame through detection and optical mapping
#[derive(Debug, Clone)]
pub struct VisionFrameResult {
    /// Original raw pixel frame
    pub raw_frame: RawFrame,
    /// Spots detected in camera coordinate space
    pub detected_spots: Vec<DetectedSpot>,
    /// High-level visual events mapped into galvo projector space
    pub visual_events: Vec<VisualEventMarker>,
    /// Frame capture timestamp in milliseconds
    pub timestamp_ms: u64,
}

/// Core computer vision engine unifying frame capture, spectral detection, and homography projection
pub struct VisionEngine {
    frame_source: Box<dyn FrameSource>,
    detection_config: DetectionConfig,
    homography: Homography,
    calibration_quad: Option<[Vec2; 4]>,
}

impl VisionEngine {
    /// Creates a new VisionEngine with the provided frame source and default configuration
    pub fn new(frame_source: Box<dyn FrameSource>) -> Self {
        Self::with_config(frame_source, DetectionConfig::default())
    }

    /// Creates a new VisionEngine with custom detection configuration
    pub fn with_config(frame_source: Box<dyn FrameSource>, config: DetectionConfig) -> Self {
        // Default calibration: camera normalized sensor space [0.0, 1.0] -> laser galvo space [-1.0, 1.0]
        let default_src = [
            Vec2::new(0.0, 0.0), // Top-Left
            Vec2::new(1.0, 0.0), // Top-Right
            Vec2::new(1.0, 1.0), // Bottom-Right
            Vec2::new(0.0, 1.0), // Bottom-Left
        ];
        let default_dst = [
            Vec2::new(-1.0, 1.0),  // Top-Left
            Vec2::new(1.0, 1.0),   // Top-Right
            Vec2::new(1.0, -1.0),  // Bottom-Right
            Vec2::new(-1.0, -1.0), // Bottom-Left
        ];

        let homography = Homography::from_four_points(&default_src, &default_dst)
            .unwrap_or_else(Homography::identity);

        Self {
            frame_source,
            detection_config: config,
            homography,
            calibration_quad: None,
        }
    }

    /// Updates the optical detection parameters (threshold, color, area filters)
    pub fn set_detection_config(&mut self, config: DetectionConfig) {
        self.detection_config = config;
    }

    /// Returns a reference to the active detection configuration
    pub fn detection_config(&self) -> &DetectionConfig {
        &self.detection_config
    }

    /// Sets an explicit 3x3 homography matrix
    pub fn set_homography(&mut self, homography: Homography) {
        self.homography = homography;
    }

    /// Returns the active homography matrix
    pub fn homography(&self) -> &Homography {
        &self.homography
    }

    /// Returns the camera normalized coordinates of the 4 calibration anchor corners, if calibrated
    pub fn calibration_quad(&self) -> Option<[Vec2; 4]> {
        self.calibration_quad
    }

    /// Calibrates the projection mapping from 4 camera normalized anchor points to 4 laser galvo points.
    ///
    /// Anchor points should be ordered: [Top-Left, Top-Right, Bottom-Right, Bottom-Left].
    /// Returns true if the homography was successfully computed and updated.
    pub fn set_calibration_quad(&mut self, camera_pts: [Vec2; 4], galvo_pts: [Vec2; 4]) -> bool {
        if let Some(h) = Homography::from_four_points(&camera_pts, &galvo_pts) {
            self.homography = h;
            self.calibration_quad = Some(camera_pts);
            true
        } else {
            false
        }
    }

    /// Resets the calibration homography back to the default [0.0, 1.0] -> [-1.0, 1.0] mapping
    pub fn reset_homography(&mut self) {
        let default_src = [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];
        let default_dst = [
            Vec2::new(-1.0, 1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(-1.0, -1.0),
        ];
        self.homography = Homography::from_four_points(&default_src, &default_dst)
            .unwrap_or_else(Homography::identity);
        self.calibration_quad = None;
    }

    /// Acquires the next frame from the camera, runs optical detection, and projects detected spots into galvo coordinates
    pub fn process_next_frame(&mut self) -> Option<VisionFrameResult> {
        let raw = self.frame_source.next_frame()?;
        let spots = detect_laser_spots(raw.width, raw.height, &raw.rgb_data, &self.detection_config);

        let visual_events = spots
            .iter()
            .map(|spot| {
                let galvo_pos = self.homography.transform_point(spot.norm_pos);
                VisualEventMarker {
                    camera_norm_pos: spot.norm_pos,
                    galvo_pos,
                    kind: VisualEventKind::PlayerLaserHit,
                    confidence: spot.peak_intensity,
                    timestamp_ms: raw.timestamp_ms,
                }
            })
            .collect();

        Some(VisionFrameResult {
            timestamp_ms: raw.timestamp_ms,
            detected_spots: spots,
            visual_events,
            raw_frame: raw,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::MockFrameSource;

    #[test]
    fn test_pipeline_with_mock_source() {
        let mock = Box::new(MockFrameSource::new(320, 240, 60));
        let mut engine = VisionEngine::new(mock);

        let result = engine.process_next_frame();
        assert!(result.is_some(), "Expected a frame from mock source");

        let frame = result.unwrap();
        assert_eq!(frame.raw_frame.width, 320);
        assert_eq!(frame.raw_frame.height, 240);
        // The mock frame source renders a simulated green laser spot, so at least 1 spot should be detected
        assert!(
            !frame.detected_spots.is_empty(),
            "Expected detected spots from mock laser source"
        );
        assert_eq!(frame.detected_spots.len(), frame.visual_events.len());

        let event = &frame.visual_events[0];
        // Galvo space must be bounded in or around [-1.0, 1.0]
        assert!(event.galvo_pos.x >= -1.2 && event.galvo_pos.x <= 1.2);
        assert!(event.galvo_pos.y >= -1.2 && event.galvo_pos.y <= 1.2);
        assert_eq!(event.kind, VisualEventKind::PlayerLaserHit);
    }

    #[test]
    fn test_pipeline_calibration_quad_update() {
        let mock = Box::new(MockFrameSource::new(320, 240, 60));
        let mut engine = VisionEngine::new(mock);

        let custom_cam = [
            Vec2::new(0.1, 0.1),
            Vec2::new(0.9, 0.1),
            Vec2::new(0.9, 0.9),
            Vec2::new(0.1, 0.9),
        ];
        let custom_galvo = [
            Vec2::new(-0.8, 0.8),
            Vec2::new(0.8, 0.8),
            Vec2::new(0.8, -0.8),
            Vec2::new(-0.8, -0.8),
        ];

        let ok = engine.set_calibration_quad(custom_cam, custom_galvo);
        assert!(ok);
        assert_eq!(engine.calibration_quad(), Some(custom_cam));

        // Center (0.5, 0.5) should map to approx (0.0, 0.0) in galvo space
        let mapped_center = engine.homography().transform_point(Vec2::new(0.5, 0.5));
        assert!(mapped_center.length() < 1e-3);
    }
}

