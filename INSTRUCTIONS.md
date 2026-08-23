# LaserTargets Developer Instructions & Stability Guidelines

This document details the core architectural rules, USB hardware constraints, Bevy ECS conventions, and network synchronization patterns for the `lasertargets` codebase. 

> [!IMPORTANT]
> **Goal**: Maintain code stability during refactoring and prevent regressions in USB DAC output, laser vector rendering, and client-server network state.

---

## 1. Helios USB DAC Hardware & Communication Rules

### 1.1 Libusb Error Code Classification & In-Place Retry Pacing
* Return values from `Helios_GetStatus` and `Helios_WriteFrame`:
  * `1`: Hardware buffer ready to receive a new frame (`Ok(true)`).
  * `0`: Hardware busy playing current frame (`Ok(false)` — micro-sleep 1ms).
  * `< 0` (such as `-1000`, `-1002`, `-1003`, `-5007`, `-7`, `-9`): Micro-glitch or endpoint busy during bulk transfer.
* **Rule**: `write_frame_native` executes up to **3 in-place retries with stepped backoff** (attempt 0: 2ms, attempt 1: 5ms, attempt 2: 10ms) for transient bulk transfer errors (`-5007`, `-1002`, `-7`, `-9`). Stepped delays allow physical USB endpoint FIFO registers time to drain. If a single retry succeeds, output continues seamlessly without dropping frames.
* **Rule**: If all 3 in-place retries fail, `write_frame_native` returns `Err(...)` so `dac_output_loop` correctly increments failure counters and executes fast USB reset. NEVER swallow or return `Ok(())` unconditionally when all retries fail.

### 1.2 USB Endpoint Polling Pacing
* Rapid status polling (e.g. 1ms/2ms spin loops) floods libusb control endpoints. When rendering 1024-point frames at 30,000 PPS (~34.1ms per frame), un-paced polling causes libusb control buffer overflows (`Closing Helios DAC: too many errors.`).
* **Rule**: Pace status polling at **5ms minimum** in `wait_for_ready` and micro-sleep **6ms** in `dac_output_loop` when busy. This keeps USB status requests under ~60 req/sec (matching physical 30-60 FPS rendering).

### 1.3 Strict Frame Point Padding & Truncation
* The DAC output thread MUST send exactly `dac_min_points` (default 1024 points) per frame to prevent USB packet negotiation stalls.
* **Rule**: 
  * Short frames are padded with blanked copies of the last point (`HeliosPoint::blanked(last.x, last.y)`).
  * Truncated frames must force $r=0, g=0, b=0, i=0$ on the final point to eliminate trailing laser lines between frames.

### 1.12 Unified `laserlogic::helios` Module & Hardware Driver Reuse
* `HeliosDacController`, `HeliosPoint`, and dynamic loading FFI logic reside inside the **`laserlogic` crate** (`laserlogic::helios`).
* **Rule**: All tools (`shape-editor`, `server`, test utilities) MUST import `HeliosDacController` and `HeliosPoint` from `laserlogic::helios` (or re-exported `server::dac::helios`). Duplicate FFI definitions or custom DAC structs are strictly forbidden to ensure identical USB driver behavior across all tools.
* Set `MAX_WRITE_FAILURES` to **5 consecutive write failures** (~150ms of communication disruption).
* Trigger fast USB endpoint reset (`close_devices()` -> `open_devices()`) and 3-frame blank priming immediately after 5 failures to restore output before noticeable frame drops occur.

### 1.5 Initial Frame Priming Before Shutter Open
* Calling `Helios_SetShutter(0, true)` on an unwritten/unprimed DAC buffer causes libusb control endpoint underflows and timeout errors (`-5007` / `-1000`), immediately closing the DAC interface (`Closing Helios DAC: too many errors.`).
* **Rule**: Always write at least 1 valid 1024-point blank frame (`controller.write_frame_native(0, pps, flags, &blank_frame)`) to prime the hardware FIFO buffer BEFORE calling `set_shutter(0, true)`.

### 1.6 Infinite Background Auto-Reconnect Loop
* If fast reset and C library reload (`HeliosDacController::new()`) both fail (e.g., USB DAC physically unplugged), **the DAC thread MUST NEVER exit or terminate**.
* **Rule**: The DAC output thread sets `connected = false` and enters a 2-second background auto-reconnect retry loop. As soon as the USB cable is re-plugged or the USB endpoint recovers, the thread auto-opens the DAC, primes 3 blank frames, opens the shutter, sets `connected = true`, and resumes output automatically.

