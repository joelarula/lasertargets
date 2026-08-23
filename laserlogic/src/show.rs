use crate::morph::{lerp_point, resample_segment};
use crate::{LaserPoint, LaserSegment};
use serde::{Deserialize, Serialize};

/// Easing functions for keyframe laser show timeline transitions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ShowEasing {
    #[default]
    Linear,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
}

impl ShowEasing {
    pub fn apply(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::EaseInQuad => t * t,
            Self::EaseOutQuad => t * (2.0 - t),
            Self::EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
        }
    }
}

/// A single keyframe in a laser show animation track
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowKeyframe {
    /// Timestamp in seconds from track start
    pub time_secs: f32,
    /// Vector shape template name (e.g. "star", "circle", "square", "crosshair", "diamond")
    #[serde(default = "default_shape")]
    pub shape_template: String,
    /// 2D Translation [X, Y] in world meters (-1.0 to 1.0)
    #[serde(default = "default_position")]
    pub position: [f32; 2],
    /// 2D Scale factor [ScaleX, ScaleY]
    #[serde(default = "default_scale")]
    pub scale: [f32; 2],
    /// 2D Rotation angle in degrees
    #[serde(default)]
    pub rotation_deg: f32,
    /// RGB color tint [R, G, B] (0 - 255)
    #[serde(default = "default_color")]
    pub color: [u8; 3],
    /// Laser power intensity (0 - 255)
    #[serde(default = "default_intensity")]
    pub intensity: u8,
    /// Corner dwell repeat count hint (0 - 8)
    #[serde(default = "default_dwell")]
    pub dwell: u8,
    /// Easing curve to transition into the NEXT keyframe
    #[serde(default)]
    pub easing: ShowEasing,
}

fn default_shape() -> String { "circle".to_string() }
fn default_position() -> [f32; 2] { [0.0, 0.0] }
fn default_scale() -> [f32; 2] { [1.0, 1.0] }
fn default_color() -> [u8; 3] { [255, 255, 255] }
fn default_intensity() -> u8 { 255 }
fn default_dwell() -> u8 { 3 }

impl Default for ShowKeyframe {
    fn default() -> Self {
        Self {
            time_secs: 0.0,
            shape_template: default_shape(),
            position: default_position(),
            scale: default_scale(),
            rotation_deg: 0.0,
            color: default_color(),
            intensity: default_intensity(),
            dwell: default_dwell(),
            easing: ShowEasing::Linear,
        }
    }
}

/// A track in a laser show containing a list of ordered keyframes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowTrack {
    pub name: String,
    #[serde(default = "default_visible")]
    pub visible: bool,
    #[serde(default)]
    pub keyframes: Vec<ShowKeyframe>,
}

fn default_visible() -> bool { true }

impl Default for ShowTrack {
    fn default() -> Self {
        Self {
            name: "Track 1".to_string(),
            visible: true,
            keyframes: Vec::new(),
        }
    }
}

/// A complete JSON-serializable Laser Show / Movie timeline document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaserShow {
    pub title: String,
    #[serde(default)]
    pub author: String,
    pub duration_secs: f32,
    #[serde(default = "default_fps")]
    pub fps: u32,
    #[serde(default = "default_loop")]
    pub loop_show: bool,
    #[serde(default)]
    pub tracks: Vec<ShowTrack>,
}

fn default_fps() -> u32 { 30 }
fn default_loop() -> bool { true }

impl Default for LaserShow {
    fn default() -> Self {
        Self {
            title: "Untitled Laser Show".to_string(),
            author: "LaserTargets Studio".to_string(),
            duration_secs: 10.0,
            fps: 30,
            loop_show: true,
            tracks: Vec::new(),
        }
    }
}

