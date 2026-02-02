import { useState } from "react";
import { Layers, ListChecks } from "lucide-react";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { Badge } from "../ui/Badge";
import type { Blueprint } from "../../types/blueprint";

interface AssignBlueprintDialogProps {
  open: boolean;
  onClose: () => void;
  onSelect: (blueprintId: string) => void;
  blueprints: Blueprint[];
  currentBlueprintId?: string | null;
  loading?: boolean;
}

export function AssignBlueprintDialog({
  open,
  onClose,
  onSelect,
  blueprints,
  currentBlueprintId,
  loading,
}: AssignBlueprintDialogProps) {
  const [selected, setSelected] = useState<string | null>(null);

  const handleSubmit = () => {
    if (selected) {
      onSelect(selected);
    }
  };

  const handleClose = () => {
    setSelected(null);
    onClose();
  };

  return (
    <Dialog open={open} onClose={handleClose} title="Assign Blueprint">
      <div className="space-y-3 max-h-[50vh] overflow-y-auto mb-4">
        {blueprints.map((bp) => (
          <button
            key={bp.id}
            type="button"
            onClick={() => setSelected(bp.id)}
            disabled={bp.id === currentBlueprintId}
            className={`w-full text-left flex items-center gap-3 p-3 rounded-lg border transition-colors ${
              selected === bp.id
                ? "border-warm-400 bg-warm-50 dark:bg-warm-900/20"
                : bp.id === currentBlueprintId
                  ? "border-border-light dark:border-border-dark bg-surface-light/50 dark:bg-surface-dark/50 opacity-50 cursor-not-allowed"
                  : "border-border-light dark:border-border-dark bg-surface-light dark:bg-surface-dark hover:bg-warm-50 dark:hover:bg-warm-900/10"
            }`}
          >
            <div className="p-2 rounded-lg bg-warm-100 dark:bg-warm-900/30 text-warm-600 dark:text-warm-400">
              <Layers size={16} />
            </div>
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <p className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark truncate">
                  {bp.name}
                </p>
                {bp.is_builtin && <Badge variant="info">Built-in</Badge>}
                {bp.id === currentBlueprintId && <Badge variant="default">Current</Badge>}
              </div>
              <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark truncate">
                {bp.description}
              </p>
            </div>
            <div className="flex items-center gap-1 text-xs text-text-secondary-light dark:text-text-secondary-dark shrink-0">
              <ListChecks size={12} />
              <span>{bp.task_entries.length}</span>
            </div>
          </button>
        ))}
        {blueprints.length === 0 && (
          <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark text-center py-4">
            No blueprints available.
          </p>
        )}
      </div>
      <div className="flex justify-end gap-2">
        <Button type="button" variant="ghost" onClick={handleClose}>
          Cancel
        </Button>
        <Button onClick={handleSubmit} disabled={!selected} loading={loading}>
          Assign
        </Button>
      </div>
    </Dialog>
  );
}
