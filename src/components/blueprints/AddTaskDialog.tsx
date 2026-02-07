import { useMemo, useState } from "react";
import { Plus } from "lucide-react";
import { Dialog } from "../ui/Dialog";
import { SearchInput } from "../ui/SearchInput";
import { Button } from "../ui/Button";
import { Badge } from "../ui/Badge";
import type { TaskInfo } from "../../types/task";

interface AddTaskDialogProps {
  open: boolean;
  onClose: () => void;
  onAdd: (taskId: string) => void;
  tasks: TaskInfo[];
  existingTaskIds: string[];
}

export function AddTaskDialog({ open, onClose, onAdd, tasks, existingTaskIds }: AddTaskDialogProps) {
  const [search, setSearch] = useState("");

  const availableTasks = useMemo(() => {
    const existing = new Set(existingTaskIds);
    return tasks
      .filter((t) => !existing.has(t.id))
      .filter(
        (t) =>
          !search ||
          t.name.toLowerCase().includes(search.toLowerCase()) ||
          t.description.toLowerCase().includes(search.toLowerCase()) ||
          t.category.toLowerCase().includes(search.toLowerCase()),
      );
  }, [tasks, existingTaskIds, search]);

  const handleAdd = (taskId: string) => {
    onAdd(taskId);
    setSearch("");
  };

  const handleClose = () => {
    setSearch("");
    onClose();
  };

  return (
    <Dialog open={open} onClose={handleClose} title="Add Task to Blueprint">
      <SearchInput
        placeholder="Search tasks..."
        onValueChange={setSearch}
        className="mb-3"
      />
      <div className="space-y-1.5 max-h-[50vh] overflow-y-auto">
        {availableTasks.length === 0 ? (
          <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark text-center py-6">
            {search ? "No matching tasks found." : "All available tasks are already in this blueprint."}
          </p>
        ) : (
          availableTasks.map((task) => (
            <button
              key={task.id}
              type="button"
              onClick={() => handleAdd(task.id)}
              className="w-full flex items-center justify-between p-3 rounded-lg border border-border-light dark:border-border-dark bg-surface-light dark:bg-surface-dark hover:bg-warm-50 dark:hover:bg-warm-900/20 transition-colors text-left"
            >
              <div className="min-w-0 flex-1">
                <p className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark truncate">
                  {task.name}
                </p>
                <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark truncate">
                  {task.description}
                </p>
              </div>
              <div className="flex items-center gap-2 shrink-0 ml-3">
                {task.category && <Badge>{task.category}</Badge>}
                {task.privilege_level === "admin" && <Badge variant="warning">Admin</Badge>}
                <Plus size={14} className="text-warm-500" />
              </div>
            </button>
          ))
        )}
      </div>
      <div className="flex justify-end pt-3">
        <Button variant="ghost" onClick={handleClose}>
          Done
        </Button>
      </div>
    </Dialog>
  );
}
