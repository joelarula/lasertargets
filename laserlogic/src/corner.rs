use crate::LaserPoint;

/// Calculate the angle (in degrees) at point B formed by vectors BA and BC.
/// Returns 180.0 for collinear points, 0.0 for points folding back on themselves.
pub fn angle_at_point(a: &LaserPoint, b: &LaserPoint, c: &LaserPoint) -> f32 {
    let ba_x = a.x as f64 - b.x as f64;
    let ba_y = a.y as f64 - b.y as f64;
    let bc_x = c.x as f64 - b.x as f64;
    let bc_y = c.y as f64 - b.y as f64;

    let dot = ba_x * bc_x + ba_y * bc_y;
    let mag_ba = (ba_x * ba_x + ba_y * ba_y).sqrt();
    let mag_bc = (bc_x * bc_x + bc_y * bc_y).sqrt();

    if mag_ba < 1e-9 || mag_bc < 1e-9 {
        return 180.0; // Degenerate — treat as collinear
    }

    let cos_angle = (dot / (mag_ba * mag_bc)).clamp(-1.0, 1.0);
    cos_angle.acos().to_degrees() as f32
}

/// For each point in the slice, determine whether it is a "corner" that needs extra dwell.
pub fn detect_corners(points: &[LaserPoint], angle_threshold: f32) -> Vec<bool> {
    let len = points.len();
    if len == 0 {
        return vec![];
    }
    let mut corners = vec![false; len];

    corners[0] = true;
    if len > 1 {
        corners[len - 1] = true;
    }

    for i in 1..len.saturating_sub(1) {
        let angle = angle_at_point(&points[i - 1], &points[i], &points[i + 1]);
        if angle < angle_threshold {
            corners[i] = true;
        }
    }

    corners
}

/// Calculate angle-proportional corner dwell count for each point in a segment.
///
/// The dwell count scales proportionally with the turn angle (`180.0 - interior_angle`):
/// - 90 deg turn angle (sharp right angle corner): returns `base_corner_dwell`
/// - 180 deg turn angle (hairpin foldback): returns `base_corner_dwell * 2`
/// - 0 deg turn angle (collinear line): returns 0
/// Check if a slice of points forms a closed loop polygon (start and end within 40 DAC units)
pub fn is_closed_points(points: &[LaserPoint]) -> bool {
    points.len() > 2 && {
        let dx = points[0].x as i32 - points[points.len() - 1].x as i32;
        let dy = points[0].y as i32 - points[points.len() - 1].y as i32;
        dx * dx + dy * dy <= 40 * 40
    }
}

