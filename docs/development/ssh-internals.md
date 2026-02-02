# SSH Internals

This document covers the SSH and remote execution internals of Anvil, including the CommandExecutor abstraction, the local and SSH executor implementations, and the SSH management commands.

## CommandExecutor Trait

The `CommandExecutor` trait (defined in `src-tauri/src/executor/mod.rs`) is the core abstraction that enables tasks to run identically on local or remote machines:

```rust
#[async_trait::async_trait]
pub trait CommandExecutor: Send + Sync {
    async fn run_command(&self, cmd: &str, args: &[&str]) -> Result<CommandOutput, AppError>;
    async fn file_exists(&self, path: &str) -> Result<bool, AppError>;
    async fn read_file(&self, path: &str) -> Result<String, AppError>;
    async fn write_file(&self, path: &str, content: &str) -> Result<(), AppError>;
    async fn create_dir_all(&self, path: &str) -> Result<(), AppError>;
    fn home_dir(&self) -> String;
    fn is_local(&self) -> bool;
}
```

### Methods

| Method | Purpose |
|--------|---------|
| `run_command(cmd, args)` | Execute a command with arguments. Returns `CommandOutput { status, stdout, stderr }`. |
| `file_exists(path)` | Check whether a file or directory exists at the given path. |
| `read_file(path)` | Read the entire contents of a file as a string. |
| `write_file(path, content)` | Write content to a file, creating parent directories if needed. |
| `create_dir_all(path)` | Recursively create directories (equivalent to `mkdir -p`). |
| `home_dir()` | Return the home directory path for the target machine's user. |
| `is_local()` | Return `true` if this executor runs commands on the local machine. |

### CommandOutput

```rust
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}
```

## LocalExecutor

Defined in `src-tauri/src/executor/local.rs`.

The `LocalExecutor` runs commands on the local machine using `tokio::process::Command` and native filesystem operations.

### Construction

```rust
let exec = LocalExecutor::new();
```

The constructor reads the current user's home directory via `dirs::home_dir()`, falling back to `/root` if detection fails.

### Implementation details

| Method | Implementation |
|--------|---------------|
| `run_command` | Spawns a process via `tokio::process::Command::new(cmd).args(args).output()`. |
| `file_exists` | Uses `std::path::Path::exists()` after expanding tildes with `shellexpand::tilde()`. |
| `read_file` | Uses `std::fs::read_to_string()` after tilde expansion. |
| `write_file` | Creates parent directories if needed, then uses `std::fs::write()`. Tilde expansion applied. |
| `create_dir_all` | Uses `std::fs::create_dir_all()` after tilde expansion. |
| `home_dir` | Returns the cached home directory string. |
| `is_local` | Returns `true`. |

## SshExecutor

Defined in `src-tauri/src/executor/ssh.rs`.

The `SshExecutor` executes commands on a remote machine via the `ssh` CLI binary. It builds SSH commands with standard security options and dispatches all operations as remote shell commands.

### Construction

```rust
let exec = SshExecutor::new(
    "192.168.1.100".to_string(),  // host
    22,                            // port
    "deploy".to_string(),          // username
);
```

The constructor takes host, port, and username. The home directory is computed as `/home/{username}` (or `/root` for the root user).

### SSH command options

Every SSH invocation includes these options:

- `-p <port>` -- Connect to the specified port.
- `-o BatchMode=yes` -- Disable interactive password prompts (fail immediately if key auth is not set up).
- `-o ConnectTimeout=10` -- 10-second connection timeout.
- `-o StrictHostKeyChecking=accept-new` -- Automatically accept new host keys but reject changed keys.

### Implementation details

| Method | Implementation |
|--------|---------------|
| `run_command` | Builds a remote command string with `shell_escape()` for safe argument quoting, then executes via `ssh <options> <user>@<host> <remote_cmd>`. Returns `CommandOutput` with exit code, stdout, and stderr. |
| `file_exists` | Dispatches via `run_command("test", &["-e", path])`, returns `true` if exit status is 0. |
| `read_file` | Dispatches via `run_command("cat", &[path])`, returns stdout. |
| `write_file` | Pipes content via stdin to `cat > <path>` on the remote machine. This avoids shell escaping issues with file contents. Uses `AsyncWriteExt` to write to the child process stdin. |
| `create_dir_all` | Dispatches via `run_command("mkdir", &["-p", path])`. |
| `home_dir` | Returns `/home/{username}` (or `/root` for root). |
| `is_local` | Returns `false`. |

