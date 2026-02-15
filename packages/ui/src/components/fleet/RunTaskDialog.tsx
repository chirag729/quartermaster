import { useState, useMemo } from "react";
import { Play, CheckCircle2, Circle } from "lucide-react";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { Badge } from "../ui/Badge";
import { SearchInput } from "../ui/SearchInput";
import type { TaskInfo, TaskStateInfo } from "@quartermaster/core";

interface RunTaskDialogProps {
  open: boolean;
  onClose: () => void;
  onRun: (taskId: string) => void;
  tasks: TaskInfo[];
  taskStates: TaskStateInfo[];
  runningTaskId: string | null;
}

export function RunTaskDialog({
  open,
  onClose,
  onRun,
  tasks,
  taskStates,
  runningTaskId,
}: RunTaskDialogProps) {
  const [search, setSearch] = useState("");

  const stateMap = useMemo(
    () => new Map(taskStates.map((t) => [t.id, t])),
    [taskStates],
  );

  const filtered = useMemo(() => {
    const q = search.toLowerCase();
    return tasks.filter(
      (t) =>
        t.name.toLowerCase().includes(q) ||
        t.category.toLowerCase().includes(q) ||
        t.tags.some((tag) => tag.toLowerCase().includes(q)),
    );
  }, [tasks, search]);

  // Group by category
  const grouped = useMemo(() => {
    const map = new Map<string, TaskInfo[]>();
    for (const task of filtered) {
      const list = map.get(task.category) ?? [];
      list.push(task);
      map.set(task.category, list);
    }
    return Array.from(map.entries()).sort(([a], [b]) => a.localeCompare(b));
  }, [filtered]);

  const handleClose = () => {
    setSearch("");
    onClose();
  };

  return (
    <Dialog open={open} onClose={handleClose} title="Run Task">
      <SearchInput
        placeholder="Search tasks..."
        value={search}
        onValueChange={setSearch}
        className="mb-4"
      />
      <div className="space-y-4 max-h-[50vh] overflow-y-auto">
        {grouped.map(([category, categoryTasks]) => (
          <div key={category}>
            <h3 className="text-xs font-semibold uppercase tracking-wider text-text-secondary-light dark:text-text-secondary-dark mb-2">
              {category}
            </h3>
            <div className="space-y-1">
              {categoryTasks.map((task) => {
                const state = stateMap.get(task.id);
                const isInstalled = state?.status === "completed";
                const isRunning = runningTaskId === task.id;

                return (
                  <div
                    key={task.id}
                    className="flex items-center gap-3 p-3 rounded-lg border border-border-light dark:border-border-dark bg-surface-light dark:bg-surface-dark"
                  >
                    {isInstalled ? (
                      <CheckCircle2 size={16} className="text-green-500 shrink-0" />
                    ) : (
                      <Circle size={16} className="text-gray-400 dark:text-gray-600 shrink-0" />
                    )}
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark truncate">
                          {task.name}
                        </span>
                        {isInstalled && <Badge variant="success">Installed</Badge>}
                      </div>
                      <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark truncate">
                        {task.description}
                      </p>
                    </div>
                    <Button
                      variant="secondary"
                      size="sm"
                      onClick={() => onRun(task.id)}
                      loading={isRunning}
                      disabled={isRunning}
                      className="shrink-0"
                    >
                      <Play size={12} />
                      Run
                    </Button>
                  </div>
                );
              })}
            </div>
          </div>
        ))}
        {grouped.length === 0 && (
          <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark text-center py-4">
            No tasks match your search.
          </p>
        )}
      </div>
    </Dialog>
  );
}