### 1.7 Process-Lifetime Dynamic Library Singleton (`OnceLock`)
* Re-invoking `dlopen("libHeliosLaserDAC.so")` multiple times in the same process causes duplicate `libusb_init()` calls and competing background event threads, triggering double-claim interface conflicts (`LIBUSB_ERROR_BUSY` / `Closing Helios DAC: too many errors.`).
* **Rule**: `HeliosLib` MUST be loaded exactly once per process lifetime using `static HELIOS_LIB_SINGLETON: OnceLock<Arc<HeliosLib>> = OnceLock::new()`. Calls to `HeliosDacController::new()` obtain a reference to the existing `OnceLock` singleton without re-running `dlopen`.

### 1.8 Single DAC Output Thread Guard
* Bevy ECS system `update_projector` MUST NOT attempt to initialize or spawn a new `dac_output_loop` thread if `dac_controller.thread_running` is already `true`.
* **Rule**: Check `if !connected && !dac_controller.thread_running` before ticking `reconnect_timer`. The background DAC output thread is the single, sole manager of its loop; spawning multiple threads causes endpoint race conditions and crashes `libHeliosLaserDAC.so`.

### 1.9 Dynamic Frame Length Pacing & FIFO Underflow Prevention
* Large complex frames (> 1500 points, such as text + multiple minigame shapes) require longer playback times (e.g. 100ms for 3000 points @ 30kpps). Fixed short polling windows or 6ms loop retry gaps cause thread wake-up misses, starving the physical DAC buffer and producing `-5007` transfer timeouts.
* **Rule**: Sleep for **80% of total frame playback time** (`sleep_micros = total_pts / pps * 800_000.0`), poll `get_status` with **1ms micro-sleeps** (up to **120 attempts = 120ms window**), and use a **1ms busy retry sleep** in `dac_output_loop`. Next frame is always delivered 1–3ms BEFORE current playback completes regardless of frame size (up to 3,600 points).

### 1.10 USB Firmware Settling Sleep & Direct Priming
* Calling `GetStatus()` or `write_frame_ready()` IMMEDIATELY after `open_devices()` fails with USB transfer timeout (`-5007`) or no device (`-1002`) because the physical DAC microcontroller requires a firmware boot delay to initialize internal USB DMA endpoint registers.
* **Rule**: Always sleep **500ms** (`std::thread::sleep(Duration::from_millis(500))`) immediately after `open_devices()`. Use direct `write_frame_native(0, pps, 0, &blank_frame)` with up to **5 attempts (100ms spacing)** to prime initial blank frames before opening the shutter, bypassing status polling during firmware initialization.

### 1.11 Transient USB Status Code Absorption & Stepped Backoff
* Helios DAC C library (`libHeliosLaserDAC.so`) returns transient return codes (`-1002`, `-1000`, `-1003`, `-5007`, `-7`, `-9`) when the USB controller or microcontroller endpoint is busy transferring frame data.
* **Rule**: `get_status` MUST treat these transient status codes as `Ok(false)` (busy playing frame), allowing `wait_for_ready` to micro-sleep (1ms) and poll up to `max_attempts` (120ms window) without incrementing failure counters or triggering false hardware resets.
* **Rule**: `write_frame_native` uses stepped backoff retries (2ms -> 5ms -> 10ms) to allow USB endpoint FIFOs to settle before reporting a hard failure.

---

## 2. Vector Path Rendering & Scene Transforms

### 2.1 Parent Transform Propagation
* Path entities parented to `SceneEntity` at local `Transform::IDENTITY` must evaluate world coordinates through the parent `SceneEntity` transform (`(0.0, 3.0, -10.0)`).
* **Rule**: Never bypass `scene_transform` when computing global transforms in `update_point_buffer`. Local scene coordinates ($x \in [-w/2, +w/2], y \in [-h/2, +h/2]$) map to world scene bounds ($X \in [-5, +5], Y \in [0, 6]$).

### 2.2 Scene Boundary Culling
* Point coordinates are checked against `min_x, max_x, min_y, max_y` in world bounds before mapping to DAC projection coordinates.
* Out-of-bounds points are safely clamped or culled to prevent projecting beyond the target wall.

### 2.3 Perpendicular Reticles & Dwell Blanking
* Center crosshairs and reticles generate 2 independent perpendicular line segments (-X to +X and -Y to +Y) with 4-sample arrival/departure blanked dwells ($r=0, g=0, b=0$).
* **Rule**: Terminal gizmo rendering (`draw_paths`) must skip line segments connecting to blanked points to prevent visible cursor/path tails.

---

## 3. Bevy ECS & System Architecture

