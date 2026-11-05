use bevy::prelude::Vec2;
use serde::{Deserialize, Serialize};

/// Target wavelength color for optical isolation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LaserColor {
    /// Green laser (520–532 nm): Score = G - max(R, B)
    Green,
    /// Red laser (635–650 nm): Score = R - max(G, B)
    Red,
    /// High-intensity spot (luminance = 0.299R + 0.587G + 0.114B)
    Any,
}

impl Default for LaserColor {
    fn default() -> Self {
        Self::Green
    }
}

/// Parameters for optical laser detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionConfig {
    /// Minimum intensity score threshold (0.0 to 1.0)
    pub threshold: f32,
    /// Target laser color for spectral isolation
    pub laser_color: LaserColor,
    /// Minimum pixel area to filter out single-pixel hot sensor noise
    pub min_area: usize,
    /// Maximum pixel area to filter out large ambient light glare
    pub max_area: usize,
}

impl Default for DetectionConfig {
    fn default() -> Self {
        Self {
            threshold: 0.35, // 35% relative contrast above background
            laser_color: LaserColor::Green,
            min_area: 2,
            max_area: 400,
        }
    }
}

/// A detected laser spot with sub-pixel precision and optical metrics
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetectedSpot {
    /// Sub-pixel centroid in camera pixel coordinates (0.0 to W, 0.0 to H)
    pub pixel_pos: Vec2,
    /// Normalized coordinates (0.0 to 1.0)
    pub norm_pos: Vec2,
    /// Peak optical intensity (0.0 to 1.0)
    pub peak_intensity: f32,
    /// Total integrated optical energy (mass of spot)
    pub energy: f32,
    /// Number of pixels above threshold
    pub pixel_count: usize,
}

