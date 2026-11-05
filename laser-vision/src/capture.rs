use std::time::{Duration, Instant};

/// A raw frame captured from a camera source
#[derive(Debug, Clone)]
pub struct RawFrame {
    pub timestamp_ms: u64,
    pub width: u32,
    pub height: u32,
    /// Packed RGB24 pixels (len == width * height * 3)
    pub rgb_data: Vec<u8>,
}

/// Abstract frame acquisition source
pub trait FrameSource: Send + 'static {
    /// Blocking or non-blocking acquisition of the next video frame
    fn next_frame(&mut self) -> Option<RawFrame>;
}

/// Synthetic mock frame source for local Windows development, automated tests, and CI
pub struct MockFrameSource {
    width: u32,
    height: u32,
    fps: u32,
    start_time: Instant,
    last_frame_time: Instant,
    laser_pos: (f32, f32),
    laser_vel: (f32, f32),
}

impl MockFrameSource {
    pub fn new(width: u32, height: u32, fps: u32) -> Self {
        Self {
            width,
            height,
            fps,
            start_time: Instant::now(),
            last_frame_time: Instant::now(),
            laser_pos: (width as f32 * 0.5, height as f32 * 0.5),
            laser_vel: (120.0, 90.0), // pixels per second
        }
    }
}

impl FrameSource for MockFrameSource {
    fn next_frame(&mut self) -> Option<RawFrame> {
        let frame_interval = Duration::from_secs_f32(1.0 / self.fps as f32);
        let now = Instant::now();
        let elapsed_since_last = now.duration_since(self.last_frame_time);

        if elapsed_since_last < frame_interval {
            std::thread::sleep(frame_interval - elapsed_since_last);
        }
        self.last_frame_time = Instant::now();

        let dt = frame_interval.as_secs_f32();

        // Animate a simulated green laser spot around the frame
        self.laser_pos.0 += self.laser_vel.0 * dt;
        self.laser_pos.1 += self.laser_vel.1 * dt;

        let margin = 50.0;
        if self.laser_pos.0 < margin || self.laser_pos.0 > self.width as f32 - margin {
            self.laser_vel.0 = -self.laser_vel.0;
        }
        if self.laser_pos.1 < margin || self.laser_pos.1 > self.height as f32 - margin {
            self.laser_vel.1 = -self.laser_vel.1;
        }

        // Render synthetic dark scene with green laser spot
        let mut rgb = vec![12u8; (self.width * self.height * 3) as usize];
        let lx = self.laser_pos.0.round() as i32;
        let ly = self.laser_pos.1.round() as i32;
        let radius = 3;

        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let px = lx + dx;
                let py = ly + dy;
                if px >= 0 && px < self.width as i32 && py >= 0 && py < self.height as i32 {
                    let idx = ((py as usize * self.width as usize) + px as usize) * 3;
                    let dist = ((dx * dx + dy * dy) as f32).sqrt();
                    let intensity = (255.0 * (1.0 - dist / (radius as f32 + 1.0)).max(0.0)) as u8;
                    rgb[idx] = 15;                     // R
                    rgb[idx + 1] = intensity.max(15);  // G
                    rgb[idx + 2] = 15;                 // B
                }
            }
        }

        Some(RawFrame {
            timestamp_ms: self.start_time.elapsed().as_millis() as u64,
            width: self.width,
            height: self.height,
            rgb_data: rgb,
        })
    }
}

/// Linux / Raspberry Pi camera pipe using `rpicam-vid` subprocess
#[cfg(target_os = "linux")]
pub struct ProcessPipeFrameSource {
    width: u32,
    height: u32,
    child: std::process::Child,
    stdout: std::io::BufReader<std::process::ChildStdout>,
    start_time: Instant,
}

#[cfg(target_os = "linux")]
impl ProcessPipeFrameSource {
    pub fn new(width: u32, height: u32, fps: u32) -> Result<Self, std::io::Error> {
        use std::process::{Command, Stdio};

        let mut child = Command::new("rpicam-vid")
            .args([
                "--codec", "yuv420",
                "--width", &width.to_string(),
                "--height", &height.to_string(),
                "--framerate", &fps.to_string(),
                "--shutter", "4000",   // 4ms shutter to isolate laser
                "--gain", "1.0",       // minimum gain to prevent bloom
                "-t", "0",             // continuous stream
                "-n",                  // no preview window
                "-o", "-",             // output to stdout
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let stdout = std::io::BufReader::new(child.stdout.take().unwrap());

        Ok(Self {
            width,
            height,
            child,
            stdout,
            start_time: Instant::now(),
        })
    }
}

#[cfg(target_os = "linux")]
impl FrameSource for ProcessPipeFrameSource {
    fn next_frame(&mut self) -> Option<RawFrame> {
        use std::io::Read;

        // YUV420 size: Y = W*H, U = (W/2)*(H/2), V = (W/2)*(H/2) => Total = W * H * 1.5
        let yuv_size = (self.width as usize * self.height as usize * 3) / 2;
        let mut yuv_buf = vec![0u8; yuv_size];

        if self.stdout.read_exact(&mut yuv_buf).is_err() {
            return None;
        }

        // Convert YUV420 to RGB24 for detection
        let mut rgb = vec![0u8; (self.width * self.height * 3) as usize];
        let w = self.width as usize;
        let h = self.height as usize;
        let y_plane = &yuv_buf[0..(w * h)];
        let u_plane = &yuv_buf[(w * h)..(w * h + (w * h) / 4)];
        let v_plane = &yuv_buf[(w * h + (w * h) / 4)..];

        for y in 0..h {
            for x in 0..w {
                let y_val = y_plane[y * w + x] as f32;
                let uv_idx = (y / 2) * (w / 2) + (x / 2);
                let u_val = u_plane[uv_idx] as f32 - 128.0;
                let v_val = v_plane[uv_idx] as f32 - 128.0;

                let r = (y_val + 1.402 * v_val).clamp(0.0, 255.0) as u8;
                let g = (y_val - 0.344136 * u_val - 0.714136 * v_val).clamp(0.0, 255.0) as u8;
                let b = (y_val + 1.772 * u_val).clamp(0.0, 255.0) as u8;

                let out_idx = (y * w + x) * 3;
                rgb[out_idx] = r;
                rgb[out_idx + 1] = g;
                rgb[out_idx + 2] = b;
            }
        }

        Some(RawFrame {
            timestamp_ms: self.start_time.elapsed().as_millis() as u64,
            width: self.width,
            height: self.height,
            rgb_data: rgb,
        })
    }
}

#[cfg(target_os = "linux")]
impl Drop for ProcessPipeFrameSource {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

