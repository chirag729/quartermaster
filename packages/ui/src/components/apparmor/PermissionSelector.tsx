import type { PermissionSuggestion } from "@quartermaster/core";
import { PermissionSuggestionCard } from "./PermissionSuggestion";

interface Props {
  suggestions: PermissionSuggestion[];
  selected: Set<string>;
  onToggle: (id: string) => void;
}

export function PermissionSelector({ suggestions, selected, onToggle }: Props) {
  const grouped = suggestions.reduce((acc, s) => {
    const group = acc.get(s.profile) || [];
    group.push(s);
    acc.set(s.profile, group);
    return acc;
  }, new Map<string, PermissionSuggestion[]>());

  return (
    <div className="space-y-4">
      {Array.from(grouped).map(([profile, items]) => (
        <div key={profile}>
          <h4 className="text-xs font-semibold text-text-secondary-light dark:text-text-secondary-dark uppercase tracking-wider mb-2 font-mono">
            {profile}
          </h4>
          <div className="space-y-2">
            {items.map((s) => (
              <PermissionSuggestionCard
                key={s.id}
                suggestion={s}
                selected={selected.has(s.id)}
                onToggle={() => onToggle(s.id)}
              />
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
