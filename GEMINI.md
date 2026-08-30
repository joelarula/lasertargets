# Workspace Rules & Context Scoping

## 1. Multi-Module Context Restriction
This workspace is a multi-crate Rust project with distinct modules:
- `server`: Laser game engine, Bevy app, networking, and hardware dispatch.
- `common`: Shared types, protocols, network packet structs, and math primitives.
- `terminal`: Terminal / CLI client interfaces.
- `minigames/hunter`, `minigames/snake`: Discrete minigame game modes.
- `laserlogic`: Path optimization, polyline generation, galvo tuning, blanking jumps.
- `gamepad`: Gamepad input handling.
- `dac-test`: DAC test harness.
- `shape-editor`: Vector shape editing tool.

### Scoping Rules:
- **Restrict Context to Active Modules**: Limit inspection, searching, and file modifications strictly to the module(s) currently being targeted by the user.
- **Do Not Scan Unrelated Crates**: Never perform repository-wide searches or bulk reads across unrelated crates unless modifying shared contracts in `common` or handling explicit cross-crate integration.
- **Targeted Tooling Commands**: When running `cargo check`, `cargo build`, or `cargo test`, always scope to the active crate using `-p <crate_name>` (e.g. `cargo check -p server`) rather than building the entire workspace.

---

## 2. Token Efficiency & Lazy Context Rules

Always follow these rules to minimize token usage and latency during code analysis, editing, and planning:

### Lazy Loading & Minimal Inspection
- **Search Before Reading**: Use `grep_search` to locate exact symbols, functions, or variable names instead of reading entire files.
- **Slice Views**: When inspecting code, use `view_file` with precise `StartLine` and `EndLine` parameters rather than viewing full 500+ line files.
- **Do Not Re-quote Code**: When explaining changes, highlight only modified symbols or diff snippets rather than echoing large blocks of unchanged code.

### Minimal File Edits
- **Use Precise Diffs**: Always use `replace_file_content` with concise target/replacement chunks rather than rewriting entire files.
- **Do Not Reformat Unrelated Code**: Keep diffs tight to prevent unnecessary token generation.

### Ignore Non-Source Artifacts
- Never read or ingest files excluded by `.geminiignore` (e.g. `*.svg`, `package-lock.json`, `Cargo.lock`, binary files, database WAL journals, or build outputs).

---

## 3. Bevy ECS Architectural Patterns for Token Efficiency

- **Schema & Data Isolation**: Separate pure data types (`Component`, `Resource`, `Event`, `States`) from systems logic.
- **Granular Feature Plugins**: Implement small, modular `Plugin` structs rather than monolithic multi-hundred line plugins.
- **Focused Systems**: Keep system functions small (< 40–50 lines) with specific query filters (`With<T>`, `Without<T>`, `Changed<T>`).
- **Declarative Scheduling**: Use `SystemSet` enums and `.run_if(in_state(...))` instead of nested conditional logic inside system bodies.
- **Shared Contracts in `common`**: Keep network DTOs, laser formats, and math primitives in `common` to prevent cross-crate context bleeding.

