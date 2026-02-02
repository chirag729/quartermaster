import type { ReactNode } from "react";
import clsx from "clsx";

interface EmptyStateProps {
  icon?: ReactNode;
  title: string;
  description?: string;
  action?: ReactNode;
  className?: string;
}

export function EmptyState({ icon, title, description, action, className }: EmptyStateProps) {
  return (
    <div className={clsx("flex flex-col items-center justify-center py-16 text-center", className)}>
      {icon && (
        <div className="mb-4 text-text-secondary-light/40 dark:text-text-secondary-dark/40">
          {icon}
        </div>
      )}
      <h3 className="text-sm font-semibold text-text-primary-light dark:text-text-primary-dark">
        {title}
      </h3>
      {description && (
        <p className="mt-1 text-sm text-text-secondary-light dark:text-text-secondary-dark max-w-sm">
          {description}
        </p>
      )}
      {action && <div className="mt-4">{action}</div>}
    </div>
  );
}
