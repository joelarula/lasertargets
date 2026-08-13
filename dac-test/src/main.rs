// Standalone Helios DAC Hardware Test Tool
// Tests device connection, frame streaming, and sweeps PPS x point-count combinations
// to discover optimal, stable performance profiles for Raspberry Pi.

#[path = "../../server/src/dac/helios.rs"]
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
    dac: &mut HeliosDacController,
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
            }
            Err(e) => {
                errors += 1;
                consecutive_errors += 1;
                if consecutive_errors == 1 || consecutive_errors % 25 == 0 {
                    error!("  WriteFrame error (#{}) : {}", consecutive_errors, e);
                }

                // Automatic recovery: USB pipe stall (-5007), busy (-1002), or device closed (-1000)
                let _ = dac.close_devices();
                std::thread::sleep(Duration::from_millis(50));
                if let Ok(count) = dac.open_devices() {
                    if count > 0 {
                        let _ = dac.set_shutter(0, true);
                        consecutive_errors = 0;
                    }
                }

                if consecutive_errors >= 25 {
                    error!("  25 consecutive errors — aborting shape test.");
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
    let mut dac = open_dac();
    open_shutter(&dac);

    let frame = make_box_smooth(2000, 100); // 400 pts
    info!(
        "Drawing box ({} pts) at 20 kPPS for {} s…",
        frame.len(),
        duration_secs
    );

    let (frames, errors) = run_loop(
        &mut dac,
        &frame,
        20_000,
        Duration::from_secs(duration_secs),
        350,
    );
    info!("Box done — {} frames, {} errors", frames, errors);
    let _ = dac.stop(0);
}

fn cmd_stress(duration_secs: u64) {
    let mut dac = open_dac();
    open_shutter(&dac);

    // 350 lit points (smooth box) @ 20 kPPS so projection is visible
    let frame = make_box_smooth(2000, 87);
    info!("Stress test: 350 lit pts @ 20 kPPS for {} s…", duration_secs);

    let (frames, errors) = run_loop(
        &mut dac,
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
                &mut dac,
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

const SHAPES_TEMPLATES_JSON: &str = include_str!("../../assets/shapes/shapePatternTemplates.json");
const SHAPES_LINES_JSON: &str = include_str!("../../assets/shapes/lineShapes.json");
const SHAPES_PICS_JSON: &str = include_str!("../../assets/shapes/picArrayShapes.json");

/// Converts shape color index to (R, G, B, Intensity)
pub fn color_to_rgba(col_idx: u8) -> (u8, u8, u8, u8) {
    match col_idx {
        0 => (0, 0, 0, 0),         // BeamColor::Blank
        1 => (255, 0, 0, 255),     // BeamColor::Red
        2 => (255, 255, 0, 255),   // BeamColor::Yellow
        3 => (0, 255, 0, 255),     // BeamColor::Green
        4 => (0, 255, 255, 255),   // BeamColor::Cyan
        5 => (0, 0, 255, 255),     // BeamColor::Blue
        6 => (255, 0, 255, 255),   // BeamColor::Purple
        7 => (255, 255, 255, 255), // BeamColor::White
        8 => (0, 0, 0, 0),         // BeamColor::Jump
        9 => (255, 255, 255, 255), // BeamColor::RGB
        _ => (255, 255, 255, 255),
    }
}

pub fn color_name(col_idx: u8) -> &'static str {
    match col_idx {
        0 => "Blank",
        1 => "Red",
        2 => "Yellow",
        3 => "Green",
        4 => "Cyan",
        5 => "Blue",
        6 => "Purple",
        7 => "White",
        8 => "Jump",
        9 => "RGB",
        _ => "Unknown",
    }
}

fn convert_shape_to_helios_frame(raw_points: &[Vec<f64>], min_pts: usize) -> Vec<HeliosPoint> {
    if raw_points.is_empty() {
        return blank_frame(min_pts);
    }

    // 1. Calculate bounding box for auto-scaling & centering
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;

    for pt in raw_points {
        if pt.len() >= 2 {
            let x = pt[0];
            let y = pt[1];
            if x < min_x { min_x = x; }
            if x > max_x { max_x = x; }
            if y < min_y { min_y = y; }
            if y > max_y { max_y = y; }
        }
    }

    let center_x = (min_x + max_x) / 2.0;
    let center_y = (min_y + max_y) / 2.0;
    let span_x = (max_x - min_x).abs();
    let span_y = (max_y - min_y).abs();
    let max_span = span_x.max(span_y).max(1.0);

    // Standard galvo viewport scale (target span ~2800 units in 4096 coordinate space)
    let scale = (2800.0 / max_span).clamp(0.5, 5.0);

    let mut points = Vec::new();

    for pt in raw_points {
        if pt.len() < 2 {
            continue;
        }
        let px = pt[0];
        let py = pt[1];
        let color_idx = pt.get(2).map(|&c| c as u8).unwrap_or(7);
        let dwell = pt.get(3).map(|&d| d as usize).unwrap_or(0);

        let (r, g, b, i) = color_to_rgba(color_idx);

        let hx = ((2048.0 + (px - center_x) * scale).clamp(0.0, 4095.0)) as u16;
        let hy = ((2048.0 + (py - center_y) * scale).clamp(0.0, 4095.0)) as u16;

        let helios_pt = HeliosPoint::new(hx, hy, r, g, b, i);
        points.push(helios_pt);

        // Handle dwell count (repeat point dwell times)
        for _ in 0..dwell {
            points.push(helios_pt);
        }
    }

    if points.is_empty() {
        return blank_frame(min_pts);
    }

    // Enforce minimum frame size (e.g. 350 pts)
    while points.len() < min_pts {
        let last = points.last().cloned().unwrap_or(HeliosPoint::blanked(HELIOS_CENTER_COORD, HELIOS_CENTER_COORD));
        points.push(HeliosPoint::blanked(last.x, last.y));
    }

    points
}

struct NamedShapeSet {
    name: String,
    shapes: Vec<Vec<Vec<f64>>>,
}

fn parse_shapes_json(name: &str, json_str: &str) -> Option<NamedShapeSet> {
    match serde_json::from_str::<Vec<Vec<Vec<f64>>>>(json_str) {
        Ok(shapes) => Some(NamedShapeSet {
            name: name.to_string(),
            shapes,
        }),
        Err(e) => {
            error!("Failed to parse shapes JSON for '{}': {}", name, e);
            None
        }
    }
}

fn cmd_shapes(duration_per_shape_secs: u64, shape_filter: Option<usize>, selection: Option<&str>) {
    let collections: Vec<NamedShapeSet> = match selection {
        Some("patterns") | Some("templates") => {
            parse_shapes_json("shapePatternTemplates.json", SHAPES_TEMPLATES_JSON).into_iter().collect()
        }
        Some("lines") => {
            parse_shapes_json("lineShapes.json", SHAPES_LINES_JSON).into_iter().collect()
        }
        Some("pics") | Some("picarray") => {
            parse_shapes_json("picArrayShapes.json", SHAPES_PICS_JSON).into_iter().collect()
        }
        Some(path) if path != "all" => {
            match std::fs::read_to_string(path) {
                Ok(content) => parse_shapes_json(path, &content).into_iter().collect(),
                Err(e) => {
                    error!("Failed to read custom shapes JSON file at '{}': {}", path, e);
                    return;
                }
            }
        }
        _ => {
            let mut list = Vec::new();
            if let Some(set) = parse_shapes_json("shapePatternTemplates.json", SHAPES_TEMPLATES_JSON) {
                list.push(set);
            }
            if let Some(set) = parse_shapes_json("lineShapes.json", SHAPES_LINES_JSON) {
                list.push(set);
            }
            if let Some(set) = parse_shapes_json("picArrayShapes.json", SHAPES_PICS_JSON) {
                list.push(set);
            }
            list
        }
    };

    if collections.is_empty() {
        error!("No valid shape collections loaded.");
        return;
    }

    let total_shapes_all: usize = collections.iter().map(|c| c.shapes.len()).sum();
    info!("Loaded {} shape collection(s) with {} total shapes.", collections.len(), total_shapes_all);

    let mut dac = open_dac();
    open_shutter(&dac);

    let pps = 20_000u32;
    let min_pts = 350;

    let mut global_step = 0;

    for set in &collections {
        info!("=== Collection: {} ({} shapes) ===", set.name, set.shapes.len());

        let shape_indices: Vec<usize> = if let Some(idx) = shape_filter {
            if idx < set.shapes.len() {
                vec![idx]
            } else {
                warn!("Shape index {} out of range for collection '{}' (total: {})", idx, set.name, set.shapes.len());
                continue;
            }
        } else {
            (0..set.shapes.len()).collect()
        };

        for &idx in &shape_indices {
            global_step += 1;
            let raw_shape = &set.shapes[idx];
            let frame = convert_shape_to_helios_frame(raw_shape, min_pts);

            let mut unique_colors: Vec<u8> = Vec::new();
            for pt in raw_shape {
                let col = pt.get(2).map(|&c| c as u8).unwrap_or(7);
                if !unique_colors.contains(&col) {
                    unique_colors.push(col);
                }
            }
            let color_names: Vec<&str> = unique_colors.iter().map(|&c| color_name(c)).collect();

            info!(
                "[{}/{}] File: {} | Shape #{} | raw pts: {}, projected pts: {} | Colors: [{}]",
                global_step,
                total_shapes_all,
                set.name,
                idx,
                raw_shape.len(),
                frame.len(),
                color_names.join(", ")
            );

            let (frames, errors) = run_loop(
                &mut dac,
                &frame,
                pps,
                Duration::from_secs(duration_per_shape_secs),
                min_pts,
            );

            if errors > 0 {
                warn!("  Shape #{} in '{}' finished with {} errors (frames: {})", idx, set.name, errors, frames);
            }
        }
    }

    let _ = dac.stop(0);
    info!("✓ All shape pattern tests completed cleanly across {} collections.", collections.len());
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
        "shapes" | "patterns" => {
            let secs = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(3u64);
            let idx_filter = args.get(3).and_then(|s| s.parse::<usize>().ok());
            let custom_file = args.get(4).map(|s| s.as_str());
            cmd_shapes(secs, idx_filter, custom_file);
        }
        _ => {
            eprintln!("Helios DAC Hardware Test & Benchmark Tool");
            eprintln!();
            eprintln!("USAGE:");
            eprintln!("  sudo dac-test <scenario> [duration_secs] [shape_idx] [collection_or_path]");
            eprintln!();
            eprintln!("SCENARIOS:");
            eprintln!("  info                           Print device name, firmware version, status");
            eprintln!("  blink   [secs]                 Alternate full-white / blank (default 30 s)");
            eprintln!("  box     [secs]                 Draw a 4-corner box (default 60 s)");
            eprintln!("  stress  [secs]                 Production-identical loop (default 300 s / 5 min)");
            eprintln!("  sweep                          Benchmark matrix sweep across PPS & frame sizes");
            eprintln!("  shapes  [secs] [idx] [set]     Loop shapes across all 3 files (presets: all, patterns, lines, pics)");
            eprintln!();
        }
    }
}

