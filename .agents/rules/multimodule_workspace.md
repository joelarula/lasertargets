# Multi-Module Workspace Context Restriction

This repository is a multi-module workspace. Context must strictly be restricted to the active modules under development.

## 1. Module Scoping
- **Active Module Isolation**: Limit exploration, searching, and file modifications strictly to the module(s) currently being targeted by the user.
- **Avoid Global Workspace Scans**: Do not run unrestricted directory-wide or workspace-wide file scans unless modifying shared interface definitions (e.g. `common/src/protocol.rs`) or explicit cross-module dependencies.

## 2. Targeted Execution & Testing
- When running cargo commands or test suites, scope commands directly to the target package:
  - `cargo check -p <crate_name>`
  - `cargo test -p <crate_name>`
- Avoid workspace-wide builds (`cargo build --workspace`) unless explicitly requested or during final release validation.
