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
}

