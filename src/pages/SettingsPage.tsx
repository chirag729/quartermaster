import { Card, CardHeader, CardTitle, CardDescription } from "../components/ui/Card";
import { useThemeStore } from "../stores/themeStore";
import { Sun, Moon, Monitor } from "lucide-react";
import clsx from "clsx";
import { ToastContainer } from "../components/ui/Toast";

const themeOptions = [
  { value: "light" as const, icon: Sun, label: "Light", description: "Always use light mode" },
  { value: "dark" as const, icon: Moon, label: "Dark", description: "Always use dark mode" },
  { value: "system" as const, icon: Monitor, label: "System", description: "Follow system preference" },
];

export function SettingsPage() {
  const { theme, setTheme } = useThemeStore();

  return (
    <div className="max-w-2xl">
      <div className="mb-6">
        <h1 className="text-xl font-semibold text-text-primary-light dark:text-text-primary-dark">
          Settings
        </h1>
        <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark mt-1">
          Configure application preferences
        </p>
      </div>
      <Card>
        <CardHeader>
          <CardTitle>Appearance</CardTitle>
          <CardDescription>Choose your preferred theme</CardDescription>
        </CardHeader>
        <div className="grid grid-cols-3 gap-3">
          {themeOptions.map(({ value, icon: Icon, label, description }) => (
            <button
              key={value}
              onClick={() => setTheme(value)}
              className={clsx(
                "flex flex-col items-center gap-2 p-4 rounded-lg border transition-colors text-center",
                theme === value
                  ? "border-warm-400 bg-warm-100/40 dark:bg-warm-900/30 shadow-sm ring-1 ring-warm-400/30"
                  : "border-border-light dark:border-border-dark hover:border-warm-300 dark:hover:border-warm-700",
              )}
            >
              <Icon size={20} className={theme === value ? "text-warm-500" : "text-text-secondary-light dark:text-text-secondary-dark"} />
              <span className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark">{label}</span>
              <span className="text-xs text-text-secondary-light dark:text-text-secondary-dark">{description}</span>
            </button>
          ))}
        </div>
      </Card>
      <ToastContainer />
    </div>
  );
}