impl LaserShow {
    /// Load a LaserShow timeline document from JSON string
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    /// Serialize LaserShow timeline to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Linear float lerp helper
fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Evaluates a single keyframe track at timestamp `time_secs`, returning the transformed LaserSegment
pub fn evaluate_track(track: &ShowTrack, time_secs: f32) -> Option<LaserSegment> {
    if !track.visible || track.keyframes.is_empty() {
        return None;
    }

    let kfs = &track.keyframes;

    if time_secs <= kfs[0].time_secs {
        return Some(render_keyframe_segment(&kfs[0]));
    }
    if time_secs >= kfs.last()?.time_secs {
        return Some(render_keyframe_segment(kfs.last()?));
    }

    let mut idx_a = 0;
    for (i, kf) in kfs.iter().enumerate() {
        if kf.time_secs <= time_secs {
            idx_a = i;
        } else {
            break;
        }
    }
    let idx_b = (idx_a + 1).min(kfs.len() - 1);
    let kf_a = &kfs[idx_a];
    let kf_b = &kfs[idx_b];

    let dt = kf_b.time_secs - kf_a.time_secs;
    let raw_t = if dt <= 0.0001 { 0.0 } else { ((time_secs - kf_a.time_secs) / dt).clamp(0.0, 1.0) };
    let eased_t = kf_a.easing.apply(raw_t);

    let seg_a = render_keyframe_segment(kf_a);
    let seg_b = render_keyframe_segment(kf_b);

    let max_len = seg_a.points.len().max(seg_b.points.len());
    if max_len == 0 {
        return None;
    }

    let s_a = if seg_a.points.len() == max_len { seg_a } else { resample_segment(&seg_a, max_len) };
    let s_b = if seg_b.points.len() == max_len { seg_b } else { resample_segment(&seg_b, max_len) };

    let pos_a = kf_a.position;
    let pos_b = kf_b.position;
    let pos = [lerp_f32(pos_a[0], pos_b[0], eased_t), lerp_f32(pos_a[1], pos_b[1], eased_t)];

    let scale_a = kf_a.scale;
    let scale_b = kf_b.scale;
    let scale = [lerp_f32(scale_a[0], scale_b[0], eased_t), lerp_f32(scale_a[1], scale_b[1], eased_t)];

    let rot_a = kf_a.rotation_deg;
    let rot_b = kf_b.rotation_deg;
    let rot_rad = lerp_f32(rot_a, rot_b, eased_t).to_radians();
    let cos_r = rot_rad.cos();
    let sin_r = rot_rad.sin();

    let mut result_points = Vec::with_capacity(max_len);

    for i in 0..max_len {
        let base_p = lerp_point(&s_a.points[i], &s_b.points[i], eased_t);

        let local_x = (base_p.x as f32 / 4095.0) * 2.0 - 1.0;
        let local_y = (base_p.y as f32 / 4095.0) * 2.0 - 1.0;

        let scaled_x = local_x * scale[0];
        let scaled_y = local_y * scale[1];

        let rx = scaled_x * cos_r - scaled_y * sin_r;
        let ry = scaled_x * sin_r + scaled_y * cos_r;

        let final_norm_x = (rx + pos[0]).clamp(-1.0, 1.0);
        let final_norm_y = (ry + pos[1]).clamp(-1.0, 1.0);

        let dac_x = (((final_norm_x + 1.0) / 2.0) * 4095.0).clamp(0.0, 4095.0) as u16;
        let dac_y = (((final_norm_y + 1.0) / 2.0) * 4095.0).clamp(0.0, 4095.0) as u16;

        result_points.push(LaserPoint {
            x: dac_x,
            y: dac_y,
            r: base_p.r,
            g: base_p.g,
            b: base_p.b,
            i: base_p.i,
        });
    }

    Some(LaserSegment::new(result_points))
}

/// Evaluates a full LaserShow document at timestamp `time_secs`, returning all active segments
pub fn evaluate_show_segments(show: &LaserShow, time_secs: f32) -> Vec<LaserSegment> {
    let mut segments = Vec::new();
    for track in &show.tracks {
        if let Some(seg) = evaluate_track(track, time_secs) {
            segments.push(seg);
        }
    }
    segments
}

/// Renders a keyframe shape template into a raw LaserSegment
fn render_keyframe_segment(kf: &ShowKeyframe) -> LaserSegment {
    let (r, g, b) = (kf.color[0], kf.color[1], kf.color[2]);
    let i = kf.intensity;

    let points = match kf.shape_template.as_str() {
        "star" => vec![
            LaserPoint::new(2047, 4095, r, g, b, i),
            LaserPoint::new(1228, 1228, r, g, b, i),
            LaserPoint::new(0,    2047, r, g, b, i),
            LaserPoint::new(1433, 819,  r, g, b, i),
            LaserPoint::new(819,  0,    r, g, b, i),
            LaserPoint::new(2047, 1024, r, g, b, i),
            LaserPoint::new(3276, 0,    r, g, b, i),
            LaserPoint::new(2662, 819,  r, g, b, i),
            LaserPoint::new(4095, 2047, r, g, b, i),
            LaserPoint::new(2867, 1228, r, g, b, i),
            LaserPoint::new(2047, 4095, r, g, b, i),
        ],
        "diamond" => vec![
            LaserPoint::new(2047, 4095, r, g, b, i),
            LaserPoint::new(4095, 2047, r, g, b, i),
            LaserPoint::new(2047, 0,    r, g, b, i),
            LaserPoint::new(0,    2047, r, g, b, i),
            LaserPoint::new(2047, 4095, r, g, b, i),
        ],
        "square" | "box" => vec![
            LaserPoint::new(512,  512,  r, g, b, i),
            LaserPoint::new(3583, 512,  r, g, b, i),
            LaserPoint::new(3583, 3583, r, g, b, i),
            LaserPoint::new(512,  3583, r, g, b, i),
            LaserPoint::new(512,  512,  r, g, b, i),
        ],
        _ => {
            let mut circle_pts = Vec::with_capacity(17);
            for idx in 0..=16 {
                let angle = (idx as f32 / 16.0) * std::f32::consts::TAU;
                let cx = ((angle.cos() + 1.0) * 0.5 * 4095.0) as u16;
                let cy = ((angle.sin() + 1.0) * 0.5 * 4095.0) as u16;
                circle_pts.push(LaserPoint::new(cx, cy, r, g, b, i));
            }
            circle_pts
        }
    };

    LaserSegment::new(points)
}
