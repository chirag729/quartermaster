import { useState } from "react";
import { Layers, ListChecks, Shield, Settings, Server, Monitor, type LucideIcon } from "lucide-react";
import { Dialog } from "../ui/Dialog";
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";
import clsx from "clsx";

const iconOptions: { name: string; icon: LucideIcon }[] = [
  { name: "Layers", icon: Layers },
  { name: "ListChecks", icon: ListChecks },
  { name: "Shield", icon: Shield },
  { name: "Settings", icon: Settings },
  { name: "Server", icon: Server },
  { name: "Monitor", icon: Monitor },
];

interface CreateBlueprintDialogProps {
  open: boolean;
  onClose: () => void;
  onSubmit: (name: string, description: string, icon?: string) => void;
  loading?: boolean;
}

export function CreateBlueprintDialog({ open, onClose, onSubmit, loading }: CreateBlueprintDialogProps) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [selectedIcon, setSelectedIcon] = useState("Layers");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit(name.trim(), description.trim(), selectedIcon);
  };

  const handleClose = () => {
    setName("");
    setDescription("");
    setSelectedIcon("Layers");
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
        <div>
          <label className="block text-sm font-medium text-text-primary-light dark:text-text-primary-dark mb-1.5">
            Icon
          </label>
          <div className="flex gap-2">
            {iconOptions.map(({ name: iconName, icon: IconComponent }) => (
              <button
                key={iconName}
                type="button"
                onClick={() => setSelectedIcon(iconName)}
                className={clsx(
                  "p-2 rounded-lg border transition-colors",
                  selectedIcon === iconName
                    ? "border-warm-500 bg-warm-50 dark:bg-warm-900/20 text-warm-600 dark:text-warm-400"
                    : "border-border-light dark:border-border-dark text-text-secondary-light dark:text-text-secondary-dark hover:border-warm-300 dark:hover:border-warm-700",
                )}
                title={iconName}
              >
                <IconComponent size={18} />
              </button>
            ))}
          </div>
        </div>
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
