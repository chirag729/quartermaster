import { useState, useEffect, useCallback } from "react";
import { Card, CardHeader, CardTitle, CardDescription } from "../components/ui/Card";
import { Button } from "../components/ui/Button";
import { Badge } from "../components/ui/Badge";
import { Input } from "../components/ui/Input";
import { Dialog } from "../components/ui/Dialog";
import { useThemeStore } from "../stores/themeStore";
import { useToastStore } from "../stores/toastStore";
import {
  Sun,
  Moon,
  Monitor,
  Key,
  Shield,
  Lock,
  Unlock,
  Plus,
  RefreshCw,
  Fingerprint,
  UsbIcon,
  Trash2,
  Pencil,
  Check,
  X,
} from "lucide-react";
import clsx from "clsx";
import * as api from "../services/tauriCommands";
import { formatError } from "../lib/formatError";
import type { SshKeyInfo } from "../services/tauriCommands";
import type { YubiKeyInfo, Fido2Credential } from "../types/node";

// ---------------------------------------------------------------------------
// Theme options (unchanged from original)
// ---------------------------------------------------------------------------

const themeOptions = [
  { value: "light" as const, icon: Sun, label: "Light", description: "Always use light mode" },
  { value: "dark" as const, icon: Moon, label: "Dark", description: "Always use dark mode" },
  { value: "system" as const, icon: Monitor, label: "System", description: "Follow system preference" },
];

// ---------------------------------------------------------------------------
// Section: Appearance
// ---------------------------------------------------------------------------

function AppearanceSection() {
  const { theme, setTheme } = useThemeStore();

  return (
    <Card>
      <CardHeader>
        <CardTitle>Appearance</CardTitle>
        <CardDescription>Choose your preferred theme</CardDescription>
      </CardHeader>
      <div className="grid grid-cols-3 gap-3">
        {themeOptions.map(({ value, icon: Icon, label, description }) => (
          <button
            key={value}
            onClick={() => setTheme(value)}
            className={clsx(
              "flex flex-col items-center gap-2 p-4 rounded-lg border transition-colors text-center",
              theme === value
                ? "border-warm-400 bg-warm-100/40 dark:bg-warm-900/30 shadow-sm ring-1 ring-warm-400/30"
                : "border-border-light dark:border-border-dark hover:border-warm-300 dark:hover:border-warm-700",
            )}
          >
            <Icon
              size={20}
              className={
                theme === value
                  ? "text-warm-500"
                  : "text-text-secondary-light dark:text-text-secondary-dark"
              }
            />
            <span className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark">
              {label}
            </span>
            <span className="text-xs text-text-secondary-light dark:text-text-secondary-dark">
              {description}
            </span>
          </button>
        ))}
      </div>
    </Card>
  );
}

// ---------------------------------------------------------------------------
// Section: Shared Variables
// ---------------------------------------------------------------------------

