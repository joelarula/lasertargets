use crate::corner::detect_corners;
use crate::simplify::simplify_segment;
use crate::{LaserPoint, LaserSegment, OptimizeConfig};

/// Produce an optimised point buffer from a list of laser segments.
///
/// The pipeline for each segment:
///   1. Optionally simplify (remove near-duplicates / near-collinear points)
///   2. Detect corners
///   3. Emit start blank dwell → lit points with corner dwells & interpolation → end blank dwell
///
/// Between segments a blanking sequence is inserted:
///   end-dwell (blanked at last point) → interpolated jump → start-dwell (blanked at first point)
pub fn optimize(segments: &[LaserSegment], config: &OptimizeConfig) -> Vec<LaserPoint> {
    let mut output: Vec<LaserPoint> = Vec::new();

    for segment in segments {
        if segment.points.is_empty() {
            continue;
        }

        // --- Simplify ---
        let simplified = simplify_segment(segment, config);
        let pts = &simplified.points;
        if pts.is_empty() {
            continue;
        }

        // --- Corner detection ---
        let corners = detect_corners(pts, config.corner_angle_threshold);

        // --- Inter-segment blanking (between shapes) ---
        if !output.is_empty() {
            let last = *output.last().unwrap();
            let first = pts[0];
            emit_blanking_jump(&mut output, &last, &first, config);
        }

        // --- Start blank dwell ---
        let first = pts[0];
        for _ in 0..config.start_dwell_points {
            output.push(LaserPoint::blanked(first.x, first.y));
        }

        // --- Emit lit points with interpolation and corner dwells ---
        for i in 0..pts.len() {
            // Interpolation between previous and current point
            if i > 0 {
                emit_interpolated_points(&mut output, &pts[i - 1], &pts[i], config);
            }

            // The actual point (possibly repeated for dwell)
            let p = pts[i];
            output.push(p);

            // Corner dwell: repeat corner points
            if corners[i] {
                for _ in 0..config.corner_dwell_points {
                    output.push(p);
                }
            }
        }

        // --- End blank dwell ---
        let last = pts[pts.len() - 1];
        for _ in 0..config.end_dwell_points {
            output.push(LaserPoint::blanked(last.x, last.y));
        }
    }

    output
}

/// Emit linearly interpolated lit points between `from` and `to` when they are far apart.
fn emit_interpolated_points(
    output: &mut Vec<LaserPoint>,
    from: &LaserPoint,
    to: &LaserPoint,
    config: &OptimizeConfig,
) {
    let dx = to.x as f32 - from.x as f32;
    let dy = to.y as f32 - from.y as f32;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist <= config.interp_distance_threshold || config.interp_spacing <= 0.0 {
        return;
    }

    let num_interp = (dist / config.interp_spacing).ceil() as usize;
    for step in 1..num_interp {
        let t = step as f32 / num_interp as f32;
        let x = (from.x as f32 + dx * t) as u16;
        let y = (from.y as f32 + dy * t) as u16;
        output.push(LaserPoint::new(x, y, from.r, from.g, from.b, from.i));
    }
}

/// Emit a blanking sequence to move the galvos from `from` to `to` with laser off.
fn emit_blanking_jump(
    output: &mut Vec<LaserPoint>,
    from: &LaserPoint,
    to: &LaserPoint,
    config: &OptimizeConfig,
) {
    // End dwell: hold blanked at the departure point
    for _ in 0..config.blank_end_dwell {
        output.push(LaserPoint::blanked(from.x, from.y));
    }

    // Interpolated blanked jump
    let steps = config.blank_jump_steps;
    if steps > 0 {
        let dx = to.x as f32 - from.x as f32;
        let dy = to.y as f32 - from.y as f32;
        for step in 1..steps {
            let t = step as f32 / steps as f32;
            let x = (from.x as f32 + dx * t) as u16;
            let y = (from.y as f32 + dy * t) as u16;
            output.push(LaserPoint::blanked(x, y));
        }
    }

    // Start dwell: hold blanked at the arrival point
    for _ in 0..config.blank_start_dwell {
        output.push(LaserPoint::blanked(to.x, to.y));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LaserPoint, LaserSegment, OptimizeConfig};

    fn pt(x: u16, y: u16) -> LaserPoint {
        LaserPoint::new(x, y, 255, 255, 255, 255)
    }

    fn default_config() -> OptimizeConfig {
        OptimizeConfig::default()
    }

    #[test]
    fn empty_segments() {
        let result = optimize(&[], &default_config());
        assert!(result.is_empty());
    }

    #[test]
    fn single_segment_has_start_end_dwells() {
        let seg = LaserSegment::new(vec![pt(100, 100), pt(200, 100)]);
        let config = default_config();
        let result = optimize(&[seg], &config);

        // Start dwell (3 blanked) + first point + first corner dwell (3) +
        // second point + second corner dwell (3) + end dwell (3 blanked)
        // First and last are both corners by default
        assert!(!result.is_empty());

        // First points should be blanked (start dwell)
        for i in 0..config.start_dwell_points as usize {
            assert!(result[i].is_blanked(), "start dwell point {} should be blanked", i);
        }

        // Last points should be blanked (end dwell)
        let len = result.len();
        for i in (len - config.end_dwell_points as usize)..len {
            assert!(result[i].is_blanked(), "end dwell point {} should be blanked", i);
        }
    }

    #[test]
    fn right_angle_corner_gets_dwell() {
        let seg = LaserSegment::new(vec![pt(0, 0), pt(1000, 0), pt(1000, 1000)]);
        let config = default_config();
        let result = optimize(&[seg], &config);

        // Count how many times pt(1000,0) appears — should be 1 (original) + 3 (corner dwell) = 4
        let corner_count = result
            .iter()
            .filter(|p| p.x == 1000 && p.y == 0 && !p.is_blanked())
            .count();
        assert_eq!(corner_count, 1 + config.corner_dwell_points as usize);
    }

    #[test]
    fn two_segments_have_blanking_between() {
        let seg1 = LaserSegment::new(vec![pt(0, 0), pt(100, 0)]);
        let seg2 = LaserSegment::new(vec![pt(2000, 2000), pt(2100, 2000)]);
        let config = default_config();
        let result = optimize(&[seg1, seg2], &config);

        // There should be blanked points between the two segments
        let has_blanked_between = result
            .windows(2)
            .any(|w| !w[0].is_blanked() && w[1].is_blanked());
        assert!(has_blanked_between, "should have blanking transition between segments");
    }

    #[test]
    fn interpolation_added_for_distant_points() {
        let seg = LaserSegment::new(vec![pt(0, 0), pt(4000, 0)]);
        let mut config = default_config();
        config.interp_distance_threshold = 200.0;
        config.interp_spacing = 100.0;
        let result = optimize(&[seg], &config);

        // Distance is 4000, spacing 100 → ~40 interpolated points between the two
        // Plus start dwell, end dwell, corner dwells, and the 2 original points
        let lit_count = result.iter().filter(|p| !p.is_blanked()).count();
        assert!(lit_count > 10, "should have interpolated points; got {}", lit_count);
    }
}