### 3.1 Safe Entity Despawning
* Calling `commands.entity(entity).despawn()` on an entity already despawned in the same frame (via recursive parent despawning or minigame cleanup) emits `WARN bevy_ecs::error::handler: The entity does not exist`.
* **Rule**: Always guard despawn commands using `if let Ok(mut entity_cmds) = commands.get_entity(entity) { entity_cmds.despawn(); }`.

### 3.3 Gamepad Control Responsibility Separation
* **Server Gamepad Plugin (`server/src/plugins/gamepad.rs`)**: Controls global system actions:
  * **Button Y (North)**: Toggle Calibration Overlay.
  * **Button X (West)**: Cycle Game Menu Switcher (Hunter $\rightarrow$ Snake $\rightarrow$ Menu).
  * **Button Start**: Laser Power Toggle On / Off.
  * **Button Select**: Server Diagnostic Report.
* **Minigame-Specific Gamepad Systems (`minigames/<game_name>/src/server.rs`)**:
  * **Rule**: Game-specific input mappings, game rules, scoring logic, and state transitions MUST be documented exclusively within each minigame's own module file: `minigames/<game_name>/INSTRUCTIONS.md` (e.g. [`minigames/hunter/INSTRUCTIONS.md`](file:///c:/Users/joela/dev/lasertargets/minigames/hunter/INSTRUCTIONS.md), [`minigames/snake/INSTRUCTIONS.md`](file:///c:/Users/joela/dev/lasertargets/minigames/snake/INSTRUCTIONS.md)).
* **Rule**: NEVER register minigame-specific button handlers (such as Hunter target spawning or Snake direction handling) in `server/src/plugins/gamepad.rs` to prevent duplicate event execution and button collision bugs.

---

## 4. Network Protocol & Synchronization

### 4.1 Single Client Connection Preservation
* Network state mutation flags (such as `attempt.in_flight = false`) must NEVER be executed unconditionally per-frame in system `else` blocks.
* **Rule**: Keep state resets strictly inside discrete connection event handlers (`TerminalState::Connected` / `TerminalState::Disconnected`).
* **Rule**: Always close existing connection handles via `client.close_connection(id)` before spawning a new connection attempt.

### 4.2 VectorFrame Temporal Snapshot Architecture
* Visual paths rendered on client terminals and streamed to the USB DAC are unified as an atomic **`VectorFrame`** snapshot every engine tick.
* **Network Stream (`BroadcastScenePaths`)**: Transmits `BroadcastScenePaths { frame: VectorFrame }` containing `frame_id` sequence index and abstract polylines (`Vec<AbstractPathData>`). Terminals render pure geometric shapes at native monitor refresh rates with zero galvo processing overhead.
* **Projector DAC Stream**: Transforms the same `VectorFrame` into 2D projector bounds ($[-1.0, 1.0]$) and runs `laserlogic::optimize` (TSP segment sorting, angle-proportional corner dwells, laser-off blanking jump step interpolation, and 1024-point frame padding) before streaming to the USB Helios Laser DAC.
* **Rule**: Despawned scene entities (calibration overlays, popped targets, snake body segments) are automatically omitted from each tick's `VectorFrame`, guaranteeing zero entity tracking desync and zero memory leaks.

---

## 5. Workspace Crate Boundaries & Module Responsibility Rules

### 5.1 Crate Isolation & Zero Cross-Contamination
* **`server/` (`server::*`)**: Host engine application.
  * **Role**: Manages Projector DAC output streaming, scene coordinate transforms, network socket broadcasting, and global gamepad shortcuts.
  * **Rule**: NEVER place minigame-specific logic (e.g. Hunter target spawning or Snake grid movement) or hardcode game IDs/names (`HUNTER_GAME_ID`, `SNAKE_GAME_ID`) inside `server/`. Query `GameRegistry` dynamically.
* **`common/` (`common::*`)**: Shared protocol data types.
  * **Role**: Network wire messages (`NetworkMessage`), common state enums (`ServerState`, `GameState`), configuration resources, and `GameRegistry` definitions.
  * **Rule**: Contains only shared types, structs, and protocol traits. Zero game execution systems, hardcoded minigame structs, or peripheral hardware drivers. All game-specific events and score stats use generic `GameDataPayload { game_id, session_id, event_tag, payload_json }`.
* **`minigames/<game_name>/` (`hunter`, `snake`, etc.)**: Self-contained game modules.
  * **Role**: Each minigame crate MUST own all of its game-specific entities, components, resources, gamepad input listeners, collision detection systems, score stats, network payload serialization, and report generators inside `minigames/<game_name>/src/server.rs` and `minigames/<game_name>/src/terminal.rs`.
  * **Rule**: Minigames register themselves dynamically with `GameRegistry` during plugin build (`game_registry.register_game(...)`).
