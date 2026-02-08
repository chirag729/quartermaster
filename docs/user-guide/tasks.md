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

### flutter-sdk

Downloads and installs the Flutter SDK into the SDK directory.

| Property | Value |
|----------|-------|
| Privilege | User |
| Execution Target | Any |
| Tags | `mobile`, `flutter` |

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

### intellij-idea

Downloads and installs IntelliJ IDEA Community Edition from the JetBrains tarball.

| Property | Value |
|----------|-------|
| Privilege | User |
| Execution Target | LocalOnly |
| Tags | `ide`, `java` |

Configuration:

| Key | Label | Type | Default | Options |
|-----|-------|------|---------|---------|
| `edition` | Edition | Select | `community` | `community`, `ultimate` |
| `install_path` | Installation path | Path | `~/.local/share/JetBrains/IntelliJIdea` | -- |

### claude-code

Installs the Claude Code CLI tool via npm.

| Property | Value |
|----------|-------|
| Privilege | User |
| Execution Target | Any |
| Tags | `ai`, `cli` |

### git-ssh

Configures Git global settings and generates an SSH key pair.

| Property | Value |
|----------|-------|
| Privilege | User |
| Execution Target | Any |
| Tags | `git`, `ssh` |

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

Some tasks support uninstallation, which reverses the changes made during installation. Tasks that support uninstall declare `uninstall` steps in their YAML definition.

To uninstall a task:

1. Navigate to the node detail page.
2. Find the installed task and click **Uninstall**.
3. Quartermaster executes the uninstall steps (e.g., removing directories, uninstalling packages).
4. The task status reverts to `NotStarted` and the installation record is removed.

Not all tasks support uninstall. The UI indicates whether uninstall is available for each task. If a task does not declare uninstall steps, the uninstall option is disabled.

## Update Detection

Quartermaster can detect when a task's defined version differs from the version installed on a node:

1. Navigate to **Check Updates** or use the `check_task_updates` command.
2. For each task with a `version` field and a `version_detect` command, Quartermaster:
   - Reads the defined version from the task YAML.
   - Runs the `version_detect` command on the target node to determine the installed version.
   - Compares the two. If they differ, an update is flagged.
3. Tasks with available updates are highlighted in the UI so you can re-apply them.

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
