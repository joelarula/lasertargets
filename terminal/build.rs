use std::fs;
use std::path::Path;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    // Find the target/debug directory
    let target_dir = Path::new(&out_dir)
        .ancestors()
        .nth(4)
        .expect("Failed to find target dir");
    let debug_assets = target_dir.join("debug/assets");
    let source_assets = Path::new("assets");
    if source_assets.exists() {
        let _ = fs::create_dir_all(&debug_assets);
        for entry in fs::read_dir(source_assets).unwrap() {
            let entry = entry.unwrap();
            let from = entry.path();
            let to = debug_assets.join(entry.file_name());
            if from.is_file() {
                let _ = fs::copy(&from, &to);
            } else if from.is_dir() {
                let _ = fs::create_dir_all(&to);
                // Optionally, recursively copy subdirectories if needed
            }
        }
        println!("cargo:rerun-if-changed=assets");
    }
}