/// Detects laser spots in an RGB24 frame buffer using wavelength isolation and centroid moments.
///
/// `rgb`: Contiguous slice of RGB bytes (len == width * height * 3)
pub fn detect_laser_spots(
    width: u32,
    height: u32,
    rgb: &[u8],
    config: &DetectionConfig,
) -> Vec<DetectedSpot> {
    if (width * height * 3) as usize != rgb.len() || width == 0 || height == 0 {
        return Vec::new();
    }

    let w = width as usize;
    let h = height as usize;
    let threshold_val = config.threshold * 255.0;

    // Grid to track visited pixels for lightweight connected component clustering
    // To minimize heap churn, we scan for local intensity peaks above threshold
    let mut spots = Vec::new();
    let mut visited = vec![false; w * h];

    for y in 1..(h - 1) {
        for x in 1..(w - 1) {
            let idx = y * w + x;
            if visited[idx] {
                continue;
            }

            let p_idx = idx * 3;
            let r = rgb[p_idx] as f32;
            let g = rgb[p_idx + 1] as f32;
            let b = rgb[p_idx + 2] as f32;

            let score = match config.laser_color {
                LaserColor::Green => g - r.max(b),
                LaserColor::Red => r - g.max(b),
                LaserColor::Any => 0.299 * r + 0.587 * g + 0.114 * b,
            };

            if score > threshold_val {
                // Found a candidate spot; flood-fill local neighborhood to calculate sub-pixel centroid
                let mut sum_x = 0.0;
                let mut sum_y = 0.0;
                let mut total_weight = 0.0;
                let mut peak_val = score;
                let mut count = 0;

                let mut queue = vec![(x, y)];
                visited[idx] = true;

                while let Some((cx, cy)) = queue.pop() {
                    let c_idx = cy * w + cx;
                    let cp_idx = c_idx * 3;
                    let cr = rgb[cp_idx] as f32;
                    let cg = rgb[cp_idx + 1] as f32;
                    let cb = rgb[cp_idx + 2] as f32;

                    let c_score = match config.laser_color {
                        LaserColor::Green => cg - cr.max(cb),
                        LaserColor::Red => cr - cg.max(cb),
                        LaserColor::Any => 0.299 * cr + 0.587 * cg + 0.114 * cb,
                    };

                    if c_score > peak_val {
                        peak_val = c_score;
                    }

                    // Weight by intensity above baseline
                    let weight = (c_score - threshold_val * 0.5).max(0.0);
                    sum_x += cx as f64 * weight as f64;
                    sum_y += cy as f64 * weight as f64;
                    total_weight += weight as f64;
                    count += 1;

                    // Expand 4-connectivity
                    let neighbors = [
                        (cx.saturating_sub(1), cy),
                        ((cx + 1).min(w - 1), cy),
                        (cx, cy.saturating_sub(1)),
                        (cx, (cy + 1).min(h - 1)),
                    ];

                    for (nx, ny) in neighbors {
                        let n_idx = ny * w + nx;
                        if !visited[n_idx] {
                            let np_idx = n_idx * 3;
                            let nr = rgb[np_idx] as f32;
                            let ng = rgb[np_idx + 1] as f32;
                            let nb = rgb[np_idx + 2] as f32;

                            let n_score = match config.laser_color {
                                LaserColor::Green => ng - nr.max(nb),
                                LaserColor::Red => nr - ng.max(nb),
                                LaserColor::Any => 0.299 * nr + 0.587 * ng + 0.114 * nb,
                            };

                            if n_score > threshold_val * 0.7 && count < config.max_area {
                                visited[n_idx] = true;
                                queue.push((nx, ny));
                            }
                        }
                    }
                }

                if count >= config.min_area && count <= config.max_area && total_weight > 0.0 {
                    let centroid_x = (sum_x / total_weight) as f32;
                    let centroid_y = (sum_y / total_weight) as f32;

                    spots.push(DetectedSpot {
                        pixel_pos: Vec2::new(centroid_x, centroid_y),
                        norm_pos: Vec2::new(centroid_x / width as f32, centroid_y / height as f32),
                        peak_intensity: (peak_val / 255.0).clamp(0.0, 1.0),
                        energy: (total_weight / (count as f64 * 255.0)) as f32,
                        pixel_count: count,
                    });
                }
            }
        }
    }

    // Sort spots by peak intensity descending (strongest laser dot first)
    spots.sort_by(|a, b| b.peak_intensity.partial_cmp(&a.peak_intensity).unwrap());
    spots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_green_laser_detection() {
        let width = 100;
        let height = 100;
        let mut rgb = vec![10u8; width * height * 3]; // dark gray background

        // Inject a simulated bright green laser dot around (50, 40)
        let center_x = 50;
        let center_y = 40;
        for dy in -2..=2 {
            for dx in -2..=2 {
                let px = (center_x + dx) as usize;
                let py = (center_y + dy) as usize;
                let idx = (py * width + px) * 3;
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                let intensity = (255.0 * (1.0 - dist / 3.0).max(0.0)) as u8;
                rgb[idx] = 10;                // Red
                rgb[idx + 1] = intensity;     // Green
                rgb[idx + 2] = 10;            // Blue
            }
        }

        let config = DetectionConfig {
            threshold: 0.3,
            laser_color: LaserColor::Green,
            min_area: 2,
            max_area: 100,
        };

        let spots = detect_laser_spots(width as u32, height as u32, &rgb, &config);
        assert_eq!(spots.len(), 1, "Expected exactly 1 detected green laser spot");

        let spot = &spots[0];
        assert!((spot.pixel_pos.x - 50.0).abs() < 0.5, "Expected x near 50, got {}", spot.pixel_pos.x);
        assert!((spot.pixel_pos.y - 40.0).abs() < 0.5, "Expected y near 40, got {}", spot.pixel_pos.y);
        assert!(spot.peak_intensity > 0.8);
    }

    #[test]
    fn test_ambient_white_glare_rejection() {
        let width = 50;
        let height = 50;
        // Bright white light glare: R=240, G=240, B=240 (not green laser)
        let rgb = vec![240u8; width * height * 3];

        let config = DetectionConfig {
            threshold: 0.3,
            laser_color: LaserColor::Green,
            ..Default::default()
        };

        let spots = detect_laser_spots(width as u32, height as u32, &rgb, &config);
        assert_eq!(spots.len(), 0, "White glare should be rejected by wavelength isolation");
    }
}

