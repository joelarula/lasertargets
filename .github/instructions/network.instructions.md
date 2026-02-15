# Network Protocol Instructions

**Applies to:** Files dealing with `NetworkMessage`, network communication, or `bevy_quinnet`

## CRITICAL RULES

### ❌ DON'T: Use QuinnetServer/QuinnetClient directly in game plugins
```rust
// ❌ WRONG: Game plugin using server directly
fn spawn_target(
    mut server: ResMut<QuinnetServer>,  // ❌ NO!
) {
    server.get_endpoint_mut().broadcast_payload(...);
}
```

### ✅ DO: Use internal message pattern
```rust
// ✅ CORRECT: Game plugin raises event
#[derive(Message, Debug, Clone)]
pub struct BroadcastTargetSpawned { /* ... */ }

fn spawn_target(
    mut events: MessageWriter<BroadcastTargetSpawned>,  // ✅ YES!
) {
    events.write(BroadcastTargetSpawned { /* ... */ });
}

// Network plugin handles broadcasting (in network.rs)
fn broadcast_target_events(
    mut server: ResMut<QuinnetServer>,
    mut events: MessageReader<BroadcastTargetSpawned>,
) {
    for event in events.read() {
        let msg = NetworkMessage::TargetSpawned { /* ... */ };
        endpoint.broadcast_payload(msg.to_bytes());
    }
}
```

**Only `server/src/plugins/network.rs` touches `QuinnetServer`.**  
**Only `terminal/src/plugins/network.rs` touches `QuinnetClient`.**

## Context
You are working with the LaserTargets network protocol. The system uses QUIC (via `bevy_quinnet`) for client-server communication with binary serialization (bincode).

## Network Architecture

### Components
- **Server**: Headless app with `QuinnetServerPlugin`
- **Terminal**: Client app with `QuinnetClientPlugin`
- **Protocol**: `NetworkMessage` enum in `common/src/network.rs`
- **Serialization**: Binary via `bincode`
- **Channels**: QUIC supports both reliable and unreliable channels

### Channel Selection
Network crate can decide which messages use reliable vs unreliable channels:
- **Reliable**: State updates, configuration changes, critical events (guaranteed delivery, ordered)
- **Unreliable**: High-frequency input, position updates, non-critical events (lower latency, can tolerate loss)
- Decision made per message type in network plugin implementation

## Message Design Patterns

### Message Categories

#### 1. Query-Response Pattern
Client requests data from server:
```rust
// Client sends:
NetworkMessage::QueryServerState

// Server responds:
NetworkMessage::ServerStateUpdate(ServerState::InGame)
```

#### 2. Update Pattern
Client or server pushes state changes:
```rust
// Client sends:
NetworkMessage::UpdateSceneConfig(new_config)

// Server broadcasts to all:
NetworkMessage::SceneConfigUpdate(new_config)
```

#### 3. Event Pattern
Notify about actions or state changes:
```rust
NetworkMessage::InitGameSession(session_id, game_id, initial_state)
NetworkMessage::GameSessionCreated(game_session)
```

#### 4. Input Pattern
Client sends user input to server:
```rust
NetworkMessage::MouseButtonInput {
    button: "Left".to_string(),
    pressed: true,
    position: Some(world_pos),
}

NetworkMessage::KeyboardInput {
    key: "Space".to_string(),
    pressed: true,
}
```

### Naming Conventions

| Pattern | Example | Direction |
|---------|---------|-----------|
| `Query*` | `QueryServerState` | Client → Server |
| `Update*` | `UpdateSceneConfig` | Client → Server |
| `*Update` | `ServerStateUpdate` | Server → Client |
| `*Created` | `GameSessionCreated` | Server → Client |
| `*Event` | `StartGameEvent` | Either direction |
| `*Input` | `MouseButtonInput` | Client → Server |
| `*Response` | `ActorResponse` | Server → Client |
| `*Error` | `ActorError` | Server → Client |

## Server-Side Implementation

### Network Plugin Structure
In `server/src/plugins/network.rs`:

```rust
pub struct NetworkingPlugin;

impl Plugin for NetworkingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(QuinnetServerPlugin::default())
            .add_systems(Startup, start_listening)
            .add_systems(Update, (
                handle_client_messages,
                handle_connection_events,
            ));
    }
}
```

### Message Handling
```rust
fn handle_client_messages(
    mut endpoint: ResMut<ServerEndpoint>,
    // Resources needed for message handling
    mut commands: Commands,
    server_state: Res<State<ServerState>>,
) {
    while let Some((client_id, message)) = endpoint.receive_message::<NetworkMessage>() {
        match message {
            NetworkMessage::QueryServerState => {
                endpoint.send_message(
                    client_id,
                    NetworkMessage::ServerStateUpdate(server_state.get().clone())
                );
            }
            
            NetworkMessage::UpdateSceneConfig(config) => {
                // Validate and apply config
                commands.insert_resource(config.clone());
                
                // Broadcast to all clients
                endpoint.broadcast_message(
                    NetworkMessage::SceneConfigUpdate(config)
                );
            }
            
            // Handle other messages...
            _ => {}
        }
    }
}
```

### Broadcasting
```rust
// Send to all connected clients
endpoint.broadcast_message(message);

// Send to specific client
endpoint.send_message(client_id, message);

// Send to all except one
for id in endpoint.clients() {
    if id != exclude_client_id {
        endpoint.send_message(id, message);
    }
}
```

## Client-Side Implementation

### Network Plugin Structure
In `terminal/src/plugins/network.rs`:

