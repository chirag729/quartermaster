import type { PermissionSuggestion as PermissionSuggestionType } from "@quartermaster/core";
import { Badge } from "../ui/Badge";
import clsx from "clsx";

interface Props {
  suggestion: PermissionSuggestionType;
  selected: boolean;
  onToggle: () => void;
}

const riskVariant = {
  low: "success" as const,
  medium: "warning" as const,
  high: "danger" as const,
  critical: "danger" as const,
};

export function PermissionSuggestionCard({ suggestion, selected, onToggle }: Props) {
  return (
    <div
      onClick={onToggle}
      className={clsx(
        "flex items-start gap-3 p-3 rounded-lg border cursor-pointer transition-colors",
        selected
          ? "border-warm-400 bg-warm-100/20 dark:bg-warm-900/20"
          : "border-border-light dark:border-border-dark hover:border-warm-300 dark:hover:border-warm-700",
      )}
    >
      <label className="relative mt-0.5 flex items-center">
        <input
          type="checkbox"
          checked={selected}
          onChange={onToggle}
          className="sr-only"
        />
        <div className={clsx(
          "h-4 w-4 rounded border flex items-center justify-center transition-colors",
          selected
            ? "bg-warm-400 border-warm-400"
            : "border-border-light dark:border-border-dark bg-card-light dark:bg-card-dark",
        )}>
          {selected && (
            <svg className="h-3 w-3 text-white" viewBox="0 0 12 12" fill="none">
              <path d="M2.5 6l2.5 2.5 4.5-5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          )}
        </div>
      </label>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-1">
          <span className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark">{suggestion.description}</span>
          <Badge variant={riskVariant[suggestion.risk_level]}>{suggestion.risk_level}</Badge>
        </div>
        <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark mb-2">{suggestion.explanation}</p>
        <pre className="text-xs font-mono bg-surface-light dark:bg-surface-dark p-2 rounded-md overflow-x-auto">{suggestion.rule_text}</pre>
      </div>
    </div>
  );
}
