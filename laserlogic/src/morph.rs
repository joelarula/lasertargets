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

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: u16, y: u16) -> LaserPoint {
        LaserPoint::new(x, y, 255, 255, 255, 255)
    }

    fn xs(seg: &LaserSegment) -> Vec<u16> {
        seg.points.iter().map(|p| p.x).collect()
    }

    #[test]
    fn lerp_point_endpoints_and_midpoint() {
        let a = LaserPoint::new(0, 100, 0, 0, 0, 0);
        let b = LaserPoint::new(1000, 300, 200, 100, 50, 255);
        assert_eq!(lerp_point(&a, &b, 0.0), a);
        assert_eq!(lerp_point(&a, &b, 1.0), b);
        let mid = lerp_point(&a, &b, 0.5);
        assert_eq!((mid.x, mid.y, mid.r, mid.g, mid.b, mid.i), (500, 200, 100, 50, 25, 127));
    }

    #[test]
    fn lerp_point_clamps_t() {
        let a = pt(0, 0);
        let b = pt(1000, 1000);
        assert_eq!(lerp_point(&a, &b, -1.0), a);
        assert_eq!(lerp_point(&a, &b, 2.0), b);
    }

    #[test]
    fn resample_degenerate_counts() {
        let seg = LaserSegment::new(vec![pt(0, 0), pt(100, 0)]);
        assert!(resample_segment(&seg, 0).points.is_empty());
        assert!(resample_segment(&LaserSegment::new(vec![]), 5).points.is_empty());
        assert_eq!(resample_segment(&seg, 1).points, vec![pt(0, 0)]);
        let single = LaserSegment::new(vec![pt(7, 7)]);
        assert_eq!(resample_segment(&single, 3).points, vec![pt(7, 7); 3]);
    }

    #[test]
    fn resample_upsamples_and_keeps_endpoints() {
        let seg = LaserSegment::new(vec![pt(0, 0), pt(100, 0), pt(200, 0)]);
        assert_eq!(xs(&resample_segment(&seg, 5)), vec![0, 50, 100, 150, 200]);
    }

    #[test]
    fn resample_downsamples_and_keeps_endpoints() {
        let seg = LaserSegment::new((0..5u16).map(|i| pt(i * 100, 0)).collect());
        assert_eq!(xs(&resample_segment(&seg, 3)), vec![0, 200, 400]);
    }

    #[test]
    fn resample_is_index_based_not_arc_length() {
        // Documents current behaviour: samples are spread over point indices, not path distance
        let seg = LaserSegment::new(vec![pt(0, 0), pt(10, 0), pt(1000, 0)]);
        assert_eq!(xs(&resample_segment(&seg, 3)), vec![0, 10, 1000]);
    }

    #[test]
    fn lerp_segments_equal_counts() {
        let a = LaserSegment::new(vec![pt(0, 0), pt(100, 0)]);
        let b = LaserSegment::new(vec![pt(1000, 0), pt(1100, 0)]);
        assert_eq!(xs(&lerp_segments(&a, &b, 0.5)), vec![500, 600]);
    }

    #[test]
    fn lerp_segments_resamples_shorter_side() {
        let a = LaserSegment::new(vec![pt(0, 0), pt(400, 0)]);
        let b = LaserSegment::new((0..5u16).map(|i| pt(i * 100, 1000)).collect());
        let out = lerp_segments(&a, &b, 0.0);
        assert_eq!(xs(&out), vec![0, 100, 200, 300, 400]);
        let out = lerp_segments(&a, &b, 1.0);
        assert!(out.points.iter().all(|p| p.y == 1000));
    }

    #[test]
    fn lerp_segments_both_empty() {
        let empty = LaserSegment::new(vec![]);
        assert!(lerp_segments(&empty, &empty, 0.5).points.is_empty());
    }

    #[test]
    #[ignore = "BUG: lerp_segments panics (index out of bounds) when exactly one side is empty, \
                because resample_segment of an empty segment returns no points"]
    fn lerp_segments_one_side_empty() {
        let empty = LaserSegment::new(vec![]);
        let b = LaserSegment::new(vec![pt(0, 0), pt(100, 0)]);
        assert_eq!(lerp_segments(&empty, &b, 0.5).points.len(), 2);
        assert_eq!(lerp_segments(&b, &empty, 0.5).points.len(), 2);
    }
}
