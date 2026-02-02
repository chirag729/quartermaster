# AppArmor Management

## Overview

Anvil includes a built-in AppArmor management tool for viewing denial logs, generating permission rules, and applying profile changes. This feature helps you diagnose and resolve AppArmor permission issues without manually editing profile files.

**This feature is local-only.** AppArmor management operates exclusively on the local node and is not available for remote nodes.

## Viewing Denial Logs

Anvil reads AppArmor denial entries from:

```
/var/log/audit/audit.log
```

The denial log viewer displays each entry with:

- Timestamp
- The denied operation (e.g., `open`, `read`, `exec`)
- The profile that triggered the denial
- The target resource (file path, network address, etc.)
- The requested permission

This gives you a clear view of what AppArmor is blocking and which profiles are involved.

## Real-time Log Monitoring

For active debugging, Anvil can monitor the audit log in real time:

- **Start monitoring** -- Begins tailing the audit log and displaying new denial entries as they occur.
- **Stop monitoring** -- Stops the real-time tail.

This is useful when reproducing an issue: start monitoring, trigger the action that is being blocked, and immediately see the resulting denial.

## Rule Suggestions

When you view a denial, Anvil can suggest a permission rule to resolve it. Each suggestion includes a **risk level** assessment:

| Risk Level | Meaning |
|------------|---------|
| **Low** | The rule grants a narrow, specific permission. Generally safe to apply. |
| **Medium** | The rule grants a moderately broad permission. Review the scope before applying. |
| **High** | The rule grants a broad permission that may weaken the profile's security posture. Apply with caution. |
| **Critical** | The rule grants a very broad or sensitive permission (e.g., unrestricted network access, write to system paths). Carefully evaluate whether this is necessary. |

Suggestions are generated based on the denied operation and target resource. Anvil attempts to create the narrowest rule that resolves the denial.

## PolicyKit Setup

AppArmor profile modification requires root privileges. Before using AppArmor management, install Anvil's PolicyKit policy:

1. Navigate to **Settings**.
2. Click **Install PolicyKit Policy**.
3. Authenticate when prompted. This installs both the PolicyKit policy and a security-hardened helper script.

Once installed, the helper script enables **credential caching**: you authenticate once, and subsequent AppArmor operations within approximately 5 minutes do not require re-authentication. Without the helper installed, each operation prompts for a password individually.

## Applying Permission Rules

To apply a suggested rule:

1. Review the rule and its risk level.
2. Click **Apply Rule**.
3. Anvil uses the privileged helper script (via PolicyKit) to write the rule to the appropriate AppArmor profile. You will be prompted for authentication if credentials are not cached.
4. The profile is reloaded so the change takes effect immediately.

## Profile Listing and Detail View

The **Profiles** page lists all AppArmor profiles on the system. For each profile, you can view:

- Profile name and enforcement mode (enforce, complain, unconfined)
- The full set of rules currently defined in the profile
- Recent denials associated with the profile

Click a profile to open its detail view, where you can inspect individual rules and manage the profile.

## Rule Consolidation

Over time, applying individual rules can result in redundant or overlapping entries. Anvil provides rule consolidation to clean this up:

- **Group similar rules** -- Rules that target the same resource type or path pattern are grouped together.
- **Merge overlapping permissions** -- If multiple rules grant different permissions to the same path, they are merged into a single, combined rule.

Consolidation reduces profile complexity and makes profiles easier to audit.

## Profile Rewriting

After consolidation or manual edits, you can rewrite a profile:

1. Open the profile detail view.
2. Click **Rewrite Profile**.
3. Anvil regenerates the profile file with the current set of rules, properly formatted and ordered.
4. The rewrite is applied via PolicyKit and the profile is reloaded.

## Batch Rule Application

When multiple denials share a common cause (e.g., an application accessing several files in the same directory), you can apply rules in batch:

1. Select multiple denial entries or suggested rules.
2. Click **Apply Selected**.
3. All selected rules are written to the profile in a single operation.

This is faster than applying rules one at a time and ensures all related permissions are granted together.
