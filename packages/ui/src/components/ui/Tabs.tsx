import clsx from "clsx";

interface Tab {
  id: string;
  label: string;
}

interface TabsProps {
  tabs: Tab[];
  activeTab: string;
  onTabChange: (id: string) => void;
  className?: string;
}

export function Tabs({ tabs, activeTab, onTabChange, className }: TabsProps) {
  return (
    <div
      className={clsx("flex gap-1 border-b border-border-light dark:border-border-dark", className)}
      role="tablist"
    >
      {tabs.map((tab) => (
        <button
          key={tab.id}
          role="tab"
          aria-selected={activeTab === tab.id}
          onClick={() => onTabChange(tab.id)}
          className={clsx(
            "px-4 py-2 text-sm font-medium transition-colors -mb-px border-b-2",
            activeTab === tab.id
              ? "border-warm-500 text-warm-700 dark:text-warm-300"
              : "border-transparent text-text-secondary-light dark:text-text-secondary-dark hover:text-text-primary-light dark:hover:text-text-primary-dark hover:border-warm-300/50",
          )}
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
}
