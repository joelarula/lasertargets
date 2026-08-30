use std::env;
use std::fs;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use crossterm::event::{read, Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use laserlogic::helios::{HeliosDacController, HeliosPoint, HELIOS_FLAGS_START_IMMEDIATELY};
use serde_json::Value;

fn main() {
    println!("==================================================");
    println!("       📡 LASERTARGETS - HELIOS DAC TEST TOOL      ");
    println!("==================================================");

    let args: Vec<String> = env::args().collect();

    println!("[1/2] Initializing Helios Laser DAC library...");
    let mut dac = match HeliosDacController::new() {
        Ok(dac) => dac,
        Err(e) => {
            eprintln!("❌ Error initializing Helios DAC library: {}", e);
            return;
        }
    };

    println!("[2/2] Opening connected USB DAC devices...");
    let _num_devices = match dac.open_devices() {
        Ok(count) => {
            println!("✅ Successfully opened {} Helios DAC device(s).", count);
            if count == 0 {
                eprintln!("⚠️ Warning: 0 DAC devices detected. Plug in USB Helios DAC and re-run.");
                let _ = dac.close_devices();
                return;
            }
            count
        }
        Err(e) => {
            eprintln!("❌ Failed to open DAC devices: {}", e);
            return;
        }
    };

    let mut current_x_scale: f32 = 1.35; // Default 1.35x X-scale multiplier to widen squished galvos

    if args.len() > 1 {
        let pattern = parse_pattern_arg(&args[1]);
        let x_scale = if args.len() > 2 { args[2].parse::<f32>().unwrap_or(current_x_scale) } else { current_x_scale };

        if pattern == 9 || pattern == 10 {
            let file_path = if pattern == 9 { "assets/shapes/lineShapes.json" } else { "assets/shapes/picArrayShapes.json" };
            let shape_idx = if args.len() > 2 { args[2].parse::<usize>().unwrap_or(0) } else { 0 };
            println!("▶️ Projecting shape #{} from '{}' for 10 seconds...", shape_idx, file_path);
            run_array_shape_duration(&mut dac, file_path, shape_idx, Duration::from_secs(10));
        } else {
            println!("▶️ Projecting Pattern #{} (X-Scale: {:.2}) for 10 seconds...", pattern, x_scale);
            run_pattern_duration(&mut dac, pattern, Duration::from_secs(10), x_scale);
        }
        println!("Closing DAC hardware...");
        let _ = dac.close_devices();
        println!("Done! 👋");
        return;
    }

    loop {
        println!("\n--------------------------------------------------");
        println!("  Current X-Aspect Ratio Calibration: {:.2}x", current_x_scale);
        println!("--------------------------------------------------");
        println!("  1) 🎯 ALL-IN-ONE 3-RGB Concentric Squares Test (X-Scaled)");
        println!("  2) 🧱 RAW 12-Bit Square Box (500 to 3595, No Software Scaling)");
        println!("  3) 🔴🟢🔵 RAW Triple RGB Circles");
        println!("  4) ⚡ RAW Red/Blue Crosstalk Separation");
        println!("  5) 🔴 RAW Red Laser Diode Solo");
        println!("  6) 🟢 RAW Green Laser Diode Solo");
        println!("  7) 🔵 RAW Blue Laser Diode Solo");
        println!("  8) ↕️ RAW Galvo X/Y Motion Sweep");
        println!("  9) 📜 Browse lineShapes.json (Arrow Keys ⬅️ ➡️ Navigation)");
        println!("  10) 🖼️ Browse picArrayShapes.json (Arrow Keys ⬅️ ➡️ Navigation)");
        println!("  11) 📐 Adjust X-Scale Multiplier Live (Current: {:.2}x)", current_x_scale);
        println!("  q) Quit and Close DAC");
        println!("--------------------------------------------------");
        print!("Select test [1-11, q]: ");
        io::stdout().flush().ok();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }

        let trimmed = input.trim().to_lowercase();
        if trimmed == "q" || trimmed == "quit" || trimmed == "exit" {
            println!("Closing DAC hardware...");
            let _ = dac.close_devices();
            println!("Goodbye! 👋");
            break;
        }

        if trimmed == "9" || trimmed == "line" || trimmed == "lineshapes" {
            browse_array_shapes_interactive(&mut dac, "assets/shapes/lineShapes.json");
            continue;
        }

        if trimmed == "10" || trimmed == "pic" || trimmed == "picarray" {
            browse_array_shapes_interactive(&mut dac, "assets/shapes/picArrayShapes.json");
            continue;
        }

        if trimmed == "11" || trimmed == "scale" || trimmed == "aspect" {
            print!("Enter new X-Scale multiplier (e.g. 1.25, 1.35, 1.45): ");
            io::stdout().flush().ok();
            let mut scale_in = String::new();
            if io::stdin().read_line(&mut scale_in).is_ok() {
                if let Ok(new_scale) = scale_in.trim().parse::<f32>() {
                    current_x_scale = new_scale.clamp(0.5, 2.5);
                    println!("✅ Updated X-Scale multiplier to {:.2}x", current_x_scale);
                } else {
                    println!("Invalid scale number.");
                }
            }
            continue;
        }

        let pattern = parse_pattern_arg(&trimmed);
        if pattern > 0 {
            println!("▶️ Projecting pattern #{} (X-Scale: {:.2}x) for 5 seconds...", pattern, current_x_scale);
            run_pattern_duration(&mut dac, pattern, Duration::from_secs(5), current_x_scale);
        } else {
            println!("Unknown option. Enter 1-11 or q.");
        }
    }
}

