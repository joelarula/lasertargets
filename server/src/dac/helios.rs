// Rust implementation of Helios DAC library
// Based on the C++ SDK and C# implementations
// Uses dynamic loading to avoid linking issues


use libloading;
use std::os::raw::{c_int, c_uchar, c_uint};
use std::sync::Arc;
use common::path::UniversalPath;
use bevy::prelude::*;
use lyon_tessellation;

// Point structures matching the working darkelf implementation
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HeliosPoint {
    pub x: u16, // 0 to 0xFFF (4095) for 12-bit DAC
    pub y: u16, // 0 to 0xFFF (4095) for 12-bit DAC
    pub r: u8,  // 0 to 0xFF (255)
    pub g: u8,  // 0 to 0xFF (255)
    pub b: u8,  // 0 to 0xFF (255)
    pub i: u8,  // Intensity, 0 to 0xFF (255)
}

impl HeliosPoint {
    pub fn new(x: u16, y: u16, r: u8, g: u8, b: u8, i: u8) -> Self {
        Self { x, y, r, g, b, i }
    }
    
    /// Create a blanked point (laser off) at the given position
    pub fn blanked(x: u16, y: u16) -> Self {
        Self { x, y, r: 0, g: 0, b: 0, i: 0 }
    }
}

// Helios DAC coordinate limits
pub const HELIOS_MAX_COORD: u16 = 0xFFF; // 4095 for 12-bit
pub const HELIOS_CENTER_COORD: u16 = 2048; // Center point

// Frame limits
pub const HELIOS_MAX_POINTS: usize = 0xFFF;
pub const HELIOS_MAX_PPS: u32 = 0xFFFF;
pub const HELIOS_MIN_PPS: u32 = 7;

// Flags
pub const HELIOS_FLAGS_START_IMMEDIATELY: u8 = 1 << 0;
pub const HELIOS_FLAGS_SINGLE_MODE: u8 = 1 << 1;
pub const HELIOS_FLAGS_DONT_BLOCK: u8 = 1 << 2;
pub const HELIOS_FLAGS_DEFAULT: u8 = HELIOS_FLAGS_SINGLE_MODE;

// Error codes
pub const HELIOS_SUCCESS: i32 = 1;
pub const HELIOS_ERROR_NOT_INITIALIZED: i32 = -1;
pub const HELIOS_ERROR_INVALID_DEVNUM: i32 = -2;
pub const HELIOS_ERROR_NULL_POINTS: i32 = -3;
pub const HELIOS_ERROR_TOO_MANY_POINTS: i32 = -4;
pub const HELIOS_ERROR_PPS_TOO_HIGH: i32 = -5;
pub const HELIOS_ERROR_PPS_TOO_LOW: i32 = -6;

// Library name for different platforms
#[cfg(windows)]
const LIB_NAME: &str = "HeliosLaserDAC.dll";
#[cfg(target_os = "linux")]
const LIB_NAME: &str = "libHeliosLaserDAC.so";
#[cfg(target_os = "macos")]
const LIB_NAME: &str = "libHeliosLaserDAC.dylib";

// Function type definitions for dynamic loading
type OpenDevicesFn = unsafe extern "C" fn() -> c_int;
type CloseDevicesFn = unsafe extern "C" fn() -> c_int;
type GetStatusFn = unsafe extern "C" fn(c_uint) -> c_int;
type WriteFrameFn =
    unsafe extern "C" fn(c_uint, c_uint, c_uchar, *const HeliosPoint, c_uint) -> c_int;
type StopFn = unsafe extern "C" fn(c_uint) -> c_int;
type SetShutterFn = unsafe extern "C" fn(c_uint, c_uchar) -> c_int;
type GetNameFn = unsafe extern "C" fn(c_uint) -> *const i8;

// Internal library handle
struct HeliosLib {
    #[allow(dead_code)]
    lib: libloading::Library,
    open_devices: OpenDevicesFn,
    close_devices: CloseDevicesFn,
    get_status: GetStatusFn,
    write_frame: WriteFrameFn,
    stop: StopFn,
    set_shutter: SetShutterFn,
    get_name: GetNameFn,
}

