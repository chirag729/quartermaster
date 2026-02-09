# Blueprints

## What Are Blueprints?

A blueprint is a reusable template that bundles a set of tasks into a single, repeatable provisioning plan. Instead of manually running tasks one by one, you assign a blueprint to a node and apply it. Quartermaster handles the rest.

## Built-in Blueprints

Quartermaster ships with three built-in blueprints:

| Blueprint | Tasks | Description |
|-----------|-------|-------------|
| Development Workstation | 7 | Full local development environment: folders, SDKs, IDE, Git/SSH, CLI tools |
| Mobile Development | 5 | Flutter and Android SDK setup with supporting directories |
| Minimal Server | 0 (empty) | A blank starting point for custom server configurations |

Built-in blueprints cannot be deleted. You can modify their task entries or create your own blueprints based on them.

## Creating a Custom Blueprint

To create a new blueprint:

1. Navigate to **Blueprints** and click **Create Blueprint**.
2. Fill in the blueprint metadata:
   - **Name** -- A descriptive name (e.g., "ML Workstation").
   - **Description** -- A short summary of what this blueprint sets up.
   - **Icon** -- An optional icon identifier for visual distinction in the UI.
3. Add **task entries** to the blueprint. Each entry references a task from the task library and includes configuration for how that task should run within this blueprint.

## Blueprint Task Entries

Each task entry in a blueprint is a `BlueprintTaskEntry` with the following fields:

| Field | Type | Description |
|-------|------|-------------|
| `task_id` | String | The identifier of the task from the task library |
| `enabled` | Boolean | Whether this task will run when the blueprint is applied. Toggle off to skip without removing. |
| `config_overrides` | Map | Key-value pairs that override the task's default configuration for this blueprint |
| `order` | Integer | Execution order. Lower numbers run first. |

This structure means the same task can appear in multiple blueprints with different configurations. For example, the `flutter-sdk` task could use the `stable` channel in one blueprint and `beta` in another via `config_overrides`.

## Assigning Blueprints to Nodes

A blueprint is assigned to a node from the node detail page:

1. Navigate to **Fleet** and click the target node.
2. Click **Assign Blueprint** and select from the list.
3. The node detail page now shows all task entries from the assigned blueprint along with their statuses.

Each node can have one blueprint assigned at a time. Changing the assignment replaces the previous one.

## Apply Flow

When you click **Apply** on a node with an assigned blueprint, Quartermaster executes the following sequence:

1. **Collect enabled entries** -- Only entries with `enabled: true` are considered.
2. **Sort by order** -- Entries are sorted by their `order` field, lowest first.
3. **Merge configuration** -- For each entry, the task's base configuration is merged with the entry's `config_overrides`. Overrides take precedence.
4. **Detect state** -- Each task checks whether its work has already been done (e.g., a directory already exists).
5. **Skip completed** -- Tasks whose desired state is already met are marked `Skipped`.
6. **Execute** -- Remaining tasks are executed sequentially in order.

## Progress Events

During blueprint application, Quartermaster emits progress events that the UI subscribes to in real time. These events report:

- Which task is currently executing.
- Per-task progress (started, in progress, completed, or failed).
- Overall blueprint progress (how many tasks are done out of the total).

If a task fails, execution halts and the error is surfaced in the UI. You can resolve the issue and re-apply; completed tasks will be detected and skipped.

## Dry Run

Before applying a blueprint to a node, you can preview what would happen without making any changes:

1. Assign the blueprint to a node and click **Dry Run**.
2. Quartermaster evaluates every enabled task entry, detecting which are already completed and which would execute.
3. For each task that would run, the dry-run report shows the commands and file operations that would be performed.
4. No changes are made to the node during a dry run.

This is useful for auditing a blueprint before deploying it to a production server.

## Inheritance (extends)

Blueprints can inherit from a parent blueprint using the `extends` field:

- A child blueprint starts with all task entries from its parent.
- The child can add new task entries or override configuration for existing ones.
- During apply, `resolve_task_entries()` merges parent and child entries. Child entries take precedence for tasks that appear in both.
- This allows you to create a base blueprint (e.g., "Core Development") and extend it with specialized variants (e.g., "ML Development" that adds GPU and Python tasks).

## Versioning

Blueprints track a semver version string that auto-increments when changes are made:

- **Minor version bump** (e.g., `1.0.0` → `1.1.0`): When task entries are added, removed, or reordered, or when a task entry's `enabled` state changes.
- **Patch version bump** (e.g., `1.0.0` → `1.0.1`): When only `config_overrides` are modified.

The version is used to detect whether a node's applied blueprint is out of date. When a blueprint is applied to a node, the node records the blueprint version. If the blueprint is subsequently modified, the node detail page indicates that an update is available.

## Import and Export (.qmbp)

Blueprints can be packaged as `.qmbp` files for sharing across machines:

### Exporting

1. Navigate to **Blueprints**, select a blueprint, and click **Export**.
2. Choose an output directory. Quartermaster creates a `.qmbp` file (a zip archive) containing the blueprint definition and all referenced task YAML files.

### Importing

1. Click **Import** on the Blueprints page.
2. Select a `.qmbp` file. Quartermaster extracts the blueprint and task definitions into the local configuration directory.
3. Imported tasks are registered in the task library; the blueprint appears in the blueprint list.

Path traversal protections ensure that malformed `.qmbp` files cannot write outside the intended directories.

## Uninstall

You can uninstall a blueprint from a node, reversing the changes made by all tasks that support uninstall:

1. Navigate to a node's blueprint detail page (Fleet > Node > assigned blueprint).
2. Click **Uninstall Blueprint** in the Actions sidebar (only appears when at least one task is installed).
3. Confirm the action. The confirmation dialog notes that tasks without uninstall support will be skipped.
4. Quartermaster uninstalls tasks in **reverse order** (dependents before dependencies) to avoid leaving the system in a broken state.
5. For each task:
   - Tasks that don't support uninstall are skipped (a warning toast appears).
   - Tasks that aren't currently installed are skipped.
   - Tasks whose execution target doesn't match the node type are skipped.
6. On completion, the node's `applied_blueprint_version` is cleared and all installation records for uninstalled tasks are removed.

### Bulk Uninstall

Like bulk apply, you can uninstall a blueprint from multiple nodes at once. The `uninstall_blueprint_bulk` command processes each node sequentially to avoid overwhelming SSH connections. Per-node results (success or failure) are reported.

## Bulk Apply

You can apply a blueprint to multiple nodes simultaneously:

1. On the **Fleet** page, enter selection mode and select the target nodes.
2. Click **Run Blueprint** in the bulk action bar.
3. Choose a blueprint from the dialog.
4. Quartermaster applies the blueprint to each selected node in parallel, emitting per-node progress events.
5. A summary shows which nodes succeeded and which failed.

## Restrictions

- Built-in blueprints cannot be deleted. Custom blueprints can be deleted at any time.
- A node can have at most one blueprint assigned. Assigning a new blueprint replaces the old one.
- Disabling a task entry (`enabled: false`) causes it to be skipped during apply without removing it from the blueprint.
