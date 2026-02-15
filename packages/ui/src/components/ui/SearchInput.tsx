import { forwardRef, type InputHTMLAttributes } from "react";
import { Search } from "lucide-react";
import clsx from "clsx";

interface SearchInputProps extends InputHTMLAttributes<HTMLInputElement> {
  onValueChange?: (value: string) => void;
}

export const SearchInput = forwardRef<HTMLInputElement, SearchInputProps>(
  ({ className, onValueChange, onChange, ...props }, ref) => {
    return (
      <div className={clsx("relative", className)}>
        <Search
          size={16}
          className="absolute left-3 top-1/2 -translate-y-1/2 text-text-secondary-light/50 dark:text-text-secondary-dark/50 pointer-events-none"
        />
        <input
          ref={ref}
          type="search"
          className="w-full rounded-lg border border-border-light dark:border-border-dark bg-surface-light dark:bg-surface-dark pl-9 pr-3 py-2 text-sm text-text-primary-light dark:text-text-primary-dark placeholder:text-text-secondary-light/50 dark:placeholder:text-text-secondary-dark/50 transition-colors focus:outline-none focus:ring-2 focus:ring-warm-300/50 focus:border-warm-400"
          onChange={(e) => {
            onChange?.(e);
            onValueChange?.(e.target.value);
          }}
          {...props}
        />
      </div>
    );
  },
);
SearchInput.displayName = "SearchInput";
