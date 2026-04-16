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
            let angle = crate::corner::detect_corners(&[pts[a], pts[b], pts[c]], 180.0);
            // We actually need the raw angle; recompute via the helper-style logic:
            let ba_x = pts[a].x as f64 - pts[b].x as f64;
            let ba_y = pts[a].y as f64 - pts[b].y as f64;
            let bc_x = pts[c].x as f64 - pts[b].x as f64;
            let bc_y = pts[c].y as f64 - pts[b].y as f64;
            let dot = ba_x * bc_x + ba_y * bc_y;
            let mag_ba = (ba_x * ba_x + ba_y * ba_y).sqrt();
            let mag_bc = (bc_x * bc_x + bc_y * bc_y).sqrt();
            if mag_ba < 1e-9 || mag_bc < 1e-9 {
                continue;
            }
            let cos_a = (dot / (mag_ba * mag_bc)).clamp(-1.0, 1.0);
            let angle_deg = cos_a.acos().to_degrees() as f32;
            // If angle is nearly straight (above threshold), remove it
            if angle_deg > collinear_angle {
                keep[b] = false;
            }
            let _ = angle; // suppress unused warning
        }
    }

    let result: Vec<LaserPoint> = pts
        .iter()
        .enumerate()
        .filter(|(i, _)| keep[*i])
        .map(|(_, p)| *p)
        .collect();

    LaserSegment::new(result)
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
}