function SharedVariablesSection() {
  const { addToast } = useToastStore();
  const [variables, setVariables] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(true);
  const [editingKey, setEditingKey] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");
  const [showAddDialog, setShowAddDialog] = useState(false);
  const [newKey, setNewKey] = useState("");
  const [newValue, setNewValue] = useState("");

  const load = useCallback(async () => {
    try {
      setLoading(true);
      const vars = await api.getSharedVariables();
      setVariables(vars);
    } catch (err) {
      addToast({ type: "error", title: "Failed to load variables", message: formatError(err) });
    } finally {
      setLoading(false);
    }
  }, [addToast]);

  useEffect(() => {
    load();
  }, [load]);

  const handleSave = async (key: string, value: string) => {
    try {
      await api.setSharedVariable(key, value);
      setVariables((prev) => ({ ...prev, [key]: value }));
      setEditingKey(null);
      addToast({ type: "success", title: "Variable updated" });
    } catch (err) {
      addToast({ type: "error", title: "Failed to save variable", message: formatError(err) });
    }
  };

  const handleDelete = async (key: string) => {
    try {
      await api.removeSharedVariable(key);
      setVariables((prev) => {
        const next = { ...prev };
        delete next[key];
        return next;
      });
      addToast({ type: "success", title: "Variable removed" });
    } catch (err) {
      addToast({ type: "error", title: "Failed to remove variable", message: formatError(err) });
    }
  };

  const handleAdd = async () => {
    const trimmedKey = newKey.trim();
    const trimmedValue = newValue.trim();
    if (!trimmedKey) return;
    try {
      await api.setSharedVariable(trimmedKey, trimmedValue);
      setVariables((prev) => ({ ...prev, [trimmedKey]: trimmedValue }));
      setNewKey("");
      setNewValue("");
      setShowAddDialog(false);
      addToast({ type: "success", title: "Variable added" });
    } catch (err) {
      addToast({ type: "error", title: "Failed to add variable", message: formatError(err) });
    }
  };

  const entries = Object.entries(variables);

  return (
    <>
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>Shared Variables</CardTitle>
              <CardDescription>
                Default paths and values shared across all nodes (dev_folder, sdk_path, git_name,
                git_email, etc.)
              </CardDescription>
            </div>
            <Button variant="secondary" size="sm" onClick={() => setShowAddDialog(true)}>
              <Plus size={14} />
              Add Variable
            </Button>
          </div>
        </CardHeader>

        {loading ? (
          <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark">
            Loading...
          </p>
        ) : entries.length === 0 ? (
          <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark">
            No shared variables configured yet.
          </p>
        ) : (
          <div className="space-y-2">
            {entries.map(([key, value]) => (
              <div
                key={key}
                className="flex items-center gap-3 rounded-lg border border-border-light dark:border-border-dark px-3 py-2"
              >
                <span className="text-xs font-mono font-semibold text-warm-600 dark:text-warm-400 min-w-[120px]">
                  {key}
                </span>

                {editingKey === key ? (
                  <>
                    <input
                      autoFocus
                      className="flex-1 rounded border border-border-light dark:border-border-dark bg-surface-light dark:bg-surface-dark px-2 py-1 text-sm text-text-primary-light dark:text-text-primary-dark focus:outline-none focus:ring-2 focus:ring-warm-300/50"
                      value={editValue}
                      onChange={(e) => setEditValue(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") handleSave(key, editValue);
                        if (e.key === "Escape") setEditingKey(null);
                      }}
                    />
                    <button
                      onClick={() => handleSave(key, editValue)}
                      className="p-1 rounded hover:bg-warm-100/50 dark:hover:bg-warm-900/20 text-green-600 dark:text-green-400"
                      aria-label="Save"
                    >
                      <Check size={14} />
                    </button>
                    <button
                      onClick={() => setEditingKey(null)}
                      className="p-1 rounded hover:bg-warm-100/50 dark:hover:bg-warm-900/20 text-text-secondary-light dark:text-text-secondary-dark"
                      aria-label="Cancel"
                    >
                      <X size={14} />
                    </button>
                  </>
                ) : (
                  <>
                    <span className="flex-1 text-sm text-text-primary-light dark:text-text-primary-dark truncate font-mono">
                      {value}
                    </span>
                    <button
                      onClick={() => {
                        setEditingKey(key);
                        setEditValue(value);
                      }}
                      className="p-1 rounded hover:bg-warm-100/50 dark:hover:bg-warm-900/20 text-text-secondary-light dark:text-text-secondary-dark"
                      aria-label="Edit"
                    >
                      <Pencil size={14} />
                    </button>
                    <button
                      onClick={() => handleDelete(key)}
                      className="p-1 rounded hover:bg-warm-100/50 dark:hover:bg-warm-900/20 text-red-500 dark:text-red-400"
                      aria-label="Delete"
                    >
                      <Trash2 size={14} />
                    </button>
                  </>
                )}
              </div>
            ))}
          </div>
        )}
      </Card>

      <Dialog open={showAddDialog} onClose={() => setShowAddDialog(false)} title="Add Shared Variable">
        <div className="space-y-4">
          <Input
            label="Key"
            placeholder="e.g. dev_folder"
            value={newKey}
            onChange={(e) => setNewKey(e.target.value)}
          />
          <Input
            label="Value"
            placeholder="e.g. ~/Development"
            value={newValue}
            onChange={(e) => setNewValue(e.target.value)}
          />
          <div className="flex justify-end gap-2 pt-2">
            <Button variant="ghost" size="sm" onClick={() => setShowAddDialog(false)}>
              Cancel
            </Button>
            <Button size="sm" onClick={handleAdd} disabled={!newKey.trim()}>
              Add
            </Button>
          </div>
        </div>
      </Dialog>
    </>
  );
}

