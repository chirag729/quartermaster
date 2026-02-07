import { Link } from "react-router-dom";
import { ChevronRight } from "lucide-react";
import clsx from "clsx";

interface BreadcrumbItem {
  label: string;
  to?: string;
}

interface BreadcrumbProps {
  items: BreadcrumbItem[];
  className?: string;
}

export function Breadcrumb({ items, className }: BreadcrumbProps) {
  return (
    <nav aria-label="Breadcrumb" className={clsx("flex items-center gap-1 text-sm", className)}>
      {items.map((item, i) => {
        const isLast = i === items.length - 1;
        return (
          <span key={i} className="flex items-center gap-1">
            {i > 0 && (
              <ChevronRight
                size={14}
                className="text-text-secondary-light/40 dark:text-text-secondary-dark/40"
              />
            )}
            {item.to && !isLast ? (
              <Link
                to={item.to}
                className="text-text-secondary-light dark:text-text-secondary-dark hover:text-text-primary-light dark:hover:text-text-primary-dark transition-colors"
              >
                {item.label}
              </Link>
            ) : (
              <span
                className={clsx(
                  isLast
                    ? "text-text-primary-light dark:text-text-primary-dark font-medium"
                    : "text-text-secondary-light dark:text-text-secondary-dark",
                )}
              >
                {item.label}
              </span>
            )}
          </span>
        );
      })}
    </nav>
  );
}
