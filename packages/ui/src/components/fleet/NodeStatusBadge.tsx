import clsx from "clsx";
import type { NodeStatus } from "@quartermaster/core";

interface NodeStatusBadgeProps {
  status: NodeStatus;
  className?: string;
}

const statusConfig: Record<NodeStatus, { label: string; dot: string; bg: string; text: string }> = {
  online: {
    label: "Online",
    dot: "bg-green-500",
    bg: "bg-green-100 dark:bg-green-900/30",
    text: "text-green-700 dark:text-green-300",
  },
  offline: {
    label: "Offline",
    dot: "bg-gray-400 dark:bg-gray-500",
    bg: "bg-gray-100 dark:bg-gray-800/30",
    text: "text-gray-600 dark:text-gray-400",
  },
  connecting: {
    label: "Connecting",
    dot: "bg-yellow-500 animate-pulse",
    bg: "bg-yellow-100 dark:bg-yellow-900/30",
    text: "text-yellow-700 dark:text-yellow-300",
  },
  error: {
    label: "Error",
    dot: "bg-red-500",
    bg: "bg-red-100 dark:bg-red-900/30",
    text: "text-red-700 dark:text-red-300",
  },
  unknown: {
    label: "Unknown",
    dot: "bg-gray-300 dark:bg-gray-600",
    bg: "bg-gray-100 dark:bg-gray-800/30",
    text: "text-gray-500 dark:text-gray-400",
  },
};

export function NodeStatusBadge({ status, className }: NodeStatusBadgeProps) {
  const config = statusConfig[status];
  return (
    <span
      className={clsx(
        "inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-md text-xs font-medium",
        config.bg,
        config.text,
        className,
      )}
    >
      <span className={clsx("h-1.5 w-1.5 rounded-full", config.dot)} />
      {config.label}
    </span>
  );
}
