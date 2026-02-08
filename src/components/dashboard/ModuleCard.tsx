import { Card } from "../ui/Card";
import { Button } from "../ui/Button";
import { Badge } from "../ui/Badge";
import { StatusIndicator } from "../layout/StatusIndicator";
import type { TaskInfo } from "../../types/task";
import * as Icons from "lucide-react";
import clsx from "clsx";
import { motion } from "motion/react";

interface Props {
  task: TaskInfo;
  executing: boolean;
  onExecute: () => void;
  readOnly?: boolean;
  onCardClick?: () => void;
}

function getIcon(iconName: string) {
  const icons = Icons as unknown as Record<string, Icons.LucideIcon>;
  return icons[iconName] || Icons.Box;
}

export function ModuleCard({ task, executing, onExecute, readOnly, onCardClick }: Props) {
  const Icon = getIcon(task.icon);
  const isCompleted = task.status === "completed";
  const isRunning = task.status === "in_progress" || executing;
  const progress = task._progress ?? 0;

  return (
    <Card
      className={clsx(
        "group flex flex-col relative overflow-hidden",
        !readOnly && isCompleted && "ring-1 ring-green-200 dark:ring-green-900/50",
        onCardClick && "cursor-pointer",
      )}
      onClick={onCardClick}
    >
      {!readOnly && isRunning && progress > 0 && (
        <motion.div
          className="absolute bottom-0 left-0 h-1 rounded-full bg-warm-400"
          initial={{ width: 0 }}
          animate={{ width: `${progress * 100}%` }}
          transition={{ duration: 0.3 }}
        />
      )}
      <div className="flex items-start justify-between mb-3">
        <div className="p-2.5 rounded-lg bg-warm-100 dark:bg-warm-900/30 transition-colors group-hover:bg-warm-200 dark:group-hover:bg-warm-900/50">
          <Icon size={20} className="text-warm-500 dark:text-warm-300" />
        </div>
        {!readOnly && <StatusIndicator status={task.status} />}
      </div>
      <h3 className="text-sm font-semibold text-text-primary-light dark:text-text-primary-dark mb-1">
        {task.name}
      </h3>
      <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark mb-1 flex-1">
        {task.description}
      </p>
      {!readOnly && isRunning && task._progressMessage && (
        <p className="text-xs text-warm-500 dark:text-warm-300 mb-3 truncate">
          {task._progressMessage}
        </p>
      )}
      {!readOnly && task.error_message && !isRunning && (
        <p className="text-xs text-red-600 dark:text-red-400 mb-3 truncate" title={task.error_message}>
          {task.error_message}
        </p>
      )}
      <div className="flex items-center justify-between mt-auto">
        <div className="flex gap-1.5">
          <Badge>{task.category}</Badge>
          {task.privilege_level === "admin" && <Badge variant="warning">Admin</Badge>}
        </div>
        {!readOnly && (
          <Button
            size="sm"
            variant={isCompleted ? "secondary" : "primary"}
            disabled={isRunning}
            loading={isRunning}
            onClick={onExecute}
          >
            {isCompleted ? "Re-run" : "Setup"}
          </Button>
        )}
      </div>
    </Card>
  );
}
