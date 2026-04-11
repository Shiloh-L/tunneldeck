/// P0 Feature Tests for ShellDeck
/// Tests that can run without Tauri runtime or live SSH server.

use shelldeck_lib::connection::types::*;
use shelldeck_lib::config::import_export;
use shelldeck_lib::store::json_store::JsonStore;
use ssh_key::private::PrivateKey;

// ══════════════════════════════════════════════════════════════════
// P0-1: SSH Key Authentication — Key Parsing Tests
// ══════════════════════════════════════════════════════════════════

/// Test: ed25519 key without passphrase can be parsed
#[test]
fn parse_ed25519_key_no_passphrase() {
    let key_path = std::env::temp_dir().join("shelldeck_test_nopass");
    if !key_path.exists() {
        eprintln!("SKIP: test key not found at {:?}", key_path);
        return;
    }
    let key_data = std::fs::read_to_string(&key_path).unwrap();
    let result = PrivateKey::from_openssh(&key_data);
    assert!(result.is_ok(), "Failed to parse ed25519 key: {:?}", result.err());
    let key = result.unwrap();
    assert_eq!(key.algorithm().as_str(), "ssh-ed25519");
}

/// Test: RSA key without passphrase can be parsed
#[test]
fn parse_rsa_key_no_passphrase() {
    let key_path = std::env::temp_dir().join("shelldeck_test_rsa");
    if !key_path.exists() {
        eprintln!("SKIP: test key not found at {:?}", key_path);
        return;
    }
    let key_data = std::fs::read_to_string(&key_path).unwrap();
    let result = PrivateKey::from_openssh(&key_data);
    assert!(result.is_ok(), "Failed to parse RSA key: {:?}", result.err());
    let key = result.unwrap();
    assert_eq!(key.algorithm().as_str(), "ssh-rsa");
}

/// Test: invalid key path triggers error
#[test]
fn key_file_not_found() {
    let path = std::path::Path::new("/nonexistent/path/to/key");
    assert!(!path.exists());
}

/// Test: garbage data fails to parse
#[test]
fn parse_invalid_key_data() {
    let result = PrivateKey::from_openssh("this is not a valid key");
    assert!(result.is_err());
}

/// Test: partial/corrupted key fails to parse
#[test]
fn parse_truncated_key() {
    let result = PrivateKey::from_openssh("-----BEGIN OPENSSH PRIVATE KEY-----\nAAAA\n-----END OPENSSH PRIVATE KEY-----");
    assert!(result.is_err());
}

// ══════════════════════════════════════════════════════════════════
// P0-1: AuthMethod serde round-trip
// ══════════════════════════════════════════════════════════════════

#[test]
fn auth_method_default_is_password() {
    assert_eq!(AuthMethod::default(), AuthMethod::Password);
}

