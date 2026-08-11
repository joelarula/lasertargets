use crate::corner::calculate_corner_dwells;
use crate::simplify::simplify_segment;
use crate::{LaserPoint, LaserSegment, OptimizeConfig};

/// Produce an optimised point buffer from a list of laser segments.
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

        let mut pts_cloned = simplified.points.clone();

        // Check if segment is a closed loop polygon (start and end points within 40 DAC units)
        let is_closed_segment = pts_cloned.len() > 2 && {
            let dx = pts_cloned[0].x as i32 - pts_cloned.last().unwrap().x as i32;
            let dy = pts_cloned[0].y as i32 - pts_cloned.last().unwrap().y as i32;
            (dx * dx + dy * dy) <= 1600
        };

        if is_closed_segment {
            // Snap last point coordinates exactly to first point to close seam seamlessly
            let last_idx = pts_cloned.len() - 1;
            pts_cloned[last_idx].x = pts_cloned[0].x;
            pts_cloned[last_idx].y = pts_cloned[0].y;
        }

        let pts = &pts_cloned;

        // --- Angle-proportional Corner Dwell Calculation ---
        let corner_dwells = calculate_corner_dwells(pts, config.corner_angle_threshold, config.corner_dwell_points as usize);

        // --- Inter-segment blanking (between shapes) ---
        if !output.is_empty() {
            let last = *output.last().unwrap();
            let first = pts[0];
            emit_blanking_jump(&mut output, &last, &first, config);
        }

        // --- Start blank dwell (only for open segments) ---
        if !is_closed_segment {
            let first = pts[0];
            for _ in 0..config.start_dwell_points {
                output.push(LaserPoint::blanked(first.x, first.y));
            }
        }

        // --- Emit lit points with interpolation and angle-based corner dwells ---
        for i in 0..pts.len() {
            // Interpolation between previous and current point
            if i > 0 {
                emit_interpolated_points(&mut output, &pts[i - 1], &pts[i], config);
            }

            let p = pts[i];
            output.push(p);

            // Angle-proportional corner dwell + point dwell hint
            let extra_dwell = corner_dwells[i].max(p.dwell as usize);
            for _ in 0..extra_dwell {
                output.push(p);
            }
        }

        if is_closed_segment {
            // Seam overlap: emit 2 extra closing points at pts[0] so color delay shift covers the closing seam 100%
            let first = pts[0];
            for _ in 0..2 {
                output.push(first);
            }
        }

        // --- End blank dwell (only for open segments) ---
        if !is_closed_segment {
            let last = pts[pts.len() - 1];
            for _ in 0..config.end_dwell_points {
                output.push(LaserPoint::blanked(last.x, last.y));
            }
        }
    }

    // --- Frame-wraparound blanking jump (from end of frame back to start of frame) ---
    if !output.is_empty() {
        let last = *output.last().unwrap();
        let first = output[0];
        if last.x != first.x || last.y != first.y {
            emit_blanking_jump(&mut output, &last, &first, config);
        }
    }

    // Apply 2-point laser diode modulation delay to compensate for galvo mirror inertia lag
    apply_laser_color_delay(&mut output, 2);

    output
}

/// Shift laser diode color channels relative to XY galvo position to compensate for galvo mechanical inertia lag.
/// Blanked travel points are never color-shifted — this prevents lit color bleeding across a laser-off jump (the "tail" artifact).
fn apply_laser_color_delay(points: &mut [LaserPoint], delay_points: usize) {
    let len = points.len();
    if len <= delay_points || delay_points == 0 {
        return;
    }
    let colors: Vec<(u8, u8, u8, u8)> = points.iter().map(|p| (p.r, p.g, p.b, p.i)).collect();

    for i in 0..len {
        // Never shift color into a blanked travel point — that would make it appear lit
        if points[i].is_blanked() {
            continue;
        }
        let src_idx = (i + len - delay_points) % len;
        let (r, g, b, intensity) = colors[src_idx];
        // Only apply delay if the source point was also lit (don't pull blank-black into lit zone)
        if r == 0 && g == 0 && b == 0 && intensity == 0 {
            continue;
        }
        points[i].r = r;
        points[i].g = g;
        points[i].b = b;
        points[i].i = intensity;
    }
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
    // 1. Departure Dwell: hold blanked at departure point (laser turns off before galvos move)
    for _ in 0..config.blank_end_dwell {
        output.push(LaserPoint::blanked(from.x, from.y));
    }

    // 2. Distance-Adaptive Blanking Jump: interpolate galvo steps across jump
    let dx = to.x as f32 - from.x as f32;
    let dy = to.y as f32 - from.y as f32;
    let dist = (dx * dx + dy * dy).sqrt();

    let max_steps = config.blank_jump_steps.max(4);
    let calc_steps = (dist / 250.0).ceil() as u16;
    let steps = calc_steps.clamp(4, max_steps);

    for step in 1..steps {
        let t = step as f32 / steps as f32;
        let x = (from.x as f32 + dx * t) as u16;
        let y = (from.y as f32 + dy * t) as u16;
        output.push(LaserPoint::blanked(x, y));
    }

    // 3. Arrival Dwell: hold blanked at arrival point (galvos settle before laser turns on)
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
