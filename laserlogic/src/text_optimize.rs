use crate::corner::calculate_corner_dwells;
use crate::{LaserPoint, LaserSegment, OptimizeConfig};

/// Optimize text path segments using a galvo-specific 5-stage pipeline.
///
/// 1. TSP Sort — nearest-neighbour ordering to minimize blanked jumps between glyphs
/// 2. Douglas-Peucker Simplification — removes collinear midpoints from straight strokes
/// 3. Corner & Dwell Injection — angle-proportional dwell at sharp glyph corners
/// 4. Blanking / Laser-Delay Alignment — departure and arrival dwells prevent tails & hot-spots
/// 5. Dynamic Downsampling — trims to fit point_budget by removing straight-run midpoints first
pub fn optimize_text(
    segments: &[LaserSegment],
    config: &OptimizeConfig,
    point_budget: Option<usize>,
) -> Vec<LaserPoint> {
    if segments.is_empty() {
        return vec![];
    }

    // Stage 1: TSP nearest-neighbour sort
    let sorted = tsp_nearest_neighbour(segments);

    // Stage 2: Douglas-Peucker simplification (epsilon in scene units, scaled to DAC)
    let simplified: Vec<LaserSegment> = sorted
        .iter()
        .map(|s| douglas_peucker_segment(s, 0.003_f32))
        .collect();

    // Stages 3 + 4: Emit with corner dwells and blanking jumps
    let mut output: Vec<LaserPoint> = Vec::new();

    for seg in &simplified {
        let pts = &seg.points;
        if pts.is_empty() {
            continue;
        }

        let is_closed = pts.len() > 2 && {
            let dx = pts[0].x as i32 - pts.last().unwrap().x as i32;
            let dy = pts[0].y as i32 - pts.last().unwrap().y as i32;
            (dx * dx + dy * dy) <= 1600
        };

        let corner_dwells = calculate_corner_dwells(
            pts,
            config.corner_angle_threshold,
            config.corner_dwell_points as usize,
        );

        // Stage 4a: Blanking jump from previous segment
        if !output.is_empty() {
            let last = *output.last().unwrap();
            let first = pts[0];
            emit_text_blanking_jump(&mut output, &last, &first, config);
        }

        // Stage 4b: Arrival dwell before laser fires
        if !is_closed {
            let first = pts[0];
            for _ in 0..config.start_dwell_points {
                output.push(LaserPoint::blanked(first.x, first.y));
            }
        }

        // Stage 3: Emit lit points with corner dwells
        for i in 0..pts.len() {
            if i > 0 {
                emit_text_interpolated(&mut output, &pts[i - 1], &pts[i], config);
            }
            let p = pts[i];
            output.push(p);
            for _ in 0..corner_dwells[i] {
                output.push(p);
            }
        }

        // Stage 4c: Departure dwell after laser fires
        if !is_closed {
            let last = pts[pts.len() - 1];
            for _ in 0..config.end_dwell_points {
                output.push(LaserPoint::blanked(last.x, last.y));
            }
        }
    }

    // Frame wrap-around blanking
    if !output.is_empty() {
        let last = *output.last().unwrap();
        let first = output[0];
        if last.x != first.x || last.y != first.y {
            emit_text_blanking_jump(&mut output, &last, &first, config);
        }
    }

    // Stage 5: Dynamic downsampling to point budget
    if let Some(budget) = point_budget {
        if output.len() > budget {
            output = downsample_to_budget(output, budget);
        }
    }

    // 2-point laser color delay to compensate galvo inertia lag
    apply_laser_color_delay(&mut output, 2);

    output
}

// Stage 1: Nearest-Neighbour TSP Sort

fn tsp_nearest_neighbour(segments: &[LaserSegment]) -> Vec<&LaserSegment> {
    if segments.is_empty() {
        return vec![];
    }
    let mut remaining: Vec<usize> = (0..segments.len()).collect();
    let mut sorted = Vec::with_capacity(segments.len());

    let start_idx = remaining
        .iter()
        .copied()
        .min_by_key(|&i| {
            let p = &segments[i].points[0];
            (p.x as i32) * (p.x as i32) + (p.y as i32) * (p.y as i32)
        })
        .unwrap();
    sorted.push(start_idx);
    remaining.retain(|&i| i != start_idx);

    while !remaining.is_empty() {
        let last_end = segments[*sorted.last().unwrap()].points.last().unwrap();
        let next_idx = *remaining
            .iter()
            .min_by_key(|&&i| {
                let p = &segments[i].points[0];
                let dx = p.x as i64 - last_end.x as i64;
                let dy = p.y as i64 - last_end.y as i64;
                dx * dx + dy * dy
            })
            .unwrap();
        sorted.push(next_idx);
        remaining.retain(|&i| i != next_idx);
    }
    sorted.iter().map(|&i| &segments[i]).collect()
}

// Stage 2: Douglas-Peucker Simplification

fn douglas_peucker_segment(seg: &LaserSegment, epsilon: f32) -> LaserSegment {
    if seg.points.len() <= 2 {
        return seg.clone();
    }
    let epsilon_dac = epsilon * 4096.0;
    let mut keep = vec![false; seg.points.len()];
    keep[0] = true;
    *keep.last_mut().unwrap() = true;
    dp_recursive(&seg.points, 0, seg.points.len() - 1, epsilon_dac, &mut keep);
    LaserSegment::new(
        seg.points
            .iter()
            .enumerate()
            .filter(|(i, _)| keep[*i])
            .map(|(_, p)| *p)
            .collect(),
    )
}

