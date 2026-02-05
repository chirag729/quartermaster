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

### create-sdk-folder

Creates the `~/SDK` directory for housing SDK installations.

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

Installs IntelliJ IDEA via snap.

| Property | Value |
|----------|-------|
| Privilege | Admin |
| Execution Target | LocalOnly |
| Tags | -- |

Configuration:

| Key | Label | Type | Default | Options |
|-----|-------|------|---------|---------|
| `edition` | IntelliJ Edition | Select | `community` | `community`, `ultimate` |

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
