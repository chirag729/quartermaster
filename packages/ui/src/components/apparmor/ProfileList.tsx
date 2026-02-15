import type { ProfileInfo } from "@quartermaster/core";
import { Badge } from "../ui/Badge";
import { Button } from "../ui/Button";
import { Layers } from "lucide-react";

interface Props {
  profiles: ProfileInfo[];
  onConsolidate?: (profileName: string) => void;
}

const modeVariant = {
  enforce: "success" as const,
  complain: "warning" as const,
  unconfined: "danger" as const,
};

export function ProfileList({ profiles, onConsolidate }: Props) {
  if (profiles.length === 0) {
    return (
      <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark py-4 text-center">
        No profiles found
      </p>
    );
  }

  return (
    <div className="space-y-2">
      {profiles.map((p) => (
        <div
          key={p.name}
          className="flex items-center justify-between py-2 px-3 rounded-lg hover:bg-warm-100/60 dark:hover:bg-warm-900/20 transition-colors duration-150"
        >
          <div>
            <p className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark font-mono">{p.name}</p>
            {p.pid_count > 0 && (
              <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark">{p.pid_count} process(es)</p>
            )}
          </div>
          <div className="flex items-center gap-2">
            {onConsolidate && (
              <Button
                variant="ghost"
                size="sm"
                onClick={() => onConsolidate(p.name)}
                title="Consolidate existing rules"
              >
                <Layers size={14} />
                Consolidate
              </Button>
            )}
            <Badge variant={modeVariant[p.mode]}>{p.mode}</Badge>
          </div>
        </div>
      ))}
    </div>
  );
}
