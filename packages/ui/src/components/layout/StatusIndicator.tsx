import clsx from "clsx";
import type { TaskStatus } from "@quartermaster/core";
import { CheckCircle, XCircle, Loader, Circle, HelpCircle, SkipForward } from "lucide-react";

const statusConfig: Record<TaskStatus, { icon: typeof CheckCircle; color: string; label: string }> = {
  completed: { icon: CheckCircle, color: "text-green-600 dark:text-green-400", label: "Completed" },
  failed: { icon: XCircle, color: "text-red-600 dark:text-red-400", label: "Failed" },
  in_progress: { icon: Loader, color: "text-warm-500 dark:text-warm-300", label: "In Progress" },
  not_started: { icon: Circle, color: "text-text-secondary-light dark:text-text-secondary-dark", label: "Not Started" },
  skipped: { icon: SkipForward, color: "text-text-secondary-light dark:text-text-secondary-dark", label: "Skipped" },
  unknown: { icon: HelpCircle, color: "text-text-secondary-light dark:text-text-secondary-dark", label: "Unknown" },
};

interface Props {
  status: TaskStatus;
  size?: number;
}

export function StatusIndicator({ status, size = 16 }: Props) {
  const config = statusConfig[status];
  const Icon = config.icon;

  return (
    <span className={clsx("inline-flex items-center gap-1.5", config.color)} title={config.label}>
      <Icon size={size} className={clsx(status === "in_progress" && "animate-spin")} />
      <span className="text-xs font-medium">{config.label}</span>
    </span>
  );
}
