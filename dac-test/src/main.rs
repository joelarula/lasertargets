//! dac-test — standalone Helios DAC hardware test tool
//!
//! Usage:
//!   sudo dac-test info               — print device info and exit
//!   sudo dac-test blink              — alternate full-white / blank for 30 s
//!   sudo dac-test box                — draw a 4-corner box for 60 s
//!   sudo dac-test stress             — production-identical loop for 5 min
//!   sudo dac-test sweep              — sweep all PPS × frame-size combinations

mod helios;

use helios::{
    HeliosDacController, HeliosPoint,
    HELIOS_CENTER_COORD, HELIOS_FLAGS_DEFAULT,
};
use std::time::{Duration, Instant};
use log::{info, warn, error};

// ─── helpers ─────────────────────────────────────────────────────────────────

fn open_dac() -> HeliosDacController {
    let mut dac = HeliosDacController::new().unwrap_or_else(|e| {
        error!("Failed to load Helios library: {}", e);
        std::process::exit(1);
    });

    info!("Opening Helios DAC devices…");
    match dac.open_devices() {
        Ok(n) if n > 0 => info!("Found {} device(s)", n),
        Ok(_) => {
            error!("No Helios DAC devices found.");
            std::process::exit(1);
        }
        Err(e) => {
            error!("OpenDevices failed: {}", e);
            std::process::exit(1);
        }
    }
    dac
}

fn open_shutter(dac: &HeliosDacController) {
    std::thread::sleep(Duration::from_millis(500)); // firmware boot settle
    match dac.get_status(0) {
        Ok(ready) => info!("Initial GetStatus: ready={}", ready),
        Err(e)    => warn!("Initial GetStatus error: {}", e),
    }
    match dac.set_shutter(0, true) {
        Ok(_)  => { info!("Shutter opened."); std::thread::sleep(Duration::from_millis(100)); }
        Err(e) => warn!("SetShutter failed: {}", e),
    }
}

