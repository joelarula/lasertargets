use crate::corner::calculate_corner_dwells;
use crate::{LaserPoint, LaserSegment, OptimizeConfig};
use crate::optimize::{emit_blanking_jump, emit_interpolated_points};

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
            emit_blanking_jump(&mut output, &last, &first, config);
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
                emit_interpolated_points(&mut output, &pts[i - 1], &pts[i], config);
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
            emit_blanking_jump(&mut output, &last, &first, config);
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

    #[test]
    fn tsp_starts_nearest_origin() {
        let far = LaserSegment::new(vec![pt(3000, 3000), pt(3100, 3000)]);
        let near = LaserSegment::new(vec![pt(50, 50), pt(100, 50)]);
        let binding = [far, near];
        let sorted = tsp_nearest_neighbour(&binding);
        assert_eq!(sorted[0].points[0], pt(50, 50));
    }

    #[test]
    fn tsp_keeps_every_segment_once() {
        let segs: Vec<LaserSegment> = (0..6u16)
            .map(|i| LaserSegment::new(vec![pt(i * 500, 4000 - i * 500), pt(i * 500 + 10, 4000 - i * 500)]))
            .collect();
        let sorted = tsp_nearest_neighbour(&segs);
        assert_eq!(sorted.len(), segs.len());
        let mut starts: Vec<u16> = sorted.iter().map(|s| s.points[0].x).collect();
        starts.sort();
        assert_eq!(starts, vec![0, 500, 1000, 1500, 2000, 2500]);
    }

    #[test]
    fn dp_keeps_sharp_corner() {
        let seg = LaserSegment::new(vec![
            pt(0, 0), pt(500, 0), pt(1000, 0), pt(1000, 500), pt(1000, 1000),
        ]);
        let result = douglas_peucker_segment(&seg, 0.003);
        assert_eq!(result.points, vec![pt(0, 0), pt(1000, 0), pt(1000, 1000)]);
    }

    #[test]
    fn dp_short_segments_unchanged() {
        let seg = LaserSegment::new(vec![pt(0, 0), pt(10, 10)]);
        assert_eq!(douglas_peucker_segment(&seg, 0.003).points, seg.points);
    }

    #[test]
    fn dp_closed_loop_keeps_shape() {
        // start == end makes the baseline degenerate; distance falls back to point distance
        let seg = LaserSegment::new(vec![pt(0, 0), pt(1000, 0), pt(1000, 1000), pt(0, 1000), pt(0, 0)]);
        let result = douglas_peucker_segment(&seg, 0.003);
        assert_eq!(result.points, seg.points);
    }

    #[test]
    fn downsample_keeps_blanked_and_endpoints() {
        let mut pts: Vec<LaserPoint> = (0..100u16).map(|i| pt(i, 0)).collect();
        pts[10] = LaserPoint::blanked(10, 0);
        pts[20] = LaserPoint::blanked(20, 0);
        let result = downsample_to_budget(pts.clone(), 40);
        assert!(result.len() <= 40, "got {}", result.len());
        assert_eq!(result[0], pts[0]);
        assert_eq!(*result.last().unwrap(), pts[99]);
        assert_eq!(result.iter().filter(|p| p.is_blanked()).count(), 2);
    }

    #[test]
    fn downsample_noop_under_budget() {
        let pts: Vec<LaserPoint> = (0..10u16).map(|i| pt(i, 0)).collect();
        assert_eq!(downsample_to_budget(pts.clone(), 50), pts);
    }

    #[test]
    fn closed_glyph_has_no_dwell_blanking() {
        let ring = LaserSegment::new(vec![pt(0, 0), pt(1000, 0), pt(1000, 1000), pt(0, 1000), pt(0, 0)]);
        let result = optimize_text(&[ring], &OptimizeConfig::default(), None);
        assert!(result.iter().all(|p| !p.is_blanked()));
    }

    #[test]
    fn text_frame_wraps_back_to_start() {
        let a = LaserSegment::new(vec![pt(0, 0), pt(300, 600)]);
        let b = LaserSegment::new(vec![pt(800, 0), pt(1100, 600)]);
        let result = optimize_text(&[a, b], &OptimizeConfig::default(), None);
        let first = result[0];
        let last = *result.last().unwrap();
        assert_eq!((first.x, first.y), (last.x, last.y));
    }
}

