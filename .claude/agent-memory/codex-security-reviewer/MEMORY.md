# Security Reviewer Memory — Quartermaster Executor & Security Review

## Key Findings (Security-Executors Review)

### 1. Shell Injection in PrivilegedLocalExecutor.write_file() — CRITICAL
- **File**: src-tauri/src/executor/privileged.rs:64-66
- **Issue**: Escaping vulnerability in shell script construction
- When path contains single quotes, escaping is insufficient for double-quoted context
- Variable replacement happens BEFORE sh -c receives it, double escaping fails
- Attack: Path like `/tmp/pwn"$(whoami)"bar` bypasses quote escaping

### 2. SSH Credential Handling — SECURE PATTERN
- **Files**: src-tauri/src/executor/ssh.rs
- Password is set as environment variable (SSHPASS), NOT passed to command line
- Private key paths passed as arguments to ssh -i, NOT content
- No credential logging in error messages
- Shell escaping proper for paths in shell context

### 3. DryRunExecutor Coverage — COMPLETE
- All mutation methods intercepted: run_command, write_file, create_dir_all
- Read operations properly delegated to inner executor
- Tests verify no mutations reach filesystem

### 4. Privilege Escalation Enforcement — CORRECT
- execute_task() checks task.privilege_level() == Admin at line 200
- Creates PrivilegedLocalExecutor only when needs_privilege=true
- Constraint checked before executor selection

### 5. Vault Security — STRONG
- Argon2id KDF with random salt
- AES-256-GCM encryption with random 12-byte nonce
- Key is zeroed on lock() — prevents memory disclosure
- Master password verification required for operations
- No plaintext secrets logged or serialized to frontend

## Patterns to Remember
- SshExecutor shell_escape() uses standard single-quote wrapping: `'...'` with `'\''` for embedded quotes
- PrivilegedLocalExecutor uses pkexec with args array (no shell for privilege escalation), except for write_file which uses sh -c
- LocalExecutor uses shellexpand::tilde() for path expansion (~/ support)

## Architecture Review Findings (Service Layer Plan)

### 1. Credential Exposure Risk — HIGH (CRITICAL)
- **Plan section**: Moving tauriCommands.ts to packages/core/
- **Risk**: Shared core package + future CLI = vault functions exposed to non-sandboxed contexts
- **Issue**: CLI can receive vault passwords in shell history, env vars, proc
- **Mitigation**: NEVER move vault functions to shared package. Create separate packages/cli/ with own auth model
- **Test needed**: Verify tauriCommands.ts stays in packages/ui/ (Tauri-only)

### 2. Service Singleton State Mutation — HIGH
- **Plan section**: Module-level services (fleetService, blueprintService, etc.)
- **Risk**: Any code importing service can call mutations without auth checks
- **Issue**: CLI without permission layer could add nodes, modify blueprints, etc.
- **Mitigation**: Implement PermissionManager interface (Tauri vs CLI contexts), or keep services Tauri-only
- **Pattern**: SshExecutor from_ssh_config() correctly takes vault_password as parameter — good pattern for limited shared code

### 3. Polling SSRF Risk — MEDIUM
- **Plan section**: FleetService polling (30s interval)
- **Risk**: Poll requests to localhost, 127.0.0.1, internal IPs could probe/attack local services
- **Mitigation**: Validate node hostnames (reject localhost, private ranges), implement backoff for unreachable nodes
- **Note**: SSH timeout (300s) is reasonable; polling interval (30s) is safe

### 4. Service Init Race Condition — MEDIUM
- **Plan section**: serviceInit.ts with Promise.allSettled
- **Risk**: Partial initialization (blueprint load fails) leaves inconsistent state
- **Mitigation**: Use Promise.all, fail fast, show error UI in App.tsx

### 5. Vault Operations Not Audited — MEDIUM
- **Current code**: commands/vault.rs has no activity log entries
- **Risk**: Vault unlock, get, set, password change leave no audit trail
- **Mitigation**: Log actions without logging sensitive data (e.g., "vault:unlock" success/fail, not the password)

## Process Notes
- Always check how paths are passed to sh -c — context matters (single vs double quote escaping)
- Verify EVERY call site of CommandExecutor methods, especially write_file
- SSH password must NEVER appear in command line or logs
- **NEW**: When reviewing architecture, trace shared boundaries — shared package = shared security context
- **NEW**: Verify vault/credential functions NEVER move to shared packages that non-Tauri code can import