### Shell escaping

The `shell_escape()` function wraps strings in single quotes with proper escaping of embedded single quotes (`'` becomes `'\''`). This is used for `run_command` arguments to prevent shell injection on the remote machine.

## Executor Dispatch in apply_blueprint

When a blueprint is applied to a node (in `src-tauri/src/commands/blueprints.rs`), the system selects the appropriate executor based on the node's `NodeKind`:

```rust
let exec: Box<dyn CommandExecutor> = match node.kind {
    NodeKind::Local => Box::new(LocalExecutor::new()),
    NodeKind::Remote => {
        let ssh_config = node.ssh_config.as_ref().ok_or_else(|| {
            AppError::Ssh(format!("Remote node '{}' has no SSH configuration", node.name))
        })?;
        Box::new(SshExecutor::new(
            ssh_config.host.clone(),
            ssh_config.port,
            ssh_config.username.clone(),
        ))
    }
};
```

For **local nodes**, a `LocalExecutor` is created. For **remote nodes**, an `SshExecutor` is created using the node's `SshConfig` (host, port, username).

The executor is then passed to each task's `detect_state()` and `execute()` methods. Because tasks use the `CommandExecutor` trait exclusively, they are agnostic to whether they run locally or remotely.

## SSH Management Commands

The SSH management commands are defined in `src-tauri/src/commands/ssh.rs`. These handle key management and connection testing.

### test_ssh_connection

```
test_ssh_connection(host: String, port: u16, username: String) -> String
```

Tests SSH connectivity to a remote host by running `ssh <user>@<host> echo ok` with `BatchMode=yes` and a 10-second connection timeout. Returns a success message or an error with connection details.

### list_ssh_keys

```
list_ssh_keys() -> Vec<SshKeyInfo>
```

Scans the `~/.ssh/` directory for public key files (`*.pub`). For each public key found:

- Reads the file content to determine the key type (first whitespace-delimited field).
- Detects FIDO2 keys by checking if the key type contains `sk-` (e.g., `sk-ssh-ed25519@openssh.com`).

Returns a list of `SshKeyInfo`:

```rust
pub struct SshKeyInfo {
    pub name: String,      // Filename without .pub extension
    pub path: String,      // Full path to the .pub file
    pub key_type: String,  // e.g., "ssh-ed25519", "sk-ssh-ed25519@openssh.com"
    pub is_fido2: bool,    // true if key type contains "sk-"
}
```

### generate_ssh_key

```
generate_ssh_key(key_type: String, comment: String) -> SshKeyInfo
```

Generates a new SSH key pair using `ssh-keygen`:

- Supported key types: `ed25519`, `ed25519-sk` (FIDO2).
- Key filename: `id_{type}_{comment}` (type hyphens replaced with underscores, comment spaces replaced with underscores and lowercased).
- Keys are created in `~/.ssh/` with an empty passphrase for non-interactive generation.
- Returns an error if a key with the same name already exists (will not overwrite).

### deploy_ssh_key

```
deploy_ssh_key(public_key_path: String, target_host: String, target_port: u16, target_username: String) -> ()
```

Deploys a public key to a remote machine's `authorized_keys` file:

1. Reads the public key from `public_key_path`.
2. Connects to the remote machine via the `ssh` command.
3. Executes a remote shell command that:
   - Creates `~/.ssh/` if it does not exist (`mkdir -p`).
   - Sets directory permissions to 700.
   - Appends the public key to `~/.ssh/authorized_keys`.
   - Sets file permissions to 600.

Uses `-o StrictHostKeyChecking=accept-new` to automatically accept new host keys on first connection.

## Fleet Node SSH Configuration

Remote nodes store their SSH configuration in the `SshConfig` struct (defined in `src-tauri/src/fleet/mod.rs`):

```rust
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: SshAuthMethod,
    pub fingerprint: Option<String>,
    pub proxy_jump: Option<String>,
}
```

The `SshAuthMethod` enum supports multiple authentication methods:

```rust
pub enum SshAuthMethod {
    KeyFile { private_key_path: String },
    Certificate { certificate_path: String, private_key_path: String },
    Fido2 { key_handle: String },
    Agent,
}
```

| Method | Description |
|--------|-------------|
| `KeyFile` | Authenticate with a private key file. |
| `Certificate` | Authenticate with an SSH certificate and corresponding private key. |
| `Fido2` | Authenticate using a FIDO2 hardware security key. |
| `Agent` | Delegate authentication to the running SSH agent. |
