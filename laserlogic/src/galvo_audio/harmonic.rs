use crate::{LaserPoint, LaserSegment};
use std::f32::consts::PI;

/// Musical interval offsets for tuning geometric laser targets into harmonious chords.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicalInterval {
    Unison,
    MinorThird,
    MajorThird,
    PerfectFourth,
    PerfectFifth,
    MajorSixth,
    Octave,
}

impl MusicalInterval {
    /// Frequency multiplier for this interval relative to root pitch.
    pub fn frequency_ratio(self) -> f32 {
        match self {
            MusicalInterval::Unison => 1.0,
            MusicalInterval::MinorThird => 1.20,      // 6/5
            MusicalInterval::MajorThird => 1.25,      // 5/4
            MusicalInterval::PerfectFourth => 1.3333,  // 4/3
            MusicalInterval::PerfectFifth => 1.50,     // 3/2
            MusicalInterval::MajorSixth => 1.6667,    // 5/3
            MusicalInterval::Octave => 2.0,           // 2/1
        }
    }
}

/// Calculate the number of points required for a closed loop to scan at a specific acoustic pitch.
pub fn points_for_pitch(pitch_hz: f32, sample_rate_pps: u32) -> usize {
    if pitch_hz <= 0.0 || sample_rate_pps == 0 {
        return 50;
    }
    let count = (sample_rate_pps as f32 / pitch_hz).round() as usize;
    count.clamp(4, 2048)
}

/// Calculate the natural acoustic fundamental pitch emitted when a loop with `point_count` points is scanned.
pub fn pitch_for_points(point_count: usize, sample_rate_pps: u32) -> f32 {
    if point_count == 0 {
        return 0.0;
    }
    sample_rate_pps as f32 / point_count as f32
}

/// Resample a closed loop of `LaserPoint`s along its perimeter to have exactly `target_point_count` points.
/// This forces the scanner to mechanically repeat the shape at precisely the desired acoustic frequency.
pub fn resample_closed_loop(points: &[LaserPoint], target_point_count: usize) -> Vec<LaserPoint> {
    if points.is_empty() || target_point_count == 0 {
        return Vec::new();
    }
    if points.len() == 1 || target_point_count == 1 {
        return vec![points[0]; target_point_count];
    }

    // Compute cumulative segment perimeter distances
    let mut cum_dist = Vec::with_capacity(points.len() + 1);
    cum_dist.push(0.0_f32);
    let mut total_perimeter = 0.0_f32;

    for i in 0..points.len() {
        let p1 = &points[i];
        let p2 = &points[(i + 1) % points.len()];
        let dx = p2.x as f32 - p1.x as f32;
        let dy = p2.y as f32 - p1.y as f32;
        let d = (dx * dx + dy * dy).sqrt();
        total_perimeter += d;
        cum_dist.push(total_perimeter);
    }

    if total_perimeter <= 0.001 {
        return vec![points[0]; target_point_count];
    }

    let mut resampled = Vec::with_capacity(target_point_count);
    let step_dist = total_perimeter / target_point_count as f32;

    for i in 0..target_point_count {
        let target_dist = i as f32 * step_dist;

        // Find which segment contains target_dist
        let mut seg_idx = 0;
        while seg_idx < points.len() && cum_dist[seg_idx + 1] < target_dist {
            seg_idx += 1;
        }
        seg_idx = seg_idx.min(points.len() - 1);

        let d_start = cum_dist[seg_idx];
        let d_end = cum_dist[seg_idx + 1];
        let seg_len = d_end - d_start;
        let t = if seg_len > 0.0001 {
            ((target_dist - d_start) / seg_len).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let p1 = &points[seg_idx];
        let p2 = &points[(seg_idx + 1) % points.len()];

        let x = (p1.x as f32 + t * (p2.x as f32 - p1.x as f32)).round() as u16;
        let y = (p1.y as f32 + t * (p2.y as f32 - p1.y as f32)).round() as u16;
        let r = (p1.r as f32 + t * (p2.r as f32 - p1.r as f32)).round() as u8;
        let g = (p1.g as f32 + t * (p2.g as f32 - p1.g as f32)).round() as u8;
        let b = (p1.b as f32 + t * (p2.b as f32 - p1.b as f32)).round() as u8;
        let i_val = (p1.i as f32 + t * (p2.i as f32 - p1.i as f32)).round() as u8;

        resampled.push(LaserPoint::new(x, y, r, g, b, i_val));
    }

    resampled
}

/// Generates a closed circular target whose point count matches the exact acoustic frequency.
pub fn generate_harmonic_circle(
    center_x: u16,
    center_y: u16,
    radius: u16,
    pitch_hz: f32,
    sample_rate_pps: u32,
    r: u8,
    g: u8,
    b: u8,
) -> LaserSegment {
    let point_count = points_for_pitch(pitch_hz, sample_rate_pps);
    let mut points = Vec::with_capacity(point_count);

    for i in 0..point_count {
        let theta = (i as f32 / point_count as f32) * 2.0 * PI;
        let x = (center_x as f32 + radius as f32 * theta.cos()).clamp(0.0, 4095.0) as u16;
        let y = (center_y as f32 + radius as f32 * theta.sin()).clamp(0.0, 4095.0) as u16;
        points.push(LaserPoint::new(x, y, r, g, b, 255));
    }

    LaserSegment::new(points)
}