fn parse_pattern_arg(arg: &str) -> u32 {
    match arg {
        "1" | "all" | "concentric" | "rgb" => 1,
        "2" | "raw" | "square" | "box" => 2,
        "3" | "circles" => 3,
        "4" | "crosstalk" | "split" => 4,
        "5" | "red" => 5,
        "6" | "green" => 6,
        "7" | "blue" => 7,
        "8" | "sweep" => 8,
        "9" | "line" | "lineshapes" => 9,
        "10" | "pic" | "picarray" => 10,
        _ => arg.parse::<u32>().unwrap_or(1),
    }
}

/// Instant non-blocking interactive shape browser using Arrow Keys (Left / Right / Up / Down / Esc / q)
fn browse_array_shapes_interactive(dac: &mut HeliosDacController, file_path: &str) {
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Failed to read '{}': {}", file_path, e);
            return;
        }
    };

    let shapes_val: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("❌ Failed to parse JSON from '{}': {}", file_path, e);
            return;
        }
    };

    let Some(shapes_array) = shapes_val.as_array() else {
        eprintln!("❌ JSON root in '{}' is not an array", file_path);
        return;
    };

    let total_shapes = shapes_array.len();
    if total_shapes == 0 {
        eprintln!("⚠️ No shapes found in '{}'", file_path);
        return;
    }

    let all_shapes_pts: Arc<Vec<Vec<HeliosPoint>>> = Arc::new(
        shapes_array
            .iter()
            .map(|s_val| convert_array_shape(s_val))
            .collect(),
    );

    let active_idx = Arc::new(AtomicUsize::new(0));
    let is_running = Arc::new(AtomicBool::new(true));

    let active_idx_clone = active_idx.clone();
    let is_running_clone = is_running.clone();
    let shapes_pts_clone = all_shapes_pts.clone();
    let dac_num_devices = dac.num_devices;

    let streaming_handle = thread::spawn(move || {
        let dac_worker = match HeliosDacController::new() {
            Ok(d) => d,
            Err(_) => return,
        };
        let pps = 30000;

        while is_running_clone.load(Ordering::Relaxed) {
            let idx = active_idx_clone.load(Ordering::Relaxed) % shapes_pts_clone.len();
            let points = &shapes_pts_clone[idx];

            if !points.is_empty() && dac_num_devices > 0 {
                if let Ok(true) = dac_worker.get_status(0) {
                    let _ = dac_worker.write_frame(0, pps, HELIOS_FLAGS_START_IMMEDIATELY, points);
                }
            }
            thread::sleep(Duration::from_millis(15));
        }
    });

    println!("\n==================================================");
    println!("  ✅ Loaded '{}' with {} shape(s)", file_path, total_shapes);
    println!("  ⚡ LIVE LASER PROJECTION ACTIVE!");
    println!("==================================================");
    println!("  🎯 ARROW KEY CONTROLS:");
    println!("     ➡️  Right Arrow / ⬇️ Down Arrow  : NEXT Shape");
    println!("     ⬅️  Left Arrow  / ⬆️ Up Arrow    : PREVIOUS Shape");
    println!("     ⏩  Page Down / ⏪ Page Up       : Jump ±10 Shapes");
    println!("     ❌  ESC / 'q'                     : Exit to Menu");
    println!("==================================================\n");

    let _ = enable_raw_mode();

    loop {
        let cur = active_idx.load(Ordering::Relaxed);
        let pts_count = all_shapes_pts[cur].len();
        print!("\r⚡ [Shape {}/{}] ({} pts) — Press ⬅️ / ➡️ Arrow Keys, ESC to Exit...   ", cur, total_shapes - 1, pts_count);
        io::stdout().flush().ok();

        if let Ok(Event::Key(key_event)) = read() {
            match key_event.code {
                KeyCode::Right | KeyCode::Down | KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Enter => {
                    let cur = active_idx.load(Ordering::Relaxed);
                    active_idx.store((cur + 1) % total_shapes, Ordering::Relaxed);
                }
                KeyCode::Left | KeyCode::Up | KeyCode::Char('p') | KeyCode::Char('P') => {
                    let cur = active_idx.load(Ordering::Relaxed);
                    let prev_i = if cur > 0 { cur - 1 } else { total_shapes - 1 };
                    active_idx.store(prev_i, Ordering::Relaxed);
                }
                KeyCode::PageDown => {
                    let cur = active_idx.load(Ordering::Relaxed);
                    active_idx.store((cur + 10) % total_shapes, Ordering::Relaxed);
                }
                KeyCode::PageUp => {
                    let cur = active_idx.load(Ordering::Relaxed);
                    let next_i = if cur >= 10 { cur - 10 } else { total_shapes.saturating_sub(1) };
                    active_idx.store(next_i, Ordering::Relaxed);
                }
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    break;
                }
                _ => {}
            }
        }
    }

    let _ = disable_raw_mode();

    is_running.store(false, Ordering::Relaxed);
    let _ = streaming_handle.join();
    println!("\nStopped live shape stream. Returned to main menu.");
}

