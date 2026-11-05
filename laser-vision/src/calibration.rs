//! Camera ↔ projector calibration maths: the dot pattern, robust per-dot measurement and the
//! camera → projector fit. The server projects the dots and collects camera samples; everything
//! here is plain functions so it is testable without hardware.

use bevy::prelude::Vec2;

use crate::homography::Homography;

/// `cols` × `rows` points spread evenly over the unit square with `margin` left free at each edge,
/// in serpentine order (left to right, then right to left on the next row) so the galvo only ever
/// steps to a neighbour.
pub fn grid_pattern(cols: u32, rows: u32, margin: f32) -> Vec<Vec2> {
    let (cols, rows) = (cols.max(2), rows.max(2));
    let span = 1.0 - 2.0 * margin;
    let mut points = Vec::with_capacity((cols * rows) as usize);
    for row in 0..rows {
        let y = margin + span * row as f32 / (rows - 1) as f32;
        let mut line: Vec<Vec2> = (0..cols).map(|col| Vec2::new(margin + span * col as f32 / (cols - 1) as f32, y)).collect();
        if row % 2 == 1 {
            line.reverse();
        }
        points.extend(line);
    }
    points
}

/// Per-axis median of the samples: one measured position that ignores the odd stray reflection.
pub fn median(samples: &[Vec2]) -> Option<Vec2> {
    if samples.is_empty() {
        return None;
    }
    let mid = |mut values: Vec<f32>| {
        values.sort_by(f32::total_cmp);
        let n = values.len();
        if n % 2 == 1 { values[n / 2] } else { (values[n / 2 - 1] + values[n / 2]) / 2.0 }
    };
    Some(Vec2::new(mid(samples.iter().map(|p| p.x).collect()), mid(samples.iter().map(|p| p.y).collect())))
}

/// A solved calibration.
#[derive(Debug, Clone)]
pub struct CalibrationFit {
    /// Normalised camera coordinates → projector coordinates
    pub homography: Homography,
    /// Indices into the input pairs that the fit used (outliers left out)
    pub used: Vec<usize>,
    /// Where each input pair's camera point lands in projector space under the fit
    pub predicted: Vec<Vec2>,
}

/// Fewest points a fit accepts: 4 define a homography, a 5th makes the error measurable
pub const MIN_CALIBRATION_POINTS: usize = 5;

/// Fits camera → projector over measured `(camera, projector)` pairs. A pair whose error is far
/// above the rest (a reflection picked up instead of the dot) is dropped once and the fit
/// repeated. `None` with fewer than [`MIN_CALIBRATION_POINTS`] usable pairs.
pub fn fit_calibration(pairs: &[(Vec2, Vec2)]) -> Option<CalibrationFit> {
    let fit_over = |indices: &[usize]| {
        let camera: Vec<Vec2> = indices.iter().map(|&i| pairs[i].0).collect();
        let projector: Vec<Vec2> = indices.iter().map(|&i| pairs[i].1).collect();
        Homography::fit(&camera, &projector)
    };
    let errors = |h: &Homography| -> Vec<f32> { pairs.iter().map(|(c, p)| h.transform_point(*c).distance(*p)).collect() };

    let mut used: Vec<usize> = (0..pairs.len()).collect();
    if used.len() < MIN_CALIBRATION_POINTS {
        return None;
    }
    let mut homography = fit_over(&used)?;

    let first = errors(&homography);
    let mut sorted = first.clone();
    sorted.sort_by(f32::total_cmp);
    let typical = sorted[sorted.len() / 2].max(0.01);
    let kept: Vec<usize> = used.iter().copied().filter(|&i| first[i] <= typical * 4.0).collect();
    if kept.len() < used.len() && kept.len() >= MIN_CALIBRATION_POINTS {
        if let Some(refit) = fit_over(&kept) {
            homography = refit;
            used = kept;
        }
    }

    let predicted = pairs.iter().map(|(c, _)| homography.transform_point(*c)).collect();
    Some(CalibrationFit { homography, used, predicted })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_covers_the_square_inside_the_margin_in_serpentine_order() {
        let grid = grid_pattern(3, 2, 0.1);
        assert_eq!(grid.len(), 6);
        let near = |a: Vec2, b: Vec2| a.distance(b) < 1e-5;
        assert!(near(grid[0], Vec2::new(0.1, 0.1)));
        assert!(near(grid[2], Vec2::new(0.9, 0.1)));
        assert!(near(grid[3], Vec2::new(0.9, 0.9)), "second row starts where the first ended");
        assert!(grid.windows(2).all(|w| w[0].distance(w[1]) < 0.9), "only neighbour steps");
    }

    #[test]
    fn median_ignores_a_stray_sample() {
        let samples = [Vec2::new(0.5, 0.5), Vec2::new(0.51, 0.49), Vec2::new(0.9, 0.1), Vec2::new(0.49, 0.5), Vec2::new(0.5, 0.51)];
        assert!(median(&samples).unwrap().distance(Vec2::new(0.5, 0.5)) < 0.011);
        assert_eq!(median(&[]), None);
    }

    #[test]
    fn fit_drops_a_bad_measurement() {
        let truth = Homography::from_four_points(
            &[Vec2::new(0.1, 0.1), Vec2::new(0.9, 0.12), Vec2::new(0.88, 0.9), Vec2::new(0.12, 0.85)],
            &[Vec2::new(-1.0, -1.0), Vec2::new(1.0, -1.0), Vec2::new(1.0, 1.0), Vec2::new(-1.0, 1.0)],
        )
        .unwrap();
        let inverse = truth.inverse().unwrap();
        let mut pairs: Vec<(Vec2, Vec2)> = grid_pattern(3, 3, 0.2)
            .into_iter()
            .map(|uv| uv * 2.0 - Vec2::ONE)
            .map(|projector| (inverse.transform_point(projector), projector))
            .collect();
        pairs[4].0 += Vec2::new(0.2, -0.1); // a reflection measured instead of the centre dot

        let fit = fit_calibration(&pairs).expect("fit");
        assert_eq!(fit.used.len(), 8, "the bad point is left out");
        assert!(!fit.used.contains(&4));
        for (i, (_, projector)) in pairs.iter().enumerate().filter(|(i, _)| *i != 4) {
            assert!(fit.predicted[i].distance(*projector) < 1e-3, "point {i}");
        }
    }

    #[test]
    fn fit_needs_enough_points() {
        let pairs: Vec<(Vec2, Vec2)> = grid_pattern(2, 2, 0.1).into_iter().map(|p| (p, p)).collect();
        assert!(fit_calibration(&pairs).is_none());
    }
}
