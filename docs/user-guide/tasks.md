# Tasks

## What Are Tasks?

A task is a discrete unit of work that Quartermaster can execute on a node. Tasks are the building blocks of blueprints. Each task handles a single concern -- creating a directory, installing an SDK, configuring a tool -- and reports its status back to Quartermaster.

## Task Library

The **Task Library** page lists all available tasks. From here you can browse tasks, view their descriptions, see their configuration schemas, and understand their requirements before adding them to a blueprint.

## Built-in Tasks

Quartermaster includes the following built-in tasks:

### create-development-folder

Creates the `~/Development` directory as a standard project root.

| Property | Value |
|----------|-------|
| Privilege | User |
| Execution Target | Any |
| Tags | -- |
| Config | None |
| Uninstall | Not supported (folder may contain user projects) |

### flutter-sdk

Downloads and installs the Flutter SDK into the SDK directory.

| Property | Value |
|----------|-------|
| Privilege | User |
| Execution Target | Any |
| Tags | `mobile`, `flutter` |
| Uninstall | Supported -- removes Flutter PATH entries from shell configs, deletes SDK directory |

Configuration:

| Key | Label | Type | Default | Options |
|-----|-------|------|---------|---------|
| `channel` | Flutter Channel | Select | `stable` | `stable`, `beta`, `dev`, `master` |

### android-sdk

Downloads Android command-line tools for Android development.

| Property | Value |
|----------|-------|
| Privilege | User |
| Execution Target | Any |
| Tags | `mobile`, `android` |
| Uninstall | Supported -- removes ANDROID_HOME/PATH entries from shell configs, deletes SDK directory |

### rust-toolchain

Installs the Rust toolchain via rustup, including `rustc`, `cargo`, and `rustup` itself.

| Property | Value |
|----------|-------|
| Privilege | User |
| Execution Target | Any |
| Tags | `rust`, `toolchain` |
| Uninstall | Supported -- runs `rustup self uninstall` which removes toolchain, cargo, and PATH entries |

### intellij-idea

Installs IntelliJ IDEA from the Snap Store using classic confinement. The snap is secured with a Quartermaster-managed AppArmor profile.

| Property | Value |
|----------|-------|
| Privilege | Admin |
| Execution Target | LocalOnly |
| Tags | `ide`, `java` |
| Uninstall | Supported -- runs `snap remove intellij-idea` |

### claude-code

Installs the Claude Code CLI tool via npm.

| Property | Value |
|----------|-------|
| Privilege | User |
| Execution Target | Any |
| Tags | `ai`, `cli` |
| Uninstall | Supported -- runs `npm uninstall -g @anthropic-ai/claude-code` |

### codex-cli

Installs the OpenAI Codex CLI tool via npm.

| Property | Value |
|----------|-------|
| Privilege | User |
| Execution Target | Any |
| Tags | `ai`, `cli` |
| Uninstall | Supported -- runs `npm uninstall -g @openai/codex` |

### git-ssh

Configures Git global settings and generates an SSH key pair.

| Property | Value |
|----------|-------|
| Privilege | User |
| Execution Target | Any |
| Tags | `git`, `ssh` |
| Uninstall | Supported -- removes SSH key pair (`~/.ssh/id_ed25519`), unsets `git config --global user.name` and `user.email` |

Configuration:

| Key | Label | Type | Default |
|-----|-------|------|---------|
| `name` | Full Name | Text | -- |
| `email` | Email Address | Text | -- |
| `comment` | SSH Key Comment | Text | -- |

## Task Status

Each task on a node tracks its execution status:

| Status | Meaning |
|--------|---------|
| `NotStarted` | The task has not been executed yet |
| `InProgress` | The task is currently executing |
| `Completed` | The task finished successfully |
| `Failed` | The task encountered an error during execution |
| `Skipped` | The task was skipped because its desired state was already met, or it was disabled in the blueprint |
| `Unknown` | The task's state has not been evaluated |

## Privilege Level

Tasks declare the privilege level they require to execute:

