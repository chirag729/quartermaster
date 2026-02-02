# Getting Started

## What Is Anvil?

Anvil is a Tauri 2 desktop application for provisioning and managing Linux machines. It targets developers and system administrators who need to set up consistent development environments across local and remote machines.

Anvil automates repetitive setup work -- installing SDKs, configuring Git, creating project directories -- by organizing discrete tasks into reusable blueprints that can be applied to any machine in your fleet.

## First Launch

When you launch Anvil for the first time, two things happen automatically:

1. **Local node detection.** Anvil detects the machine it is running on and registers it as the local node in your fleet. No manual configuration is required.
2. **Default blueprints.** A set of built-in blueprints is created (Development Workstation, Mobile Development, Minimal Server). These serve as starting points that you can use directly or customize.

After initialization, Anvil opens to the Fleet page, which shows all managed machines and their statuses.

## Applying a Blueprint to the Local Node

To provision your local machine:

1. Navigate to **Fleet** in the sidebar.
2. Click the **local node** entry in the fleet list.
3. On the node detail page, click **Assign Blueprint** and select a blueprint (for example, "Development Workstation").
4. Review the list of tasks included in the blueprint. Each task shows its current status and whether it is enabled.
5. Click **Apply** to begin execution.

## Task Execution Flow

When you apply a blueprint, Anvil processes each enabled task in order:

1. **Detect state** -- Anvil checks whether the task has already been completed (for example, whether a directory already exists or a package is already installed).
2. **Skip completed** -- Tasks whose desired state is already met are marked as `Skipped` and not re-executed.
3. **Execute remaining** -- Tasks that still need work are executed sequentially in their defined order.
4. **Progress events** -- Each task emits progress events as it runs. The UI displays real-time status updates, including per-task progress indicators and any output or errors.

If a task fails, execution stops and the failure is reported. You can fix the underlying issue and re-apply; previously completed tasks will be skipped on the next run.

## Where Data Is Stored

All persistent data is stored under:

```
~/.config/anvil/
```

This directory contains:

| Path | Purpose |
|------|---------|
| `config.json` | Application settings, theme preference, and task configurations |
| `nodes/` | Fleet node definitions (one JSON file per node) |
| `blueprints/` | Blueprint definitions (one JSON file per blueprint) |

If you need to reset Anvil to a clean state, remove or rename this directory. Anvil will re-create it with defaults on the next launch.
