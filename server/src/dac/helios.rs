// Rust implementation of Helios DAC library
// Based on the C++ SDK and C# implementations
// Uses dynamic loading to avoid linking issues

use libloading;
use std::os::raw::{c_char, c_int, c_uchar, c_uint};
use std::sync::{Arc, OnceLock};
use log::info;

// Point structures matching the working darkelf implementation
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl From<laserlogic::LaserPoint> for HeliosPoint {
    fn from(p: laserlogic::LaserPoint) -> Self {
        Self {
            x: p.x,
            y: p.y,
            r: p.r,
            g: p.g,
            b: p.b,
            i: p.i,
        }
    }
}

// Helios DAC coordinate limits
#[allow(dead_code)]
pub const HELIOS_MAX_COORD: u16 = 0xFFF; // 4095 for 12-bit
#[allow(dead_code)]
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
type GetNameFn = unsafe extern "C" fn(c_uint, *mut c_char) -> c_int;
type GetFirmwareVersionFn = unsafe extern "C" fn(c_uint) -> c_int;

// Internal library handle
struct HeliosLib {
    open_devices: OpenDevicesFn,
    close_devices: CloseDevicesFn,
    get_status: GetStatusFn,
    write_frame: WriteFrameFn,
    stop: StopFn,
    set_shutter: SetShutterFn,
    get_name: GetNameFn,
    get_firmware_version: GetFirmwareVersionFn,
}

static HELIOS_LIB_SINGLETON: OnceLock<Arc<HeliosLib>> = OnceLock::new();

impl HeliosLib {
    fn get_or_load() -> Result<Arc<Self>, String> {
        if let Some(lib) = HELIOS_LIB_SINGLETON.get() {
            return Ok(lib.clone());
        }
        let loaded = Self::load()?;
        let arc = Arc::new(loaded);
        let _ = HELIOS_LIB_SINGLETON.set(arc.clone());
        Ok(arc)
    }

    fn load() -> Result<Self, String> {
        unsafe {
            // The build script copies the DLL to target/<profile>/ next to the executable
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
            let get_firmware_version = *lib
                .get::<GetFirmwareVersionFn>(b"GetFirmwareVersion")
                .map_err(|e| format!("Failed to load GetFirmwareVersion: {}", e))?;

            // Keep library mapped in process memory forever to prevent libusb background thread segfaults on dlclose()
            std::mem::forget(lib);

            Ok(Self {
                open_devices,
                close_devices,
                get_status,
                write_frame,
                stop,
                set_shutter,
                get_name,
                get_firmware_version,
            })
        }
    }
}

/// Helios DAC Controller for Rust
pub struct HeliosDacController {
    pub num_devices: i32,
    lib: Arc<HeliosLib>,
}

