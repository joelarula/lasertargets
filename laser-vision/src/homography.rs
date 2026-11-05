use bevy::prelude::Vec2;
use serde::{Deserialize, Serialize};

/// 3x3 Planar Homography matrix representing a 2D projective transformation.
/// Maps points between camera pixel space and normalized laser projector space [-1.0, 1.0].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Homography {
    /// 3x3 row-major elements:
    /// [ h00, h01, h02,
    ///   h10, h11, h12,
    ///   h20, h21, h22 ]
    pub data: [f32; 9],
}

impl Default for Homography {
    fn default() -> Self {
        Self::identity()
    }
}

impl Homography {
    /// Identity transformation (maps (x, y) -> (x, y))
    pub const fn identity() -> Self {
        Self {
            data: [
                1.0, 0.0, 0.0,
                0.0, 1.0, 0.0,
                0.0, 0.0, 1.0,
            ],
        }
    }

    /// Computes a homography matrix from 4 source points to 4 destination points
    /// using Direct Linear Transformation (DLT) with Gaussian elimination.
    ///
    /// `src`: 4 points in source coordinates (e.g. camera sensor [0..W, 0..H])
    /// `dst`: 4 matching points in destination coordinates (e.g. galvo space [-1.0..1.0])
    pub fn from_four_points(src: &[Vec2; 4], dst: &[Vec2; 4]) -> Option<Self> {
        // Set up 8x8 system Ah = b where h = [h00, h01, h02, h10, h11, h12, h20, h21]^T
        // and h22 is normalized to 1.0
        let mut a = [[0.0f64; 8]; 8];
        let mut b = [0.0f64; 8];

        for i in 0..4 {
            let x = src[i].x as f64;
            let y = src[i].y as f64;
            let u = dst[i].x as f64;
            let v = dst[i].y as f64;

            let row1 = 2 * i;
            let row2 = 2 * i + 1;

            // Row 1: h00*x + h01*y + h02 - h20*x*u - h21*y*u = u
            a[row1][0] = x;
            a[row1][1] = y;
            a[row1][2] = 1.0;
            a[row1][3] = 0.0;
            a[row1][4] = 0.0;
            a[row1][5] = 0.0;
            a[row1][6] = -u * x;
            a[row1][7] = -u * y;
            b[row1] = u;

            // Row 2: h10*x + h11*y + h12 - h20*x*v - h21*y*v = v
            a[row2][0] = 0.0;
            a[row2][1] = 0.0;
            a[row2][2] = 0.0;
            a[row2][3] = x;
            a[row2][4] = y;
            a[row2][5] = 1.0;
            a[row2][6] = -v * x;
            a[row2][7] = -v * y;
            b[row2] = v;
        }

        // Solve using Gaussian elimination with partial pivoting
        let h_sol = solve_8x8(&mut a, &mut b)?;

        Some(Self {
            data: [
                h_sol[0] as f32, h_sol[1] as f32, h_sol[2] as f32,
                h_sol[3] as f32, h_sol[4] as f32, h_sol[5] as f32,
                h_sol[6] as f32, h_sol[7] as f32, 1.0,
            ],
        })
    }

    /// Least-squares homography over 4 or more point pairs (`src[i]` → `dst[i]`), e.g. measured
    /// calibration dots. Points are normalised first (Hartley), so camera and projector scales
    /// don't hurt the conditioning. `None` for fewer than 4 pairs or a degenerate layout.
    pub fn fit(src: &[Vec2], dst: &[Vec2]) -> Option<Self> {
        if src.len() != dst.len() || src.len() < 4 {
            return None;
        }
        let (src_norm, src_t) = normalise(src)?;
        let (dst_norm, dst_t) = normalise(dst)?;

        // Normal equations (AᵀA) h = Aᵀb of the DLT rows, with h22 fixed to 1
        let mut ata = [[0.0f64; 8]; 8];
        let mut atb = [0.0f64; 8];
        for (s, d) in src_norm.iter().zip(&dst_norm) {
            let (x, y, u, v) = (s[0], s[1], d[0], d[1]);
            let rows = [
                ([x, y, 1.0, 0.0, 0.0, 0.0, -u * x, -u * y], u),
                ([0.0, 0.0, 0.0, x, y, 1.0, -v * x, -v * y], v),
            ];
            for (row, rhs) in rows {
                for i in 0..8 {
                    for j in 0..8 {
                        ata[i][j] += row[i] * row[j];
                    }
                    atb[i] += row[i] * rhs;
                }
            }
        }
        let h = solve_8x8(&mut ata, &mut atb)?;
        let normalised = [h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7], 1.0];

