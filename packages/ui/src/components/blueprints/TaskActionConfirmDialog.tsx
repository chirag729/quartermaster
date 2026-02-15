import { useState, useCallback } from "react";
import { CheckCircle2, AlertTriangle } from "lucide-react";
import { Dialog } from "../ui/Dialog";
import { ExecutionOutputPanel } from "../ui/ExecutionOutputPanel";
import { Button } from "../ui/Button";
import * as api from "../../services/tauriCommands";
import type { TaskInfo } from "@quartermaster/core";
import type { BlueprintTaskEntry } from "@quartermaster/core";

type Phase = "confirm" | "executing" | "done";

interface TaskResult {
  taskId: string;
  success: boolean;
  error?: string;
}

interface TaskActionConfirmDialogProps {
  action: "install" | "uninstall";
  taskChain: string[];
  taskInfoMap: Record<string, TaskInfo>;
  nodeId: string;
  blueprintId: string;
  pendingEntries: BlueprintTaskEntry[];
  onClose: (refreshStates: boolean) => void;
}

export function TaskActionConfirmDialog({
  action,
  taskChain,
  taskInfoMap,
  nodeId,
  blueprintId,
  pendingEntries,
  onClose,
}: TaskActionConfirmDialogProps) {
  const [phase, setPhase] = useState<Phase>("confirm");
  const [results, setResults] = useState<TaskResult[]>([]);
  const [currentTaskIndex, setCurrentTaskIndex] = useState(0);

  const targetTaskId = taskChain[taskChain.length - 1];
  const targetName = taskInfoMap[targetTaskId]?.name ?? targetTaskId;

  const title =
    phase === "confirm"
      ? `${action === "install" ? "Install" : "Uninstall"} Task`
      : phase === "executing"
        ? `${action === "install" ? "Installing" : "Uninstalling"}...`
        : `${action === "install" ? "Installation" : "Uninstall"} Complete`;

  // Build a lookup of config_overrides by task_id from blueprint entries
  const overridesMap = new Map<string, Record<string, unknown>>();
  for (const entry of pendingEntries) {
    if (Object.keys(entry.config_overrides).length > 0) {
      overridesMap.set(entry.task_id, entry.config_overrides);
    }
  }

  const handleProceed = useCallback(async () => {
    setPhase("executing");
    const taskResults: TaskResult[] = [];

    for (let i = 0; i < taskChain.length; i++) {
      const taskId = taskChain[i];
      setCurrentTaskIndex(i);

      try {
        if (action === "install") {
          const overrides = overridesMap.get(taskId);
          await api.executeTask(taskId, nodeId, blueprintId, overrides);
        } else {
          await api.uninstallTask(taskId, nodeId);
        }
        taskResults.push({ taskId, success: true });
      } catch (err) {
        taskResults.push({
          taskId,
          success: false,
          error: err instanceof Error ? err.message : String(err),
        });
        // Stop on first failure
        break;
      }
    }

    setResults(taskResults);
    setPhase("done");
  }, [taskChain, action, nodeId, blueprintId, overridesMap]);

  const currentTaskId = taskChain[currentTaskIndex] ?? taskChain[0];

  return (
    <Dialog
      open
      onClose={() => onClose(phase === "done")}
      title={title}
      className="max-w-2xl"
    >
      {/* ── Confirm phase ──────────────────────────────────────────── */}
      {phase === "confirm" && (
        <div className="space-y-4">
          {taskChain.length === 1 ? (
            <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark">
              {action === "install" ? "Install" : "Uninstall"}{" "}
              <span className="font-medium text-text-primary-light dark:text-text-primary-dark">
                {targetName}
              </span>
              ?
            </p>
          ) : (
            <>
              <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark">
                The following tasks will also be{" "}
                {action === "install" ? "installed" : "uninstalled"}:
              </p>
              <ul className="space-y-1.5">
                {taskChain.map((taskId, idx) => {
                  const isTarget = taskId === targetTaskId;
                  const name = taskInfoMap[taskId]?.name ?? taskId;
                  return (
                    <li
                      key={taskId}
                      className="flex items-center gap-2 text-sm"
                    >
                      <span className="text-xs font-mono text-text-secondary-light dark:text-text-secondary-dark w-5 text-right">
                        {idx + 1}.
                      </span>
                      <span
                        className={
                          isTarget
                            ? "font-medium text-text-primary-light dark:text-text-primary-dark"
                            : "text-text-secondary-light dark:text-text-secondary-dark"
                        }
                      >
                        {name}
                      </span>
                      {isTarget && (
                        <span className="text-xs text-warm-500">(target)</span>
                      )}
                    </li>
                  );
                })}
              </ul>
            </>
          )}
          <div className="flex justify-end gap-2 pt-2">
            <Button
              size="sm"
              variant="ghost"
              onClick={() => onClose(false)}
            >
              Cancel
            </Button>
            <Button
              size="sm"
              variant={action === "uninstall" ? "danger" : "primary"}
              onClick={handleProceed}
            >
              {action === "install" ? "Install" : "Uninstall"}
            </Button>
          </div>
        </div>
      )}

      {/* ── Executing + Done phases (output panel persists across both) ── */}
      {(phase === "executing" || phase === "done") && (
        <div className="space-y-3">
          {phase === "executing" && (
            <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark">
              {action === "install" ? "Installing" : "Uninstalling"} task{" "}
              {currentTaskIndex + 1} of {taskChain.length}:{" "}
              <span className="font-medium text-text-primary-light dark:text-text-primary-dark">
                {taskInfoMap[currentTaskId]?.name ?? currentTaskId}
              </span>
            </p>
          )}

          {phase === "done" && (
            <>
              {results.every((r) => r.success) ? (
                <div className="flex items-center gap-2 text-green-600 dark:text-green-400">
                  <CheckCircle2 size={18} />
                  <span className="text-sm font-medium">
                    All tasks {action === "install" ? "installed" : "uninstalled"}{" "}
                    successfully
                  </span>
                </div>
              ) : (
                <div className="space-y-2">
                  <div className="flex items-center gap-2 text-red-600 dark:text-red-400">
                    <AlertTriangle size={18} />
                    <span className="text-sm font-medium">
                      {action === "install" ? "Installation" : "Uninstall"} stopped
                      due to an error
                    </span>
                  </div>
                  {results.filter((r) => !r.success).map((r) => (
                    <p key={r.taskId} className="text-xs text-red-500">
                      {taskInfoMap[r.taskId]?.name ?? r.taskId}: {r.error}
                    </p>
                  ))}
                </div>
              )}
            </>
          )}

          <ExecutionOutputPanel
            nodeId={nodeId}
            isActive={phase === "executing"}
          />

          {phase === "done" && (
            <div className="flex justify-end pt-1">
              <Button size="sm" onClick={() => onClose(true)}>
                Close
              </Button>
            </div>
          )}
        </div>
      )}
    </Dialog>
  );
}
