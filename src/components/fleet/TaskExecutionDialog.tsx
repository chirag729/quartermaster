import { Dialog } from "../ui/Dialog";
import { ExecutionOutputPanel } from "../ui/ExecutionOutputPanel";

interface TaskExecutionDialogProps {
  open: boolean;
  onClose: () => void;
  nodeId: string;
  taskId: string;
  taskName: string;
  isRunning: boolean;
}

export function TaskExecutionDialog({
  open,
  onClose,
  nodeId,
  taskId,
  taskName,
  isRunning,
}: TaskExecutionDialogProps) {
  return (
    <Dialog
      open={open}
      onClose={onClose}
      title={`Running: ${taskName}`}
      className="max-w-3xl"
    >
      <ExecutionOutputPanel
        nodeId={nodeId}
        taskIdFilter={taskId}
        isActive={isRunning}
      />
      {!isRunning && (
        <div className="mt-3 flex justify-end">
          <button
            onClick={onClose}
            className="px-3 py-1.5 text-sm rounded-md bg-warm-100 dark:bg-warm-900/30 text-text-primary-light dark:text-text-primary-dark hover:bg-warm-200 dark:hover:bg-warm-900/50 transition-colors"
          >
            Close
          </button>
        </div>
      )}
    </Dialog>
  );
}
