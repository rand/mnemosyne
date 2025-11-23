#[cfg(test)]
mod tests {
    use crate::api::events::{Event, EventType};
    use chrono::Utc;

    #[test]
    fn test_deserialize_missing_instance_id() {
        // Test case 1: instance_id present
        let json = r#"{"id":"123","type":"heartbeat","instance_id":"test","timestamp":"2023-01-01T00:00:00Z"}"#;
        let event: Event = serde_json::from_str(json).expect("Should deserialize with instance_id");
        assert_eq!(event.instance_id, Some("test".to_string()));

        // Test case 2: instance_id missing (should verify #[serde(default)])
        let json_missing = r#"{"id":"123","type":"heartbeat","timestamp":"2023-01-01T00:00:00Z"}"#;
        let event: Event = serde_json::from_str(json_missing).expect("Should deserialize without instance_id");
        assert_eq!(event.instance_id, None);
        
        // Note: If this fails, it might be because EventType::Heartbeat *also* requires instance_id?
        // Let's check EventType definition.
        if let EventType::Heartbeat { instance_id, .. } = event.event_type {
             // If EventType::Heartbeat requires instance_id, then the flattened structure requires it 
             // unless EventType also defaults it?
             // Actually, in the Event struct, `instance_id` is at the top level.
             // But `EventType` is flattened.
             // If `type`="heartbeat", serde looks at `EventType::Heartbeat`.
             // `EventType::Heartbeat` has `instance_id: String`.
             // So `instance_id` MUST be present for the enum variant, even if `Event` struct handles it.
             // The JSON representation of `Event` flattened means fields go to both? 
             // Or does `Event` peel it off?
             // `flatten` means fields are mixed. 
             // If `EventType::Heartbeat` expects `instance_id`, it MUST be there.
             // We might need to make `instance_id` optional or default in `EventType` variants too,
             // OR ensure `Event` deserialization handles it.
             // Wait, `Event` struct has `instance_id: Option<String>`.
             // But `EventType` has `instance_id: String` for Heartbeat!
             // That is the conflict. 
             // If the JSON is missing `instance_id`, `EventType::Heartbeat` fails to deserialize.
             // We need to fix `EventType` to allow missing instance_id (default to empty string?) 
             // OR change the logic so `Event` handles it and `EventType` doesn't need it?
             // `EventType` variants *define* the payload.
             // If `Heartbeat` is defined as having `instance_id`, then it must have it.
             
             // FIX: We should probably make `instance_id` in `EventType` variants `#[serde(default)]` too 
             // if we want to support missing fields, OR ensure it's always sent.
             // But we are fixing the *subscriber* which receives *whatever the server sends*.
             // If server sends without instance_id (because it's None), then subscriber crashes.
        }
    }
}
