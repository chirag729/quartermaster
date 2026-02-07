import { useState, type ReactNode } from "react";
import { ChevronDown } from "lucide-react";
import clsx from "clsx";

interface AccordionItemProps {
  title: string;
  children: ReactNode;
  defaultOpen?: boolean;
  className?: string;
}

export function AccordionItem({ title, children, defaultOpen = false, className }: AccordionItemProps) {
  const [open, setOpen] = useState(defaultOpen);

  return (
    <div className={clsx("border border-border-light dark:border-border-dark rounded-lg", className)}>
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className="flex items-center justify-between w-full px-4 py-3 text-sm font-medium text-text-primary-light dark:text-text-primary-dark hover:bg-warm-50 dark:hover:bg-warm-900/10 transition-colors rounded-lg"
      >
        {title}
        <ChevronDown
          size={16}
          className={clsx(
            "text-text-secondary-light dark:text-text-secondary-dark transition-transform duration-200",
            open && "rotate-180",
          )}
        />
      </button>
      {open && (
        <div className="px-4 pb-4 pt-1 text-sm text-text-primary-light dark:text-text-primary-dark">
          {children}
        </div>
      )}
    </div>
  );
}

interface AccordionProps {
  children: ReactNode;
  className?: string;
}

export function Accordion({ children, className }: AccordionProps) {
  return <div className={clsx("space-y-2", className)}>{children}</div>;
}
