import { useState } from "react";
import { formatError } from "../../lib/formatError";
import { Wifi } from "lucide-react";
import { Button } from "../ui/Button";
import { Badge } from "../ui/Badge";
import { testSshConnection } from "../../services/tauriCommands";

interface SshConnectionTestProps {
  host: string;
  port: number;
  username: string;
}

type TestState =
  | { status: "idle" }
  | { status: "testing" }
  | { status: "success"; message: string }
  | { status: "error"; message: string };

export function SshConnectionTest({ host, port, username }: SshConnectionTestProps) {
  const [state, setState] = useState<TestState>({ status: "idle" });

  const canTest = host.trim() !== "" && username.trim() !== "";

  const handleTest = async () => {
    setState({ status: "testing" });
    try {
      const message = await testSshConnection(host, port, username);
      setState({ status: "success", message });
    } catch (err) {
      setState({ status: "error", message: formatError(err) });
    }
  };

  return (
    <div className="flex items-center gap-3">
      <Button
        type="button"
        variant="secondary"
        size="sm"
        onClick={handleTest}
        loading={state.status === "testing"}
        disabled={!canTest}
      >
        <Wifi size={14} />
        {state.status === "testing" ? "Testing..." : "Test Connection"}
      </Button>
      {state.status === "success" && (
        <Badge variant="success">{state.message || "Connected"}</Badge>
      )}
      {state.status === "error" && (
        <Badge variant="danger">{state.message}</Badge>
      )}
    </div>
  );
}
