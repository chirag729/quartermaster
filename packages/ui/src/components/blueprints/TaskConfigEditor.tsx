import { useState, useEffect } from "react";
import { Settings, ChevronDown, ChevronRight } from "lucide-react";
import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import { Button } from "../ui/Button";
import type { ConfigField } from "@quartermaster/core";
import type { BlueprintTaskEntry } from "@quartermaster/core";

interface TaskConfigEditorProps {
  taskId: string;
  taskName: string;
  configSchema: ConfigField[];
  entry: BlueprintTaskEntry;
  onSave: (taskId: string, overrides: Record<string, unknown>) => void;
  disabled?: boolean;
}

export function TaskConfigEditor({
  taskId,
  taskName,
  configSchema,
  entry,
  onSave,
  disabled,
}: TaskConfigEditorProps) {
  const [expanded, setExpanded] = useState(false);
  const [values, setValues] = useState<Record<string, string>>({});

  useEffect(() => {
    const initial: Record<string, string> = {};
    for (const field of configSchema) {
      const override = entry.config_overrides[field.key];
      if (override !== undefined) {
        initial[field.key] = String(override);
      } else {
        initial[field.key] = field.default_value;
      }
    }
    setValues(initial);
  }, [configSchema, entry.config_overrides]);

  if (configSchema.length === 0) return null;

  const hasChanges = configSchema.some((field) => {
    const current = values[field.key] ?? field.default_value;
    const saved = entry.config_overrides[field.key];
    if (saved !== undefined) {
      return current !== String(saved);
    }
    return current !== field.default_value;
  });

  const handleSave = () => {
    const overrides: Record<string, unknown> = {};
    for (const field of configSchema) {
      const val = values[field.key];
      if (val !== undefined && val !== field.default_value) {
        overrides[field.key] = val;
      }
    }
    onSave(taskId, overrides);
  };

  return (
    <div className="border border-border-light dark:border-border-dark rounded-lg overflow-hidden">
      <button
        type="button"
        onClick={() => setExpanded(!expanded)}
        className="w-full flex items-center gap-2 p-3 text-left hover:bg-warm-50 dark:hover:bg-warm-900/10 transition-colors"
      >
        {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        <Settings size={14} className="text-text-secondary-light dark:text-text-secondary-dark" />
        <span className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark flex-1">
          {taskName} Configuration
        </span>
        {Object.keys(entry.config_overrides).length > 0 && (
          <span className="text-xs text-warm-500 dark:text-warm-300">
            {Object.keys(entry.config_overrides).length} override{Object.keys(entry.config_overrides).length !== 1 ? "s" : ""}
          </span>
        )}
      </button>
      {expanded && (
        <div className="p-3 border-t border-border-light dark:border-border-dark space-y-3">
          {configSchema.map((field) => {
            if (field.field_type === "select" && field.options) {
              return (
                <Select
                  key={field.key}
                  label={field.label}
                  value={values[field.key] ?? field.default_value}
                  onChange={(e) => setValues({ ...values, [field.key]: e.target.value })}
                  options={field.options.map((o) => ({ value: o, label: o }))}
                  disabled={disabled}
                />
              );
            }
            return (
              <Input
                key={field.key}
                label={field.label}
                value={values[field.key] ?? field.default_value}
                onChange={(e) => setValues({ ...values, [field.key]: e.target.value })}
                placeholder={field.default_value}
                disabled={disabled}
              />
            );
          })}
          {hasChanges && !disabled && (
            <div className="flex justify-end pt-1">
              <Button size="sm" onClick={handleSave}>
                Save
              </Button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
