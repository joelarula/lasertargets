# lasertargets
Augmented reality game 

## Building

### Terminal (GUI Client)
```bash
cargo run --package terminal --features bevy/dynamic_linking
```

### Server (Headless)
```bash
cargo run --package server --features bevy/dynamic_linking
```

### Development Tips
- Use `--features bevy/dynamic_linking` to reduce memory usage during compilation
- Use `-j 1` flag if you experience out-of-memory errors: `cargo build -j 1`
- Server can run on both Windows and Raspberry Pi 4

## Testing
```bash
# Unit tests for network messages
cargo test --package common --features bevy/dynamic_linking

# Integration tests for server
cargo test --package server --features bevy/dynamic_linking
```