import { useState } from "react";
import { Dialog } from "../ui/Dialog";
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";

interface CreateBlueprintDialogProps {
  open: boolean;
  onClose: () => void;
  onSubmit: (name: string, description: string) => void;
  loading?: boolean;
}

export function CreateBlueprintDialog({ open, onClose, onSubmit, loading }: CreateBlueprintDialogProps) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit(name.trim(), description.trim());
  };

  const handleClose = () => {
    setName("");
    setDescription("");
    onClose();
  };

  return (
    <Dialog open={open} onClose={handleClose} title="Create Blueprint">
      <form onSubmit={handleSubmit} className="space-y-4">
        <Input
          label="Name"
          placeholder="e.g. My Custom Setup"
          value={name}
          onChange={(e) => setName(e.target.value)}
          required
        />
        <Input
          label="Description"
          placeholder="e.g. Development environment for web projects"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
        />
        <div className="flex justify-end gap-2 pt-2">
          <Button type="button" variant="ghost" onClick={handleClose}>
            Cancel
          </Button>
          <Button type="submit" disabled={!name.trim()} loading={loading}>
            Create
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
