use crate::{LaserPoint, LaserSegment};

/// Linear interpolation between two u8 color/intensity channels.
fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).clamp(0.0, 255.0) as u8
}

/// Linear interpolation between two u16 DAC coordinate channels.
fn lerp_u16(a: u16, b: u16, t: f32) -> u16 {
    (a as f32 + (b as f32 - a as f32) * t).clamp(0.0, 4095.0) as u16
}

/// Interpolates between two individual DAC laser points at parameter t in [0.0, 1.0].
pub fn lerp_point(p1: &LaserPoint, p2: &LaserPoint, t: f32) -> LaserPoint {
    let t = t.clamp(0.0, 1.0);
    LaserPoint {
        x: lerp_u16(p1.x, p2.x, t),
        y: lerp_u16(p1.y, p2.y, t),
        r: lerp_u8(p1.r, p2.r, t),
        g: lerp_u8(p1.g, p2.g, t),
        b: lerp_u8(p1.b, p2.b, t),
        i: lerp_u8(p1.i, p2.i, t),
    }
}

/// Resamples a LaserSegment so it contains exactly `target_count` points along its path length.
pub fn resample_segment(segment: &LaserSegment, target_count: usize) -> LaserSegment {
    if segment.points.is_empty() || target_count == 0 {
        return LaserSegment::new(Vec::new());
    }
    if segment.points.len() == 1 || target_count == 1 {
        let p = segment.points[0];
        return LaserSegment::new(vec![p; target_count]);
    }

    let mut resampled = Vec::with_capacity(target_count);
    let n = segment.points.len();

    for i in 0..target_count {
        let t = i as f32 / (target_count - 1) as f32;
        let index_f = t * (n - 1) as f32;
        let idx0 = (index_f.floor() as usize).min(n - 1);
        let idx1 = (idx0 + 1).min(n - 1);
        let sub_t = index_f - idx0 as f32;

        resampled.push(lerp_point(&segment.points[idx0], &segment.points[idx1], sub_t));
    }

    LaserSegment::new(resampled)
}

/// Interpolates between two LaserSegments at parameter t in [0.0, 1.0].
/// Automatically resamples point counts if seg1 and seg2 have differing point counts.
pub fn lerp_segments(seg1: &LaserSegment, seg2: &LaserSegment, t: f32) -> LaserSegment {
    if seg1.points.is_empty() && seg2.points.is_empty() {
        return LaserSegment::new(Vec::new());
    }

    let max_pts = seg1.points.len().max(seg2.points.len());
    let s1 = if seg1.points.len() == max_pts { seg1.clone() } else { resample_segment(seg1, max_pts) };
    let s2 = if seg2.points.len() == max_pts { seg2.clone() } else { resample_segment(seg2, max_pts) };

    let mut result_pts = Vec::with_capacity(max_pts);
    for i in 0..max_pts {
        result_pts.push(lerp_point(&s1.points[i], &s2.points[i], t));
    }

    LaserSegment::new(result_pts)
}
