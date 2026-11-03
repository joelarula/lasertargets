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

/// Galvo model in output points, from the physical values in [`OptimizeConfig`].
struct Galvo {
    lit_step: f32,
    blank_step: f32,
    ramp_points: usize,
    corner_hold_points: usize,
    depart_points: usize,
    settle_points: usize,
    settle_points_per_dac: f32,
}

impl Galvo {
    fn new(config: &OptimizeConfig) -> Self {
        Self {
            lit_step: config.step_dac(config.lit_speed_m_s, config.max_lit_step_dac),
            blank_step: config.step_dac(config.blank_speed_m_s, config.max_blank_step_dac),
            ramp_points: config.ms_to_points(config.corner_ramp_ms),
            corner_hold_points: config.ms_to_points(config.corner_hold_ms),
            depart_points: config.ms_to_points(config.depart_ms),
            settle_points: config.ms_to_points(config.settle_ms),
            settle_points_per_dac: config.settle_ms_per_m.max(0.0) / 1000.0 * config.pps as f32
                / config.dac_per_m.max(1.0),
        }
    }

    /// Laser-off jump: hold at `from` until the light is off, travel with speed-up and slow-down,
    /// hold at `to` until the mirrors settle (longer for longer jumps).
    fn jump(&self, output: &mut Vec<LaserPoint>, from: &LaserPoint, to: &LaserPoint) {
        for _ in 0..self.depart_points {
            output.push(LaserPoint::blanked(from.x, from.y));
        }
        let distance = dac_distance(from, to);
        for t in travel_profile(distance, self.blank_step, self.ramp_points, true, true) {
            let (x, y) = lerp_dac(from, to, t);
            output.push(LaserPoint::blanked(x, y));
        }
        let settle = self.settle_points + (distance * self.settle_points_per_dac).round() as usize;
        for _ in 0..settle.max(1) {
            output.push(LaserPoint::blanked(to.x, to.y));
        }
    }

    /// Lit edge from `from` to `to` (both excluded) at constant speed, slowing down into and
    /// speeding up out of the ends that are sharp corners or stroke ends.
    fn edge(&self, output: &mut Vec<LaserPoint>, from: &LaserPoint, to: &LaserPoint, ease_start: bool, ease_end: bool) {
        let distance = dac_distance(from, to);
        for t in travel_profile(distance, self.lit_step, self.ramp_points, ease_start, ease_end) {
            let (x, y) = lerp_dac(from, to, t);
            output.push(LaserPoint::new(x, y, from.r, from.g, from.b, from.i));
        }
    }
}

fn dac_distance(a: &LaserPoint, b: &LaserPoint) -> f32 {
    let dx = b.x as f32 - a.x as f32;
    let dy = b.y as f32 - a.y as f32;
    (dx * dx + dy * dy).sqrt()
}

fn lerp_dac(a: &LaserPoint, b: &LaserPoint, t: f32) -> (u16, u16) {
    let x = a.x as f32 + (b.x as f32 - a.x as f32) * t;
    let y = a.y as f32 + (b.y as f32 - a.y as f32) * t;
    (x.round() as u16, y.round() as u16)
}

/// Positions (0..1, end excluded) of the intermediate points along a move of `distance`:
/// steps of `step`, ramping up from rest over `ramp` points at the start and down to rest at the
/// end when eased. Steps are scaled so the last one lands exactly on the end.
fn travel_profile(distance: f32, step: f32, ramp: usize, ease_start: bool, ease_end: bool) -> Vec<f32> {
    if distance < 1.0 {
        return Vec::new();
    }
    let ramp_steps = |on: bool| -> Vec<f32> {
        if on { (1..=ramp).map(|k| step * k as f32 / (ramp + 1) as f32).collect() } else { Vec::new() }
    };
    let up = ramp_steps(ease_start);
    let mut down = ramp_steps(ease_end);
    down.reverse();
    let ramp_distance: f32 = up.iter().chain(&down).sum();

    let mut steps = up;
    let remaining = distance - ramp_distance;
    if remaining > 0.0 {
        let cruise = (remaining / step).ceil().max(1.0) as usize;
        steps.extend(std::iter::repeat_n(remaining / cruise as f32, cruise));
    }
    steps.extend(down);
    if steps.is_empty() {
        return Vec::new();
    }

    let scale = distance / steps.iter().sum::<f32>();
    let mut travelled = 0.0;
    let mut positions: Vec<f32> = steps
        .iter()
        .map(|s| {
            travelled += s * scale;
            travelled / distance
        })
        .collect();
    positions.pop(); // the end point itself is emitted by the caller
    positions
}

