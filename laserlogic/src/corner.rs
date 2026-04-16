use crate::LaserPoint;

/// Calculate the angle (in degrees) at point B formed by vectors BA and BC.
/// Returns 180.0 for collinear points, 0.0 for points folding back on themselves.
fn angle_at_point(a: &LaserPoint, b: &LaserPoint, c: &LaserPoint) -> f32 {
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
///
/// A corner is any point where the angle formed by its neighbours is less than `angle_threshold`
/// degrees. The first and last points are always marked as corners.
///
/// Returns a `Vec<bool>` parallel to the input slice.
pub fn detect_corners(points: &[LaserPoint], angle_threshold: f32) -> Vec<bool> {
    let len = points.len();
    if len == 0 {
        return vec![];
    }
    let mut corners = vec![false; len];

    // First and last are always corners
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
}
