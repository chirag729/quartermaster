# Blueprints

## What Are Blueprints?

A blueprint is a reusable template that bundles a set of tasks into a single, repeatable provisioning plan. Instead of manually running tasks one by one, you assign a blueprint to a node and apply it. Anvil handles the rest.

## Built-in Blueprints

Anvil ships with three built-in blueprints:

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

When you click **Apply** on a node with an assigned blueprint, Anvil executes the following sequence:

1. **Collect enabled entries** -- Only entries with `enabled: true` are considered.
2. **Sort by order** -- Entries are sorted by their `order` field, lowest first.
3. **Merge configuration** -- For each entry, the task's base configuration is merged with the entry's `config_overrides`. Overrides take precedence.
4. **Detect state** -- Each task checks whether its work has already been done (e.g., a directory already exists).
5. **Skip completed** -- Tasks whose desired state is already met are marked `Skipped`.
6. **Execute** -- Remaining tasks are executed sequentially in order.

## Progress Events

During blueprint application, Anvil emits progress events that the UI subscribes to in real time. These events report:

- Which task is currently executing.
- Per-task progress (started, in progress, completed, or failed).
- Overall blueprint progress (how many tasks are done out of the total).

If a task fails, execution halts and the error is surfaced in the UI. You can resolve the issue and re-apply; completed tasks will be detected and skipped.

## Restrictions

- Built-in blueprints cannot be deleted. Custom blueprints can be deleted at any time.
- A node can have at most one blueprint assigned. Assigning a new blueprint replaces the old one.
- Disabling a task entry (`enabled: false`) causes it to be skipped during apply without removing it from the blueprint.
