import { Sun, Moon, Monitor } from "lucide-react";
import { useThemeStore } from "../../stores/themeStore";
import clsx from "clsx";

const themeOptions = [
  { value: "light" as const, icon: Sun, label: "Light" },
  { value: "dark" as const, icon: Moon, label: "Dark" },
  { value: "system" as const, icon: Monitor, label: "System" },
];

export function TopBar() {
  const { theme, setTheme } = useThemeStore();

  return (
    <header className="h-14 border-b border-border-light dark:border-border-dark bg-card-light dark:bg-card-dark flex items-center justify-between px-6">
      <div />
      <div className="flex items-center gap-1 bg-surface-light dark:bg-surface-dark rounded-lg p-1">
        {themeOptions.map(({ value, icon: Icon, label }) => (
          <button
            key={value}
            onClick={() => setTheme(value)}
            title={label}
            aria-label={`Switch to ${label} theme`}
            aria-pressed={theme === value}
            className={clsx(
              "p-1.5 rounded-md transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-warm-300/50",
              theme === value
                ? "bg-card-light dark:bg-card-dark shadow-sm text-warm-600 dark:text-warm-300"
                : "text-text-secondary-light dark:text-text-secondary-dark hover:text-text-primary-light dark:hover:text-text-primary-dark",
            )}
          >
            <Icon size={16} />
          </button>
        ))}
      </div>
    </header>
  );
}
