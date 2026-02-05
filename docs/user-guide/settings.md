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
