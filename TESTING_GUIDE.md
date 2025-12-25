# Testing Guide for LaserTargets Network

## Overview
Due to memory constraints during compilation, here's how to test the server and network messages:

## Unit Tests (Lightweight - No Bevy required)

### Message Serialization Tests
Located in: `common/tests/network_test.rs`

These tests verify:
- Message creation (Ping/Pong)
- Serialization/deserialization with bincode
- Round-trip integrity
- Size validation
- Configuration constants

**Run with:**
```bash
# Once Bevy is pre-compiled, run:
cargo test --package common network_test --features bevy/dynamic_linking
```

## Integration Tests (Full System)

### Server-Client Communication Tests
Located in: `server/tests/integration_test.rs`

These tests verify:
1. **Server Startup**: Server starts and listens correctly
2. **Client Connection**: Clients can connect to server
3. **Ping-Pong Flow**: Full message exchange works
4. **Multiple Clients**: Server handles multiple connections
5. **Message Integrity**: Data remains intact through network

**Run with:**
```bash
# Full integration tests
cargo test --package server integration_test --features bevy/dynamic_linking

# Specific test
cargo test --package server test_ping_pong_flow --features bevy/dynamic_linking
```

## Manual Testing

### 1. Start the Server
```bash
cargo run --package server --features bevy/dynamic_linking
```

Expected output:
```
Server started on 0.0.0.0:6000
```

### 2. Start the Terminal (Client)
In another terminal:
```bash
cargo run --package terminal --features bevy/dynamic_linking
```

Expected logs:
```
Connecting to server at 127.0.0.1:6000
Received ping from server at timestamp XXXXX
Sent pong response
```

### 3. Verify Network Traffic
Server logs should show:
```
Sent ping to client 0 at timestamp XXXXX
Received pong from client 0 at timestamp XXXXX
```

## Test on Raspberry Pi 4

### Cross-Compilation (from Windows)
```bash
# Install cross-compilation target
rustup target add armv7-unknown-linux-gnueabihf

# Build for Pi
cargo build --package server --target armv7-unknown-linux-gnueabihf --release
```

### Native Compilation (on Pi)
```bash
# SSH to Pi
ssh your-pi-address

# Clone and build
git clone your-repo
cd lasertargets
cargo build --package server --release

# Run
./target/release/server
```

## Testing Checklist

- [ ] Server starts successfully (Windows)
- [ ] Server starts successfully (Raspberry Pi)
- [ ] Client connects to server
- [ ] Ping messages sent every 2 seconds
- [ ] Pong responses received
- [ ] Multiple clients can connect
- [ ] Messages serialize/deserialize correctly
- [ ] No memory leaks (run for 5+ minutes)
- [ ] Clean disconnect handling

## Debugging Tips

### Enable Debug Logs
Set environment variable:
```bash
# Windows PowerShell
$env:RUST_LOG="debug"
cargo run --package server

# Linux/Pi
RUST_LOG=debug cargo run --package server
```

### Test Message Serialization Manually
```rust
use common::network::NetworkMessage;

let ping = NetworkMessage::Ping { timestamp: 123 };
let bytes = bincode::serialize(&ping).unwrap();
println!("Serialized size: {} bytes", bytes.len());

let deserialized: NetworkMessage = bincode::deserialize(&bytes).unwrap();
assert!(matches!(deserialized, NetworkMessage::Ping { timestamp: 123 }));
```

### Network Diagnostics
```bash
# Check if server is listening (Windows)
netstat -an | findstr "6000"

# Check if server is listening (Linux/Pi)
netstat -tuln | grep 6000

# Test connectivity
telnet localhost 6000
```

## Performance Testing

### Stress Test (Multiple Clients)
Create multiple client instances:
```bash
# Terminal 1-5
for i in {1..5}; do
  cargo run --package terminal &
done
```

Expected: All clients receive pings simultaneously.

### Latency Test
Measure round-trip time by comparing timestamps in logs.

### Throughput Test
Modify ping interval to increase message frequency and monitor CPU/memory usage.

## Continuous Integration

For CI/CD, use these commands:
```yaml
- name: Test Network Messages
  run: cargo test --package common --lib -- --test-threads=1

- name: Build Server
  run: cargo build --package server --release -j 1

- name: Test Server API
  run: cargo test --package server -j 1
```

## Common Issues

### Out of Memory During Test Compilation
**Solution**: Build in release mode or limit parallel jobs:
```bash
cargo test -j 1 --release
```

### Certificate Errors
**Solution**: Tests use `SkipVerification` mode. For production, use proper certificates.

### Port Already in Use
**Solution**: Tests use different ports (6001, 6002, etc.). Change if needed.

### Connection Timeout
**Solution**: Increase sleep durations in tests or check firewall settings.

## Next Steps

1. Add more message types to `NetworkMessage` enum
2. Implement authentication/authorization
3. Add encryption for production use
4. Create performance benchmarks
5. Add chaos testing (network failures, packet loss)