fn convert_array_shape(shape_val: &Value) -> Vec<HeliosPoint> {
    let mut points = Vec::new();
    let Some(pts_array) = shape_val.as_array() else {
        return points;
    };

    for item in pts_array {
        let Some(pt) = item.as_array() else { continue; };
        if pt.len() < 2 { continue; }

        let x_raw = pt[0].as_f64().unwrap_or(0.0) as f32;
        let y_raw = pt[1].as_f64().unwrap_or(0.0) as f32;
        let color_code = if pt.len() > 2 { pt[2].as_u64().unwrap_or(7) as u8 } else { 7 };
        let blank_flag = if pt.len() > 3 { pt[3].as_u64().unwrap_or(0) as u8 } else { 0 };

        let x_dac = (((x_raw + 400.0) / 800.0).clamp(0.0, 1.0) * 3595.0 + 250.0) as u16;
        let y_dac = (((y_raw + 400.0) / 800.0).clamp(0.0, 1.0) * 3595.0 + 250.0) as u16;

        let (r, g, b) = match color_code {
            0 => (0, 0, 0),        // Blanked
            1 => (255, 0, 0),      // Red
            2 => (0, 255, 0),      // Green
            3 => (0, 0, 255),      // Blue
            4 => (255, 255, 0),    // Yellow
            5 => (0, 255, 255),    // Cyan
            6 => (255, 0, 255),    // Magenta
            7 => (255, 255, 255),  // White
            _ => (255, 255, 255),
        };

        if blank_flag == 1 || color_code == 0 {
            points.push(HeliosPoint::blanked(x_dac, y_dac));
        } else {
            points.push(HeliosPoint::new(x_dac, y_dac, r, g, b, 255));
        }
    }

    points
}

