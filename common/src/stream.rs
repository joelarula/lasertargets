use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::path::{PathPoint, PathSegment, UniversalPath};

/// Windowing function applied to the spatial coordinate s in [0, 1] to prevent
/// laser clipping, tearing, or abrupt endpoint jumps.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum WindowFunction {
    #[default]
    Hann,
    Sine,
    Tukey { alpha: f32 },
    Triangle,
    None,
}

impl WindowFunction {
    /// Evaluates window envelope value at parametric coordinate s in [0.0, 1.0].
    pub fn evaluate(&self, s: f32) -> f32 {
        let s_clamped = s.clamp(0.0, 1.0);
        match self {
            Self::Hann => {
                // 0.5 * (1 - cos(2*pi*s)) -> 0 at ends, 1 in center
                0.5 * (1.0 - (s_clamped * std::f32::consts::TAU).cos())
            }
            Self::Sine => {
                (s_clamped * std::f32::consts::PI).sin()
            }
            Self::Tukey { alpha } => {
                let a = alpha.clamp(0.001, 1.0);
                if s_clamped < a / 2.0 {
                    0.5 * (1.0 + (std::f32::consts::PI * (2.0 * s_clamped / a - 1.0)).cos())
                } else if s_clamped <= 1.0 - a / 2.0 {
                    1.0
                } else {
                    0.5 * (1.0 + (std::f32::consts::PI * (2.0 * (1.0 - s_clamped) / a - 1.0)).cos())
                }
            }
            Self::Triangle => {
                1.0 - (2.0 * s_clamped - 1.0).abs()
            }
            Self::None => 1.0,
        }
    }
}

/// Base carrier curve geometry along which displacement modulation occurs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CarrierGeometry {
    /// Straight line between start and end coordinates.
    Line {
        start: Vec2,
        end: Vec2,
    },
    /// Circle with given center and radius (closed loop).
    Circle {
        center: Vec2,
        radius: f32,
    },
    /// Circular arc between start_angle_rad and end_angle_rad.
    Arc {
        center: Vec2,
        radius: f32,
        start_angle_rad: f32,
        end_angle_rad: f32,
    },
    /// Custom polyline carrier points.
    Polyline {
        points: Vec<Vec2>,
        closed: bool,
    },
}

impl Default for CarrierGeometry {
    fn default() -> Self {
        Self::Line {
            start: Vec2::new(-0.8, 0.0),
            end: Vec2::new(0.8, 0.0),
        }
    }
}

impl CarrierGeometry {
    /// Evaluates base point B(s) and unit normal N(s) at coordinate s in [0.0, 1.0].
    pub fn evaluate_at(&self, s: f32) -> (Vec2, Vec2) {
        let s = s.clamp(0.0, 1.0);
        match self {
            Self::Line { start, end } => {
                let diff = *end - *start;
                let len = diff.length().max(1e-6);
                let tangent = diff / len;
                let normal = Vec2::new(-tangent.y, tangent.x);
                let pos = *start + diff * s;
                (pos, normal)
            }
            Self::Circle { center, radius } => {
                let angle = s * std::f32::consts::TAU;
                let cos = angle.cos();
                let sin = angle.sin();
                let normal = Vec2::new(cos, sin);
                let pos = *center + normal * *radius;
                (pos, normal)
            }
            Self::Arc {
                center,
                radius,
                start_angle_rad,
                end_angle_rad,
            } => {
                let angle = start_angle_rad + (end_angle_rad - start_angle_rad) * s;
                let normal = Vec2::new(angle.cos(), angle.sin());
                let pos = *center + normal * *radius;
                (pos, normal)
            }
            Self::Polyline { points, closed } => {
                if points.is_empty() {
                    return (Vec2::ZERO, Vec2::Y);
                }
                if points.len() == 1 {
                    return (points[0], Vec2::Y);
                }

                let num_segments = if *closed {
                    points.len()
                } else {
                    points.len() - 1
                };

                let scaled_s = s * num_segments as f32;
                let seg_idx = (scaled_s.floor() as usize).min(num_segments - 1);
                let local_t = scaled_s - seg_idx as f32;

                let p0 = points[seg_idx];
                let p1 = points[(seg_idx + 1) % points.len()];

                let diff = p1 - p0;
                let len = diff.length().max(1e-6);
                let tangent = diff / len;
                let normal = Vec2::new(-tangent.y, tangent.x);
                let pos = p0 + diff * local_t;
                (pos, normal)
            }
        }
    }