impl HeliosLib {
    fn load() -> Result<Self, String> {
        unsafe {
            // The build script should have copied the DLL to the target directory
            info!("Loading Helios DAC library: {}", LIB_NAME);
            let lib = libloading::Library::new(LIB_NAME)
                .map_err(|e| format!("Failed to load Helios DAC library {}: {}", LIB_NAME, e))?;

            let open_devices = *lib
                .get::<OpenDevicesFn>(b"OpenDevices")
                .map_err(|e| format!("Failed to load OpenDevices: {}", e))?;
            let close_devices = *lib
                .get::<CloseDevicesFn>(b"CloseDevices")
                .map_err(|e| format!("Failed to load CloseDevices: {}", e))?;
            let get_status = *lib
                .get::<GetStatusFn>(b"GetStatus")
                .map_err(|e| format!("Failed to load GetStatus: {}", e))?;
            let write_frame = *lib
                .get::<WriteFrameFn>(b"WriteFrame")
                .map_err(|e| format!("Failed to load WriteFrame: {}", e))?;
            let stop = *lib
                .get::<StopFn>(b"Stop")
                .map_err(|e| format!("Failed to load Stop: {}", e))?;
            let set_shutter = *lib
                .get::<SetShutterFn>(b"SetShutter")
                .map_err(|e| format!("Failed to load SetShutter: {}", e))?;
            let get_name = *lib
                .get::<GetNameFn>(b"GetName")
                .map_err(|e| format!("Failed to load GetName: {}", e))?;

            Ok(Self {
                lib,
                open_devices,
                close_devices,
                get_status,
                write_frame,
                stop,
                set_shutter,
                get_name,
            })
        }
    }
}

/// Helios DAC Controller for Rust
pub struct HeliosDacController {
    num_devices: i32,
    lib: Arc<HeliosLib>,
}

impl HeliosDacController {
    /// Create a new controller instance and load the library
    pub fn new() -> Result<Self, String> {
        let lib = HeliosLib::load()?;
        Ok(Self {
            num_devices: 0,
            lib: Arc::new(lib),
        })
    }

    /// Open and initialize all connected Helios DAC devices
    /// Returns the number of devices found
    pub fn open_devices(&mut self) -> Result<i32, String> {
        unsafe {
            self.num_devices = (self.lib.open_devices)();
            if self.num_devices < 0 {
                Err(format!(
                    "Failed to open devices: error {}",
                    self.num_devices
                ))
            } else {
                Ok(self.num_devices)
            }
        }
    }

    /// Get device status (returns true if ready to receive new frame)
    pub fn get_status(&self, device_num: i32) -> Result<bool, String> {
        unsafe {
            let result = (self.lib.get_status)(device_num as c_uint);
            if result >= 0 {
                Ok(result == 1) // 1 = ready, 0 = busy
            } else {
                Err(format!("GetStatus failed with error: {}", result))
            }
        }
    }

    /// Close all Helios DAC devices
    pub fn close_devices(&mut self) -> Result<(), String> {
        unsafe {
            let result = (self.lib.close_devices)();
            if result < 0 {
                Err(format!("Failed to close devices: error {}", result))
            } else {
                self.num_devices = 0;
                Ok(())
            }
        }
    }

    /// Get the number of opened devices
    pub fn num_devices(&self) -> i32 {
        self.num_devices
    }

    /// Write frame data to the specified DAC with shift parameter (matches working example API)
    pub fn write_frame(
        &self,
        device_num: i32,
        pps: u32,
        flags: u8,
        points: &[HeliosPoint],
    ) -> Result<(), String> {
        if points.len() > HELIOS_MAX_POINTS {
            return Err(format!("Too many points: {} (max: {})", points.len(), HELIOS_MAX_POINTS));
        }
        if pps > HELIOS_MAX_PPS {
            return Err(format!("PPS too high: {} (max is {})", pps, HELIOS_MAX_PPS));
        }
        if pps < HELIOS_MIN_PPS {
            return Err(format!("PPS too low: {} (min is {})", pps, HELIOS_MIN_PPS));
        }

        unsafe {
            let result = (self.lib.write_frame)(
                device_num as c_uint,
                pps,
                flags,
                points.as_ptr(),
                points.len() as c_uint,
            );
            if result != HELIOS_SUCCESS {
                Err(format!("WriteFrame failed: error {}", result))
            } else {
                Ok(())
            }
        }
    }