fn run_array_shape_duration(dac: &mut HeliosDacController, file_path: &str, shape_idx: usize, duration: Duration) {
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Failed to read '{}': {}", file_path, e);
            return;
        }
    };

    let shapes_val: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("❌ Failed to parse JSON: {}", e);
            return;
        }
    };

    if let Some(array) = shapes_val.as_array() {
        if shape_idx < array.len() {
            let points = convert_array_shape(&array[shape_idx]);
            run_helios_points_duration(dac, &points, duration);
        } else {
            eprintln!("❌ Shape index {} out of bounds (Max: {})", shape_idx, array.len() - 1);
        }
    }
}

fn run_helios_points_duration(dac: &mut HeliosDacController, points: &[HeliosPoint], duration: Duration) {
    if points.is_empty() { return; }
    let pps = 30000;
    let start = Instant::now();

    while start.elapsed() < duration {
        if dac.num_devices > 0 {
            if let Ok(true) = dac.get_status(0) {
                let _ = dac.write_frame(0, pps, HELIOS_FLAGS_START_IMMEDIATELY, points);
            }
        }
        thread::sleep(Duration::from_millis(15));
    }
}

fn run_pattern_duration(dac: &mut HeliosDacController, pattern: u32, duration: Duration, x_scale: f32) {
    let pps = 30000;
    let start = Instant::now();
    let mut frame_count = 0u64;

    while start.elapsed() < duration {
        let points = generate_raw_points(pattern, frame_count, x_scale);

        if dac.num_devices > 0 {
            if let Ok(true) = dac.get_status(0) {
                let _ = dac.write_frame(0, pps, HELIOS_FLAGS_START_IMMEDIATELY, &points);
            }
        }

        frame_count += 1;
        thread::sleep(Duration::from_millis(15));
    }
}

/// Applies center-relative X scaling for aspect ratio calibration
fn map_x(x: u16, x_scale: f32) -> u16 {
    let cx = 2048.0f32;
    let dx = (x as f32 - cx) * x_scale;
    (cx + dx).clamp(0.0, 4095.0) as u16
}

