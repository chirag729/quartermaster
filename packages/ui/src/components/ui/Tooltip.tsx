import type { ReactNode } from "react";
import clsx from "clsx";

interface TooltipProps {
  content: string;
  children: ReactNode;
  position?: "top" | "bottom" | "left" | "right";
  className?: string;
}

const positionStyles: Record<string, string> = {
  top: "bottom-full left-1/2 -translate-x-1/2 mb-2",
  bottom: "top-full left-1/2 -translate-x-1/2 mt-2",
  left: "right-full top-1/2 -translate-y-1/2 mr-2",
  right: "left-full top-1/2 -translate-y-1/2 ml-2",
};

export function Tooltip({ content, children, position = "top", className }: TooltipProps) {
  return (
    <div className={clsx("relative group inline-flex", className)}>
      {children}
      <div
        role="tooltip"
        className={clsx(
          "absolute z-50 hidden group-hover:block pointer-events-none whitespace-nowrap rounded-md px-2.5 py-1.5 text-xs font-medium bg-warm-900 dark:bg-warm-100 text-white dark:text-warm-900 shadow-lg",
          positionStyles[position],
        )}
      >
        {content}
      </div>
    </div>
  );
}