    /// Write a frame to the specified DAC (native HeliosPoint format)
    /// This will block until the transfer is complete (unless HELIOS_FLAGS_DONT_BLOCK is set)
    pub fn write_frame_native(
        &self,
        dac_num: u32,
        pps: u32,
        flags: u8,
        points: &[HeliosPoint],
    ) -> Result<(), String> {
        if points.is_empty() {
            return Err("Points array is empty".to_string());
        }
        if points.len() > HELIOS_MAX_POINTS {
            return Err(format!(
                "Too many points: {} (max is {})",
                points.len(),
                HELIOS_MAX_POINTS
            ));
        }
        if pps > HELIOS_MAX_PPS {
            return Err(format!("PPS too high: {} (max is {})", pps, HELIOS_MAX_PPS));
        }
        if pps < HELIOS_MIN_PPS {
            return Err(format!("PPS too low: {} (min is {})", pps, HELIOS_MIN_PPS));
        }

        unsafe {
            let result = (self.lib.write_frame)(
                dac_num,
                pps,
                flags,
                points.as_ptr(),
                points.len() as c_uint,
            );
            if result < 0 {
                Err(format!("Failed to write frame: error {}", result))
            } else {
                Ok(())
            }
        }
    }

    /// Write a UniversalPath frame to the specified DAC
    /// Automatically handles coordinate conversion from path coordinates to DAC coordinates
    /// Uses native Helios 4095x4095 range for better precision
    pub fn write_frame_path(
        &self,
        dac_num: u32,
        pps: u32,
        flags: u8,
        path: &UniversalPath,
    ) -> Result<(), String> {
        // Flatten the path to a series of points with tolerance
        let tolerance = 1.0;
        let flattened_points = path.flatten(tolerance);
        
        if flattened_points.is_empty() {
            return Err("Path is empty".to_string());
        }
        
        // Convert to Helios points with color from path segments
        let mut helios_points = Vec::new();
        
        for segment in &path.segments {
            let segment_points = flatten_segment(segment, tolerance);
            for point in segment_points {
                helios_points.push(PathPoint {
                    pos: point,
                    color: segment.color,
                });
            }
        }
        
        // Convert to HeliosPoint format
        let points: Vec<HeliosPoint> = helios_points
            .iter()
            .map(path_point_to_helios)
            .collect();

        self.write_frame_native(dac_num, pps, flags, &points)
    }

    /// Stop output on the specified DAC
    pub fn stop(&self, dac_num: u32) -> Result<(), String> {
        unsafe {
            let result = (self.lib.stop)(dac_num);
            if result < 0 {
                Err(format!("Failed to stop DAC: error {}", result))
            } else {
                Ok(())
            }
        }
    }

    /// Set shutter level for the specified DAC
    /// level: 0 = closed, 1 = open
    pub fn set_shutter(&self, dac_num: u32, level: bool) -> Result<(), String> {
        unsafe {
            let result = (self.lib.set_shutter)(dac_num, if level { 1 } else { 0 });
            if result < 0 {
                Err(format!("Failed to set shutter: error {}", result))
            } else {
                Ok(())
            }
        }
    }

    /// Get the name of the specified DAC
    pub fn get_name(&self, dac_num: u32) -> Result<String, String> {
        unsafe {
            let name_ptr = (self.lib.get_name)(dac_num);
            if name_ptr.is_null() {
                Err("Failed to get device name".to_string())
            } else {
                let c_str = std::ffi::CStr::from_ptr(name_ptr);
                Ok(c_str.to_string_lossy().into_owned())
            }
        }
    }

    /// Wait for the DAC to be ready to receive a new frame
    /// max_attempts: maximum number of status checks before giving up (0 = infinite)
    /// Returns true if ready, false if timed out
    pub fn wait_for_ready(&self, dac_num: u32, max_attempts: u32) -> Result<bool, String> {
        let mut attempts = 0;
        loop {
            match self.get_status(dac_num as i32) {
                Ok(true) => return Ok(true),
                Ok(false) => {
                    attempts += 1;
                    if max_attempts > 0 && attempts >= max_attempts {
                        return Ok(false);
                    }
                    std::thread::yield_now();
                }
                Err(e) => return Err(e),
            }
        }
    }
}

