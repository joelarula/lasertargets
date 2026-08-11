// Standalone Helios DAC Hardware Test Tool
// Tests device connection, frame streaming, and sweeps PPS x point-count combinations
// to discover optimal, stable performance profiles for Raspberry Pi.

mod helios;

use helios::{
    HeliosDacController, HeliosPoint, HELIOS_CENTER_COORD, HELIOS_FLAGS_DEFAULT,
};
use log::{error, info, warn};
use std::time::{Duration, Instant};

fn open_dac() -> HeliosDacController {
    let mut dac = HeliosDacController::new().expect("Failed to load Helios DAC library");
    let dev_count = dac.open_devices().expect("Failed to query Helios DAC devices");
    if dev_count <= 0 {
        eprintln!("No Helios DAC devices found over USB.");
        std::process::exit(1);
    }
    info!("Found {} Helios DAC device(s)", dev_count);
    dac
}

fn open_shutter(dac: &HeliosDacController) {
    if let Err(e) = dac.set_shutter(0, true) {
        warn!("Failed to open shutter: {}", e);
    } else {
        info!("Shutter opened.");
    }
}

// ─── Loop runner with high-precision microsecond sleep ─────────────────────────

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

    let total_pts = frame.len().max(min_pts) as f64;
    let frame_duration_sec = total_pts / pps as f64;
    // Target sleep time = 75% of frame duration
    let target_sleep_micros = (frame_duration_sec * 750_000.0) as u64;

    while Instant::now() < deadline {
        match dac.write_frame_ready(0, pps, HELIOS_FLAGS_DEFAULT, frame, min_pts) {
            Ok(_) => {
                frames += 1;
                consecutive_errors = 0;

                // High-precision adaptive sleep:
                // For sub-10ms frames, use microsecond sleep to avoid OS scheduler starvation
                if target_sleep_micros > 0 {
                    if target_sleep_micros < 10_000 {
                        std::thread::sleep(Duration::from_micros(target_sleep_micros));
                    } else {
                        std::thread::sleep(Duration::from_millis(target_sleep_micros / 1000));
                    }
                }
            }
            Err(e) => {
                errors += 1;
                consecutive_errors += 1;
                if consecutive_errors == 1 || consecutive_errors % 25 == 0 {
                    error!("  WriteFrame error (#{}) : {}", consecutive_errors, e);
                }
                if consecutive_errors >= 25 {
                    error!("  25 consecutive errors — aborting combo test.");
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }

    (frames, errors)
}

// ─── Geometry helpers ─────────────────────────────────────────────────────────

fn make_box_smooth(size: u16, pts_per_edge: usize) -> Vec<HeliosPoint> {
    let cx = HELIOS_CENTER_COORD as f32;
    let cy = HELIOS_CENTER_COORD as f32;
    let h = size as f32 / 2.0;

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

fn blank_frame(n: usize) -> Vec<HeliosPoint> {
    vec![HeliosPoint::blanked(HELIOS_CENTER_COORD, HELIOS_CENTER_COORD); n]
}

// ─── Scenarios ────────────────────────────────────────────────────────────────

fn cmd_info() {
    let mut dac = open_dac();
    let n = dac.num_devices;

    for dev in 0..n as u32 {
        match dac.get_name(dev) {
            Ok(name) => info!("  Device {} name: {}", dev, name),
            Err(e) => warn!("  Device {} get_name error: {}", dev, e),
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
    let white = vec![
        HeliosPoint::new(
            HELIOS_CENTER_COORD,
            HELIOS_CENTER_COORD,
            255,
            255,
            255,
            255,
        );
        350
    ];
    let blank = blank_frame(350);
    let pps = 20_000u32;
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

    let frame = make_box_smooth(2000, 100); // 400 pts
    info!(
        "Drawing box ({} pts) at 20 kPPS for {} s…",
        frame.len(),
        duration_secs
    );

    let (frames, errors) = run_loop(
        &dac,
        &frame,
        20_000,
        Duration::from_secs(duration_secs),
        350,
    );
    info!("Box done — {} frames, {} errors", frames, errors);
    let _ = dac.stop(0);
}

fn cmd_stress(duration_secs: u64) {
    let dac = open_dac();
    open_shutter(&dac);

    // 350 lit points (smooth box) @ 20 kPPS so projection is visible
    let frame = make_box_smooth(2000, 87);
    info!("Stress test: 350 lit pts @ 20 kPPS for {} s…", duration_secs);

    let (frames, errors) = run_loop(
        &dac,
        &frame,
        20_000,
        Duration::from_secs(duration_secs),
        350,
    );

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
    let pps_values: &[u32] = &[10_000, 15_000, 20_000, 25_000, 30_000];
    let frame_sizes: &[usize] = &[100, 200, 350, 500, 750];
    let test_secs_per_combo: u64 = 4;

    let mut dac = open_dac();
    open_shutter(&dac);

    info!("Starting Performance & Stability Matrix Sweep (4s per combination)...");
    println!(
        "\n{:>8}  {:>10}  {:>10}  {:>8}  {:>8}  {:>12}",
        "PPS", "frame_pts", "hz_target", "frames", "errors", "stability"
    );
    println!("{}", "-".repeat(68));

    #[allow(dead_code)]
    struct ResultRow {
        pps: u32,
        pts: usize,
        hz: f64,
        frames: u64,
        errors: u64,
        grade: &'static str,
    }

    let mut results: Vec<ResultRow> = Vec::new();

    for &pps in pps_values {
        for &min_pts in frame_sizes {
            // Reset & re-open DAC + shutter before each combination to ensure clean C-library state
            let _ = dac.close_devices();
            let _ = dac.open_devices();
            let _ = dac.set_shutter(0, true);

            // Draw a lit box with min_pts points so projection is clearly visible on the wall
            let frame = make_box_smooth(2000, (min_pts / 4).max(1));
            let hz_target = pps as f64 / min_pts as f64;

            let (frames, errors) = run_loop(
                &dac,
                &frame,
                pps,
                Duration::from_secs(test_secs_per_combo),
                min_pts,
            );

            let error_rate = if frames > 0 {
                errors as f64 / frames as f64
            } else {
                1.0
            };
            let grade = if errors == 0 {
                "PERFECT"
            } else if error_rate < 0.01 {
                "STABLE"
            } else {
                "UNSTABLE"
            };

            results.push(ResultRow {
                pps,
                pts: min_pts,
                hz: hz_target,
                frames,
                errors,
                grade,
            });

            println!(
                "{:>8}  {:>10}  {:>10.1}  {:>8}  {:>8}  {:>12}",
                pps, min_pts, hz_target, frames, errors, grade
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    let _ = dac.stop(0);

    // Print summary report
    println!("\n{}", "=".repeat(68));
    println!("  PERFORMANCE & STABILITY RECOMMENDATION REPORT");
    println!("{}", "=".repeat(68));

    let perfect_count = results.iter().filter(|r| r.grade == "PERFECT").count();
    let stable_count = results.iter().filter(|r| r.grade == "STABLE").count();
    let unstable_count = results.iter().filter(|r| r.grade == "UNSTABLE").count();

    println!("  Tested Combinations : {}", results.len());
    println!("  Perfect (0 errors)  : {}", perfect_count);
    println!("  Stable (< 1% err)   : {}", stable_count);
    println!("  Unstable (underrun) : {}", unstable_count);

    println!("\n  Optimal Operating Profile Recommendations:");
    if let Some(best) = results
        .iter()
        .filter(|r| r.grade == "PERFECT" && r.hz >= 30.0 && r.hz <= 70.0)
        .max_by_key(|r| r.pps)
    {
        println!(
            "  ★ Best Performance : {} PPS @ {} points/frame ({:.1} Hz)",
            best.pps, best.pts, best.hz
        );
    } else if let Some(any_perfect) = results.iter().find(|r| r.grade == "PERFECT") {
        println!(
            "  ★ Reliable Profile : {} PPS @ {} points/frame ({:.1} Hz)",
            any_perfect.pps, any_perfect.pts, any_perfect.hz
        );
    }

    println!("{}\n", "=".repeat(68));
}

// ─── Entry point ──────────────────────────────────────────────────────────────

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = std::env::args().collect();
    let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match subcmd {
        "info" => cmd_info(),
        "blink" => {
            let secs = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(30u64);
            cmd_blink(secs);
        }
        "box" => {
            let secs = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(60u64);
            cmd_box(secs);
        }
        "stress" => {
            let secs = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(300u64);
            cmd_stress(secs);
        }
        "sweep" | "benchmark" => cmd_sweep(),
        _ => {
            eprintln!("Helios DAC Hardware Test & Benchmark Tool");
            eprintln!();
            eprintln!("USAGE:");
            eprintln!("  sudo dac-test <scenario> [duration_secs]");
            eprintln!();
            eprintln!("SCENARIOS:");
            eprintln!("  info           Print device name, firmware version, status");
            eprintln!("  blink  [secs]  Alternate full-white / blank (default 30 s)");
            eprintln!("  box    [secs]  Draw a 4-corner box (default 60 s)");
            eprintln!("  stress [secs]  Production-identical loop (default 300 s / 5 min)");
            eprintln!("  sweep          Benchmark matrix sweep across PPS & frame sizes");
            eprintln!();
        }
    }
}
