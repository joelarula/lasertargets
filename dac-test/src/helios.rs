// Helios DAC FFI bindings — Bevy-free copy for standalone hardware testing.
// Mirrors server/src/dac/helios.rs but uses the standard `log` crate instead of bevy::prelude.

use libloading;
use std::os::raw::{c_char, c_int, c_uchar, c_uint};
use std::sync::Arc;
use log::{info, warn};

// ─── Point structure ────────────────────────────────────────────────────────

/// One laser output point in Helios native format.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HeliosPoint {
    pub x: u16, // 0–4095 (12-bit)
    pub y: u16, // 0–4095 (12-bit)
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub i: u8, // intensity
}

impl HeliosPoint {
    pub fn new(x: u16, y: u16, r: u8, g: u8, b: u8, i: u8) -> Self {
        Self { x, y, r, g, b, i }
    }

    /// Laser-off point at the given position.
    pub fn blanked(x: u16, y: u16) -> Self {
        Self { x, y, r: 0, g: 0, b: 0, i: 0 }
    }

    /// Full-white point at the given position.
    #[allow(dead_code)]
    pub fn white(x: u16, y: u16) -> Self {
        Self { x, y, r: 255, g: 255, b: 255, i: 255 }
    }
}

// ─── Constants ──────────────────────────────────────────────────────────────

#[allow(dead_code)] pub const HELIOS_MAX_COORD: u16  = 0xFFF;
pub const HELIOS_CENTER_COORD: u16 = 2048;

pub const HELIOS_MAX_POINTS: usize = 0xFFF;
pub const HELIOS_MAX_PPS: u32      = 0xFFFF;
pub const HELIOS_MIN_PPS: u32      = 7;

#[allow(dead_code)] pub const HELIOS_FLAGS_START_IMMEDIATELY: u8 = 1 << 0;
pub const HELIOS_FLAGS_SINGLE_MODE: u8       = 1 << 1;
#[allow(dead_code)] pub const HELIOS_FLAGS_DONT_BLOCK: u8        = 1 << 2;
pub const HELIOS_FLAGS_DEFAULT: u8           = HELIOS_FLAGS_SINGLE_MODE;

#[allow(dead_code)] pub const HELIOS_SUCCESS: i32 = 1;

// ─── Platform library name ───────────────────────────────────────────────────

#[cfg(windows)]
const LIB_NAME: &str = "HeliosLaserDAC.dll";
#[cfg(target_os = "linux")]
const LIB_NAME: &str = "libHeliosLaserDAC.so";
#[cfg(target_os = "macos")]
const LIB_NAME: &str = "libHeliosLaserDAC.dylib";

// ─── FFI function types ──────────────────────────────────────────────────────

type OpenDevicesFn  = unsafe extern "C" fn() -> c_int;
type CloseDevicesFn = unsafe extern "C" fn() -> c_int;
type GetStatusFn    = unsafe extern "C" fn(c_uint) -> c_int;
type WriteFrameFn   =
    unsafe extern "C" fn(c_uint, c_uint, c_uchar, *const HeliosPoint, c_uint) -> c_int;
type StopFn         = unsafe extern "C" fn(c_uint) -> c_int;
type SetShutterFn   = unsafe extern "C" fn(c_uint, c_uchar) -> c_int;
/// Real C++ signature: `int GetName(unsigned int devNum, char* name)`
/// We declare the *correct* two-argument form here (unlike the old server binding).
type GetNameFn      = unsafe extern "C" fn(c_uint, *mut c_char) -> c_int;
type GetFirmwareVersionFn = unsafe extern "C" fn(c_uint) -> c_int;

// ─── Internal library wrapper ────────────────────────────────────────────────

struct HeliosLib {
    #[allow(dead_code)]
    lib: libloading::Library,
    open_devices:         OpenDevicesFn,
    close_devices:        CloseDevicesFn,
    get_status:           GetStatusFn,
    write_frame:          WriteFrameFn,
    stop:                 StopFn,
    set_shutter:          SetShutterFn,
    get_name:             GetNameFn,
    get_firmware_version: GetFirmwareVersionFn,
}

