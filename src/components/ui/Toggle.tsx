import clsx from "clsx";

interface ToggleProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: string;
  disabled?: boolean;
}

export function Toggle({ checked, onChange, label, disabled }: ToggleProps) {
  return (
    <label className={clsx("inline-flex items-center gap-2 cursor-pointer", disabled && "opacity-50 cursor-not-allowed")}>
      <button
        role="switch"
        aria-checked={checked}
        disabled={disabled}
        onClick={() => onChange(!checked)}
        className={clsx(
          "relative inline-flex h-5 w-9 items-center shrink-0 rounded-full transition-colors duration-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-warm-300/50 focus-visible:ring-offset-2",
          checked ? "bg-warm-400" : "bg-border-light dark:bg-border-dark",
        )}
      >
        <span
          className={clsx(
            "pointer-events-none inline-block h-3.5 w-3.5 rounded-full bg-white shadow-sm transition-transform duration-200",
            checked ? "translate-x-[18px]" : "translate-x-[3px]",
          )}
        />
      </button>
      {label && <span className="text-sm text-text-primary-light dark:text-text-primary-dark">{label}</span>}
    </label>
  );
}