pub fn calculate_corner_dwells(
    points: &[LaserPoint],
    angle_threshold: f32,
    base_corner_dwell: usize,
) -> Vec<usize> {
    let len = points.len();
    if len == 0 {
        return vec![0; 0];
    }
    let mut dwells = vec![0; len];

    if base_corner_dwell == 0 {
        return dwells;
    }

    // Check if segment is a closed loop polygon (first and last point within 40 DAC units)
    let is_closed = is_closed_points(points);






    if is_closed {
        // Calculate seam interior angle between last-1 point -> seam point -> 1st point
        let angle = angle_at_point(&points[len - 2], &points[0], &points[1]);
        let turn_angle = (180.0 - angle).max(0.0);
        if turn_angle >= 25.0 && angle < angle_threshold {
            let calculated = ((turn_angle / 90.0) * base_corner_dwell as f32).round() as usize;
            dwells[0] = calculated;
            dwells[len - 1] = 0;
        } else {
            dwells[0] = 0;
            dwells[len - 1] = 0;
        }
    } else {
        dwells[0] = base_corner_dwell;
        if len > 1 {
            dwells[len - 1] = base_corner_dwell;
        }
    }

    for i in 1..len.saturating_sub(1) {
        let angle = angle_at_point(&points[i - 1], &points[i], &points[i + 1]);
        if angle < angle_threshold {
            let turn_angle = (180.0 - angle).max(0.0);
            let calculated = ((turn_angle / 90.0) * base_corner_dwell as f32).round() as usize;
            dwells[i] = calculated.max(1);
        }
    }

    dwells
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LaserPoint;

    fn pt(x: u16, y: u16) -> LaserPoint {
        LaserPoint::new(x, y, 255, 255, 255, 255)
    }

    #[test]
    fn collinear_points_no_corners() {
        // Three collinear points — middle point should NOT be a corner
        let points = vec![pt(0, 0), pt(100, 0), pt(200, 0)];
        let corners = detect_corners(&points, 135.0);
        assert!(corners[0]); // first
        assert!(!corners[1]); // middle — angle is 180°
        assert!(corners[2]); // last
    }

    #[test]
    fn right_angle_is_corner() {
        // 90° turn
        let points = vec![pt(0, 0), pt(100, 0), pt(100, 100)];
        let corners = detect_corners(&points, 135.0);
        assert!(corners[1]); // 90° < 135° threshold
    }

    #[test]
    fn obtuse_angle_not_corner() {
        // ~150° angle — should NOT be a corner at 135° threshold
        let points = vec![pt(0, 0), pt(100, 0), pt(200, 58)]; // atan(58/100) ≈ 30° off straight
        let corners = detect_corners(&points, 135.0);
        assert!(!corners[1]); // 150° > 135°
    }

    #[test]
    fn single_point() {
        let points = vec![pt(50, 50)];
        let corners = detect_corners(&points, 135.0);
        assert_eq!(corners.len(), 1);
        assert!(corners[0]);
    }

    #[test]
    fn empty_input() {
        let corners = detect_corners(&[], 135.0);
        assert!(corners.is_empty());
    }

    #[test]
    fn angle_at_point_known_values() {
        assert!((angle_at_point(&pt(0, 0), &pt(100, 0), &pt(200, 0)) - 180.0).abs() < 1e-3);
        assert!((angle_at_point(&pt(0, 0), &pt(100, 0), &pt(100, 100)) - 90.0).abs() < 1e-3);
        assert!(angle_at_point(&pt(0, 0), &pt(100, 0), &pt(50, 0)).abs() < 1e-3);
    }

    #[test]
    fn angle_at_point_degenerate_is_collinear() {
        // Duplicate neighbour has no direction — treated as straight, never a corner
        assert_eq!(angle_at_point(&pt(100, 0), &pt(100, 0), &pt(200, 50)), 180.0);
        assert_eq!(angle_at_point(&pt(0, 0), &pt(100, 0), &pt(100, 0)), 180.0);
    }

    #[test]
    fn corner_dwells_empty_and_disabled() {
        assert!(calculate_corner_dwells(&[], 135.0, 3).is_empty());
        let points = vec![pt(0, 0), pt(1000, 0), pt(1000, 1000)];
        assert_eq!(calculate_corner_dwells(&points, 135.0, 0), vec![0, 0, 0]);
    }

    #[test]
    fn corner_dwells_open_segment_endpoints_get_base() {
        let points = vec![pt(0, 0), pt(500, 0), pt(1000, 0)];
        assert_eq!(calculate_corner_dwells(&points, 135.0, 3), vec![3, 0, 3]);
    }

    #[test]
    fn corner_dwells_scale_with_turn_angle() {
        // 90 deg turn -> base
        let right = vec![pt(0, 0), pt(1000, 0), pt(1000, 1000)];
        assert_eq!(calculate_corner_dwells(&right, 135.0, 3)[1], 3);

        // 180 deg hairpin -> 2 * base
        let hairpin = vec![pt(0, 0), pt(1000, 0), pt(100, 0)];
        assert_eq!(calculate_corner_dwells(&hairpin, 135.0, 3)[1], 6);
    }

    #[test]
    fn corner_dwells_below_threshold_get_at_least_one() {
        // ~20 deg turn under a 170 deg threshold: 20/90 * 1 rounds to 0 but is floored to 1
        let points = vec![pt(0, 0), pt(1000, 0), pt(1940, 342)];
        let angle = angle_at_point(&points[0], &points[1], &points[2]);
        assert!(angle < 170.0 && angle > 155.0, "angle was {}", angle);
        assert_eq!(calculate_corner_dwells(&points, 170.0, 1)[1], 1);
    }

    #[test]
    fn corner_dwells_closed_square_dwells_once_at_seam() {
        let square = vec![pt(0, 0), pt(1000, 0), pt(1000, 1000), pt(0, 1000), pt(0, 0)];
        let dwells = calculate_corner_dwells(&square, 135.0, 3);
        // Seam corner dwell lives on the first point only, so it is not emitted twice
        assert_eq!(dwells, vec![3, 3, 3, 3, 0]);
    }

    #[test]
    fn corner_dwells_closed_straight_seam_has_no_dwell() {
        // Seam placed mid-edge: the loop passes straight through it
        let loop_pts = vec![pt(500, 0), pt(1000, 0), pt(1000, 1000), pt(0, 1000), pt(0, 0), pt(500, 0)];
        let dwells = calculate_corner_dwells(&loop_pts, 135.0, 3);
        assert_eq!(dwells[0], 0);
        assert_eq!(dwells[loop_pts.len() - 1], 0);
        assert_eq!(&dwells[1..5], &[3, 3, 3, 3]);
    }

    #[test]
    fn is_closed_points_threshold() {
        assert!(is_closed_points(&[pt(0, 0), pt(100, 0), pt(24, 32)])); // distance 40
        assert!(!is_closed_points(&[pt(0, 0), pt(100, 0), pt(25, 32)]));
        assert!(!is_closed_points(&[pt(0, 0), pt(0, 0)]));
    }
}