impl Drop for HeliosDacController {
    fn drop(&mut self) {
        if self.num_devices > 0 {
            let _ = self.close_devices();
        }
    }
}

/// Internal point structure with position and color
struct PathPoint {
    pos: Vec2,
    color: Color,
}

/// Helper function to flatten a path segment into points
fn flatten_segment(segment: &common::path::PathSegment, tolerance: f32) -> Vec<Vec2> {
    use lyon_tessellation::path::PathEvent;
    let mut points = Vec::new();
    
    for event in segment.path.iter() {
        match event {
            PathEvent::Begin { at } => {
                points.push(Vec2::new(at.x, at.y));
            }
            PathEvent::Line { to, .. } => {
                points.push(Vec2::new(to.x, to.y));
            }
            PathEvent::Quadratic { ctrl, to, .. } => {
                let from = points.last().copied().unwrap_or(Vec2::ZERO);
                let control = Vec2::new(ctrl.x, ctrl.y);
                let end = Vec2::new(to.x, to.y);
                let samples = sample_quadratic(from, control, end, tolerance);
                points.extend(samples);
            }
            PathEvent::Cubic { ctrl1, ctrl2, to, .. } => {
                let from = points.last().copied().unwrap_or(Vec2::ZERO);
                let c1 = Vec2::new(ctrl1.x, ctrl1.y);
                let c2 = Vec2::new(ctrl2.x, ctrl2.y);
                let end = Vec2::new(to.x, to.y);
                let samples = sample_cubic(from, c1, c2, end, tolerance);
                points.extend(samples);
            }
            PathEvent::End { close, .. } => {
                if close && !points.is_empty() {
                    points.push(points[0]);
                }
            }
        }
    }
    points
}

fn sample_quadratic(start: Vec2, control: Vec2, end: Vec2, tolerance: f32) -> Vec<Vec2> {
    let mut points = Vec::new();
    let steps = ((start.distance(control) + control.distance(end)) / tolerance)
        .ceil()
        .max(2.0) as usize;

    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let mt = 1.0 - t;
        let point = start * mt * mt + control * 2.0 * mt * t + end * t * t;
        points.push(point);
    }
    points
}

fn sample_cubic(start: Vec2, c1: Vec2, c2: Vec2, end: Vec2, tolerance: f32) -> Vec<Vec2> {
    let mut points = Vec::new();
    let steps = ((start.distance(c1) + c1.distance(c2) + c2.distance(end)) / tolerance)
        .ceil()
        .max(2.0) as usize;

    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;

        let point = start * mt3 + c1 * 3.0 * mt2 * t + c2 * 3.0 * mt * t2 + end * t3;
        points.push(point);
    }
    points
}

/// Helper function to convert a PathPoint to a HeliosPoint
/// Uses native Helios 12-bit coordinate space (0-4095)
/// Input coordinates are normalized [-1, 1] and converted to unsigned [0, 4095]
#[inline]
fn path_point_to_helios(p: &PathPoint) -> HeliosPoint {
    // Convert normalized coordinates [-1, 1] to Helios range [0, 4095]
    // -1 maps to 0, 0 maps to 2048 (center), +1 maps to 4095
    let x_helios = ((p.pos.x + 1.0) * (HELIOS_MAX_COORD as f32 / 2.0)).clamp(0.0, HELIOS_MAX_COORD as f32) as u16;
    let y_helios = ((p.pos.y + 1.0) * (HELIOS_MAX_COORD as f32 / 2.0)).clamp(0.0, HELIOS_MAX_COORD as f32) as u16;

    // Extract color components - access linear color directly for performance
    let color_linear = p.color.to_linear();
    
    // Fast float-to-u8 conversion
    let r = (color_linear.red * 255.0).min(255.0).max(0.0) as u8;
    let g = (color_linear.green * 255.0).min(255.0).max(0.0) as u8;
    let b = (color_linear.blue * 255.0).min(255.0).max(0.0) as u8;

    HeliosPoint::new(x_helios, y_helios, r, g, b, 255)
}