/// Simple run loop: write `frame` at `pps` until `duration` elapses or an
/// unrecoverable error occurs.  Returns `(frames_sent, error_count)`.
fn run_loop(
    dac: &HeliosDacController,
    frame: &[HeliosPoint],
    pps: u32,
    duration: Duration,
    min_pts: usize,
) -> (u64, u64) {
    let deadline = Instant::now() + duration;
    let mut frames: u64 = 0;
    let mut errors: u64 = 0;
    let mut consecutive_errors: u64 = 0;

    while Instant::now() < deadline {
        match dac.write_frame_ready(0, pps, HELIOS_FLAGS_DEFAULT, frame, min_pts) {
            Ok(_) => {
                frames += 1;
                consecutive_errors = 0;

                if frames % 600 == 0 {
                    let elapsed = deadline - Instant::now();
                    info!(
                        "  {} frames sent  ({:.0}s remaining)",
                        frames,
                        elapsed.as_secs_f32()
                    );
                }

                // Sleep for ~80 % of the frame play time so we aren't hammering USB.
                let pts = frame.len().max(min_pts) as u32;
                let sleep_ms = (pts as f64 / pps as f64 * 800.0) as u64;
                std::thread::sleep(Duration::from_millis(sleep_ms.clamp(4, 50)));
            }
            Err(e) => {
                errors += 1;
                consecutive_errors += 1;
                if consecutive_errors == 1 || consecutive_errors % 20 == 0 {
                    error!("  WriteFrame error (#{}) : {}", consecutive_errors, e);
                }
                if consecutive_errors >= 100 {
                    error!("  100 consecutive errors — aborting loop.");
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }

    (frames, errors)
}

// ─── geometry helpers ─────────────────────────────────────────────────────────

/// Interpolate `n` lit points along each box edge for smoother output.
fn make_box_smooth(size: u16, pts_per_edge: usize) -> Vec<HeliosPoint> {
    let cx = HELIOS_CENTER_COORD as f32;
    let cy = HELIOS_CENTER_COORD as f32;
    let h  = size as f32 / 2.0;

    let corners: [(f32, f32); 4] = [
        (cx - h, cy + h), // TL
        (cx + h, cy + h), // TR
        (cx + h, cy - h), // BR
        (cx - h, cy - h), // BL
    ];

    let mut pts = Vec::new();
    for i in 0..4 {
        let (ax, ay) = corners[i];
        let (bx, by) = corners[(i + 1) % 4];
        for s in 0..pts_per_edge {
            let t = s as f32 / pts_per_edge as f32;
            let x = (ax + t * (bx - ax)) as u16;
            let y = (ay + t * (by - ay)) as u16;
            pts.push(HeliosPoint::new(x, y, 255, 255, 255, 255));
        }
    }
    pts
}

/// Full-bright 350-point blank frame (centre position).
fn blank_frame(n: usize) -> Vec<HeliosPoint> {
    vec![HeliosPoint::blanked(HELIOS_CENTER_COORD, HELIOS_CENTER_COORD); n]
}

// ─── scenarios ────────────────────────────────────────────────────────────────

fn cmd_info() {
    let mut dac = open_dac();
    let n = dac.num_devices;

    for dev in 0..n as u32 {
        match dac.get_name(dev) {
            Ok(name) => info!("  Device {} name: {}", dev, name),
            Err(e)   => warn!("  Device {} get_name error: {}", dev, e),
        }
        match dac.get_firmware_version(dev) {
            Ok(fw) => info!("  Device {} firmware: v{}", dev, fw),
            Err(e) => warn!("  Device {} get_firmware_version error: {}", dev, e),
        }
        match dac.get_status(dev) {
            Ok(r) => info!("  Device {} status: ready={}", dev, r),
            Err(e) => warn!("  Device {} get_status error: {}", dev, e),
        }
    }

    let _ = dac.close_devices();
    info!("Done.");
}

fn cmd_blink(duration_secs: u64) {
    let dac = open_dac();
    open_shutter(&dac);

    info!("Blinking for {} seconds…", duration_secs);
    let deadline = Instant::now() + Duration::from_secs(duration_secs);
    let white = vec![HeliosPoint::new(HELIOS_CENTER_COORD, HELIOS_CENTER_COORD, 255, 255, 255, 255); 350];
    let blank = blank_frame(350);
    let pps   = 20_000u32;
    let mut phase = false;

    while Instant::now() < deadline {
        let frame = if phase { &white } else { &blank };
        if let Err(e) = dac.write_frame_ready(0, pps, HELIOS_FLAGS_DEFAULT, frame, 350) {
            error!("Write error: {}", e);
        }
        std::thread::sleep(Duration::from_millis(500));
        phase = !phase;
    }

    let _ = dac.stop(0);
    info!("Blink complete.");
}

fn cmd_box(duration_secs: u64) {
    let dac = open_dac();
    open_shutter(&dac);

    let frame = make_box_smooth(2000, 100); // 400 pts, nice box
    info!(
        "Drawing box ({} pts) at 20 kPPS for {} s…",
        frame.len(), duration_secs
    );

    let (frames, errors) = run_loop(&dac, &frame, 20_000, Duration::from_secs(duration_secs), 350);
    info!("Box done — {} frames, {} errors", frames, errors);
    let _ = dac.stop(0);
}

fn cmd_stress(duration_secs: u64) {
    let dac = open_dac();
    open_shutter(&dac);

    // Identical to production: 350-point blank frame at 20 kPPS
    let frame = blank_frame(350);
    info!("Stress test: 350 pts @ 20 kPPS for {} s…", duration_secs);

    let (frames, errors) = run_loop(&dac, &frame, 20_000, Duration::from_secs(duration_secs), 350);

    info!("===== STRESS TEST RESULT =====");
    info!("  Duration : {} s", duration_secs);
    info!("  Frames   : {}", frames);
    info!("  Errors   : {}", errors);
    if errors == 0 {
        info!("  ✓ PASS — zero errors");
    } else {
        error!("  ✗ FAIL — {} write errors", errors);
    }
    let _ = dac.stop(0);
}

fn cmd_sweep() {
    // PPS × frame_size combinations to test
    let pps_values:         &[u32]   = &[7_000, 10_000, 15_000, 20_000];
    let frame_size_values:  &[usize] = &[40, 100, 350, 500];
    let secs_per_combo: u64 = 30;

    let dac = open_dac();
    open_shutter(&dac);

    println!(
        "\n{:>8}  {:>10}  {:>10}  {:>8}  {:>8}  {:>8}",
        "PPS", "frame_pts", "hz_target", "frames", "errors", "result"
    );
    println!("{}", "-".repeat(64));

    for &pps in pps_values {
        for &min_pts in frame_size_values {
            let frame = blank_frame(min_pts);
            let hz_target = pps as f64 / min_pts as f64;

            let (frames, errors) = run_loop(
                &dac,
                &frame,
                pps,
                Duration::from_secs(secs_per_combo),
                min_pts,
            );

            let result = if errors == 0 { "PASS" } else { "FAIL" };
            println!(
                "{:>8}  {:>10}  {:>10.1}  {:>8}  {:>8}  {:>8}",
                pps, min_pts, hz_target, frames, errors, result
            );
        }
    }

    let _ = dac.stop(0);
    info!("Sweep complete.");
}

// ─── entry point ──────────────────────────────────────────────────────────────

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    let args: Vec<String> = std::env::args().collect();
    let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match subcmd {
        "info"   => cmd_info(),
        "blink"  => {
            let secs = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(30u64);
            cmd_blink(secs);
        }
        "box"    => {
            let secs = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(60u64);
            cmd_box(secs);
        }
        "stress" => {
            let secs = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(300u64);
            cmd_stress(secs);
        }
        "sweep"  => cmd_sweep(),
        _        => {
            eprintln!("Helios DAC Hardware Test Tool");
            eprintln!();
            eprintln!("USAGE:");
            eprintln!("  sudo dac-test <scenario> [duration_secs]");
            eprintln!();
            eprintln!("SCENARIOS:");
            eprintln!("  info           Print device name, firmware version, status");
            eprintln!("  blink  [secs]  Alternate full-white / blank (default 30 s)");
            eprintln!("  box    [secs]  Draw a 4-corner box (default 60 s)");
            eprintln!("  stress [secs]  Production-identical loop (default 300 s / 5 min)");
            eprintln!("  sweep          Sweep PPS × frame-size combos (30 s each)");
            eprintln!();
            eprintln!("EXAMPLES:");
            eprintln!("  sudo dac-test info");
            eprintln!("  sudo dac-test blink 10");
            eprintln!("  sudo dac-test box 120");
            eprintln!("  sudo dac-test stress 300");
            eprintln!("  sudo dac-test sweep");
        }
    }
}
