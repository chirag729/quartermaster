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

## Process Notes
- Always check how paths are passed to sh -c — context matters (single vs double quote escaping)
- Verify EVERY call site of CommandExecutor methods, especially write_file
- SSH password must NEVER appear in command line or logs
