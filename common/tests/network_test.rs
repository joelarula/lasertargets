use common::network::{NetworkMessage, SERVER_PORT, SERVER_HOST};

#[test]
fn test_network_message_ping_creation() {
    let timestamp = 123456789u64;
    let ping = NetworkMessage::Ping { timestamp };
    
    match ping {
        NetworkMessage::Ping { timestamp: t } => assert_eq!(t, timestamp),
        _ => panic!("Expected Ping variant"),
    }
}

#[test]
fn test_network_message_pong_creation() {
    let timestamp = 987654321u64;
    let pong = NetworkMessage::Pong { timestamp };
    
    match pong {
        NetworkMessage::Pong { timestamp: t } => assert_eq!(t, timestamp),
        _ => panic!("Expected Pong variant"),
    }
}

#[test]
fn test_message_serialization_deserialization() {
    let original = NetworkMessage::Ping { timestamp: 42 };
    
    // Serialize
    let serialized = bincode::serialize(&original)
        .expect("Should serialize NetworkMessage");
    
    // Deserialize
    let deserialized: NetworkMessage = bincode::deserialize(&serialized)
        .expect("Should deserialize NetworkMessage");
    
    // Verify
    match deserialized {
        NetworkMessage::Ping { timestamp } => assert_eq!(timestamp, 42),
        _ => panic!("Deserialized wrong variant"),
    }
}

#[test]
fn test_message_roundtrip_ping() {
    let messages = vec![
        NetworkMessage::Ping { timestamp: 0 },
        NetworkMessage::Ping { timestamp: u64::MAX },
        NetworkMessage::Ping { timestamp: 12345 },
    ];
    
    for msg in messages {
        let serialized = bincode::serialize(&msg).unwrap();
        let deserialized: NetworkMessage = bincode::deserialize(&serialized).unwrap();
        
        match (msg, deserialized) {
            (NetworkMessage::Ping { timestamp: t1 }, NetworkMessage::Ping { timestamp: t2 }) => {
                assert_eq!(t1, t2);
            }
            _ => panic!("Roundtrip failed"),
        }
    }
}

#[test]
fn test_message_roundtrip_pong() {
    let messages = vec![
        NetworkMessage::Pong { timestamp: 0 },
        NetworkMessage::Pong { timestamp: u64::MAX },
        NetworkMessage::Pong { timestamp: 98765 },
    ];
    
    for msg in messages {
        let serialized = bincode::serialize(&msg).unwrap();
        let deserialized: NetworkMessage = bincode::deserialize(&serialized).unwrap();
        
        match (msg, deserialized) {
            (NetworkMessage::Pong { timestamp: t1 }, NetworkMessage::Pong { timestamp: t2 }) => {
                assert_eq!(t1, t2);
            }
            _ => panic!("Roundtrip failed"),
        }
    }
}

#[test]
fn test_server_configuration_constants() {
    assert_eq!(SERVER_PORT, 6000, "Server port should be 6000");
    assert_eq!(SERVER_HOST, "0.0.0.0", "Server host should be 0.0.0.0");
}

#[test]
fn test_message_debug_format() {
    let ping = NetworkMessage::Ping { timestamp: 123 };
    let debug_str = format!("{:?}", ping);
    assert!(debug_str.contains("Ping"));
    assert!(debug_str.contains("123"));
    
    let pong = NetworkMessage::Pong { timestamp: 456 };
    let debug_str = format!("{:?}", pong);
    assert!(debug_str.contains("Pong"));
    assert!(debug_str.contains("456"));
}

#[test]
fn test_message_clone() {
    let original = NetworkMessage::Ping { timestamp: 777 };
    let cloned = original.clone();
    
    match (original, cloned) {
        (NetworkMessage::Ping { timestamp: t1 }, NetworkMessage::Ping { timestamp: t2 }) => {
            assert_eq!(t1, t2);
        }
        _ => panic!("Clone failed"),
    }
}

#[test]
fn test_message_size() {
    let ping = NetworkMessage::Ping { timestamp: 0 };
    let size = bincode::serialized_size(&ping).expect("Should get size");
    
    // bincode typically uses 1 byte for enum variant + 8 bytes for u64
    assert!(size > 0 && size < 100, "Serialized size should be reasonable: {} bytes", size);
}

#[test]
fn test_different_messages_different_serialization() {
    let ping = NetworkMessage::Ping { timestamp: 100 };
    let pong = NetworkMessage::Pong { timestamp: 100 };
    
    let ping_bytes = bincode::serialize(&ping).unwrap();
    let pong_bytes = bincode::serialize(&pong).unwrap();
    
    // Even with same timestamp, the variant discriminant should make them different
    assert_ne!(ping_bytes, pong_bytes, "Ping and Pong should serialize differently");
}
