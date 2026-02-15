import { useState } from "react";
import { Dialog } from "../ui/Dialog";
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";

interface ImportBlueprintDialogProps {
  open: boolean;
  onClose: () => void;
  onImport: (path: string) => Promise<void>;
  loading?: boolean;
}

export function ImportBlueprintDialog({ open, onClose, onImport, loading }: ImportBlueprintDialogProps) {
  const [path, setPath] = useState("");
  const [error, setError] = useState("");

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");
    try {
      await onImport(path.trim());
    } catch (err) {
      // Error is handled by the parent via toast; keep dialog open
      setError(typeof err === "object" && err !== null && "message" in err ? (err as { message: string }).message : String(err));
    }
  };

  const handleClose = () => {
    setPath("");
    setError("");
    onClose();
  };

  return (
    <Dialog open={open} onClose={handleClose} title="Import Blueprint">
      <form onSubmit={handleSubmit} className="space-y-4">
        <Input
          label="Package File Path"
          placeholder="/path/to/blueprint.qmbp"
          value={path}
          onChange={(e) => setPath(e.target.value)}
          error={error}
          required
        />
        <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark">
          Enter the full path to a <code>.qmbp</code> blueprint package file.
        </p>
        <div className="flex justify-end gap-2 pt-2">
          <Button type="button" variant="ghost" onClick={handleClose}>
            Cancel
          </Button>
          <Button type="submit" disabled={!path.trim()} loading={loading}>
            Import
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