// ---------------------------------------------------------------------------
// Section: Master Password (Vault)
// ---------------------------------------------------------------------------

function VaultSection() {
  const { addToast } = useToastStore();
  const [exists, setExists] = useState(false);
  const [unlocked, setUnlocked] = useState(false);
  const [loading, setLoading] = useState(true);

  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [createPassword, setCreatePassword] = useState("");
  const [createConfirm, setCreateConfirm] = useState("");

  const [showUnlockDialog, setShowUnlockDialog] = useState(false);
  const [unlockPassword, setUnlockPassword] = useState("");

  const [showChangeDialog, setShowChangeDialog] = useState(false);
  const [currentPassword, setCurrentPassword] = useState("");
  const [changeNewPassword, setChangeNewPassword] = useState("");
  const [changeConfirm, setChangeConfirm] = useState("");

  const load = useCallback(async () => {
    try {
      setLoading(true);
      const [e, u] = await Promise.all([api.vaultExists(), api.vaultIsUnlocked()]);
      setExists(e);
      setUnlocked(u);
    } catch (err) {
      addToast({ type: "error", title: "Failed to check vault status", message: formatError(err) });
    } finally {
      setLoading(false);
    }
  }, [addToast]);

  useEffect(() => {
    load();
  }, [load]);

  const handleCreate = async () => {
    if (createPassword !== createConfirm) {
      addToast({ type: "error", title: "Passwords do not match" });
      return;
    }
    if (!createPassword) return;
    try {
      await api.vaultCreate(createPassword);
      setExists(true);
      setUnlocked(true);
      setShowCreateDialog(false);
      setCreatePassword("");
      setCreateConfirm("");
      addToast({ type: "success", title: "Vault created and unlocked" });
    } catch (err) {
      addToast({ type: "error", title: "Failed to create vault", message: formatError(err) });
    }
  };

  const handleUnlock = async () => {
    if (!unlockPassword) return;
    try {
      await api.vaultUnlock(unlockPassword);
      setUnlocked(true);
      setShowUnlockDialog(false);
      setUnlockPassword("");
      addToast({ type: "success", title: "Vault unlocked" });
    } catch (err) {
      addToast({ type: "error", title: "Failed to unlock vault", message: formatError(err) });
    }
  };

  const handleLock = async () => {
    try {
      await api.vaultLock();
      setUnlocked(false);
      addToast({ type: "success", title: "Vault locked" });
    } catch (err) {
      addToast({ type: "error", title: "Failed to lock vault", message: formatError(err) });
    }
  };

  const handleChangePassword = async () => {
    if (changeNewPassword !== changeConfirm) {
      addToast({ type: "error", title: "New passwords do not match" });
      return;
    }
    if (!currentPassword || !changeNewPassword) return;
    try {
      await api.vaultChangePassword(currentPassword, changeNewPassword);
      setShowChangeDialog(false);
      setCurrentPassword("");
      setChangeNewPassword("");
      setChangeConfirm("");
      addToast({ type: "success", title: "Vault password changed" });
    } catch (err) {
      addToast({ type: "error", title: "Failed to change password", message: formatError(err) });
    }
  };

  return (
    <>
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>
                <span className="inline-flex items-center gap-2">
                  <Shield size={16} />
                  Master Password (Vault)
                </span>
              </CardTitle>
              <CardDescription>
                Encrypted vault for storing SSH passwords and other secrets
              </CardDescription>
            </div>
          </div>
        </CardHeader>

        {loading ? (
          <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark">
            Checking vault status...
          </p>
        ) : (
          <div className="space-y-4">
            {/* Status row */}
            <div className="flex items-center gap-3">
              <span className="text-sm text-text-secondary-light dark:text-text-secondary-dark">
                Status:
              </span>
              {!exists ? (
                <Badge variant="warning">Not created</Badge>
              ) : unlocked ? (
                <Badge variant="success">
                  <Unlock size={12} className="mr-1" />
                  Unlocked
                </Badge>
              ) : (
                <Badge variant="danger">
                  <Lock size={12} className="mr-1" />
                  Locked
                </Badge>
              )}
            </div>

            {/* Action buttons */}
            <div className="flex flex-wrap gap-2">
              {!exists ? (
                <Button
                  size="sm"
                  onClick={() => {
                    setCreatePassword("");
                    setCreateConfirm("");
                    setShowCreateDialog(true);
                  }}
                >
                  <Plus size={14} />
                  Create Vault
                </Button>
              ) : unlocked ? (
                <>
                  <Button variant="secondary" size="sm" onClick={handleLock}>
                    <Lock size={14} />
                    Lock
                  </Button>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => {
                      setCurrentPassword("");
                      setChangeNewPassword("");
                      setChangeConfirm("");
                      setShowChangeDialog(true);
                    }}
                  >
                    <Key size={14} />
                    Change Password
                  </Button>
                </>
              ) : (
                <Button
                  size="sm"
                  onClick={() => {
                    setUnlockPassword("");
                    setShowUnlockDialog(true);
                  }}
                >
                  <Unlock size={14} />
                  Unlock
                </Button>
              )}
            </div>
          </div>
        )}
      </Card>

      {/* Create vault dialog */}
      <Dialog
        open={showCreateDialog}
        onClose={() => setShowCreateDialog(false)}
        title="Create Vault"
      >
        <div className="space-y-4">
          <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark">
            Choose a master password. This password encrypts your stored secrets and cannot be
            recovered if lost.
          </p>
          <Input
            label="Password"
            type="password"
            placeholder="Enter master password"
            value={createPassword}
            onChange={(e) => setCreatePassword(e.target.value)}
          />
          <Input
            label="Confirm Password"
            type="password"
            placeholder="Re-enter password"
            value={createConfirm}
            onChange={(e) => setCreateConfirm(e.target.value)}
            error={
              createConfirm && createPassword !== createConfirm
                ? "Passwords do not match"
                : undefined
            }
          />
          <div className="flex justify-end gap-2 pt-2">
            <Button variant="ghost" size="sm" onClick={() => setShowCreateDialog(false)}>
              Cancel
            </Button>
            <Button
              size="sm"
              onClick={handleCreate}
              disabled={!createPassword || createPassword !== createConfirm}
            >
              Create
            </Button>
          </div>
        </div>
      </Dialog>

      {/* Unlock dialog */}
      <Dialog
        open={showUnlockDialog}
        onClose={() => setShowUnlockDialog(false)}
        title="Unlock Vault"
      >
        <div className="space-y-4">
          <Input
            label="Master Password"
            type="password"
            placeholder="Enter your master password"
            value={unlockPassword}
            onChange={(e) => setUnlockPassword(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleUnlock();
            }}
          />
          <div className="flex justify-end gap-2 pt-2">
            <Button variant="ghost" size="sm" onClick={() => setShowUnlockDialog(false)}>
              Cancel
            </Button>
            <Button size="sm" onClick={handleUnlock} disabled={!unlockPassword}>
              Unlock
            </Button>
          </div>
        </div>
      </Dialog>

      {/* Change password dialog */}
      <Dialog
        open={showChangeDialog}
        onClose={() => setShowChangeDialog(false)}
        title="Change Vault Password"
      >
        <div className="space-y-4">
          <Input
            label="Current Password"
            type="password"
            placeholder="Enter current password"
            value={currentPassword}
            onChange={(e) => setCurrentPassword(e.target.value)}
          />
          <Input
            label="New Password"
            type="password"
            placeholder="Enter new password"
            value={changeNewPassword}
            onChange={(e) => setChangeNewPassword(e.target.value)}
          />
          <Input
            label="Confirm New Password"
            type="password"
            placeholder="Re-enter new password"
            value={changeConfirm}
            onChange={(e) => setChangeConfirm(e.target.value)}
            error={
              changeConfirm && changeNewPassword !== changeConfirm
                ? "Passwords do not match"
                : undefined
            }
          />
          <div className="flex justify-end gap-2 pt-2">
            <Button variant="ghost" size="sm" onClick={() => setShowChangeDialog(false)}>
              Cancel
            </Button>
            <Button
              size="sm"
              onClick={handleChangePassword}
              disabled={
                !currentPassword || !changeNewPassword || changeNewPassword !== changeConfirm
              }
            >
              Change Password
            </Button>
          </div>
        </div>
      </Dialog>
    </>
  );
}