```rust
pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(QuinnetClientPlugin::default())
            .add_systems(Startup, connect_to_server)
            .add_systems(Update, (
                handle_server_messages,
                sync_local_changes,
            ));
    }
}
```

### Message Handling
```rust
fn handle_server_messages(
    mut client: ResMut<ClientEndpoint>,
    mut commands: Commands,
) {
    while let Some(message) = client.receive_message::<NetworkMessage>() {
        match message {
            NetworkMessage::ServerStateUpdate(state) => {
                commands.insert_resource(state);
            }
            
            NetworkMessage::SceneConfigUpdate(config) => {
                commands.insert_resource(config);
            }
            
            // Handle other messages...
            _ => {}
        }
    }
}
```

### Sending Messages
```rust
fn sync_local_changes(
    config: Res<SceneConfiguration>,
    mut client: ResMut<ClientEndpoint>,
) {
    // Only send if changed (not on first add)
    if config.is_changed() && !config.is_added() {
        client.send_message(
            NetworkMessage::UpdateSceneConfig(config.clone())
        );
    }
}
```

## Adding New Messages

### 1. Define in common/src/network.rs
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    // ... existing variants
    
    // Add your new message
    QueryMyFeature,
    UpdateMyFeature(MyFeatureData),
    MyFeatureUpdate(MyFeatureData),
}
```

### 2. Handle on Server
In `server/src/plugins/network.rs`:
```rust
fn handle_client_messages(/* ... */) {
    while let Some((client_id, message)) = endpoint.receive_message::<NetworkMessage>() {
        match message {
            // ... existing handlers
            
            NetworkMessage::QueryMyFeature => {
                let data = my_feature_resource.data.clone();
                endpoint.send_message(
                    client_id,
                    NetworkMessage::MyFeatureUpdate(data)
                );
            }
            
            NetworkMessage::UpdateMyFeature(data) => {
                // Validate
                commands.insert_resource(MyFeature(data.clone()));
                
                // Broadcast
                endpoint.broadcast_message(
                    NetworkMessage::MyFeatureUpdate(data)
                );
            }
        }
    }
}
```

### 3. Handle on Client
In `terminal/src/plugins/network.rs`:
```rust
fn handle_server_messages(/* ... */) {
    while let Some(message) = client.receive_message::<NetworkMessage>() {
        match message {
            // ... existing handlers
            
            NetworkMessage::MyFeatureUpdate(data) => {
                commands.insert_resource(MyFeature(data));
            }
        }
    }
}
```

## Connection Management

### Reconnection Behavior
- **Terminal and server must support reconnection** when either is restarted
- Terminal should automatically attempt to reconnect on connection loss
- Server should accept reconnections from previously connected clients
- Both sides should re-sync state after reconnection

### Server
```rust
fn handle_connection_events(
    mut connection_events: EventReader<ConnectionEvent>,
) {
    for event in connection_events.read() {
        match event {
            ConnectionEvent::Connected(client_id) => {
                info!("Client {} connected", client_id);
                // Send initial state
            }
            ConnectionEvent::Disconnected(client_id) => {
                info!("Client {} disconnected", client_id);
                // Clean up client resources
            }
        }
    }
}
```

### Client
```rust
fn handle_connection_state(
    client: Res<ClientEndpoint>,
    mut next_state: ResMut<NextState<TerminalState>>,
) {
    if client.is_connected() {
        next_state.set(TerminalState::Connected);
    } else {
        next_state.set(TerminalState::Connecting);
    }
}
```

## Serialization

### Requirements
All message payload types must derive `Serialize` and `Deserialize`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyFeatureData {
    pub field: f32,
}
```

### Bevy Types
Most Bevy types already implement serde traits:
- `Vec3`, `Vec2`, `Quat`
- `Transform` (use `TransformBundle` if needed)
- `Color`

### Custom Types
For complex types, ensure all nested types are serializable:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Complex {
    pub position: Vec3,
    pub rotation: Quat,
    pub metadata: HashMap<String, String>,
}
```

## Best Practices

### Server
- ✅ Validate all client input
- ✅ Maintain authoritative state
- ✅ Broadcast state changes to all clients
- ✅ Handle disconnections gracefully
- ✅ Rate limit client messages if needed
- ❌ Don't trust client state without validation
- ❌ Don't allow clients to dictate server state directly

### Client
- ✅ Query initial state on connection
- ✅ Cache server state in Resources
- ✅ Re-query on reconnection
- ✅ Show connection status to user
- ✅ Handle message errors gracefully
- ❌ Don't store authoritative game state
- ❌ Don't send messages when disconnected
- ❌ Don't spam server with identical messages

### Protocol Design
- ✅ Keep messages small and focused
- ✅ Use binary serialization (bincode)
- ✅ Version your protocol if needed
- ✅ Document message flow
- ✅ Use enums for message types
- ❌ Don't send entire world state every frame
- ❌ Don't use strings for frequently-sent data
- ❌ Don't create circular message dependencies

## Debugging

### Enable Logging
```rust
RUST_LOG=bevy_quinnet=debug cargo run
```

### Message Tracing
Add debug logging in handlers:
```rust
while let Some((client_id, message)) = endpoint.receive_message::<NetworkMessage>() {
    debug!("Received from {}: {:?}", client_id, message);
    // Handle message
}
```

### Common Issues
1. **Serialization failure**: Ensure all types derive Serialize/Deserialize
2. **Message not received**: Check both sender and receiver are registered for NetworkMessage type
3. **Connection drops**: Check network firewall settings, use connection events
4. **State desync**: Ensure server broadcasts all state changes