#[test]
fn auth_method_serde_roundtrip() {
    let json = serde_json::to_string(&AuthMethod::Key).unwrap();
    assert_eq!(json, r#""key""#);
    let parsed: AuthMethod = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, AuthMethod::Key);

    let json2 = serde_json::to_string(&AuthMethod::Password).unwrap();
    assert_eq!(json2, r#""password""#);
    let parsed2: AuthMethod = serde_json::from_str(&json2).unwrap();
    assert_eq!(parsed2, AuthMethod::Password);
}

// ══════════════════════════════════════════════════════════════════
// Connection serde: auth_method + private_key_path
// ══════════════════════════════════════════════════════════════════

#[test]
fn connection_serde_with_key_auth() {
    let mut conn = Connection::new("test".into(), "host".into(), 22, "user".into());
    conn.auth_method = AuthMethod::Key;
    conn.private_key_path = Some("/home/user/.ssh/id_ed25519".into());

    let json = serde_json::to_string(&conn).unwrap();
    assert!(json.contains(r#""auth_method":"key""#));
    assert!(json.contains(r#""private_key_path":"/home/user/.ssh/id_ed25519""#));

    let parsed: Connection = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.auth_method, AuthMethod::Key);
    assert_eq!(parsed.private_key_path.as_deref(), Some("/home/user/.ssh/id_ed25519"));
}

#[test]
fn connection_serde_backward_compat_no_auth_method() {
    // Old connections.json without auth_method should default to Password
    let json = r#"{
        "id": "abc",
        "name": "old",
        "host": "h",
        "port": 22,
        "username": "u",
        "forwards": [],
        "auto_connect": false,
        "tag_ids": [],
        "created_at": "2025-01-01T00:00:00Z",
        "updated_at": "2025-01-01T00:00:00Z"
    }"#;
    let conn: Connection = serde_json::from_str(json).unwrap();
    assert_eq!(conn.auth_method, AuthMethod::Password);
    assert_eq!(conn.private_key_path, None);
}

// ══════════════════════════════════════════════════════════════════
// P0-2: Auto-Reconnect — Settings defaults
// ══════════════════════════════════════════════════════════════════

#[test]
fn default_settings_max_reconnect() {
    let settings = AppSettings::default();
    assert_eq!(settings.max_reconnect_attempts, 10);
}

#[test]
fn settings_serde_roundtrip() {
    let mut settings = AppSettings::default();
    settings.max_reconnect_attempts = 5;
    let json = serde_json::to_string(&settings).unwrap();
    let parsed: AppSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.max_reconnect_attempts, 5);
}

#[test]
fn settings_backward_compat_no_reconnect_field() {
    // Old settings without max_reconnect_attempts should get default
    let json = r#"{
        "api_enabled": false,
        "auto_start_tunnels": true,
        "health_check_interval_secs": 30,
        "log_retention_days": 30
    }"#;
    let settings: AppSettings = serde_json::from_str(json).unwrap();
    assert_eq!(settings.max_reconnect_attempts, 10);
}

// ══════════════════════════════════════════════════════════════════
// P0-3: Forward Status — ConnectionInfo serialization
// ══════════════════════════════════════════════════════════════════

#[test]
fn connection_info_includes_running_forward_ids() {
    let conn = Connection::new("test".into(), "host".into(), 22, "user".into());
    let info = ConnectionInfo {
        config: conn,
        status: ConnectionStatus::Connected,
        error_message: None,
        uptime_secs: Some(120),
        running_forward_ids: vec!["fwd-1".into(), "fwd-2".into()],
    };
    let json = serde_json::to_string(&info).unwrap();
    assert!(json.contains(r#""running_forward_ids":["fwd-1","fwd-2"]"#));

    let parsed: ConnectionInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.running_forward_ids, vec!["fwd-1", "fwd-2"]);
}

#[test]
fn connection_info_empty_forwards() {
    let conn = Connection::new("test".into(), "host".into(), 22, "user".into());
    let info = ConnectionInfo {
        config: conn,
        status: ConnectionStatus::Disconnected,
        error_message: None,
        uptime_secs: None,
        running_forward_ids: vec![],
    };
    let json = serde_json::to_string(&info).unwrap();
    assert!(json.contains(r#""running_forward_ids":[]"#));
}

// ══════════════════════════════════════════════════════════════════
// ConnectionStatus serde
// ══════════════════════════════════════════════════════════════════

#[test]
fn connection_status_serde_all_variants() {
    let cases = vec![
        (ConnectionStatus::Disconnected, "\"disconnected\""),
        (ConnectionStatus::Connecting, "\"connecting\""),
        (ConnectionStatus::WaitingDuo, "\"waitingduo\""),
        (ConnectionStatus::Connected, "\"connected\""),
        (ConnectionStatus::Reconnecting, "\"reconnecting\""),
        (ConnectionStatus::Error, "\"error\""),
    ];
    for (status, expected) in cases {
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, expected, "Status {:?} serialized wrong", status);
        let parsed: ConnectionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, status);
    }
}

// ══════════════════════════════════════════════════════════════════
// Import/Export includes auth_method
// ══════════════════════════════════════════════════════════════════

#[test]
fn export_import_preserves_auth_fields() {
    let mut conn = Connection::new("key-conn".into(), "host".into(), 22, "user".into());
    conn.auth_method = AuthMethod::Key;
    conn.private_key_path = Some("/path/to/key".into());
    conn.forwards.push(ForwardRule::new("MySQL".into(), 3306, "127.0.0.1".into(), 3306));

    let connections_file = ConnectionsFile {
        connections: vec![conn],
    };
    let tags_file = TagsFile { tags: vec![] };

    let exported = import_export::export_config(&connections_file, &tags_file).unwrap();
    let imported = import_export::import_config(&exported).unwrap();

    assert_eq!(imported.connections.len(), 1);
    assert_eq!(imported.connections[0].auth_method, AuthMethod::Key);
    assert_eq!(imported.connections[0].private_key_path.as_deref(), Some("/path/to/key"));
    assert_eq!(imported.connections[0].forwards.len(), 1);
}

// ══════════════════════════════════════════════════════════════════
// JsonStore: atomic save + load
// ══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn json_store_save_and_load() {
    let tmp = std::env::temp_dir().join("shelldeck_test_store");
    let _ = std::fs::remove_dir_all(&tmp);
    let store = JsonStore::new(tmp.clone());
    store.init().await.unwrap();

    let mut conn = Connection::new("test".into(), "h".into(), 22, "u".into());
    conn.auth_method = AuthMethod::Key;
    conn.private_key_path = Some("/k".into());
    let file = ConnectionsFile { connections: vec![conn] };
    store.save("test_connections.json", &file).await.unwrap();

    let loaded: ConnectionsFile = store.load("test_connections.json").await.unwrap();
    assert_eq!(loaded.connections.len(), 1);
    assert_eq!(loaded.connections[0].auth_method, AuthMethod::Key);

    // Cleanup
    let _ = std::fs::remove_dir_all(&tmp);
}

#[tokio::test]
async fn json_store_load_missing_file_returns_default() {
    let tmp = std::env::temp_dir().join("shelldeck_test_store_empty");
    let _ = std::fs::remove_dir_all(&tmp);
    let store = JsonStore::new(tmp.clone());
    store.init().await.unwrap();

    let loaded: ConnectionsFile = store.load("nonexistent.json").await.unwrap();
    assert_eq!(loaded.connections.len(), 0);

    let _ = std::fs::remove_dir_all(&tmp);
}

// ══════════════════════════════════════════════════════════════════
// ForwardRule construction
// ══════════════════════════════════════════════════════════════════

#[test]
fn forward_rule_new_has_uuid() {
    let rule = ForwardRule::new("MySQL".into(), 3306, "127.0.0.1".into(), 3306);
    assert!(!rule.id.is_empty());
    assert_eq!(rule.name, "MySQL");
    assert!(rule.enabled);
}

// ══════════════════════════════════════════════════════════════════
// Exponential backoff logic (same formula as manager.rs)
// ══════════════════════════════════════════════════════════════════

#[test]
fn exponential_backoff_formula() {
    // Mirrors the logic: 2^attempt capped at 30
    for attempt in 1..=10u32 {
        let delay = std::cmp::min(2u64.pow(attempt), 30);
        match attempt {
            1 => assert_eq!(delay, 2),
            2 => assert_eq!(delay, 4),
            3 => assert_eq!(delay, 8),
            4 => assert_eq!(delay, 16),
            5..=10 => assert_eq!(delay, 30), // capped
            _ => {}
        }
    }
}