/// Generates 12-bit Helios Points (0-4095) with customizable X-Scale aspect ratio calibration
fn generate_raw_points(pattern: u32, frame_idx: u64, x_scale: f32) -> Vec<HeliosPoint> {
    const CORNER_DWELL: usize = 35;
    const LINE_STEPS: usize = 25;
    const CIRCLE_STEPS: usize = 28;

    match pattern {
        // 1: ALL-IN-ONE Concentric RGB Squares Test Pattern
        // Outer Red Square, Middle Green Square, Inner Blue Square
        1 => {
            let mut pts = Vec::with_capacity(750);
            let cx = 2048u16;
            let cy = 2048u16;

            // Outer Square: PURE RED (Half-Size = 1250)
            push_square(&mut pts, cx, cy, 1250, 255, 0, 0, x_scale);

            // Blanked Jump to Middle Green Square
            push_blank_jump(&mut pts, map_x(798, x_scale), 3298, map_x(1198, x_scale), 2898);

            // Middle Square: PURE GREEN (Half-Size = 850)
            push_square(&mut pts, cx, cy, 850, 0, 255, 0, x_scale);

            // Blanked Jump to Inner Blue Square
            push_blank_jump(&mut pts, map_x(1198, x_scale), 2898, map_x(1598, x_scale), 2498);

            // Inner Square: PURE BLUE (Half-Size = 450)
            push_square(&mut pts, cx, cy, 450, 0, 0, 255, x_scale);

            // Blanked Jump back to Outer Red Square
            push_blank_jump(&mut pts, map_x(1598, x_scale), 2498, map_x(798, x_scale), 3298);

            pts
        }

        // 2: RAW 12-Bit Outer Square Box
        2 => {
            let mut pts = Vec::with_capacity(250);
            let min_c = 500u16;
            let max_c = 3595u16;

            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(min_c, x_scale), max_c, 255, 0, 0, 255)); }
            for i in 1..=LINE_STEPS {
                let x = min_c as usize + ((max_c - min_c) as usize * i / LINE_STEPS);
                pts.push(HeliosPoint::new(map_x(x as u16, x_scale), max_c, 255, 0, 0, 255));
            }

            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(max_c, x_scale), max_c, 0, 255, 0, 255)); }
            for i in 1..=LINE_STEPS {
                let y = max_c as usize - ((max_c - min_c) as usize * i / LINE_STEPS);
                pts.push(HeliosPoint::new(map_x(max_c, x_scale), y as u16, 0, 255, 0, 255));
            }

            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(max_c, x_scale), min_c, 0, 0, 255, 255)); }
            for i in 1..=LINE_STEPS {
                let x = max_c as usize - ((max_c - min_c) as usize * i / LINE_STEPS);
                pts.push(HeliosPoint::new(map_x(x as u16, x_scale), min_c, 0, 0, 255, 255));
            }

            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(min_c, x_scale), min_c, 255, 255, 255, 255)); }
            for i in 1..=LINE_STEPS {
                let y = min_c as usize + ((max_c - min_c) as usize * i / LINE_STEPS);
                pts.push(HeliosPoint::new(map_x(min_c, x_scale), y as u16, 255, 255, 255, 255));
            }
            pts
        }

        // 3: RAW Triple RGB Circles
        3 => {
            let mut pts = Vec::with_capacity(150);
            let radius = 450.0f32;

            let cx1 = 900.0f32;
            let cy1 = 2048.0f32;
            for i in 0..=CIRCLE_STEPS {
                let a = (i as f32 / CIRCLE_STEPS as f32) * std::f32::consts::TAU;
                let x = (cx1 + a.cos() * radius) as u16;
                let y = (cy1 + a.sin() * radius) as u16;
                pts.push(HeliosPoint::new(map_x(x, x_scale), y, 255, 0, 0, 255));
            }

            let start_x1 = (cx1 + radius) as u16;
            let end_x2 = (2048.0 - radius) as u16;
            for i in 1..=10 {
                let x = start_x1 as usize + ((end_x2 as usize - start_x1 as usize) * i / 10);
                pts.push(HeliosPoint::blanked(map_x(x as u16, x_scale), 2048));
            }

            let cx2 = 2048.0f32;
            let cy2 = 2048.0f32;
            for i in 0..=CIRCLE_STEPS {
                let a = (i as f32 / CIRCLE_STEPS as f32) * std::f32::consts::TAU;
                let x = (cx2 + a.cos() * radius) as u16;
                let y = (cy2 + a.sin() * radius) as u16;
                pts.push(HeliosPoint::new(map_x(x, x_scale), y, 0, 255, 0, 255));
            }

            let start_x2 = (cx2 + radius) as u16;
            let end_x3 = (3196.0 - radius) as u16;
            for i in 1..=10 {
                let x = start_x2 as usize + ((end_x3 as usize - start_x2 as usize) * i / 10);
                pts.push(HeliosPoint::blanked(map_x(x as u16, x_scale), 2048));
            }

            let cx3 = 3196.0f32;
            let cy3 = 2048.0f32;
            for i in 0..=CIRCLE_STEPS {
                let a = (i as f32 / CIRCLE_STEPS as f32) * std::f32::consts::TAU;
                let x = (cx3 + a.cos() * radius) as u16;
                let y = (cy3 + a.sin() * radius) as u16;
                pts.push(HeliosPoint::new(map_x(x, x_scale), y, 0, 0, 255, 255));
            }

            let start_x3 = (cx3 + radius) as u16;
            let end_x1 = (900.0 - radius) as u16;
            for i in 1..=12 {
                let x = start_x3 as usize - ((start_x3 as usize - end_x1 as usize) * i / 12);
                pts.push(HeliosPoint::blanked(map_x(x as u16, x_scale), 2048));
            }

            pts
        }

        // 4: RAW Red vs Blue Crosstalk
        4 => {
            let mut pts = Vec::with_capacity(300);
            let min_c = 500u16;
            let max_c = 3595u16;
            let mid_x = 2048u16;

            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(min_c, x_scale), max_c, 0, 0, 255, 255)); }
            for i in 1..=LINE_STEPS { let x = min_c as usize + ((mid_x - min_c) as usize * i / LINE_STEPS); pts.push(HeliosPoint::new(map_x(x as u16, x_scale), max_c, 0, 0, 255, 255)); }
            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(mid_x, x_scale), max_c, 0, 0, 255, 255)); }
            for i in 1..=LINE_STEPS { let y = max_c as usize - ((max_c - min_c) as usize * i / LINE_STEPS); pts.push(HeliosPoint::new(map_x(mid_x, x_scale), y as u16, 0, 0, 255, 255)); }
            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(mid_x, x_scale), min_c, 0, 0, 255, 255)); }
            for i in 1..=LINE_STEPS { let x = mid_x as usize - ((mid_x - min_c) as usize * i / LINE_STEPS); pts.push(HeliosPoint::new(map_x(x as u16, x_scale), min_c, 0, 0, 255, 255)); }
            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(min_c, x_scale), min_c, 0, 0, 255, 255)); }
            for i in 1..=LINE_STEPS { let y = min_c as usize + ((max_c - min_c) as usize * i / LINE_STEPS); pts.push(HeliosPoint::new(map_x(min_c, x_scale), y as u16, 0, 0, 255, 255)); }

            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(mid_x, x_scale), max_c, 255, 0, 0, 255)); }
            for i in 1..=LINE_STEPS { let x = mid_x as usize + ((max_c - mid_x) as usize * i / LINE_STEPS); pts.push(HeliosPoint::new(map_x(x as u16, x_scale), max_c, 255, 0, 0, 255)); }
            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(max_c, x_scale), max_c, 255, 0, 0, 255)); }
            for i in 1..=LINE_STEPS { let y = max_c as usize - ((max_c - min_c) as usize * i / LINE_STEPS); pts.push(HeliosPoint::new(map_x(max_c, x_scale), y as u16, 255, 0, 0, 255)); }
            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(max_c, x_scale), min_c, 255, 0, 0, 255)); }
            for i in 1..=LINE_STEPS { let x = max_c as usize - ((max_c - mid_x) as usize * i / LINE_STEPS); pts.push(HeliosPoint::new(map_x(x as u16, x_scale), min_c, 255, 0, 0, 255)); }
            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(mid_x, x_scale), min_c, 255, 0, 0, 255)); }
            for i in 1..=LINE_STEPS { let y = min_c as usize + ((max_c - min_c) as usize * i / LINE_STEPS); pts.push(HeliosPoint::new(map_x(mid_x, x_scale), y as u16, 255, 0, 0, 255)); }
            pts
        }

        // 5: RAW Red Solo
        5 => {
            let mut pts = Vec::with_capacity(250);
            let min_c = 500u16; let max_c = 3595u16;
            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(min_c, x_scale), max_c, 255, 0, 0, 255)); }
            for i in 1..=LINE_STEPS { let x = min_c as usize + ((max_c - min_c) as usize * i / LINE_STEPS); pts.push(HeliosPoint::new(map_x(x as u16, x_scale), max_c, 255, 0, 0, 255)); }
            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(max_c, x_scale), max_c, 255, 0, 0, 255)); }
            for i in 1..=LINE_STEPS { let y = max_c as usize - ((max_c - min_c) as usize * i / LINE_STEPS); pts.push(HeliosPoint::new(map_x(max_c, x_scale), y as u16, 255, 0, 0, 255)); }
            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(max_c, x_scale), min_c, 255, 0, 0, 255)); }
            for i in 1..=LINE_STEPS { let x = max_c as usize - ((max_c - min_c) as usize * i / LINE_STEPS); pts.push(HeliosPoint::new(map_x(x as u16, x_scale), min_c, 255, 0, 0, 255)); }
            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(min_c, x_scale), min_c, 255, 0, 0, 255)); }
            for i in 1..=LINE_STEPS { let y = min_c as usize + ((max_c - min_c) as usize * i / LINE_STEPS); pts.push(HeliosPoint::new(map_x(min_c, x_scale), y as u16, 255, 0, 0, 255)); }
            pts
        }

        // 6: RAW Green Solo
        6 => {
            let mut pts = Vec::with_capacity(250);
            let min_c = 500u16; let max_c = 3595u16;
            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(min_c, x_scale), max_c, 0, 255, 0, 255)); }
            for i in 1..=LINE_STEPS { let x = min_c as usize + ((max_c - min_c) as usize * i / LINE_STEPS); pts.push(HeliosPoint::new(map_x(x as u16, x_scale), max_c, 0, 255, 0, 255)); }
            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(max_c, x_scale), max_c, 0, 255, 0, 255)); }
            for i in 1..=LINE_STEPS { let y = max_c as usize - ((max_c - min_c) as usize * i / LINE_STEPS); pts.push(HeliosPoint::new(map_x(max_c, x_scale), y as u16, 0, 255, 0, 255)); }
            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(max_c, x_scale), min_c, 0, 255, 0, 255)); }
            for i in 1..=LINE_STEPS { let x = max_c as usize - ((max_c - min_c) as usize * i / LINE_STEPS); pts.push(HeliosPoint::new(map_x(x as u16, x_scale), min_c, 0, 255, 0, 255)); }
            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(min_c, x_scale), min_c, 0, 255, 0, 255)); }
            for i in 1..=LINE_STEPS { let y = min_c as usize + ((max_c - min_c) as usize * i / LINE_STEPS); pts.push(HeliosPoint::new(map_x(min_c, x_scale), y as u16, 0, 255, 0, 255)); }
            pts
        }

        // 7: RAW Blue Solo
        7 => {
            let mut pts = Vec::with_capacity(250);
            let min_c = 500u16; let max_c = 3595u16;
            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(min_c, x_scale), max_c, 0, 0, 255, 255)); }
            for i in 1..=LINE_STEPS { let x = min_c as usize + ((max_c - min_c) as usize * i / LINE_STEPS); pts.push(HeliosPoint::new(map_x(x as u16, x_scale), max_c, 0, 0, 255, 255)); }
            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(max_c, x_scale), max_c, 0, 0, 255, 255)); }
            for i in 1..=LINE_STEPS { let y = max_c as usize - ((max_c - min_c) as usize * i / LINE_STEPS); pts.push(HeliosPoint::new(map_x(max_c, x_scale), y as u16, 0, 0, 255, 255)); }
            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(max_c, x_scale), min_c, 0, 0, 255, 255)); }
            for i in 1..=LINE_STEPS { let x = max_c as usize - ((max_c - min_c) as usize * i / LINE_STEPS); pts.push(HeliosPoint::new(map_x(x as u16, x_scale), min_c, 0, 0, 255, 255)); }
            for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(min_c, x_scale), min_c, 0, 0, 255, 255)); }
            for i in 1..=LINE_STEPS { let y = min_c as usize + ((max_c - min_c) as usize * i / LINE_STEPS); pts.push(HeliosPoint::new(map_x(min_c, x_scale), y as u16, 0, 0, 255, 255)); }
            pts
        }

        // 8: RAW Motion Sweep
        _ => {
            let t = (frame_idx as f32 * 0.05).sin() * 0.4 + 0.5;
            let cx = (t * 3095.0 + 500.0) as u16;
            let mut pts = Vec::with_capacity(50);
            for _ in 0..15 { pts.push(HeliosPoint::new(map_x(cx, x_scale), 500, 0, 255, 255, 255)); }
            for i in 1..=20 {
                let y = 500 + ((3595 - 500) * i / 20);
                pts.push(HeliosPoint::new(map_x(cx, x_scale), y as u16, 0, 255, 255, 255));
            }
            for _ in 0..15 { pts.push(HeliosPoint::new(map_x(cx, x_scale), 3595, 0, 255, 255, 255)); }
            pts
        }
    }
}

