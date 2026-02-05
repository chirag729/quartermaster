# SSH Setup

## Overview

Quartermaster uses SSH to connect to remote nodes for task execution. This guide covers SSH configuration, key management, authentication methods, and FIDO2/YubiKey support.

## SSH Configuration

When adding a remote node, you provide an `SshConfig` with the following fields:

| Field | Required | Description |
|-------|----------|-------------|
| `host` | Yes | Hostname or IP address of the remote machine |
| `port` | No | SSH port (defaults to 22) |
| `username` | Yes | The user account to authenticate as on the remote machine |
| `auth_method` | Yes | How to authenticate (see Authentication Methods below) |
| `fingerprint` | No | Expected host key fingerprint for verification |
| `proxy_jump` | No | An intermediate SSH host to tunnel through (equivalent to `ssh -J`) |

## Authentication Methods

Quartermaster supports four SSH authentication methods:

### KeyFile

Authenticate using a private key file stored on disk.

```
Auth method: KeyFile
Private key path: ~/.ssh/id_ed25519
```

This is the most common method. Point Quartermaster to the private key file; the corresponding public key is derived automatically.

### Certificate

Authenticate using an SSH certificate paired with a private key. This is used in environments with an SSH certificate authority.

```
Auth method: Certificate
Certificate path: ~/.ssh/id_ed25519-cert.pub
Private key path: ~/.ssh/id_ed25519
```

### FIDO2

Authenticate using a FIDO2 hardware key (such as a YubiKey). The key handle references the resident credential on the device.

```
Auth method: Fido2
Key handle: <device-specific handle>
```

Each authentication requires physical touch on the hardware key.

### Agent

Delegate authentication to the running SSH agent. Quartermaster does not manage keys directly; it relies on keys already loaded into `ssh-agent`.

```
Auth method: Agent
```

This is useful when your keys are managed externally or when using forwarded agents.

## SSH Key Management

Quartermaster provides built-in SSH key management accessible from the settings or the Add Node dialog.

### Listing Existing Keys

Quartermaster scans `~/.ssh/` and lists all detected key pairs. For each key, it shows:

- Key type (e.g., `ed25519`, `ed25519-sk`)
- File path
- Public key fingerprint
- Associated comment

### Generating New Keys

To generate a new SSH key pair:

1. Open the SSH key management panel.
2. Choose the key type:
   - **ed25519** -- Standard elliptic curve key. Works everywhere.
   - **ed25519-sk** -- Security key variant for YubiKey/FIDO2 devices. Requires a connected hardware key and physical touch during generation.
3. Provide an optional comment (e.g., your email address).
4. Click **Generate**. The key pair is written to `~/.ssh/`.

## Testing SSH Connections

Before saving a remote node, use the **Test Connection** button in the Add Node dialog. This performs a full SSH connection attempt using the configured settings and reports:

- Whether the connection succeeded or failed.
- The remote host's key fingerprint (for first-time verification).
- Any error details if the connection failed (wrong credentials, unreachable host, refused connection, etc.).

Testing the connection before saving avoids adding nodes with broken configurations to your fleet.

## Deploying Public Keys

To set up key-based authentication on a remote server:

1. Generate a key pair (or select an existing one) in Quartermaster's SSH key management panel.
2. Use the **Deploy Key** action to copy the public key to the remote server's `~/.ssh/authorized_keys` file.
3. Verify by running **Test Connection** against the remote node.

This is equivalent to running `ssh-copy-id` manually but handled entirely within the Quartermaster UI.

## YubiKey / FIDO2 Support

Quartermaster supports hardware security keys that implement the FIDO2 standard.

### Generating an ed25519-sk Key

1. Insert your YubiKey or other FIDO2 device.
2. Open SSH key management and select **ed25519-sk** as the key type.
3. Click **Generate**. You will be prompted to touch the hardware key to authorize key creation.
4. The generated key pair is saved to `~/.ssh/`. The private key is a handle that references the credential stored on the device.

### Using FIDO2 for Authentication

When a remote node is configured with the `Fido2` auth method:

- Each SSH connection requires physical touch on the hardware key.
- The private key file on disk is not sufficient on its own; the hardware device must be present.
- This provides strong two-factor authentication: possession of the device plus physical presence.

### Requirements

- A FIDO2-compatible security key (YubiKey 5 series or newer recommended).
- OpenSSH 8.2 or later on both the local and remote machines.
- The `libfido2` library installed on the local machine.