    /// Whether this carrier forms a closed loop.
    pub fn is_closed(&self) -> bool {
        match self {
            Self::Circle { .. } => true,
            Self::Polyline { closed, .. } => *closed,
            _ => false,
        }
    }
}

/// Instantaneous or historical audio/signal data stream snapshot.
#[derive(Clone, Debug, Serialize, Deserialize, Resource)]
pub struct AudioStreamData {
    /// Low frequency band envelope (e.g. 20-250 Hz, bass/kick) [0.0, 1.0]
    pub low: f32,
    /// Mid frequency band envelope (e.g. 250-2500 Hz, lead/vocals) [0.0, 1.0]
    pub mid: f32,
    /// High frequency band envelope (e.g. 2.5k-16k Hz, hi-hats/cymbals) [0.0, 1.0]
    pub high: f32,
    /// Overall RMS or root energy [0.0, 1.0]
    pub rms: f32,
    /// Raw sliding-window waveform or spectrum bins
    pub waveform_buffer: Vec<f32>,
}

impl Default for AudioStreamData {
    fn default() -> Self {
        Self {
            low: 0.0,
            mid: 0.0,
            high: 0.0,
            rms: 0.0,
            waveform_buffer: Vec::new(),
        }
    }
}

impl AudioStreamData {
    /// Samples the waveform or spectrum buffer at relative coordinate s in [0.0, 1.0].
    pub fn sample_waveform(&self, s: f32) -> f32 {
        if self.waveform_buffer.is_empty() {
            return 0.0;
        }
        let s_clamped = s.clamp(0.0, 1.0);
        let max_idx = self.waveform_buffer.len() - 1;
        let index_f = s_clamped * max_idx as f32;
        let idx0 = index_f.floor() as usize;
        let idx1 = (idx0 + 1).min(max_idx);
        let frac = index_f - idx0 as f32;

        self.waveform_buffer[idx0] * (1.0 - frac) + self.waveform_buffer[idx1] * frac
    }
}

/// Parameters for mapping multi-band signal values to 2D spatial displacements.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamModulationConfig {
    /// Total number of sampled points along the carrier path.
    pub point_count: usize,
    /// Master displacement scale multiplier.
    pub amplitude_scale: f32,
    /// Low band spatial frequency (cycles along the line) and amplitude gain.
    pub low_spatial_freq: f32,
    pub low_gain: f32,
    /// Mid band spatial frequency (cycles along the line) and amplitude gain.
    pub mid_spatial_freq: f32,
    pub mid_gain: f32,
    /// High band spatial frequency (cycles along the line) and amplitude gain.
    pub high_spatial_freq: f32,
    pub high_gain: f32,
    /// Gain applied to direct waveform / FFT buffer samples.
    pub waveform_gain: f32,
    /// Traveling phase speed (radians per second).
    pub phase_speed: f32,
    /// Base laser color (RGB).
    pub base_color: Color,
    /// Optional dynamic color tint from highs/lows.
    pub dynamic_color: bool,
}

impl Default for StreamModulationConfig {
    fn default() -> Self {
        Self {
            point_count: 64,
            amplitude_scale: 0.25,
            low_spatial_freq: 2.0,
            low_gain: 1.0,
            mid_spatial_freq: 6.0,
            mid_gain: 0.8,
            high_spatial_freq: 18.0,
            high_gain: 0.5,
            waveform_gain: 1.0,
            phase_speed: 4.0,
            base_color: Color::srgb(0.0, 1.0, 0.8),
            dynamic_color: true,
        }
    }
}

/// Bevy Component attached to entities that generate a continuous modulated stream polyline.
#[derive(Component, Clone, Debug, Serialize, Deserialize)]
pub struct StreamPolylineModulator {
    pub carrier: CarrierGeometry,
    pub window: WindowFunction,
    pub config: StreamModulationConfig,
    /// Internal accumulated phase over time.
    pub phase: f32,
}

impl Default for StreamPolylineModulator {
    fn default() -> Self {
        Self {
            carrier: CarrierGeometry::default(),
            window: WindowFunction::default(),
            config: StreamModulationConfig::default(),
            phase: 0.0,
        }
    }
}

