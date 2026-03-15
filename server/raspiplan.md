# Plan: Cross-compile & Deploy Server to Raspberry Pi 4

## TL;DR
Build the server binary for aarch64 Linux using `cross` with a custom Docker image that also compiles `libHeliosLaserDAC.so` from source (Grix/helios_dac SDK). Deploy to Raspberry Pi 4 as a systemd service.

## Overview
`cross` uses Docker containers with pre-configured ARM64 toolchains. We create a custom Dockerfile that extends the `cross` base image with the Helios DAC C++ SDK build dependencies (libusb-dev, g++), clones and compiles `libHeliosLaserDAC.so` for aarch64 inside the container, then places it where the Rust build can bundle it. On the Pi side, we deploy the binary + .so + a systemd unit file.

---

## Phase 1: Custom Cross Docker Image

**Goal**: Create a Docker image that extends `cross`'s aarch64 image with Helios DAC SDK build tooling.

1. **Create `docker/Dockerfile.aarch64`** — custom cross-compilation image
   - Base: `ghcr.io/cross-rs/aarch64-unknown-linux-gnu:main` (the official cross base image)
   - Install aarch64 cross-compile dependencies: `libusb-1.0-0-dev:arm64`, `g++-aarch64-linux-gnu`
   - Clone `https://github.com/Grix/helios_dac.git` (pinned to specific commit for reproducibility)
   - Cross-compile `libHeliosLaserDAC.so` for aarch64:
     - Source files: `sdk/cpp/shared_library/HeliosDacAPI.cpp`, `sdk/cpp/HeliosDac.cpp`, `sdk/cpp/idn/*.cpp`
     - Compile command: `aarch64-linux-gnu-g++ -shared -fPIC -std=c++14 -o libHeliosLaserDAC.so HeliosDacAPI.cpp ../HeliosDac.cpp ../idn/idn.cpp ../idn/idnServerList.cpp ../idn/plt-posix.cpp -I.. -I../idn -lusb-1.0 -lpthread`
   - Place the built `.so` at `/opt/helios/libHeliosLaserDAC.so` in the image

2. **Create `Cross.toml`** at workspace root — configure `cross` to use the custom Docker image:
   ```toml
   [target.aarch64-unknown-linux-gnu]
   image = "lasertargets-cross-aarch64"