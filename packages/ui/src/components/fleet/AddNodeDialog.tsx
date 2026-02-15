import { useState } from "react";
import { Dialog } from "../ui/Dialog";
import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import { Button } from "../ui/Button";
import { SshConfigForm } from "../ssh/SshConfigForm";
import { SshConnectionTest } from "../ssh/SshConnectionTest";
import type { NodeKind, SshConfig, SshAuthMethod } from "@quartermaster/core";

interface AddNodeDialogProps {
  open: boolean;
  onClose: () => void;
  onSubmit: (data: AddNodeFormData) => void;
  loading?: boolean;
}

export interface AddNodeFormData {
  name: string;
  kind: NodeKind;
  hostname: string;
  sshHost: string;
  sshPort: number;
  sshUsername: string;
  authMethod: SshAuthMethod;
}

const kindOptions = [
  { value: "local", label: "Local" },
  { value: "remote", label: "Remote" },
];

export function AddNodeDialog({ open, onClose, onSubmit, loading }: AddNodeDialogProps) {
  const [name, setName] = useState("");
  const [kind, setKind] = useState<NodeKind>("remote");
  const [hostname, setHostname] = useState("");
  const [sshConfig, setSshConfig] = useState<Partial<SshConfig>>({
    host: "",
    port: 22,
    username: "",
    auth_method: { type: "agent" },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit({
      name,
      kind,
      hostname,
      sshHost: sshConfig.host || hostname,
      sshPort: sshConfig.port ?? 22,
      sshUsername: sshConfig.username ?? "",
      authMethod: sshConfig.auth_method ?? { type: "agent" },
    });
  };

  const canSubmit =
    name.trim() !== "" &&
    hostname.trim() !== "" &&
    (kind === "local" || (sshConfig.username ?? "").trim() !== "");

  return (
    <Dialog open={open} onClose={onClose} title="Add Node">
      <form onSubmit={handleSubmit} className="space-y-4">
        <Input
          label="Node Name"
          placeholder="e.g. staging-web-01"
          value={name}
          onChange={(e) => setName(e.target.value)}
          required
        />
        <Select
          label="Node Kind"
          value={kind}
          onChange={(e) => setKind(e.target.value as NodeKind)}
          options={kindOptions}
        />
        <Input
          label="Hostname"
          placeholder="e.g. 192.168.1.100 or node.example.com"
          value={hostname}
          onChange={(e) => setHostname(e.target.value)}
          required
        />
        {kind === "remote" && (
          <>
            <div className="border-t border-border-light dark:border-border-dark pt-4">
              <h3 className="text-sm font-semibold text-text-primary-light dark:text-text-primary-dark mb-3">
                SSH Configuration
              </h3>
              <SshConfigForm
                config={sshConfig}
                onChange={setSshConfig}
                disabled={loading}
              />
            </div>
            <div className="border-t border-border-light dark:border-border-dark pt-4">
              <SshConnectionTest
                host={sshConfig.host || hostname}
                port={sshConfig.port ?? 22}
                username={sshConfig.username ?? ""}
              />
            </div>
          </>
        )}
        <div className="flex justify-end gap-2 pt-2">
          <Button type="button" variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button type="submit" disabled={!canSubmit} loading={loading}>
            Add Node
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
