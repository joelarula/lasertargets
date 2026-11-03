use crate::{LaserPoint, LaserSegment, OptimizeConfig};
use crate::corner::detect_corners;

fn distance(a: &LaserPoint, b: &LaserPoint) -> f32 {
    let dx = a.x as f32 - b.x as f32;
    let dy = a.y as f32 - b.y as f32;
    (dx * dx + dy * dy).sqrt()
}

/// Simplify a laser segment by removing near-duplicate and near-collinear points.
///
/// - Points closer than `config.simplify_min_distance` to their predecessor are removed.
/// - Points where the angle formed by their neighbours exceeds `config.simplify_collinear_angle`
///   (i.e. nearly straight) are removed.
///
/// First/last points and detected corners are always preserved.
/// Returns the segment unchanged when both thresholds are 0.0 (disabled).
pub fn simplify_segment(segment: &LaserSegment, config: &OptimizeConfig) -> LaserSegment {
    let pts = &segment.points;
    if pts.len() <= 2 {
        return segment.clone();
    }

    let min_dist = config.simplify_min_distance;
    let collinear_angle = config.simplify_collinear_angle;

    // Nothing to do if both features are disabled
    if min_dist <= 0.0 && collinear_angle <= 0.0 {
        return segment.clone();
    }

    // Pre-compute corners so we never remove them
    let corners = detect_corners(pts, config.corner_angle_threshold);

    let mut keep = vec![true; pts.len()];

    // Pass 1: mark near-duplicate points for removal
    if min_dist > 0.0 {
        let mut last_kept = 0;
        for i in 1..pts.len() {
            if keep[i] && !corners[i] && distance(&pts[last_kept], &pts[i]) < min_dist {
                keep[i] = false;
            } else if keep[i] {
                last_kept = i;
            }
        }
        // Always keep last
        keep[pts.len() - 1] = true;
    }

    // Pass 2: mark near-collinear points for removal
    if collinear_angle > 0.0 {
        // We iterate over triples of *kept* indices
        let kept_indices: Vec<usize> = (0..pts.len()).filter(|&i| keep[i]).collect();
        for window in kept_indices.windows(3) {
            let (a, b, c) = (window[0], window[1], window[2]);
            if corners[b] {
                continue; // never remove corners
            }
            let angle_deg = crate::corner::angle_at_point(&pts[a], &pts[b], &pts[c]);

            // If angle is nearly straight (above threshold), remove it
            if angle_deg > collinear_angle {
                keep[b] = false;
            }
        }
    }

    let kept: Vec<usize> = (0..pts.len()).filter(|&i| keep[i]).collect();
    let points = kept.iter().map(|&i| pts[i]).collect();
    if segment.holds.is_empty() {
        LaserSegment::new(points)
    } else {
        LaserSegment::with_holds(points, kept.iter().map(|&i| segment.hold(i)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LaserPoint;

    fn pt(x: u16, y: u16) -> LaserPoint {
        LaserPoint::new(x, y, 255, 255, 255, 255)
    }

    #[test]
    fn no_simplification_when_disabled() {
        let seg = LaserSegment::new(vec![pt(0, 0), pt(1, 1), pt(2, 2), pt(3, 3)]);
        let config = OptimizeConfig::default(); // both thresholds are 0.0
        let result = simplify_segment(&seg, &config);
        assert_eq!(result.points.len(), seg.points.len());
    }

    #[test]
    fn removes_near_duplicate_points() {
        let seg = LaserSegment::new(vec![pt(0, 0), pt(1, 0), pt(100, 0), pt(200, 0)]);
        let mut config = OptimizeConfig::default();
        config.simplify_min_distance = 10.0;
        let result = simplify_segment(&seg, &config);
        // pt(1,0) is within 10.0 of pt(0,0), should be removed
        assert_eq!(result.points.len(), 3);
        assert_eq!(result.points[0], pt(0, 0));
        assert_eq!(result.points[1], pt(100, 0));
        assert_eq!(result.points[2], pt(200, 0));
    }

    #[test]
    fn preserves_first_and_last() {
        let seg = LaserSegment::new(vec![pt(0, 0), pt(1, 0)]);
        let mut config = OptimizeConfig::default();
        config.simplify_min_distance = 100.0;
        let result = simplify_segment(&seg, &config);
        assert_eq!(result.points.len(), 2);
    }

    #[test]
    fn removes_collinear_points() {
        // Three collinear points — middle should be removed
        let seg = LaserSegment::new(vec![pt(0, 0), pt(100, 0), pt(200, 0)]);
        let mut config = OptimizeConfig::default();
        config.simplify_collinear_angle = 170.0; // angle at middle is 180° > 170° → remove
        let result = simplify_segment(&seg, &config);
        assert_eq!(result.points.len(), 2);
    }

    #[test]
    fn collinear_pass_never_removes_corners() {
        // Collinear threshold below the 90 deg corner would remove it if corners weren't protected
        let seg = LaserSegment::new(vec![pt(0, 0), pt(1000, 0), pt(1000, 1000)]);
        let mut config = OptimizeConfig::default();
        config.simplify_collinear_angle = 45.0;
        let result = simplify_segment(&seg, &config);
        assert_eq!(result.points, seg.points);
    }

    #[test]
    fn min_distance_pass_never_removes_corners() {
        // pt(5,0) is a sharp corner 5 units from the start; it must survive a 10 unit filter
        let seg = LaserSegment::new(vec![pt(0, 0), pt(5, 0), pt(5, 1000), pt(5, 2000)]);
        let mut config = OptimizeConfig::default();
        config.simplify_min_distance = 10.0;
        let result = simplify_segment(&seg, &config);
        assert!(result.points.contains(&pt(5, 0)));
    }

    #[test]
    fn removes_run_of_collinear_points() {
        let seg = LaserSegment::new((0..=10u16).map(|i| pt(i * 100, 0)).collect());
        let mut config = OptimizeConfig::default();
        config.simplify_collinear_angle = 170.0;
        let result = simplify_segment(&seg, &config);
        assert_eq!(result.points, vec![pt(0, 0), pt(1000, 0)]);
    }

    #[test]
    fn min_distance_measured_from_last_kept_point() {
        // Each step is 6 units: individually under 10, so every other point is dropped
        let seg = LaserSegment::new((0..=6u16).map(|i| pt(i * 6, 0)).collect());
        let mut config = OptimizeConfig::default();
        config.simplify_min_distance = 10.0;
        let result = simplify_segment(&seg, &config);
        let xs: Vec<u16> = result.points.iter().map(|p| p.x).collect();
        assert_eq!(xs, vec![0, 12, 24, 36]);
    }
}
