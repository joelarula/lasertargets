use clap::Parser;
use common::shape_v2::Shape;
use common::shapes::ShapeTemplate;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "transform-v2",
    about = "Converts v1 shape templates to Shape v2 format with laser requirements scoping"
)]
struct Args {
    /// Tag filter to select shapes for conversion (e.g. 'pattern:calibration')
    #[arg(long, default_value = "pattern:calibration")]
    tag: String,

    /// Convert all shapes regardless of tag
    #[arg(long)]
    all: bool,

    /// Source directory containing v1 shapes
    #[arg(long, default_value = "assets/shapes/unsorted")]
    input: PathBuf,

    /// Destination directory for converted v2 shapes
    #[arg(long, default_value = "assets/shapes/unsorted_v2")]
    output: PathBuf,

    /// Optional path to export a combined JSON collection file
    #[arg(long, default_value = "assets/shapes/unsorted_v2.json")]
    combined: Option<PathBuf>,

    /// Physical size in metres corresponding to the normalized [-1.0, 1.0] range
    #[arg(long, default_value = "1.0")]
    size_m: f32,

    /// Perform conversion and validation without writing files to disk
    #[arg(long)]
    dry_run: bool,
}

fn format_v2_json(shape: &Shape) -> Result<String, serde_json::Error> {
    let json = serde_json::to_string_pretty(shape)?;
    
    // Format point objects compactly on a single line like builtin templates
    // e.g. "{\n          \"x\": 0.0,\n          \"y\": 0.0\n        }" -> "{\"x\": 0.0, \"y\": 0.0}"
    let re = regex_lite_compact_points(&json);
    Ok(re)
}

/// Compact inline formatting for point items inside points array
fn regex_lite_compact_points(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_points_array = false;
    let mut point_buffer = String::new();
    let mut in_point_obj = false;

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("\"points\": [") {
            in_points_array = true;
            out.push_str(line);
            out.push('\n');
            continue;
        }

        if in_points_array {
            if trimmed == "]" || trimmed == "]," {
                in_points_array = false;
                out.push_str(line);
                out.push('\n');
                continue;
            }

            if trimmed == "{" {
                in_point_obj = true;
                point_buffer.clear();
                point_buffer.push('{');
                continue;
            }

            if in_point_obj {
                if trimmed == "}" || trimmed == "}," {
                    in_point_obj = false;
                    let has_comma = trimmed.ends_with(',');
                    let compacted = point_buffer.trim_end_matches(',').trim().to_string();
                    let indent = "        ";
                    out.push_str(indent);
                    out.push_str(&compacted);
                    out.push('}');
                    if has_comma {
                        out.push(',');
                    }
                    out.push('\n');
                    continue;
                }

                // Append field inside point object
                if !point_buffer.ends_with('{') {
                    point_buffer.push(' ');
                }
                point_buffer.push_str(trimmed);
                continue;
            }
        }

        out.push_str(line);
        out.push('\n');
    }

    out
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("============================================================");
    println!("  Shape Language v2 Transformer (LaserTargets)");
    println!("============================================================");
    println!("Input Directory:  {}", args.input.display());
    println!("Output Directory: {}", args.output.display());
    println!("Filter Tag:       {}", if args.all { "<ALL>" } else { &args.tag });
    println!("Physical Size:    {:.2} m (scale = {:.3})", args.size_m, args.size_m / 2.0);
    println!("Dry Run:          {}", args.dry_run);
    println!("------------------------------------------------------------");

    if !args.input.exists() {
        return Err(format!("Input directory not found: {}", args.input.display()).into());
    }

    if !args.dry_run {
        fs::create_dir_all(&args.output)?;
    }

    let mut entries: Vec<PathBuf> = fs::read_dir(&args.input)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "json"))
        .collect();
    entries.sort();

    let mut converted_count = 0;
    let mut total_scanned = 0;
    let mut converted_shapes: Vec<Shape> = Vec::new();

    for path in &entries {
        total_scanned += 1;
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();
        let content = fs::read_to_string(path)?;

        let template = match ShapeTemplate::from_json(&content) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("  [SKIP] {}: failed to parse as ShapeTemplate: {}", file_name, e);
                continue;
            }
        };

        // Check filter tag
        if !args.all && !template.tags.iter().any(|t| t == &args.tag) {
            continue;
        }

        // Convert v1 -> v2 using canonical converter
        let shape = Shape::from_v1(&template, args.size_m);

        // Validate according to v2 specification rules
        if let Err(e) = shape.validate() {
            eprintln!("  [ERROR] {}: validation failed: {}", file_name, e);
            continue;
        }

        let total_points: usize = shape.strokes.iter().map(|s| s.points.len()).sum();
        let stroke_count = shape.strokes.len();
        
        // Compute bounding box in metres
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for s in &shape.strokes {
            for p in &s.points {
                min_x = min_x.min(p.x);
                max_x = max_x.max(p.x);
                min_y = min_y.min(p.y);
                max_y = max_y.max(p.y);
            }
        }

        println!(
            "  [CONVERT] {:<10} -> {:<20} strokes: {:2}, pts: {:3}, bounds: [{:.2},{:.2}] x [{:.2},{:.2}] m",
            file_name,
            shape.name,
            stroke_count,
            total_points,
            if min_x.is_finite() { min_x } else { 0.0 },
            if max_x.is_finite() { max_x } else { 0.0 },
            if min_y.is_finite() { min_y } else { 0.0 },
            if max_y.is_finite() { max_y } else { 0.0 },
        );

        if !args.dry_run {
            let json_str = format_v2_json(&shape)?;
            let out_path = args.output.join(&file_name);
            fs::write(&out_path, json_str)?;
        }

        converted_shapes.push(shape);
        converted_count += 1;
    }

    // Write combined JSON file if specified
    if let Some(ref combined_path) = args.combined {
        if !args.dry_run && !converted_shapes.is_empty() {
            if let Some(parent) = combined_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let combined_json = serde_json::to_string_pretty(&converted_shapes)?;
            fs::write(combined_path, combined_json)?;
            println!("------------------------------------------------------------");
            println!("  Exported combined collection to {}", combined_path.display());
        }
    }

    println!("------------------------------------------------------------");
    println!(
        "Successfully converted {} of {} shapes matching '{}'",
        converted_count,
        total_scanned,
        if args.all { "<ALL>" } else { &args.tag }
    );
    println!("Output location: {}", args.output.display());
    println!("============================================================");

    Ok(())
}