// ---------------------------------------------------------------------------
// Section: SSH Key Management
// ---------------------------------------------------------------------------

function SshKeysSection() {
  const { addToast } = useToastStore();
  const [keys, setKeys] = useState<SshKeyInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [showGenerateDialog, setShowGenerateDialog] = useState(false);
  const [genKeyType, setGenKeyType] = useState("ed25519");
  const [genComment, setGenComment] = useState("");
  const [generating, setGenerating] = useState(false);

  const load = useCallback(async () => {
    try {
      setLoading(true);
      const sshKeys = await api.listSshKeys();
      setKeys(sshKeys);
    } catch (err) {
      addToast({ type: "error", title: "Failed to list SSH keys", message: formatError(err) });
    } finally {
      setLoading(false);
    }
  }, [addToast]);

  useEffect(() => {
    load();
  }, [load]);

  const handleGenerate = async () => {
    if (!genComment.trim()) return;
    try {
      setGenerating(true);
      const newKey = await api.generateSshKey(genKeyType, genComment.trim());
      setKeys((prev) => [...prev, newKey]);
      setShowGenerateDialog(false);
      setGenComment("");
      addToast({ type: "success", title: "SSH key generated", message: newKey.name });
    } catch (err) {
      addToast({ type: "error", title: "Failed to generate SSH key", message: formatError(err) });
    } finally {
      setGenerating(false);
    }
  };

  return (
    <>
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>
                <span className="inline-flex items-center gap-2">
                  <Key size={16} />
                  SSH Key Management
                </span>
              </CardTitle>
              <CardDescription>Manage SSH keys for authentication</CardDescription>
            </div>
            <div className="flex gap-2">
              <Button variant="ghost" size="sm" onClick={load} disabled={loading}>
                <RefreshCw size={14} className={loading ? "animate-spin" : ""} />
              </Button>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => {
                  setGenComment("");
                  setGenKeyType("ed25519");
                  setShowGenerateDialog(true);
                }}
              >
                <Plus size={14} />
                Generate Key
              </Button>
            </div>
          </div>
        </CardHeader>

        {loading ? (
          <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark">
            Loading SSH keys...
          </p>
        ) : keys.length === 0 ? (
          <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark">
            No SSH keys found.
          </p>
        ) : (
          <div className="space-y-2">
            {keys.map((k) => (
              <div
                key={k.path}
                className="flex items-center gap-3 rounded-lg border border-border-light dark:border-border-dark px-3 py-2"
              >
                <Key
                  size={16}
                  className="text-text-secondary-light dark:text-text-secondary-dark shrink-0"
                />
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark truncate">
                    {k.name}
                  </p>
                  <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark truncate font-mono">
                    {k.path}
                  </p>
                </div>
                <div className="flex items-center gap-2 shrink-0">
                  <Badge variant="default">{k.key_type}</Badge>
                  {k.is_fido2 && (
                    <Badge variant="info">
                      <Fingerprint size={10} className="mr-1" />
                      FIDO2
                    </Badge>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </Card>

      <Dialog
        open={showGenerateDialog}
        onClose={() => setShowGenerateDialog(false)}
        title="Generate SSH Key"
      >
        <div className="space-y-4">
          <div className="flex flex-col gap-1.5">
            <label className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark">
              Key Type
            </label>
            <select
              value={genKeyType}
              onChange={(e) => setGenKeyType(e.target.value)}
              className="w-full rounded-lg border border-border-light dark:border-border-dark bg-surface-light dark:bg-surface-dark px-3 py-2 text-sm text-text-primary-light dark:text-text-primary-dark focus:outline-none focus:ring-2 focus:ring-warm-300/50 focus:border-warm-400"
            >
              <option value="ed25519">Ed25519 (recommended)</option>
              <option value="rsa">RSA (4096-bit)</option>
              <option value="ecdsa">ECDSA</option>
            </select>
          </div>
          <Input
            label="Comment"
            placeholder="e.g. user@hostname"
            value={genComment}
            onChange={(e) => setGenComment(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleGenerate();
            }}
          />
          <div className="flex justify-end gap-2 pt-2">
            <Button variant="ghost" size="sm" onClick={() => setShowGenerateDialog(false)}>
              Cancel
            </Button>
            <Button
              size="sm"
              onClick={handleGenerate}
              disabled={!genComment.trim()}
              loading={generating}
            >
              Generate
            </Button>
          </div>
        </div>
      </Dialog>
    </>
  );
}

// ---------------------------------------------------------------------------
// Section: YubiKey Management
// ---------------------------------------------------------------------------

function YubiKeySection() {
  const { addToast } = useToastStore();
  const [yubikeys, setYubikeys] = useState<YubiKeyInfo[]>([]);
  const [fido2Supported, setFido2Supported] = useState(false);
  const [credentials, setCredentials] = useState<Fido2Credential[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadingCredentials, setLoadingCredentials] = useState(false);

  const [showGenerateDialog, setShowGenerateDialog] = useState(false);
  const [fido2KeyName, setFido2KeyName] = useState("");
  const [fido2Comment, setFido2Comment] = useState("");
  const [fido2Resident, setFido2Resident] = useState(true);
  const [fido2Application, setFido2Application] = useState("");
  const [generating, setGenerating] = useState(false);

  const load = useCallback(async () => {
    try {
      setLoading(true);
      const [yks, supported] = await Promise.all([api.detectYubikeys(), api.isFido2Supported()]);
      setYubikeys(yks);
      setFido2Supported(supported);
    } catch (err) {
      addToast({ type: "error", title: "Failed to detect YubiKeys", message: formatError(err) });
    } finally {
      setLoading(false);
    }
  }, [addToast]);

  useEffect(() => {
    load();
  }, [load]);

  const loadCredentials = async (serial?: string) => {
    try {
      setLoadingCredentials(true);
      const creds = await api.listFido2Credentials(serial);
      setCredentials(creds);
    } catch (err) {
      addToast({
        type: "error",
        title: "Failed to list FIDO2 credentials",
        message: formatError(err),
      });
    } finally {
      setLoadingCredentials(false);
    }
  };

  const handleGenerateFido2 = async () => {
    if (!fido2KeyName.trim() || !fido2Comment.trim()) return;
    try {
      setGenerating(true);
      await api.generateFido2SshKey(
        fido2KeyName.trim(),
        fido2Comment.trim(),
        fido2Resident,
        fido2Application.trim() || undefined,
      );
      setShowGenerateDialog(false);
      setFido2KeyName("");
      setFido2Comment("");
      setFido2Application("");
      addToast({ type: "success", title: "FIDO2 SSH key generated" });
      // Reload credentials after generating
      if (yubikeys.length > 0) {
        loadCredentials(yubikeys[0].serial);
      }
    } catch (err) {
      addToast({
        type: "error",
        title: "Failed to generate FIDO2 SSH key",
        message: formatError(err),
      });
    } finally {
      setGenerating(false);
    }
  };

  return (
    <>
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>
                <span className="inline-flex items-center gap-2">
                  <Fingerprint size={16} />
                  YubiKey Management
                </span>
              </CardTitle>
              <CardDescription>
                Detect connected YubiKeys and manage FIDO2 credentials
              </CardDescription>
            </div>
            <div className="flex gap-2">
              <Button variant="ghost" size="sm" onClick={load} disabled={loading}>
                <RefreshCw size={14} className={loading ? "animate-spin" : ""} />
              </Button>
              {fido2Supported && (
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => {
                    setFido2KeyName("");
                    setFido2Comment("");
                    setFido2Resident(true);
                    setFido2Application("");
                    setShowGenerateDialog(true);
                  }}
                >
                  <Plus size={14} />
                  Generate FIDO2 Key
                </Button>
              )}
            </div>
          </div>
        </CardHeader>

        {/* FIDO2 system support */}
        <div className="mb-4 flex items-center gap-2">
          <span className="text-sm text-text-secondary-light dark:text-text-secondary-dark">
            FIDO2 Support:
          </span>
          {loading ? (
            <Badge variant="default">Checking...</Badge>
          ) : fido2Supported ? (
            <Badge variant="success">Available</Badge>
          ) : (
            <Badge variant="warning">Not available</Badge>
          )}
        </div>

        {loading ? (
          <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark">
            Detecting YubiKeys...
          </p>
        ) : yubikeys.length === 0 ? (
          <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark">
            No YubiKeys detected. Insert a YubiKey and press refresh.
          </p>
        ) : (
          <div className="space-y-3">
            {yubikeys.map((yk) => (
              <div
                key={yk.serial}
                className="rounded-lg border border-border-light dark:border-border-dark p-3"
              >
                <div className="flex items-center gap-3">
                  <UsbIcon
                    size={18}
                    className="text-text-secondary-light dark:text-text-secondary-dark shrink-0"
                  />
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark">
                      {yk.model}
                    </p>
                    <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark">
                      Serial: {yk.serial} &middot; Firmware: {yk.firmware}
                    </p>
                  </div>
                  <div className="flex items-center gap-2 shrink-0">
                    {yk.fido2_supported ? (
                      <Badge variant="success">
                        <Fingerprint size={10} className="mr-1" />
                        FIDO2
                      </Badge>
                    ) : (
                      <Badge variant="warning">No FIDO2</Badge>
                    )}
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => loadCredentials(yk.serial)}
                      disabled={loadingCredentials || !yk.fido2_supported}
                    >
                      <Key size={14} />
                      Credentials
                    </Button>
                  </div>
                </div>
              </div>
            ))}

            {/* FIDO2 Credentials list */}
            {credentials.length > 0 && (
              <div className="mt-3">
                <h4 className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark mb-2">
                  FIDO2 Credentials
                </h4>
                {loadingCredentials ? (
                  <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark">
                    Loading credentials...
                  </p>
                ) : (
                  <div className="space-y-2">
                    {credentials.map((cred) => (
                      <div
                        key={cred.credential_id}
                        className="flex items-center gap-3 rounded-lg border border-border-light dark:border-border-dark px-3 py-2"
                      >
                        <Fingerprint
                          size={14}
                          className="text-text-secondary-light dark:text-text-secondary-dark shrink-0"
                        />
                        <div className="flex-1 min-w-0">
                          <p className="text-sm text-text-primary-light dark:text-text-primary-dark truncate">
                            {cred.user_name || cred.credential_id}
                          </p>
                          <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark truncate font-mono">
                            RP: {cred.rp_id}
                          </p>
                        </div>
                        {cred.resident && <Badge variant="info">Resident</Badge>}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>
        )}
      </Card>

      <Dialog
        open={showGenerateDialog}
        onClose={() => setShowGenerateDialog(false)}
        title="Generate FIDO2 SSH Key"
      >
        <div className="space-y-4">
          <Input
            label="Key Name"
            placeholder="e.g. my-yubikey"
            value={fido2KeyName}
            onChange={(e) => setFido2KeyName(e.target.value)}
          />
          <Input
            label="Comment"
            placeholder="e.g. user@hostname"
            value={fido2Comment}
            onChange={(e) => setFido2Comment(e.target.value)}
          />
          <Input
            label="Application (optional)"
            placeholder="e.g. ssh:myapp"
            value={fido2Application}
            onChange={(e) => setFido2Application(e.target.value)}
          />
          <label className="flex items-center gap-2 text-sm text-text-primary-light dark:text-text-primary-dark cursor-pointer">
            <input
              type="checkbox"
              checked={fido2Resident}
              onChange={(e) => setFido2Resident(e.target.checked)}
              className="rounded border-border-light dark:border-border-dark accent-warm-400"
            />
            Resident key (discoverable credential)
          </label>
          <div className="flex justify-end gap-2 pt-2">
            <Button variant="ghost" size="sm" onClick={() => setShowGenerateDialog(false)}>
              Cancel
            </Button>
            <Button
              size="sm"
              onClick={handleGenerateFido2}
              disabled={!fido2KeyName.trim() || !fido2Comment.trim()}
              loading={generating}
            >
              Generate
            </Button>
          </div>
        </div>
      </Dialog>
    </>
  );
}

// ---------------------------------------------------------------------------
// Main SettingsPage
// ---------------------------------------------------------------------------

export function SettingsPage() {
  return (
    <div className="max-w-2xl">
      <div className="mb-6">
        <h1 className="text-xl font-semibold text-text-primary-light dark:text-text-primary-dark">
          Settings
        </h1>
        <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark mt-1">
          Configure application preferences
        </p>
      </div>

      <div className="space-y-6">
        <AppearanceSection />
        <SharedVariablesSection />
        <VaultSection />
        <SshKeysSection />
        <YubiKeySection />
      </div>

    </div>
  );
}
