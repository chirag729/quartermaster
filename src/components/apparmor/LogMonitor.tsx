import type { DenialEvent } from "../../types/apparmor";
import { Card, CardHeader, CardTitle } from "../ui/Card";
import { Radio } from "lucide-react";

interface Props {
  denials: DenialEvent[];
}

export function LogMonitor({ denials }: Props) {
  const recentDenials = denials.slice(0, 20);

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-2">
          <Radio size={14} className="text-red-500 animate-pulse" />
          <CardTitle>Live Monitor</CardTitle>
        </div>
      </CardHeader>
      <div className="max-h-64 overflow-y-auto space-y-1 font-mono text-[13px] leading-relaxed">
        {recentDenials.length === 0 ? (
          <p className="text-text-secondary-light dark:text-text-secondary-dark py-4 text-center text-sm font-sans">
            Waiting for denial events...
          </p>
        ) : (
          recentDenials.map((d) => (
            <div key={d.id} className="py-1 px-2 rounded hover:bg-warm-50/50 dark:hover:bg-warm-900/10">
              <span className="text-text-secondary-light dark:text-text-secondary-dark">
                {new Date(d.timestamp).toLocaleTimeString()}
              </span>{" "}
              <span className="text-warm-600 dark:text-warm-300">{d.profile}</span>{" "}
              <span className="text-text-primary-light dark:text-text-primary-dark">{d.operation}</span>{" "}
              <span className="text-red-600 dark:text-red-400">{d.denied_mask}</span>{" "}
              <span className="text-text-secondary-light dark:text-text-secondary-dark truncate">{d.name}</span>
            </div>
          ))
        )}
      </div>
    </Card>
  );
}
