use quartermaster_lib::vault::Vault;

#[test]
fn vault_full_lifecycle() {
    let dir = tempfile::tempdir().unwrap();
    let mut vault = Vault::with_path(dir.path().join("vault.enc"));

    // Create
    vault.create("master-pass-123").unwrap();
    assert!(vault.is_unlocked());
    assert!(vault.exists());

    // Store 3 secrets
    vault.set("ssh:server1".to_string(), "pass1".to_string()).unwrap();
    vault.set("ssh:server2".to_string(), "pass2".to_string()).unwrap();
    vault.set("api:github".to_string(), "ghp_token".to_string()).unwrap();

    // Lock
    vault.lock();
    assert!(!vault.is_unlocked());

    // Unlock
    vault.unlock("master-pass-123").unwrap();
    assert!(vault.is_unlocked());

    // Verify all 3 secrets
    assert_eq!(vault.get("ssh:server1").unwrap(), Some("pass1".to_string()));
    assert_eq!(vault.get("ssh:server2").unwrap(), Some("pass2".to_string()));
    assert_eq!(vault.get("api:github").unwrap(), Some("ghp_token".to_string()));

    // Remove 1
    assert!(vault.remove("ssh:server2").unwrap());

    // Verify 2 remain
    let mut keys = vault.list_keys().unwrap();
    keys.sort();
    assert_eq!(keys.len(), 2);
    assert!(keys.contains(&"ssh:server1".to_string()));
    assert!(keys.contains(&"api:github".to_string()));
}

#[test]
fn vault_change_password_preserves_data() {
    let dir = tempfile::tempdir().unwrap();
    let mut vault = Vault::with_path(dir.path().join("vault.enc"));

    vault.create("old-password").unwrap();
    vault.set("key1".to_string(), "secret1".to_string()).unwrap();
    vault.set("key2".to_string(), "secret2".to_string()).unwrap();

    // Change password
    vault.change_password("old-password", "new-password").unwrap();
    vault.lock();

    // Old password should fail
    let result = vault.unlock("old-password");
    assert!(result.is_err());

    // New password should work
    vault.unlock("new-password").unwrap();
    assert_eq!(vault.get("key1").unwrap(), Some("secret1".to_string()));
    assert_eq!(vault.get("key2").unwrap(), Some("secret2".to_string()));
}

#[test]
fn vault_operations_fail_when_locked() {
    let dir = tempfile::tempdir().unwrap();
    let mut vault = Vault::with_path(dir.path().join("vault.enc"));

    // New vault (not created) — all operations should fail
    assert!(vault.get("key").is_err());
    assert!(vault.list_keys().is_err());

    // set and remove need &mut, test them too
    let set_result = vault.set("key".to_string(), "value".to_string());
    assert!(set_result.is_err());

    let remove_result = vault.remove("key");
    assert!(remove_result.is_err());
}

#[test]
fn vault_overwrite_secret() {
    let dir = tempfile::tempdir().unwrap();
    let mut vault = Vault::with_path(dir.path().join("vault.enc"));

    vault.create("password").unwrap();
    vault.set("key".to_string(), "v1".to_string()).unwrap();
    vault.set("key".to_string(), "v2".to_string()).unwrap();

    // Lock and unlock to verify persistence
    vault.lock();
    vault.unlock("password").unwrap();

    assert_eq!(vault.get("key").unwrap(), Some("v2".to_string()));
}