| Level | Behavior |
|-------|----------|
| **User** | Runs with the current user's permissions. No elevation required. |
| **Admin** | Requires elevated privileges. Quartermaster uses PolicyKit (`pkexec`) to prompt for authorization. The application never runs as root directly. |

Tasks that require Admin privilege will trigger a system authentication dialog when executed.

## Execution Target

Tasks declare where they can run:

| Target | Meaning |
|--------|---------|
| **LocalOnly** | Can only run on the local node. Typically tasks that use system package managers (e.g., snap) or require local hardware access. |
| **RemoteOnly** | Can only run on remote nodes. |
| **Any** | Can run on both local and remote nodes. |

When assigning a blueprint to a node, tasks whose execution target does not match the node kind will not execute.

## Config Schema

Each task can declare a configuration schema that describes its configurable fields. The schema is a list of field definitions:

| Schema Field | Description |
|--------------|-------------|
| `key` | The configuration key (used in `config_overrides` within blueprint task entries) |
| `label` | Human-readable label shown in the UI |
| `type` | Field type: `Text`, `Select`, `Boolean`, etc. |
| `default` | Default value used when no override is provided |
| `options` | For `Select` fields, the list of valid choices |
| `required` | Whether a value must be provided before execution |

Configuration values are set either in the task's base config or overridden per-blueprint via `config_overrides` in the blueprint task entry.

## Task Uninstall

Tasks can support uninstallation, which reverses all changes made during installation and returns the system to its pre-task state. Each uninstall is thorough -- if a task added directories to `PATH` in shell config files, the uninstall removes those entries. If it installed packages, the uninstall removes them.

### How Uninstall Works

1. Navigate to the **Task Library** and click the installed task to open its detail page.
2. In the Properties sidebar, click **Uninstall** (only appears for installed tasks that support uninstall).
3. Confirm the action in the confirmation dialog.
4. Quartermaster executes the uninstall steps in order, emitting real-time progress events.
5. On success, the task status reverts to `NotStarted` and the installation record is removed from config.

### Uninstall Support by Task

| Task | Uninstall Supported | What It Removes |
|------|--------------------:|-----------------|
| `create-development-folder` | No | Folder may contain user projects |
| `flutter-sdk` | Yes | PATH entries from shell configs, SDK directory |
| `android-sdk` | Yes | ANDROID_HOME/PATH entries from shell configs, SDK directory |
| `rust-toolchain` | Yes | Full toolchain via `rustup self uninstall` |
| `intellij-idea` | Yes | Snap package via `snap remove` |
| `claude-code` | Yes | npm global package |
| `codex-cli` | Yes | npm global package |
| `git-ssh` | Yes | SSH key pair, git global user.name/email |

### Custom Tasks

YAML-defined custom tasks support uninstall automatically when they include an `uninstall:` section in their definition. The uninstall steps follow the same format as install steps and can use the same variables and template syntax.

## Update Detection

Quartermaster can detect when a task's defined version differs from the version installed on a node. For each task with a `version` field and a `version_detect` command, the `check_task_updates` backend command:

1. Reads the defined version from the task YAML.
2. Runs the `version_detect` command on the target node to determine the installed version.
3. Compares the two. If they differ, an update is flagged.

Tasks with available updates can then be re-applied to bring the node up to date.

## Version Tracking

When a task is executed on a node, Quartermaster records an installation state that includes:

| Field | Description |
|-------|-------------|
| `task_id` | The task that was installed |
| `node_id` | The node it was installed on |
| `version` | The task version at the time of installation |
| `config_hash` | A hash of the configuration used during installation |
| `installed_at` | Timestamp of the installation |
| `blueprint_id` | Which blueprint triggered the installation, if any |

This state enables two drift-detection features:

- **Config drift**: If the task's configuration has changed since it was last installed, the node detail page shows a "config drifted" indicator.
- **Version drift**: If the task's defined version has been updated since it was last installed, a "version changed" indicator appears.

Both indicators suggest that re-applying the task may be needed to bring the node up to date.