fn push_square(pts: &mut Vec<HeliosPoint>, cx: u16, cy: u16, half_size: u16, r: u8, g: u8, b: u8, x_scale: f32) {
    const CORNER_DWELL: usize = 35;
    const LINE_STEPS: usize = 25;

    let min_x = cx.saturating_sub(half_size);
    let max_x = (cx + half_size).min(4095);
    let min_y = cy.saturating_sub(half_size);
    let max_y = (cy + half_size).min(4095);

    let map_x = |x: u16| -> u16 {
        let center = 2048.0f32;
        let dx = (x as f32 - center) * x_scale;
        (center + dx).clamp(0.0, 4095.0) as u16
    };

    // Top-Left Corner (Start)
    for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(min_x), max_y, r, g, b, 255)); }
    // Top Edge (Left to Right)
    for i in 1..=LINE_STEPS {
        let x = min_x as usize + ((max_x - min_x) as usize * i / LINE_STEPS);
        pts.push(HeliosPoint::new(map_x(x as u16), max_y, r, g, b, 255));
    }

    // Top-Right Corner
    for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(max_x), max_y, r, g, b, 255)); }
    // Right Edge (Top to Bottom)
    for i in 1..=LINE_STEPS {
        let y = max_y as usize - ((max_y - min_y) as usize * i / LINE_STEPS);
        pts.push(HeliosPoint::new(map_x(max_x), y as u16, r, g, b, 255));
    }

    // Bottom-Right Corner
    for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(max_x), min_y, r, g, b, 255)); }
    // Bottom Edge (Right to Left)
    for i in 1..=LINE_STEPS {
        let x = max_x as usize - ((max_x - min_x) as usize * i / LINE_STEPS);
        pts.push(HeliosPoint::new(map_x(x as u16), min_y, r, g, b, 255));
    }

    // Bottom-Left Corner
    for _ in 0..CORNER_DWELL { pts.push(HeliosPoint::new(map_x(min_x), min_y, r, g, b, 255)); }
    // Left Edge (Bottom to Top)
    for i in 1..=LINE_STEPS {
        let y = min_y as usize + ((max_y - min_y) as usize * i / LINE_STEPS);
        pts.push(HeliosPoint::new(map_x(min_x), y as u16, r, g, b, 255));
    }
}

fn push_blank_jump(pts: &mut Vec<HeliosPoint>, from_x: u16, from_y: u16, to_x: u16, to_y: u16) {
    for i in 1..=12 {
        let x = from_x as i32 + ((to_x as i32 - from_x as i32) * i / 12);
        let y = from_y as i32 + ((to_y as i32 - from_y as i32) * i / 12);
        pts.push(HeliosPoint::blanked(x as u16, y as u16));
    }
}