impl HeliosDacController {
    /// Create a new controller instance and obtain the library singleton
    pub fn new() -> Result<Self, String> {
        let lib = HeliosLib::get_or_load()?;
        Ok(Self {
            num_devices: 0,
            lib,
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
    pub fn get_status(&self, device_num: u32) -> Result<bool, String> {
        unsafe {
            let result = (self.lib.get_status)(device_num as c_uint);
            if result == 1 {
                Ok(true) // 1 = ready to receive new frame
            } else if result == 0 || result == -1002 || result == -1000 || result == -1003 || result == -5007 || result == -7 || result == -9 {
                Ok(false) // 0 or transient USB busy status code = DAC is busy playing current frame
            } else {
                Err(format!("GetStatus failed: error {}", result))
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

    /// Wait until the specified DAC is ready to receive a frame or max_attempts is reached
    pub fn wait_for_ready(&self, dac_num: u32, max_attempts: usize) -> Result<bool, String> {
        for i in 0..max_attempts {
            let mut status_res = Err("Unknown status error".to_string());
            for attempt in 0..3 {
                match self.get_status(dac_num) {
                    Ok(ready) => {
                        status_res = Ok(ready);
                        break;
                    }
                    Err(e) => {
                        status_res = Err(e);
                        let backoff_ms = match attempt {
                            0 => 2,
                            1 => 5,
                            _ => 10,
                        };
                        std::thread::sleep(std::time::Duration::from_millis(backoff_ms));
                    }
                }
            }

            match status_res {
                Ok(true) => return Ok(true),
                Ok(false) => {
                    if i + 1 < max_attempts {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                }
                Err(e) => return Err(e),
            }
        }
        Ok(false)
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

        let mut last_err = 0;
        for attempt in 0..3 {
            unsafe {
                let result = (self.lib.write_frame)(
                    dac_num,
                    pps,
                    flags,
                    points.as_ptr(),
                    points.len() as c_uint,
                );
                if result >= 0 {
                    return Ok(());
                }
                last_err = result;
                if result == -1000 || result == -1002 || result == -1003 || result == -5007 || result == -7 || result == -9 {
                    // Stepped backoff: 2ms -> 5ms -> 10ms to allow USB controller DMA buffer to settle
                    let backoff_ms = match attempt {
                        0 => 2,
                        1 => 5,
                        _ => 10,
                    };
                    std::thread::sleep(std::time::Duration::from_millis(backoff_ms));
                } else {
                    // Hard error - stop retrying immediately
                    break;
                }
            }
        }

        Err(format!("Failed to write frame: error {}", last_err))
    }

    /// Build a frame padded to `min_pts` with blanked copies of the last point.
    pub fn pad_frame(points: &[HeliosPoint], min_pts: usize) -> Vec<HeliosPoint> {
        let mut v = points.to_vec();
        if v.len() < min_pts {
            let last = *v.last().unwrap_or(&HeliosPoint::blanked(2048, 2048));
            while v.len() < min_pts {
                v.push(HeliosPoint::blanked(last.x, last.y));
            }
        }
        v
    }

    /// Convenience: wait for ready and write frame to DAC with padding to min_pts.
    /// Returns:
    /// - Ok(true) if frame was successfully written to DAC
    /// - Ok(false) if DAC is busy playing current frame (NOT an error!)
    /// - Err(String) if an actual USB hardware error occurred
    pub fn write_frame_ready(
        &self,
        dac_num: u32,
        pps: u32,
        flags: u8,
        points: &[HeliosPoint],
        min_pts: usize,
    ) -> Result<bool, String> {
        let padded = Self::pad_frame(points, min_pts);
        // Wait for DAC ready status (max 120 polls with 1ms spacing = 120ms max polling window to support large 3,600pt frames)
        match self.wait_for_ready(dac_num, 120) {
            Err(e) => return Err(e), // Propagate USB errors immediately
            Ok(false) => return Ok(false), // DAC busy playing frame — NOT an error!
            Ok(true) => {}  // Ready
        }
        let res = self.write_frame_native(dac_num, pps, flags, &padded);
        if res.is_ok() {
            // Sleep for 80% of total frame playback time (~27.3ms for 1024 pts @ 30kpps, ~80ms for 3000 pts @ 30kpps).
            // Waking up 20% before frame end ensures wait_for_ready with 1ms polling delivers next frame 1-3ms BEFORE playback finishes!
            let total_pts = padded.len().max(min_pts) as f32;
            let sleep_micros = ((total_pts / pps.max(HELIOS_MIN_PPS) as f32) * 800_000.0) as u64;
            if sleep_micros > 0 {
                std::thread::sleep(std::time::Duration::from_micros(sleep_micros));
            }
            Ok(true)
        } else {
            res.map(|_| true)
        }
    }

    /// Write a PathSegment frame to the specified DAC
    pub fn write_frame_path(
        &self,
        dac_num: u32,
        pps: u32,
        flags: u8,
        segment: &common::path::PathSegment,
    ) -> Result<(), String> {
        let mut helios_points = Vec::new();
        
        for path_point in &segment.points {
            let x_helios = ((path_point.x + 1.0) * (HELIOS_MAX_COORD as f32 / 2.0)) as u16;
            let y_helios = ((path_point.y + 1.0) * (HELIOS_MAX_COORD as f32 / 2.0)) as u16;
            
            let dwell_count = if path_point.dwell == 0 { 1 } else { path_point.dwell as usize };
            for _ in 0..dwell_count {
                helios_points.push(HeliosPoint::new(
                    x_helios,
                    y_helios,
                    path_point.r,
                    path_point.g,
                    path_point.b,
                    255,
                ));
            }
        }
        
        if helios_points.is_empty() {
            return Err("Segment is empty".to_string());
        }

        self.write_frame_native(dac_num, pps, flags, &helios_points)
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
        let mut buf = [0u8; 64];
        unsafe {
            let r = (self.lib.get_name)(dac_num, buf.as_mut_ptr() as *mut c_char);
            if r < 0 {
                return Err(format!("GetName error: {}", r));
            }
        }
        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        Ok(String::from_utf8_lossy(&buf[..end]).into_owned())
    }

    /// Get firmware version of specified DAC
    pub fn get_firmware_version(&self, dac_num: u32) -> Result<i32, String> {
        unsafe {
            let r = (self.lib.get_firmware_version)(dac_num);
            if r < 0 {
                Err(format!("GetFirmwareVersion error: {}", r))
            } else {
                Ok(r)
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