        // Undo the normalisation: H = T_dst⁻¹ · H' · T_src
        let full = mul3(&mul3(&invert_similarity(&dst_t), &normalised), &src_t);
        let scale = if full[8].abs() > 1e-12 { full[8] } else { 1.0 };
        Some(Self { data: full.map(|v| (v / scale) as f32) })
    }

    /// Transforms a point from source space into destination space
    pub fn transform_point(&self, p: Vec2) -> Vec2 {
        let x = p.x;
        let y = p.y;
        let d = &self.data;

        let w = d[6] * x + d[7] * y + d[8];
        if w.abs() < 1e-7 {
            return Vec2::ZERO;
        }

        let nx = (d[0] * x + d[1] * y + d[2]) / w;
        let ny = (d[3] * x + d[4] * y + d[5]) / w;
        Vec2::new(nx, ny)
    }

    /// Calculates the inverse homography (destination space back to source space)
    pub fn inverse(&self) -> Option<Self> {
        let m = &self.data;
        // Determinant of 3x3 matrix
        let det = m[0] * (m[4] * m[8] - m[5] * m[7])
            - m[1] * (m[3] * m[8] - m[5] * m[6])
            + m[2] * (m[3] * m[7] - m[4] * m[6]);

        if det.abs() < 1e-9 {
            return None;
        }
        let inv_det = 1.0 / det;

        Some(Self {
            data: [
                (m[4] * m[8] - m[5] * m[7]) * inv_det,
                (m[2] * m[7] - m[1] * m[8]) * inv_det,
                (m[1] * m[5] - m[2] * m[4]) * inv_det,

                (m[5] * m[6] - m[3] * m[8]) * inv_det,
                (m[0] * m[8] - m[2] * m[6]) * inv_det,
                (m[2] * m[3] - m[0] * m[5]) * inv_det,

                (m[3] * m[7] - m[4] * m[6]) * inv_det,
                (m[1] * m[6] - m[0] * m[7]) * inv_det,
                (m[0] * m[4] - m[1] * m[3]) * inv_det,
            ],
        })
    }
}

/// Moves points to their centroid and scales them to a mean distance of √2. Returns the points
/// and the 3x3 similarity that does it (row-major), or `None` when all points coincide.
fn normalise(points: &[Vec2]) -> Option<(Vec<[f64; 2]>, [f64; 9])> {
    let n = points.len() as f64;
    let cx = points.iter().map(|p| p.x as f64).sum::<f64>() / n;
    let cy = points.iter().map(|p| p.y as f64).sum::<f64>() / n;
    let mean_distance = points.iter().map(|p| ((p.x as f64 - cx).powi(2) + (p.y as f64 - cy).powi(2)).sqrt()).sum::<f64>() / n;
    if mean_distance < 1e-12 {
        return None;
    }
    let s = std::f64::consts::SQRT_2 / mean_distance;
    let normalised = points.iter().map(|p| [(p.x as f64 - cx) * s, (p.y as f64 - cy) * s]).collect();
    Some((normalised, [s, 0.0, -s * cx, 0.0, s, -s * cy, 0.0, 0.0, 1.0]))
}

/// Inverse of a [`normalise`] similarity (uniform scale + translation)
fn invert_similarity(t: &[f64; 9]) -> [f64; 9] {
    let s = t[0];
    [1.0 / s, 0.0, -t[2] / s, 0.0, 1.0 / s, -t[5] / s, 0.0, 0.0, 1.0]
}

