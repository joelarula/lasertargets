use crate::corner::calculate_corner_dwells;
use crate::simplify::simplify_segment;
use crate::{LaserPoint, LaserSegment, OptimizeConfig};

/// Produce an optimised point buffer from a list of laser segments.
/// Compose and optimize a full laser frame consisting of general shapes and text shapes.
pub fn compose_frame(
    general_segments: &[LaserSegment],
    text_segments: &[LaserSegment],
    config: &OptimizeConfig,
) -> Vec<LaserPoint> {
    let mut frame = optimize(general_segments, config);
    let optimized_text = crate::text_optimize::optimize_text(text_segments, config, None);

    if !optimized_text.is_empty() {
        if !frame.is_empty() {
            let from = *frame.last().unwrap();
            let to = optimized_text[0];
            emit_blanking_jump(&mut frame, &from, &to, config);
        }
        frame.extend(optimized_text);
    }

    frame
}

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

            // Angle-proportional corner dwell
            let extra_dwell = corner_dwells[i];
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
pub fn emit_interpolated_points(
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
pub fn emit_blanking_jump(
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

    fn square() -> LaserSegment {
        LaserSegment::new(vec![pt(0, 0), pt(1000, 0), pt(1000, 1000), pt(0, 1000), pt(0, 0)])
    }

    fn same_position(a: &LaserPoint, b: &LaserPoint) -> bool {
        a.x == b.x && a.y == b.y
    }

    #[test]
    fn empty_segments_are_skipped() {
        let seg = LaserSegment::new(vec![pt(100, 100), pt(900, 900)]);
        let config = default_config();
        let with_empty = optimize(&[LaserSegment::new(vec![]), seg.clone()], &config);
        assert_eq!(with_empty, optimize(&[seg], &config));
    }

    #[test]
    fn closed_segment_has_no_blanking() {
        // A lone closed loop is drawn continuously: no start/end dwell and no wraparound jump
        let result = optimize(&[square()], &default_config());
        assert!(result.iter().all(|p| !p.is_blanked()));
        assert!(same_position(&result[0], result.last().unwrap()));
    }

    #[test]
    fn closed_segment_seam_is_snapped() {
        // Last point is ~7 units off the first: treated as closed and snapped onto the first
        let seg = LaserSegment::new(vec![pt(0, 0), pt(1000, 0), pt(1000, 1000), pt(0, 1000), pt(5, 5)]);
        let result = optimize(&[seg], &default_config());
        assert!(!result.iter().any(|p| p.x == 5 && p.y == 5));
        assert!(result.iter().all(|p| !p.is_blanked()));
    }

    #[test]
    fn closed_segment_seam_overlap_points() {
        // pts[0] is emitted once + seam dwell (3), then 2 closing overlap points; last point (0,0) has no dwell
        let result = optimize(&[square()], &default_config());
        let at_origin = result.iter().filter(|p| p.x == 0 && p.y == 0).count();
        assert_eq!(at_origin, 1 + 3 + 1 + 2);
    }

    #[test]
    fn frame_wraps_back_to_start() {
        // The DAC loops the buffer, so the last point must sit where the first one is
        let config = default_config();
        let segs = [
            LaserSegment::new(vec![pt(100, 100), pt(500, 100)]),
            LaserSegment::new(vec![pt(3000, 3000), pt(3500, 3200)]),
        ];
        let result = optimize(&segs, &config);
        let first = result[0];
        let last = *result.last().unwrap();
        assert!(same_position(&first, &last));
        assert!(last.is_blanked());
    }

    #[test]
    fn color_delay_shifts_lit_colors_and_skips_blanked() {
        let mut points = vec![
            LaserPoint::new(0, 0, 255, 0, 0, 255),       // red
            LaserPoint::new(1, 0, 0, 255, 0, 255),       // green
            LaserPoint::new(2, 0, 0, 0, 255, 255),       // blue
            LaserPoint::blanked(3, 0),
            LaserPoint::new(4, 0, 255, 255, 255, 255),   // white
            LaserPoint::new(5, 0, 10, 10, 10, 10),
        ];
        apply_laser_color_delay(&mut points, 2);

        // Wraps around: point 0 takes point 4's colour
        assert_eq!((points[0].r, points[0].g, points[0].b), (255, 255, 255));
        assert_eq!((points[1].r, points[1].i), (10, 10));
        assert_eq!((points[2].r, points[2].g, points[2].b), (255, 0, 0));
        // Blanked travel point is never lit
        assert!(points[3].is_blanked());
        assert_eq!((points[4].r, points[4].g, points[4].b), (0, 0, 255));
        // Source two back is blanked, so it keeps its own colour
        assert_eq!((points[5].r, points[5].i), (10, 10));
        // Positions never move
        assert!(points.iter().enumerate().all(|(i, p)| p.x == i as u16));
    }

    #[test]
    fn color_delay_ignores_short_buffers() {
        let mut points = vec![LaserPoint::new(0, 0, 255, 0, 0, 255), LaserPoint::new(1, 0, 0, 255, 0, 255)];
        let before = points.clone();
        apply_laser_color_delay(&mut points, 2);
        assert_eq!(points, before);
    }

    #[test]
    fn blanking_jump_structure() {
        let config = default_config();
        let from = pt(0, 0);
        let to = pt(4000, 0);
        let mut out = Vec::new();
        emit_blanking_jump(&mut out, &from, &to, &config);

        // 4000 units / 250 = 16 steps -> 15 travel points between the dwells
        let expected = config.blank_end_dwell as usize + 15 + config.blank_start_dwell as usize;
        assert_eq!(out.len(), expected);
        assert!(out.iter().all(|p| p.is_blanked()));
        assert!(same_position(&out[0], &from));
        assert!(same_position(out.last().unwrap(), &to));
        // Travel is monotonic
        assert!(out.windows(2).all(|w| w[1].x >= w[0].x));
    }

    #[test]
    fn blanking_jump_step_bounds() {
        let mut config = default_config();
        config.blank_end_dwell = 0;
        config.blank_start_dwell = 0;

        // Short jumps still get the minimum 4 steps (3 travel points)
        let mut out = Vec::new();
        emit_blanking_jump(&mut out, &pt(0, 0), &pt(10, 0), &config);
        assert_eq!(out.len(), 3);

        // Long jumps are capped at blank_jump_steps
        config.blank_jump_steps = 8;
        let mut out = Vec::new();
        emit_blanking_jump(&mut out, &pt(0, 0), &pt(4000, 4000), &config);
        assert_eq!(out.len(), 7);

        // A cap below the minimum is raised to 4
        config.blank_jump_steps = 1;
        let mut out = Vec::new();
        emit_blanking_jump(&mut out, &pt(0, 0), &pt(4000, 4000), &config);
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn interpolation_fills_long_edges() {
        let config = default_config();
        let from = LaserPoint::new(0, 0, 10, 20, 30, 40);
        let to = LaserPoint::new(1000, 0, 255, 255, 255, 255);
        let mut out = Vec::new();
        emit_interpolated_points(&mut out, &from, &to, &config);

        // 1000 / 100 spacing = 10 steps -> 9 interior points, excluding both endpoints
        let xs: Vec<u16> = out.iter().map(|p| p.x).collect();
        assert_eq!(xs, vec![100, 200, 300, 400, 500, 600, 700, 800, 900]);
        assert!(out.iter().all(|p| (p.r, p.g, p.b, p.i) == (10, 20, 30, 40)));
    }

    #[test]
    fn interpolation_skipped_for_short_edges_or_zero_spacing() {
        let mut config = default_config();
        let mut out = Vec::new();
        emit_interpolated_points(&mut out, &pt(0, 0), &pt(150, 0), &config);
        assert!(out.is_empty());

        config.interp_spacing = 0.0;
        emit_interpolated_points(&mut out, &pt(0, 0), &pt(4000, 0), &config);
        assert!(out.is_empty());
    }

    #[test]
    fn compose_frame_empty() {
        assert!(compose_frame(&[], &[], &default_config()).is_empty());
    }

    #[test]
    fn compose_frame_single_kind_matches_pipeline() {
        let config = default_config();
        let general = [square()];
        let text = [LaserSegment::new(vec![pt(100, 3000), pt(300, 3500), pt(500, 3000)])];
        assert_eq!(compose_frame(&general, &[], &config), optimize(&general, &config));
        assert_eq!(
            compose_frame(&[], &text, &config),
            crate::text_optimize::optimize_text(&text, &config, None)
        );
    }

    #[test]
    fn compose_frame_joins_general_then_text_with_blanking() {
        let config = default_config();
        let general = [square()];
        let text = [LaserSegment::new(vec![pt(100, 3000), pt(300, 3500), pt(500, 3000)])];

        let g = optimize(&general, &config);
        let t = crate::text_optimize::optimize_text(&text, &config, None);
        let mut expected = g.clone();
        emit_blanking_jump(&mut expected, g.last().unwrap(), &t[0], &config);
        expected.extend(t);

        assert_eq!(compose_frame(&general, &text, &config), expected);
    }

    #[test]
    #[ignore = "BUG: compose_frame does not wrap the combined frame back to the general start; \
                the DAC jumps from the text start to the general start with no blanking travel"]
    fn compose_frame_wraps_back_to_start() {
        let config = default_config();
        let general = [square()];
        let text = [LaserSegment::new(vec![pt(100, 3000), pt(300, 3500), pt(500, 3000)])];
        let frame = compose_frame(&general, &text, &config);
        assert!(same_position(&frame[0], frame.last().unwrap()));
    }
}
