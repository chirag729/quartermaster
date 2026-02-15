import type { DenialEvent } from "@quartermaster/core";

interface Props {
  denials: DenialEvent[];
}

export function DenialLogTable({ denials }: Props) {
  if (denials.length === 0) {
    return (
      <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark py-4 text-center">
        No denial events found
      </p>
    );
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-border-light dark:border-border-dark bg-surface-light dark:bg-surface-dark">
            <th className="text-left py-2 px-3 text-xs font-medium text-text-secondary-light dark:text-text-secondary-dark uppercase tracking-wider">Time</th>
            <th className="text-left py-2 px-3 text-xs font-medium text-text-secondary-light dark:text-text-secondary-dark uppercase tracking-wider">Profile</th>
            <th className="text-left py-2 px-3 text-xs font-medium text-text-secondary-light dark:text-text-secondary-dark uppercase tracking-wider">Operation</th>
            <th className="text-left py-2 px-3 text-xs font-medium text-text-secondary-light dark:text-text-secondary-dark uppercase tracking-wider">Resource</th>
            <th className="text-left py-2 px-3 text-xs font-medium text-text-secondary-light dark:text-text-secondary-dark uppercase tracking-wider">Denied</th>
          </tr>
        </thead>
        <tbody>
          {denials.map((d) => (
            <tr key={d.id} className="border-b border-border-light/50 dark:border-border-dark/50 hover:bg-warm-50/50 dark:hover:bg-warm-900/10">
              <td className="py-2 px-3 font-mono text-xs whitespace-nowrap">{new Date(d.timestamp).toLocaleTimeString()}</td>
              <td className="py-2 px-3 font-mono text-xs">{d.profile}</td>
              <td className="py-2 px-3 text-xs">{d.operation}</td>
              <td className="py-2 px-3 font-mono text-xs truncate max-w-xs" title={d.name}>{d.name}</td>
              <td className="py-2 px-3 font-mono text-xs text-red-600 dark:text-red-400">{d.denied_mask}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