fn mul3(a: &[f64; 9], b: &[f64; 9]) -> [f64; 9] {
    let mut out = [0.0; 9];
    for row in 0..3 {
        for col in 0..3 {
            out[row * 3 + col] = (0..3).map(|k| a[row * 3 + k] * b[k * 3 + col]).sum();
        }
    }
    out
}

/// Solves Ax = b for an 8x8 matrix via Gaussian elimination with partial pivoting
fn solve_8x8(a: &mut [[f64; 8]; 8], b: &mut [f64; 8]) -> Option<[f64; 8]> {
    const N: usize = 8;
    for i in 0..N {
        // Find pivot
        let mut max_row = i;
        let mut max_val = a[i][i].abs();
        for k in (i + 1)..N {
            let val = a[k][i].abs();
            if val > max_val {
                max_val = val;
                max_row = k;
            }
        }

        if max_val < 1e-12 {
            return None; // Singular or collinear
        }

        // Swap rows
        a.swap(i, max_row);
        b.swap(i, max_row);

        // Eliminate below
        let pivot = a[i][i];
        for k in (i + 1)..N {
            let factor = a[k][i] / pivot;
            for j in i..N {
                a[k][j] -= factor * a[i][j];
            }
            b[k] -= factor * b[i];
        }
    }

    // Back-substitution
    let mut x = [0.0; N];
    for i in (0..N).rev() {
        let mut sum = b[i];
        for j in (i + 1)..N {
            sum -= a[i][j] * x[j];
        }
        x[i] = sum / a[i][i];
    }

    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_transform() {
        let h = Homography::identity();
        let pt = Vec2::new(10.0, 25.0);
        assert!((h.transform_point(pt) - pt).length() < 1e-5);
    }

    #[test]
    fn test_four_point_quad_mapping() {
        let src = [
            Vec2::new(0.0, 0.0),
            Vec2::new(640.0, 0.0),
            Vec2::new(640.0, 480.0),
            Vec2::new(0.0, 480.0),
        ];
        let dst = [
            Vec2::new(-1.0, 1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(-1.0, -1.0),
        ];

        let h = Homography::from_four_points(&src, &dst).expect("Failed to compute homography");

        // Verify the 4 corners map correctly
        for i in 0..4 {
            let mapped = h.transform_point(src[i]);
            assert!(
                (mapped - dst[i]).length() < 1e-3,
                "Point {i}: expected {:?}, got {:?}",
                dst[i],
                mapped
            );
        }

        // Center should map to (0, 0)
        let center = h.transform_point(Vec2::new(320.0, 240.0));
        assert!(center.length() < 1e-3, "Center expected (0,0), got {:?}", center);
    }

    #[test]
    fn fit_recovers_a_perspective_mapping_from_many_points() {
        // A tilted, shifted camera view of the projector
        let truth = Homography::from_four_points(
            &[Vec2::new(0.1, 0.15), Vec2::new(0.85, 0.1), Vec2::new(0.9, 0.9), Vec2::new(0.15, 0.8)],
            &[Vec2::new(-1.0, -1.0), Vec2::new(1.0, -1.0), Vec2::new(1.0, 1.0), Vec2::new(-1.0, 1.0)],
        )
        .unwrap();
        let src: Vec<Vec2> = (0..5).flat_map(|i| (0..4).map(move |j| Vec2::new(0.2 + 0.15 * i as f32, 0.2 + 0.18 * j as f32))).collect();
        let dst: Vec<Vec2> = src.iter().map(|p| truth.transform_point(*p)).collect();

        let fitted = Homography::fit(&src, &dst).expect("fit");
        for p in [Vec2::new(0.5, 0.5), Vec2::new(0.12, 0.88), Vec2::new(0.95, 0.05)] {
            assert!(fitted.transform_point(p).distance(truth.transform_point(p)) < 1e-3, "{p}");
        }
    }

    #[test]
    fn fit_rejects_too_few_or_degenerate_points() {
        let three = [Vec2::ZERO, Vec2::X, Vec2::Y];
        assert!(Homography::fit(&three, &three).is_none());
        let collinear: Vec<Vec2> = (0..5).map(|i| Vec2::new(i as f32, i as f32)).collect();
        assert!(Homography::fit(&collinear, &collinear).is_none());
    }
}