impl HeliosLib {
    fn load() -> Result<Self, String> {
        unsafe {
            info!("Loading Helios DAC library: {}", LIB_NAME);
            let lib = libloading::Library::new(LIB_NAME)
                .map_err(|e| format!("Failed to load {}: {}", LIB_NAME, e))?;

            macro_rules! sym {
                ($name:expr, $T:ty) => {
                    *lib.get::<$T>($name)
                        .map_err(|e| format!("Failed to load {}: {}", stringify!($name), e))?
                };
            }

            let open_devices         = sym!(b"OpenDevices",        OpenDevicesFn);
            let close_devices        = sym!(b"CloseDevices",       CloseDevicesFn);
            let get_status           = sym!(b"GetStatus",          GetStatusFn);
            let write_frame          = sym!(b"WriteFrame",         WriteFrameFn);
            let stop                 = sym!(b"Stop",               StopFn);
            let set_shutter          = sym!(b"SetShutter",         SetShutterFn);
            let get_name             = sym!(b"GetName",            GetNameFn);
            let get_firmware_version = sym!(b"GetFirmwareVersion", GetFirmwareVersionFn);

            Ok(Self {
                lib,
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

// ─── Public controller ────────────────────────────────────────────────────────

pub struct HeliosDacController {
    pub num_devices: i32,
    lib: Arc<HeliosLib>,
}

impl HeliosDacController {
    pub fn new() -> Result<Self, String> {
        let lib = HeliosLib::load()?;
        Ok(Self { num_devices: 0, lib: Arc::new(lib) })
    }

    // ── Device lifecycle ───────────────────────────────────────────────────

    pub fn open_devices(&mut self) -> Result<i32, String> {
        unsafe {
            self.num_devices = (self.lib.open_devices)();
            if self.num_devices < 0 {
                Err(format!("OpenDevices error: {}", self.num_devices))
            } else {
                Ok(self.num_devices)
            }
        }
    }

    pub fn close_devices(&mut self) -> Result<(), String> {
        unsafe {
            let r = (self.lib.close_devices)();
            self.num_devices = 0;
            if r < 0 { Err(format!("CloseDevices error: {}", r)) } else { Ok(()) }
        }
    }

    // ── Status ─────────────────────────────────────────────────────────────

    /// Returns `Ok(true)` = ready, `Ok(false)` = busy, `Err` = error code.
    pub fn get_status(&self, dev: u32) -> Result<bool, String> {
        unsafe {
            let r = (self.lib.get_status)(dev);
            match r {
                r if r >= 0 => Ok(r == 1),
                r => Err(format!("GetStatus error: {}", r)),
            }
        }
    }

    // ── Device info ────────────────────────────────────────────────────────

    /// Returns the device name by writing into a local 64-byte buffer (correct
    /// two-argument FFI call — fixes the server-side SEGV caused by the old
    /// single-argument binding).
    pub fn get_name(&self, dev: u32) -> Result<String, String> {
        let mut buf = [0u8; 64];
        unsafe {
            let r = (self.lib.get_name)(dev, buf.as_mut_ptr() as *mut c_char);
            if r < 0 {
                return Err(format!("GetName error: {}", r));
            }
        }
        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        Ok(String::from_utf8_lossy(&buf[..end]).into_owned())
    }

    pub fn get_firmware_version(&self, dev: u32) -> Result<i32, String> {
        unsafe {
            let r = (self.lib.get_firmware_version)(dev);
            if r < 0 { Err(format!("GetFirmwareVersion error: {}", r)) } else { Ok(r) }
        }
    }

    // ── Shutter / stop ─────────────────────────────────────────────────────

    pub fn set_shutter(&self, dev: u32, open: bool) -> Result<(), String> {
        unsafe {
            let r = (self.lib.set_shutter)(dev, if open { 1 } else { 0 });
            if r < 0 { Err(format!("SetShutter error: {}", r)) } else { Ok(()) }
        }
    }

    pub fn stop(&self, dev: u32) -> Result<(), String> {
        unsafe {
            let r = (self.lib.stop)(dev);
            if r < 0 { Err(format!("Stop error: {}", r)) } else { Ok(()) }
        }
    }

    // ── Frame write ────────────────────────────────────────────────────────

    pub fn write_frame(&self, dev: u32, pps: u32, flags: u8, points: &[HeliosPoint])
        -> Result<(), String>
    {
        if points.is_empty() {
            return Err("Points array is empty".into());
        }
        if points.len() > HELIOS_MAX_POINTS {
            return Err(format!("Too many points: {} (max {})", points.len(), HELIOS_MAX_POINTS));
        }
        if pps > HELIOS_MAX_PPS {
            return Err(format!("PPS too high: {} (max {})", pps, HELIOS_MAX_PPS));
        }
        if pps < HELIOS_MIN_PPS {
            return Err(format!("PPS too low: {} (min {})", pps, HELIOS_MIN_PPS));
        }

        unsafe {
            let r = (self.lib.write_frame)(dev, pps, flags, points.as_ptr(), points.len() as c_uint);
            if r < 0 {
                Err(format!("WriteFrame error: {}", r))
            } else {
                Ok(())
            }
        }
    }

    // ── Helpers ────────────────────────────────────────────────────────────

    /// Block until the DAC reports ready (or max_attempts exceeded).
    /// Pass `max_attempts = 0` for unlimited.
    pub fn wait_ready(&self, dev: u32, max_attempts: u32) -> Result<bool, String> {
        let mut n = 0u32;
        loop {
            match self.get_status(dev)? {
                true  => return Ok(true),
                false => {
                    n += 1;
                    if max_attempts > 0 && n >= max_attempts {
                        return Ok(false);
                    }
                    std::thread::yield_now();
                }
            }
        }
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

    /// Convenience: write a frame, padding it first, waiting for ready first.
    pub fn write_frame_ready(
        &self,
        dev: u32,
        pps: u32,
        flags: u8,
        points: &[HeliosPoint],
        min_pts: usize,
    ) -> Result<(), String> {
        let padded = Self::pad_frame(points, min_pts);
        // Best-effort busy-wait (200 retries ≈ ~2 ms typical)
        match self.wait_ready(dev, 200) {
            Ok(false) => warn!("DAC not ready after 200 polls — writing anyway"),
            Err(e)    => return Err(e),
            Ok(true)  => {}
        }
        self.write_frame(dev, pps, flags, &padded)
    }
}

impl Drop for HeliosDacController {
    fn drop(&mut self) {
        if self.num_devices > 0 {
            let _ = self.close_devices();
        }
    }
}
