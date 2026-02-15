import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import type { SshConfig, SshAuthMethod } from "@quartermaster/core";

interface SshConfigFormProps {
  config: Partial<SshConfig>;
  onChange: (config: Partial<SshConfig>) => void;
  disabled?: boolean;
}

const authMethodOptions = [
  { value: "agent", label: "SSH Agent" },
  { value: "key_file", label: "Key File" },
  { value: "certificate", label: "Certificate" },
  { value: "password", label: "Password" },
  { value: "fido2_resident", label: "FIDO2 Resident Key" },
];

function getAuthType(method?: SshAuthMethod): string {
  return method?.type ?? "agent";
}

function buildAuthMethod(
  type: string,
  current?: SshAuthMethod,
): SshAuthMethod {
  switch (type) {
    case "key_file":
      return {
        type: "key_file",
        private_key_path:
          current?.type === "key_file" ? current.private_key_path : "",
      };
    case "certificate":
      return {
        type: "certificate",
        certificate_path:
          current?.type === "certificate" ? current.certificate_path : "",
        private_key_path:
          current?.type === "certificate" ? current.private_key_path : "",
      };
    case "password":
      return {
        type: "password",
        vault_key: current?.type === "password" ? current.vault_key : undefined,
      };
    case "fido2_resident":
      return {
        type: "fido2_resident",
        application: current?.type === "fido2_resident" ? current.application : undefined,
      };
    default:
      return { type: "agent" };
  }
}

export function SshConfigForm({ config, onChange, disabled }: SshConfigFormProps) {
  const authType = getAuthType(config.auth_method);

  const update = (patch: Partial<SshConfig>) => {
    onChange({ ...config, ...patch });
  };

  const handleAuthTypeChange = (type: string) => {
    update({ auth_method: buildAuthMethod(type, config.auth_method) });
  };

  const updateAuthField = (field: string, value: string) => {
    const method = config.auth_method;
    if (!method) return;

    if (method.type === "key_file" && field === "private_key_path") {
      update({ auth_method: { ...method, private_key_path: value } });
    } else if (method.type === "certificate") {
      update({ auth_method: { ...method, [field]: value } });
    } else if (method.type === "password" && field === "vault_key") {
      update({ auth_method: { ...method, vault_key: value || undefined } });
    } else if (method.type === "fido2_resident" && field === "application") {
      update({ auth_method: { ...method, application: value || undefined } });
    }
  };

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 gap-3">
        <Input
          label="SSH Host"
          placeholder="Same as hostname"
          value={config.host ?? ""}
          onChange={(e) => update({ host: e.target.value })}
          disabled={disabled}
        />
        <Input
          label="SSH Port"
          type="number"
          placeholder="22"
          value={config.port?.toString() ?? "22"}
          onChange={(e) => update({ port: parseInt(e.target.value, 10) || 22 })}
          disabled={disabled}
        />
      </div>
      <Input
        label="SSH Username"
        placeholder="e.g. deploy"
        value={config.username ?? ""}
        onChange={(e) => update({ username: e.target.value })}
        disabled={disabled}
        required
      />
      <Select
        label="Auth Method"
        value={authType}
        onChange={(e) => handleAuthTypeChange(e.target.value)}
        options={authMethodOptions}
        disabled={disabled}
      />
      {authType === "key_file" && (
        <Input
          label="Private Key Path"
          placeholder="~/.ssh/id_ed25519"
          value={
            config.auth_method?.type === "key_file"
              ? config.auth_method.private_key_path
              : ""
          }
          onChange={(e) => updateAuthField("private_key_path", e.target.value)}
          disabled={disabled}
        />
      )}
      {authType === "certificate" && (
        <>
          <Input
            label="Certificate Path"
            placeholder="~/.ssh/id_ed25519-cert.pub"
            value={
              config.auth_method?.type === "certificate"
                ? config.auth_method.certificate_path
                : ""
            }
            onChange={(e) =>
              updateAuthField("certificate_path", e.target.value)
            }
            disabled={disabled}
          />
          <Input
            label="Private Key Path"
            placeholder="~/.ssh/id_ed25519"
            value={
              config.auth_method?.type === "certificate"
                ? config.auth_method.private_key_path
                : ""
            }
            onChange={(e) =>
              updateAuthField("private_key_path", e.target.value)
            }
            disabled={disabled}
          />
        </>
      )}
      {authType === "password" && (
        <Input
          label="Vault Key (optional)"
          placeholder="Key in vault for stored password"
          value={
            config.auth_method?.type === "password"
              ? config.auth_method.vault_key ?? ""
              : ""
          }
          onChange={(e) => updateAuthField("vault_key", e.target.value)}
          disabled={disabled}
        />
      )}
      {authType === "fido2_resident" && (
        <Input
          label="Application (optional)"
          placeholder="e.g. ssh:myapp"
          value={
            config.auth_method?.type === "fido2_resident"
              ? config.auth_method.application ?? ""
              : ""
          }
          onChange={(e) => updateAuthField("application", e.target.value)}
          disabled={disabled}
        />
      )}
    </div>
  );
}