impl StreamPolylineModulator {
    /// Generates a `UniversalPath` from the current carrier, modulation configuration,
    /// accumulated phase, and input audio/signal stream.
    pub fn generate_path(&self, stream: &AudioStreamData) -> UniversalPath {
        let count = self.config.point_count.max(2);
        let mut path_points = Vec::with_capacity(count + 4);
        let is_closed = self.carrier.is_closed();

        let (base_r, base_g, base_b) = {
            let srgb = self.config.base_color.to_srgba();
            (
                (srgb.red * 255.0) as u8,
                (srgb.green * 255.0) as u8,
                (srgb.blue * 255.0) as u8,
            )
        };

        for i in 0..count {
            let s = if is_closed {
                i as f32 / count as f32
            } else {
                i as f32 / (count - 1) as f32
            };

            let (base_pos, normal) = self.carrier.evaluate_at(s);

            // Calculate spatial harmonics
            let low_component = (s * self.config.low_spatial_freq * std::f32::consts::TAU + self.phase).sin()
                * stream.low
                * self.config.low_gain;

            let mid_component = (s * self.config.mid_spatial_freq * std::f32::consts::TAU - self.phase * 1.5).sin()
                * stream.mid
                * self.config.mid_gain;

            let high_component = (s * self.config.high_spatial_freq * std::f32::consts::TAU + self.phase * 3.0).sin()
                * stream.high
                * self.config.high_gain;

            let wave_component = stream.sample_waveform(s) * self.config.waveform_gain;

            let total_mod = low_component + mid_component + high_component + wave_component;
            let window_val = self.window.evaluate(s);

            let displacement = total_mod * self.config.amplitude_scale * window_val;
            let modulated_pos = base_pos + normal * displacement;

            let (r, g, b) = if self.config.dynamic_color {
                // Modulate laser color based on spectral content
                let r_tint = ((base_r as f32 + stream.low * 150.0).min(255.0)) as u8;
                let g_tint = ((base_g as f32 + stream.mid * 100.0).min(255.0)) as u8;
                let b_tint = ((base_b as f32 + stream.high * 200.0).min(255.0)) as u8;
                (r_tint, g_tint, b_tint)
            } else {
                (base_r, base_g, base_b)
            };

            // Add dwells at ends if open line, or smooth continuous points
            let dwell = if !is_closed && (i == 0 || i == count - 1) { 3 } else { 0 };
            path_points.push(PathPoint::new(modulated_pos.x, modulated_pos.y, r, g, b, dwell));
        }

        // Close loop if carrier is closed
        if is_closed && !path_points.is_empty() {
            let first = path_points[0].clone();
            path_points.push(PathPoint::new(first.x, first.y, first.r, first.g, first.b, 0));
        }

        UniversalPath::from_segment(PathSegment::new(path_points))
    }
}

/// Bevy system to tick and generate streaming polylines every frame.
pub fn update_streaming_polylines(
    time: Res<Time>,
    audio_stream: Option<Res<AudioStreamData>>,
    mut query: Query<(&mut StreamPolylineModulator, &mut UniversalPath)>,
) {
    let delta_secs = time.delta_secs();
    let default_stream = AudioStreamData::default();
    let stream = audio_stream.as_deref().unwrap_or(&default_stream);

    for (mut modulator, mut path) in query.iter_mut() {
        modulator.phase = (modulator.phase + modulator.config.phase_speed * delta_secs) % std::f32::consts::TAU;
        *path = modulator.generate_path(stream);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_function_hann() {
        let window = WindowFunction::Hann;
        assert!(window.evaluate(0.0).abs() < 1e-5);
        assert!((window.evaluate(0.5) - 1.0).abs() < 1e-5);
        assert!(window.evaluate(1.0).abs() < 1e-5);
    }

    #[test]
    fn test_carrier_geometry_line() {
        let carrier = CarrierGeometry::Line {
            start: Vec2::new(-1.0, 0.0),
            end: Vec2::new(1.0, 0.0),
        };
        let (pos, normal) = carrier.evaluate_at(0.5);
        assert_eq!(pos, Vec2::new(0.0, 0.0));
        assert_eq!(normal, Vec2::new(0.0, 1.0));
    }

    #[test]
    fn test_stream_polyline_generation() {
        let modulator = StreamPolylineModulator::default();
        let mut stream = AudioStreamData::default();
        stream.low = 0.8;
        stream.mid = 0.5;
        stream.high = 0.3;

        let path = modulator.generate_path(&stream);
        assert_eq!(path.segments.len(), 1);
        assert!(path.segments[0].points.len() >= 64);
    }
}
