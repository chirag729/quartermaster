import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import type { SshConfig, SshAuthMethod } from "../../types/node";

interface SshConfigFormProps {
  config: Partial<SshConfig>;
  onChange: (config: Partial<SshConfig>) => void;
  disabled?: boolean;
}

const authMethodOptions = [
  { value: "agent", label: "SSH Agent" },
  { value: "key_file", label: "Key File" },
  { value: "certificate", label: "Certificate" },
  { value: "fido2", label: "FIDO2" },
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
    case "fido2":
      return {
        type: "fido2",
        key_handle: current?.type === "fido2" ? current.key_handle : "",
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
    } else if (method.type === "fido2" && field === "key_handle") {
      update({ auth_method: { ...method, key_handle: value } });
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
      {authType === "fido2" && (
        <Input
          label="Key Handle"
          placeholder="FIDO2 key handle"
          value={
            config.auth_method?.type === "fido2"
              ? config.auth_method.key_handle
              : ""
          }
          onChange={(e) => updateAuthField("key_handle", e.target.value)}
          disabled={disabled}
        />
      )}
    </div>
  );
}
