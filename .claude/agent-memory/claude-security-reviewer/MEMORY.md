# Security Reviewer Memory

## Review Completed
- **2026-02-07**: Executor layer + polkit + vault security audit (see `findings-executors-vault.md`)

## Key Security Architecture Patterns
- `CommandExecutor` trait: 5 implementations (Local, Privileged, SSH, DryRun)
- `PrivilegedLocalExecutor::run_command` wraps via `pkexec <cmd> <args>` (no shell)
- `PrivilegedLocalExecutor::write_file` wraps via `pkexec sh -c <script>` (shell interpolation)
- `SshExecutor::run_command` only escapes `args`, NOT `cmd` parameter
- `SshExecutor` password auth uses `sshpass -e` with `SSHPASS` env var
- `AuthMode::Password(String)` derives `Debug` -- password would appear in debug output
- Vault: Argon2id + AES-256-GCM, proper nonce generation, key zeroing on lock
- Task scripts come from YAML (built-in embedded + user `~/.config/quartermaster/tasks/`)
- Config values interpolated into shell scripts via `{{var}}` template expansion (no shell escaping)

## Critical Finding Patterns
- Shell commands built by string formatting without escaping user-controlled config values
- `create_dir_all` in privileged.rs applies shell escaping then passes to non-shell exec (wrong layer)
- SSH `cmd` parameter not escaped, but currently only called with static command names
- `vault_get` Tauri command returns plaintext secrets to frontend (by design, but exposed to webview)

## Files Reviewed
- `src-tauri/src/executor/mod.rs` (trait definition)
- `src-tauri/src/executor/local.rs` (LocalExecutor)
- `src-tauri/src/executor/ssh.rs` (SshExecutor)
- `src-tauri/src/executor/privileged.rs` (PrivilegedLocalExecutor)
- `src-tauri/src/executor/dry_run.rs` (DryRunExecutor)
- `src-tauri/src/polkit/auth.rs` (policy check)
- `src-tauri/src/polkit/mod.rs`
- `src-tauri/src/vault/mod.rs` (Vault encryption)
- `src-tauri/src/commands/vault.rs` (vault commands)
- `src-tauri/src/commands/tasks.rs` (task execution)
- `src-tauri/src/tasks/script_task.rs` (template expansion + sh -c execution)
- `src-tauri/src/tasks/validate.rs` (task validation)
- `src-tauri/src/tasks/registry.rs` (task loading)
- `src-tauri/src/tasks/yaml_schema.rs` (task definition schema)
