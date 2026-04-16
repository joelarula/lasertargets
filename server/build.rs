use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    // Navigate from server/ up to workspace root, then into target/<profile>
    let workspace_root = manifest_dir.parent().expect("Failed to find workspace root");
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let profile_dir = workspace_root.join("target").join(&profile);

    if target_os == "windows" {
        copy_windows_dll(&profile_dir);
    } else if target_os == "linux" {
        copy_linux_so(&profile_dir);
    }
}

/// Copy HeliosLaserDAC.dll from server/libs/ to the target output directory (Windows)
fn copy_windows_dll(profile_dir: &std::path::Path) {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let libs_dir = manifest_dir.join("libs");
    let dll_name = "HeliosLaserDAC.dll";
    let src_path = libs_dir.join(dll_name);

    println!("cargo:rerun-if-changed=libs/HeliosLaserDAC.dll");

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

/// Copy libHeliosLaserDAC.so from /opt/ (inside the cross Docker container) to the target output directory.
/// When not cross-compiling, falls back to server/libs/ if available.
fn copy_linux_so(profile_dir: &std::path::Path) {
    let so_name = "libHeliosLaserDAC.so";

    // In the cross Docker container, the .so is pre-built at /opt/
    let cross_path = PathBuf::from("/opt").join(so_name);
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let local_path = manifest_dir.join("libs").join(so_name);

    let src_path = if cross_path.exists() {
        cross_path
    } else if local_path.exists() {
        local_path
    } else {
        println!("cargo:warning={} not found — DAC will be unavailable at runtime", so_name);
        return;
    };

    println!("cargo:rerun-if-changed={}", src_path.display());

    let dest_path = profile_dir.join(so_name);

    match fs::copy(&src_path, &dest_path) {
        Ok(_) => println!(
            "cargo:warning=Copied {} to {}",
            so_name,
            dest_path.display()
        ),
        Err(e) => println!("cargo:warning=Failed to copy .so: {}", e),
    }
}