* **`gamepad/` (`gamepad::*`)**: Hardware controller driver crate.
  * **Role**: Polls raw controller inputs via `gilrs` and exposes thread-safe `GamepadState` and `ServerGamepadCursor` resources.
  * **Rule**: Contains zero game rules, UI logic, or application state transitions.
* **`laserlogic/` (`laserlogic::*`)**: Vector laser math, optimization & DAC driver crate.
  * **Role**: TSP path sorting, corner dwell calculation, blanking jump insertion, vector text rendering, AND shared Helios USB DAC controller driver (`laserlogic::helios`).
  * **Skill**: Detailed polyline preparation, corner dwell formulas, and scanner optimization guides are documented in the [**`laser-path-prep` Skill**](file:///.agents/skills/laser-path-prep/SKILL.md).
  * **Rule**: Holds all DAC FFI logic (`libloading`), status code handling, stepped USB backoffs, and frame point padding. Zero Bevy ECS dependencies, zero network protocols.
* **`shape-editor/` (`shape-editor::*`)**: Interactive Local Shape Studio & Laser Test Tool.
  * **Role**: Pure `eframe` + `egui` Painter canvas application for interactive 2D shape editing and live USB DAC preview.
  * **Rule**: Runs locally on Windows PC with physical USB Helios Laser DAC plugged in. Watches `assets/shapes/templates/active_shape.json` for live Copilot chat edits. Uses `laserlogic::optimize` and `laserlogic::helios` for real-time telemetry stats (Input Vertices vs. Optimized DAC Points).

---

## 6. Build Pipeline & Automation Scripts

### 6.1 Local PC Shape Development & Laser Testing
* **Script**: `.\scripts\run-shape-editor.ps1`
* **Purpose**: Launches `cargo run --package shape-editor`.
* **Workflow**:
  1. Open `shape-editor` on local Windows PC with USB Helios DAC attached.
  2. Drag vertices on screen or request shape modifications in Copilot chat.
  3. Copilot edits [`assets/shapes/templates/active_shape.json`](file:///c:/Users/joela/dev/lasertargets/assets/shapes/templates/active_shape.json).
  4. `shape-editor` disk watcher detects file modification instantly, reloads template, runs `laserlogic::optimize`, and streams optimized points live to USB DAC.

### 6.2 Remote Docker ARM64 Cross-Compilation (Pi 4)
* **Script**: `.\scripts\docker-build-rpi4-remote.ps1 -RemoteHost <user@host>`
* **Purpose**: Compiles Linux ARM64 binaries (`aarch64-unknown-linux-gnu`) inside a Docker container on a fast local workstation.
* **Workflow**: Prevents slow compilation and thermal throttling on physical Raspberry Pi hardware.

### 6.3 Deployment & Remote Execution
* **Script**: `.\scripts\deploy-pi.ps1 -TargetHost <user@host>`
  * Deploys compiled `server` binary, `libHeliosLaserDAC.so`, assets, and templates via SSH/SCP to `/opt/lasertargets/`.
* **Script**: `.\scripts\run-server-pi.ps1 -TargetHost <user@host>`
  * Executes the host server interactively on Pi with live systemd journal log streaming.
* **Script**: `.\scripts\optimize-pi-system.ps1`
  * Tunes Raspberry Pi 4 OS settings: sets CPU governor to `performance`, configures USB buffer depth, and grants `CAP_SYS_NICE` for real-time laser thread scheduling.

---

## 7. Command Reference Summary

| Action | Command |
| :--- | :--- |
| **Check Workspace Compilation** | `cargo check --workspace` |
| **Check Core Server & Terminal** | `cargo check --package server --package terminal` |
| **Run Local Shape Studio (PC)** | `.\scripts\run-shape-editor.ps1` (or `cargo run --package shape-editor`) |
| **Remote Docker Build (ARM64)** | `.\scripts\docker-build-rpi4-remote.ps1 -RemoteHost joel@192.168.1.110` |
| **Deploy Assets & Binaries to Pi** | `.\scripts\deploy-pi.ps1 -TargetHost lasertargets@lasertargets.local` |
| **Run Interactive Server on Pi** | `.\scripts\run-server-pi.ps1 -TargetHost lasertargets@lasertargets.local` |
| **Optimize Pi OS Performance** | `.\scripts\optimize-pi-system.ps1` |