fn dp_recursive(pts: &[LaserPoint], start: usize, end: usize, epsilon: f32, keep: &mut Vec<bool>) {
    if end <= start + 1 {
        return;
    }
    let ax = pts[start].x as f32;
    let ay = pts[start].y as f32;
    let bx = pts[end].x as f32;
    let by = pts[end].y as f32;
    let len_sq = (bx - ax) * (bx - ax) + (by - ay) * (by - ay);
    let mut max_dist = 0.0_f32;
    let mut max_idx = start;

    for i in (start + 1)..end {
        let px = pts[i].x as f32;
        let py = pts[i].y as f32;
        let dist = if len_sq < 1e-6 {
            ((px - ax).powi(2) + (py - ay).powi(2)).sqrt()
        } else {
            let t = ((px - ax) * (bx - ax) + (py - ay) * (by - ay)) / len_sq;
            let t = t.clamp(0.0, 1.0);
            let proj_x = ax + t * (bx - ax);
            let proj_y = ay + t * (by - ay);
            ((px - proj_x).powi(2) + (py - proj_y).powi(2)).sqrt()
        };
        if dist > max_dist {
            max_dist = dist;
            max_idx = i;
        }
    }

    if max_dist > epsilon {
        keep[max_idx] = true;
        dp_recursive(pts, start, max_idx, epsilon, keep);
        dp_recursive(pts, max_idx, end, epsilon, keep);
    }
}

// Stage 4 helpers

fn emit_text_blanking_jump(
    output: &mut Vec<LaserPoint>,
    from: &LaserPoint,
    to: &LaserPoint,
    config: &OptimizeConfig,
) {
    for _ in 0..config.blank_end_dwell {
        output.push(LaserPoint::blanked(from.x, from.y));
    }
    let dx = to.x as f32 - from.x as f32;
    let dy = to.y as f32 - from.y as f32;
    let dist = (dx * dx + dy * dy).sqrt();
    let max_steps = config.blank_jump_steps.max(4);
    let steps = ((dist / 250.0).ceil() as u16).clamp(4, max_steps);
    for step in 1..steps {
        let t = step as f32 / steps as f32;
        output.push(LaserPoint::blanked(
            (from.x as f32 + dx * t) as u16,
            (from.y as f32 + dy * t) as u16,
        ));
    }
    for _ in 0..config.blank_start_dwell {
        output.push(LaserPoint::blanked(to.x, to.y));
    }
}

fn emit_text_interpolated(
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
        output.push(LaserPoint::new(
            (from.x as f32 + dx * t) as u16,
            (from.y as f32 + dy * t) as u16,
            from.r, from.g, from.b, from.i,
        ));
    }
}

// Stage 5: Dynamic downsampling

fn downsample_to_budget(mut pts: Vec<LaserPoint>, budget: usize) -> Vec<LaserPoint> {
    let mut step = 2usize;
    while pts.len() > budget && step < pts.len() {
        let len = pts.len();
        let mut next = Vec::with_capacity(len);
        for (i, p) in pts.iter().enumerate() {
            if p.is_blanked() || i == 0 || i == len - 1 || i % step != 0 {
                next.push(*p);
            }
        }
        pts = next;
        step += 1;
    }
    pts
}

fn apply_laser_color_delay(points: &mut [LaserPoint], delay_points: usize) {
    let len = points.len();
    if len <= delay_points || delay_points == 0 {
        return;
    }
    let colors: Vec<(u8, u8, u8, u8)> = points.iter().map(|p| (p.r, p.g, p.b, p.i)).collect();
    for i in 0..len {
        // Never shift color into a blanked travel point — prevents tail artifacts
        if points[i].is_blanked() {
            continue;
        }
        let src = (i + len - delay_points) % len;
        let (r, g, b, intensity) = colors[src];
        // Don't pull blank-black into a lit zone
        if r == 0 && g == 0 && b == 0 && intensity == 0 {
            continue;
        }
        points[i].r = r;
        points[i].g = g;
        points[i].b = b;
        points[i].i = intensity;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LaserPoint, LaserSegment, OptimizeConfig};

    fn pt(x: u16, y: u16) -> LaserPoint {
        LaserPoint::new(x, y, 255, 255, 255, 255)
    }

    #[test]
    fn optimize_text_empty() {
        assert!(optimize_text(&[], &OptimizeConfig::default(), None).is_empty());
    }

    #[test]
    fn tsp_sorts_nearest() {
        let a = LaserSegment::new(vec![pt(0, 0), pt(100, 0)]);
        let c = LaserSegment::new(vec![pt(3000, 3000), pt(3100, 3000)]);
        let b = LaserSegment::new(vec![pt(110, 0), pt(200, 0)]);
        let binding = [a, c, b];
        let sorted = tsp_nearest_neighbour(&binding);
        let b_pos = sorted.iter().position(|s| s.points[0].x == 110).unwrap();
        let c_pos = sorted.iter().position(|s| s.points[0].x == 3000).unwrap();
        assert!(b_pos < c_pos, "B should be visited before C");
    }

    #[test]
    fn dp_removes_collinear_midpoints() {
        let seg = LaserSegment::new(vec![
            pt(0, 0), pt(500, 0), pt(1000, 0), pt(1500, 0), pt(2000, 0),
        ]);
        let result = douglas_peucker_segment(&seg, 0.001);
        assert!(result.points.len() <= 3, "got {} points", result.points.len());
    }

    #[test]
    fn budget_trim_reduces_points() {
        let seg = LaserSegment::new((0..200u16).map(|i| pt(i * 20, 0)).collect());
        let result = optimize_text(&[seg], &OptimizeConfig::default(), Some(50));
        assert!(result.len() <= 70, "got {} points", result.len());
    }
}

