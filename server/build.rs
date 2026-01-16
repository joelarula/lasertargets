use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Only copy if we are on Windows, as the DLL is Windows-specific
    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        let libs_dir = manifest_dir.join("libs");
        let dll_name = "HeliosLaserDAC.dll";
        let src_path = libs_dir.join(dll_name);

        println!("cargo:rerun-if-changed=libs/HeliosLaserDAC.dll");

        // The OUT_DIR is set by Cargo during build, e.g., target/debug/build/project-hash/out
        // We want to copy the DLL to the same directory as the binary, which is typically
        // 3 levels up: target/debug/
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

        let profile_dir = out_dir
            .ancestors()
            .nth(3)
            .expect("Failed to find profile directory");

        let dest_path = profile_dir.join(dll_name);

        match fs::copy(&src_path, &dest_path) {
            Ok(_) => println!(
                "cargo:warning=Copied {} to {}",
                dll_name,
                dest_path.display()
            ),
            Err(e) => println!("cargo:warning=Failed to copy DLL: {}", e),
        }
    }
}