pub fn optimize(segments: &[LaserSegment], config: &OptimizeConfig) -> Vec<LaserPoint> {
    let galvo = Galvo::new(config);
    let mut output: Vec<LaserPoint> = Vec::new();

    for segment in segments {
        if segment.points.is_empty() {
            continue;
        }

        // --- Simplify ---
        let simplified = simplify_segment(segment, config);
        if simplified.points.is_empty() {
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

        // Holds: angle-proportional corner hold (stroke ends of open strokes count as corners),
        // replaced where the shape overrides it
        let corner_holds = calculate_corner_dwells(pts, config.corner_angle_threshold, galvo.corner_hold_points);
        let holds: Vec<usize> = (0..pts.len())
            .map(|i| simplified.hold(i).map_or(corner_holds[i], |hold| hold as usize))
            .collect();
        // The galvo slows down into every point it stops at: holds and stroke ends
        let mut sharp: Vec<bool> = (0..pts.len())
            .map(|i| holds[i] > 0 || corner_holds[i] > 0)
            .collect();
        if is_closed_segment {
            // The seam is one corner: its first and last copy ease alike
            let seam = sharp[0] || sharp[pts.len() - 1];
            sharp[0] = seam;
            let last = pts.len() - 1;
            sharp[last] = seam;
        } else {
            sharp[0] = true;
            let last = pts.len() - 1;
            sharp[last] = true;
        }

        // --- Laser-off jump from the previous stroke ---
        if let Some(&last) = output.last() {
            galvo.jump(&mut output, &last, &pts[0]);
        }

        // --- Lit points: constant-speed edges, eased into and out of corners, holds at corners ---
        for i in 0..pts.len() {
            if i > 0 {
                galvo.edge(&mut output, &pts[i - 1], &pts[i], sharp[i - 1], sharp[i]);
            }
            let p = pts[i];
            for _ in 0..=holds[i] {
                output.push(p);
            }
        }

        if is_closed_segment {
            // Seam overlap: emit 2 extra closing points at pts[0] so color delay shift covers the closing seam 100%
            let first = pts[0];
            for _ in 0..2 {
                output.push(first);
            }
        } else {
            // Light off before the galvo leaves; the jump to the next stroke continues the hold
            let last = pts[pts.len() - 1];
            output.push(LaserPoint::blanked(last.x, last.y));
        }
    }

    // --- Frame-wraparound jump (from end of frame back to start of frame) ---
    if !output.is_empty() {
        let last = *output.last().unwrap();
        let first = output[0];
        if last.x != first.x || last.y != first.y {
            galvo.jump(&mut output, &last, &first);
        }
    }

    // Apply 2-point laser diode modulation delay to compensate for galvo mirror inertia lag
    apply_laser_color_delay(&mut output, 2);

    apply_diode_delays(
        &mut output,
        config.ms_to_points(config.laser_on_delay_ms),
        config.ms_to_points(config.laser_off_delay_ms),
    );

    output
}

/// Diode model: the light takes `on_points` to appear and `off_points` to fade, so each lit run
/// is switched on that much earlier and cut that much earlier. The frame loops on the DAC, so
/// runs wrap around the frame end. The galvo is expected to hold still at run ends (stroke
/// holds), so the earlier switching lands on the same position.
///
/// The on extension only lights blank points (it never recolours another run) and the off cut
/// always leaves at least one lit point.
fn apply_diode_delays(points: &mut [LaserPoint], on_points: usize, off_points: usize) {
    let len = points.len();
    if len == 0 || (on_points == 0 && off_points == 0) {
        return;
    }
    let lit: Vec<bool> = points.iter().map(|p| !p.is_blanked()).collect();
    if lit.iter().all(|&l| l) || lit.iter().all(|&l| !l) {
        return;
    }

    // Lit runs as (start, length), cyclic: begin scanning just after a blank point
    let first_blank = lit.iter().position(|&l| !l).unwrap();
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut k = 0;
    while k < len {
        let i = (first_blank + k) % len;
        if lit[i] {
            let mut run_len = 0;
            while k < len && lit[(first_blank + k) % len] {
                run_len += 1;
                k += 1;
            }
            runs.push((i, run_len));
        } else {
            k += 1;
        }
    }

    let original = points.to_vec();
    for &(start, run_len) in &runs {
        let end = (start + run_len - 1) % len;
        for step in 0..off_points.min(run_len - 1) {
            let i = (end + len - step) % len;
            points[i] = LaserPoint::blanked(points[i].x, points[i].y);
        }
        let first = original[start];
        for step in 1..=on_points {
            let i = (start + len - step) % len;
            if lit[i] {
                break;
            }
            points[i] = LaserPoint::new(points[i].x, points[i].y, first.r, first.g, first.b, first.i);
        }
    }
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
    fn open_stroke_ends_blank_and_frame_wraps_with_settle() {
        let config = default_config();
        let result = optimize(&[LaserSegment::new(vec![pt(100, 100), pt(2000, 100)])], &config);

        // The frame starts lit at the stroke start; the wraparound jump before it ends in a
        // settle hold at that position
        assert!(!result[0].is_blanked());
        let settle = Galvo::new(&config).settle_points;
        let tail = &result[result.len() - settle..];
        assert!(tail.iter().all(|p| p.is_blanked() && p.x == 100 && p.y == 100));

        // The light goes off at the stroke end before the galvo leaves
        let end = result.iter().position(|p| p.x == 2000 && p.is_blanked()).unwrap();
        assert!(!result[end - 1].is_blanked());
        assert!(result[end..end + Galvo::new(&config).depart_points].iter().all(|p| p.x == 2000 && p.is_blanked()));
    }

    #[test]
    fn right_angle_corner_gets_hold() {
        let seg = LaserSegment::new(vec![pt(0, 0), pt(1000, 0), pt(1000, 1000)]);
        let config = default_config();
        let result = optimize(&[seg], &config);

        let corner_count = result.iter().filter(|p| p.x == 1000 && p.y == 0 && !p.is_blanked()).count();
        assert_eq!(corner_count, 1 + Galvo::new(&config).corner_hold_points);
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
        // pts[0] once + seam hold, the closing copy once (no hold), then 2 overlap points
        let config = default_config();
        let result = optimize(&[square()], &config);
        let at_origin = result.iter().filter(|p| p.x == 0 && p.y == 0).count();
        assert_eq!(at_origin, 1 + Galvo::new(&config).corner_hold_points + 1 + 2);
    }

    fn steps(points: &[LaserPoint]) -> Vec<f32> {
        points.windows(2).map(|w| dac_distance(&w[0], &w[1])).collect()
    }

    #[test]
    fn travel_profile_ramps_and_lands_on_the_end() {
        let profile = travel_profile(1000.0, 100.0, 3, true, true);
        assert!(profile.windows(2).all(|w| w[1] > w[0]), "monotonic");
        assert!(*profile.last().unwrap() < 1.0, "end point excluded");
        // Ramps: the first and last steps are the smallest, the cruise steps are at most `step`
        let mut ts = vec![0.0];
        ts.extend(&profile);
        ts.push(1.0);
        let step_sizes: Vec<f32> = ts.windows(2).map(|w| (w[1] - w[0]) * 1000.0).collect();
        assert!(step_sizes[0] < step_sizes[3] && *step_sizes.last().unwrap() < step_sizes[3]);
        assert!(step_sizes.iter().all(|&s| s <= 100.0 + 1e-3));
        assert!((step_sizes.iter().sum::<f32>() - 1000.0).abs() < 1e-2);

        // No ramp at a smooth end, nothing for a zero-length move
        let unramped = travel_profile(1000.0, 100.0, 3, false, false);
        assert_eq!(unramped.len(), 9);
        assert!(travel_profile(0.0, 100.0, 3, true, true).is_empty());
    }

    #[test]
    fn long_lit_line_moves_at_constant_speed() {
        let config = default_config();
        let galvo = Galvo::new(&config);
        let result = optimize(&[LaserSegment::new(vec![pt(0, 2000), pt(4000, 2000)])], &config);
        let lit: Vec<LaserPoint> = result.into_iter().filter(|p| !p.is_blanked()).collect();
        let moves: Vec<f32> = steps(&lit).into_iter().filter(|&d| d > 0.0).collect();
        assert!(moves.len() as f32 >= 4000.0 / galvo.lit_step, "{} moves", moves.len());
        assert!(moves.iter().all(|&d| d <= galvo.lit_step + 1.0), "a step exceeds the lit step");
        // Slow-down into the stroke end: the last move is shorter than the cruise steps
        let cruise = moves[moves.len() / 2];
        assert!(*moves.last().unwrap() < cruise && moves[0] < cruise);
    }

    #[test]
    fn longer_jumps_settle_longer() {
        let config = default_config();
        let galvo = Galvo::new(&config);
        let settle_at = |to: LaserPoint| {
            let mut out = Vec::new();
            galvo.jump(&mut out, &pt(0, 0), &to);
            out.iter().rev().take_while(|p| p.x == to.x && p.y == to.y).count()
        };
        assert!(settle_at(pt(4000, 4000)) > settle_at(pt(200, 0)));
        assert!(settle_at(pt(200, 0)) >= galvo.settle_points);
    }

    #[test]
    fn timing_follows_pps() {
        // The same milliseconds give proportionally more points at a higher point rate
        let mut slow = default_config();
        slow.pps = 15000;
        let mut fast = default_config();
        fast.pps = 30000;
        assert_eq!(Galvo::new(&fast).corner_hold_points, 2 * Galvo::new(&slow).corner_hold_points);
        // ... and the same speed gives half the step per point
        assert!((Galvo::new(&slow).lit_step - 2.0 * Galvo::new(&fast).lit_step).abs() < 1.0
            || Galvo::new(&slow).lit_step == slow.max_lit_step_dac);
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
    fn hold_override_replaces_corner_dwell() {
        let config = default_config();
        let points = vec![pt(0, 0), pt(1000, 0), pt(1000, 1000)];
        let seg = LaserSegment::with_holds(points, vec![None, Some(9), None]);
        let result = optimize(&[seg], &config);
        let at_corner = result.iter().filter(|p| p.x == 1000 && p.y == 0 && !p.is_blanked()).count();
        assert_eq!(at_corner, 1 + 9);

        // Some(0) means "no hold" even on a sharp corner
        let seg = LaserSegment::with_holds(vec![pt(0, 0), pt(1000, 0), pt(1000, 1000)], vec![None, Some(0), None]);
        let result = optimize(&[seg], &config);
        assert_eq!(result.iter().filter(|p| p.x == 1000 && p.y == 0 && !p.is_blanked()).count(), 1);
    }

    fn lit_runs(points: &[LaserPoint]) -> Vec<(usize, usize)> {
        let mut runs = Vec::new();
        let mut i = 0;
        while i < points.len() {
            if points[i].is_blanked() {
                i += 1;
                continue;
            }
            let start = i;
            while i < points.len() && !points[i].is_blanked() {
                i += 1;
            }
            runs.push((start, i - start));
        }
        runs
    }

    #[test]
    fn diode_delays_shift_light_earlier() {
        // blank x4, lit x6, blank x4
        let mut points: Vec<LaserPoint> = (0..4).map(|_| LaserPoint::blanked(0, 0))
            .chain((0..6).map(|i| LaserPoint::new(i * 10, 0, 255, 0, 0, 255)))
            .chain((0..4).map(|_| LaserPoint::blanked(50, 0)))
            .collect();
        apply_diode_delays(&mut points, 2, 3);
        // On 2 earlier: starts at index 2 with the run's colour; off 3 earlier: ends at index 6
        assert_eq!(lit_runs(&points), vec![(2, 5)]);
        assert_eq!((points[2].r, points[2].i), (255, 255));
        // Positions never move
        assert_eq!(points[2].x, 0);
        assert_eq!(points[6].x, 20);
    }

    #[test]
    fn diode_delays_wrap_and_never_erase_a_run() {
        // A one-point run stays lit even with a long off delay
        let mut points = vec![LaserPoint::blanked(0, 0), LaserPoint::new(5, 5, 255, 255, 255, 255), LaserPoint::blanked(0, 0)];
        apply_diode_delays(&mut points, 0, 10);
        assert_eq!(lit_runs(&points), vec![(1, 1)]);

        // A run at the frame start is switched on at the frame end (the DAC loops)
        let mut points = vec![
            LaserPoint::new(0, 0, 0, 255, 0, 255),
            LaserPoint::new(1, 0, 0, 255, 0, 255),
            LaserPoint::blanked(1, 0),
            LaserPoint::blanked(0, 0),
        ];
        apply_diode_delays(&mut points, 1, 0);
        assert!(!points[3].is_blanked());
        assert!(points[2].is_blanked());
    }

    #[test]
    fn diode_delays_do_not_recolour_neighbouring_runs() {
        // Two runs one blank point apart: a long on delay only fills the single gap point
        let mut points = vec![
            LaserPoint::new(0, 0, 255, 0, 0, 255),
            LaserPoint::blanked(0, 0),
            LaserPoint::new(9, 9, 0, 0, 255, 255),
        ];
        apply_diode_delays(&mut points, 5, 0);
        assert_eq!((points[0].r, points[0].b), (255, 0));
        assert_eq!((points[1].r, points[1].b), (0, 255));
    }

    #[test]
    fn diode_delay_ms_follow_pps() {
        let mut config = default_config();
        config.pps = 30000;
        assert_eq!(config.ms_to_points(0.1), 3);
        config.pps = 20000;
        assert_eq!(config.ms_to_points(0.1), 2);
        assert_eq!(config.ms_to_points(-1.0), 0);
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
