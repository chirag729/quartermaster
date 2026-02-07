# Settings

## Theme

Quartermaster supports three theme modes:

| Mode | Behavior |
|------|----------|
| **Light** | Always uses the light color scheme |
| **Dark** | Always uses the dark color scheme |
| **System** | Automatically matches the operating system's current theme preference. If the OS switches between light and dark mode, Quartermaster follows. |

The theme setting takes effect immediately. It is persisted across sessions.

## Per-task Configuration

Each task can declare configurable fields (see [Tasks](tasks.md) for the full config schema). Task configuration values are stored in a `task_configs` HashMap within the application config.

To configure a task:

1. Navigate to the task in the Task Library or within a blueprint's task list.
2. Open the task's configuration panel.
3. Set values for the available fields (e.g., Flutter channel, IntelliJ edition, Git name/email).
4. Save. The values are persisted to disk.

These base configuration values apply whenever the task is executed. They can be overridden on a per-blueprint basis using `config_overrides` in the blueprint task entry (see [Blueprints](blueprints.md)).

The precedence order is:

1. **Blueprint config_overrides** (highest priority)
2. **Per-task config** (from task_configs)
3. **Task default values** (lowest priority)

## Config Persistence

Application state is stored under `~/.config/quartermaster/`:

| Path | Contents |
|------|----------|
| `config.json` | Theme preference, task configuration values (`task_configs`), completed tasks list |
| `nodes/*.json` | Fleet node definitions (one file per node, including SSH configurations for remote nodes) |
| `blueprints/*.json` | Blueprint definitions and task entries (one file per blueprint) |

The config file is read at startup and written whenever settings change. Node and blueprint files are managed by the `FleetManager` and `BlueprintManager` respectively.

These files are human-readable JSON, but manual editing is not recommended; use the Quartermaster UI to make changes.

### Backup

To back up your Quartermaster configuration, copy the entire `~/.config/quartermaster/` directory. To restore, replace the directory contents and restart Quartermaster.

## Auto-migration from Legacy Path

If you previously used an earlier version of the application that stored its configuration at:

```
~/.config/machine-setup/
```

Quartermaster automatically migrates data from the legacy path to `~/.config/quartermaster/` on first launch. The migration:

- Copies existing configuration to the new location.
- Preserves all settings, task configs, and fleet data.
- Does not delete the old directory (you can remove `~/.config/machine-setup/` manually after verifying the migration).

No action is required on your part. The migration is transparent and happens only once.

## Encrypted Vault

Quartermaster includes an encrypted credential vault for storing sensitive data such as SSH passwords and API keys.

### Creating a Vault

1. Navigate to **Settings** and find the **Vault** section.
2. Click **Create Vault** and enter a master password.
3. The vault is created at `~/.config/quartermaster/vault.enc`, encrypted with AES-256-GCM using a key derived from the master password via Argon2id.

### Using the Vault

- **Unlock**: Enter your master password to decrypt the vault. It remains unlocked in memory for the duration of the session.
- **Lock**: Explicitly lock the vault or it locks automatically when the application closes. The encryption key is zeroed out in memory.
- **Store secrets**: Store key-value pairs (e.g., `ssh:myserver` → password). Secrets are encrypted and written to disk immediately.
- **Retrieve**: When a task or SSH connection needs a stored credential, it retrieves the decrypted value from the in-memory vault.

### Password Authentication with Vault

For remote nodes that require password authentication:

1. Store the SSH password in the vault under a key (e.g., `ssh:production-server`).
2. When adding or editing a remote node, select **Password** as the auth method and provide the vault key.
3. During SSH connections, Quartermaster retrieves the password from the unlocked vault. The password is never stored in plaintext on disk.

### Changing the Master Password

Use the **Change Password** option in the vault settings. This re-encrypts the vault with a new key derived from the new password. All stored secrets are preserved.

## Variable System

Quartermaster provides a four-layer variable system that tasks can reference in their scripts using `{{variable_name}}` syntax.

### Precedence Order (highest to lowest)

| Layer | Description | Scope |
|-------|-------------|-------|
| **Node variable overrides** | Per-node overrides for specific variables | Single node |
| **User shared variables** | User-configured defaults | All nodes |
| **Blueprint config overrides** | Per-blueprint task configuration | Single blueprint |
| **Task defaults** | Default values declared in the task YAML | All uses of the task |

### Managing Variables

- **Shared variables**: Navigate to **Settings** > **Variables** to set user-level defaults (e.g., `dev_folder` = `/home/user/Development`).
- **Node overrides**: On a node's detail page, set variable overrides that apply only to that node (e.g., `dev_folder` = `/opt/dev` for a specific server).

Variables are resolved at task execution time. The system allows the same task definition to behave differently across nodes without duplicating task code.

### Built-in Variables

| Variable | Description |
|----------|-------------|
| `home` | The home directory of the executing user (from the executor) |
| `version` | The task's declared version, if any |

## Activity Log

Quartermaster maintains an activity log that records significant actions:

- Task executions (success/failure, node, timestamp)
- Task uninstalls
- Blueprint applies
- Node additions and removals

The log is stored at `~/.config/quartermaster/activity_log.json` with a maximum of 500 entries (oldest entries are pruned automatically).

### Viewing the Log

Navigate to **Settings** > **Activity Log** to view recent actions. Each entry shows:

| Field | Description |
|-------|-------------|
| Action | What happened (e.g., `task_executed`, `task_uninstalled`) |
| Target | The task or resource involved |
| Detail | Additional context (node ID, error message) |
| Success | Whether the action succeeded |
| Timestamp | When the action occurred |

You can clear the activity log from the settings page.
