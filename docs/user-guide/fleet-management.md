# Fleet Management

## Overview

A **Fleet** is the collection of all machines that Anvil manages. Each machine in the fleet is called a **Node**. The Fleet page is the central hub for viewing, adding, and organizing your machines.

## Node Kinds

Every node has a kind that determines how Anvil connects to it:

| Kind | Description |
|------|-------------|
| **Local** | The machine Anvil is running on. Auto-detected at first launch. There is always exactly one local node. |
| **Remote** | A machine accessible over SSH. Added manually through the Add Node dialog. |

## Node Status

Each node reports a status that reflects its current connectivity:

| Status | Indicator | Meaning |
|--------|-----------|---------|
| Online | Green | Node is reachable and ready for task execution |
| Offline | Red | Node is not reachable |
| Connecting | Yellow | Connection attempt in progress |
| Error | Red | A connection or configuration error occurred |
| Unknown | Gray | Status has not been determined yet |

The local node is always Online unless there is a system-level error.

## Adding a Remote Node

To add a remote machine to your fleet:

1. Navigate to **Fleet** and click **Add Node**.
2. Fill in the required fields:
   - **Name** -- A human-readable label for the node (e.g., "Staging Server").
   - **Hostname** -- The IP address or DNS name of the remote machine.
   - **Tags** -- Optional labels for filtering and organization (e.g., `production`, `gpu`, `arm64`).
   - **SSH Config** -- Authentication and connection details. See the [SSH Setup guide](ssh-setup.md) for full details.
3. Optionally, click **Test Connection** to verify SSH connectivity before saving.
4. Click **Save** to add the node to your fleet.

## Node Tags

Tags are freeform labels attached to nodes. They serve two purposes:

- **Filtering** -- Use tags to filter the fleet list. For example, show only nodes tagged `staging`.
- **Organization** -- Group related machines logically without rigid hierarchies.

Tags are edited on the node detail page or during node creation.

## Node Detail Page

Click any node in the fleet list to open its detail page. The detail page shows:

- **Node info** -- Name, hostname, kind, tags, and current status.
- **Assigned blueprint** -- The blueprint currently assigned to this node, if any. You can change or remove the assignment here.
- **Task statuses** -- A list of all tasks from the assigned blueprint, each showing its current status (NotStarted, Completed, Failed, etc.). This gives a clear picture of how far provisioning has progressed.

## Status Indicators

Throughout the Fleet UI, status is communicated visually:

- **Green dot** -- Online, healthy, or completed.
- **Red dot** -- Offline, error, or failed.
- **Yellow dot** -- Connecting or in progress.
- **Gray dot** -- Unknown or not yet evaluated.

These indicators appear next to node names in the fleet list and next to individual tasks on the node detail page.
