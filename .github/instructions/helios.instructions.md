# Helios DAC Instructions

**Applies to:** `server/src/dac/helios.rs` and related DAC integration code

## Context
This file wraps the Helios Laser DAC C API via `libloading` and is used by the headless server to send laser point data to hardware. It supports Windows (`HeliosLaserDAC.dll`) and Linux (`libHeliosLaserDAC.so`). The appropriate library is copied to the output directory by `server/build.rs`.

- **Windows**: `HeliosLaserDAC.dll` from `server/libs/`
- **Linux (cross-compiled for Pi)**: `libHeliosLaserDAC.so` built inside the `docker/Dockerfile.aarch64` cross-compilation container from the [Helios DAC C++ SDK](https://github.com/Grix/helios_dac/tree/master/sdk/cpp/shared_library)

## Core Responsibilities
- Load and bind Helios DLL functions safely
- Provide a small, stable Rust API for DAC operations
- Convert Rust types to raw C types
- Handle all errors without panics
- Keep hardware access isolated to the server crate

## Safety & FFI Rules
- **Never** call DLL functions without validating the function pointer
- **Never** `unwrap()` or `expect()` in production paths
- **Always** map Helios error codes to `Result` with descriptive errors
- Keep `unsafe` blocks minimal and contained
- Document any `unsafe` assumptions with `// SAFETY:` comments

## Threading & Performance
- Avoid locks in hot paths if possible
- Do not allocate in per-frame send loops
- Prefer reusing buffers for point data
- Keep timing deterministic; avoid sleeps in update loops unless required by the DAC API

## API Design
- Expose ergonomic Rust methods (e.g., `connect()`, `send_frame()`, `shutdown()`)
- Keep raw FFI types private
- Use `Result<T, HeliosError>` for fallible operations
- Ensure all public APIs are documented with `///` comments

## Dependency Constraints
- Do not add new dependencies unless strictly necessary
- Keep binary size impact minimal

## Error Handling
- Provide a clear error enum for:
  - DLL load failure
  - Missing symbols
  - Device connection failure
  - Send/queue errors
- Include device index and operation context in errors when available

## Platform Notes
- Platform-specific shared library: `HeliosLaserDAC.dll` (Windows), `libHeliosLaserDAC.so` (Linux), `libHeliosLaserDAC.dylib` (macOS)
- Library name selected at compile time via `#[cfg(target_os)]` constants
- On Windows: ensure `HeliosLaserDAC.dll` is present in the runtime directory (copied by `build.rs`)
- On Linux (Raspberry Pi): ensure `libHeliosLaserDAC.so` is in `LD_LIBRARY_PATH` or alongside the binary
- DAC initialization is gracefully handled — server continues without hardware if the library fails to load

## Reference SDK
- https://github.com/Grix/helios_dac/tree/master/sdk (Helios DAC SDK and know-how)

## ILDA Reference
- https://www.ilda.com/technical.htm

## Testing Guidance
- Use mocks or feature flags for hardware-dependent paths
- Avoid requiring hardware in automated tests
