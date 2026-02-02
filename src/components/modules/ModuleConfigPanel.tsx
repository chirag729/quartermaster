import { useState, useEffect } from "react";
import type { ConfigField, TaskInfo } from "../../types/task";
import { Button } from "../ui/Button";
import { Toggle } from "../ui/Toggle";
import { Card, CardHeader, CardTitle } from "../ui/Card";

interface Props {
  task: TaskInfo;
  config: Record<string, string>;
  onSave: (config: Record<string, string>) => void;
  onCancel: () => void;
}

function FieldInput({
  field,
  value,
  onChange,
}: {
  field: ConfigField;
  value: string;
  onChange: (val: string) => void;
}) {
  switch (field.field_type) {
    case "toggle":
      return (
        <Toggle
          checked={value === "true"}
          onChange={(checked) => onChange(String(checked))}
          label={field.label}
        />
      );
    case "select":
      return (
        <select
          value={value}
          onChange={(e) => onChange(e.target.value)}
          className="w-full px-3 py-2 rounded-lg border border-border-light dark:border-border-dark bg-card-light dark:bg-card-dark text-text-primary-light dark:text-text-primary-dark text-sm focus:outline-none focus:ring-2 focus:ring-warm-300/50"
        >
          {field.options?.map((opt) => (
            <option key={opt} value={opt}>
              {opt}
            </option>
          ))}
        </select>
      );
    case "path":
    case "text":
    default:
      return (
        <input
          type="text"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={field.default_value}
          className="w-full px-3 py-2 rounded-lg border border-border-light dark:border-border-dark bg-card-light dark:bg-card-dark text-text-primary-light dark:text-text-primary-dark text-sm focus:outline-none focus:ring-2 focus:ring-warm-300/50 placeholder:text-text-secondary-light/50 dark:placeholder:text-text-secondary-dark/50"
        />
      );
  }
}

export function ModuleConfigPanel({ task, config, onSave, onCancel }: Props) {
  const [values, setValues] = useState<Record<string, string>>({});

  useEffect(() => {
    const initial: Record<string, string> = {};
    for (const field of task.config_schema) {
      initial[field.key] = config[field.key] ?? field.default_value;
    }
    setValues(initial);
  }, [task.id, config]);

  const handleChange = (key: string, val: string) => {
    setValues((prev) => ({ ...prev, [key]: val }));
  };

  if (task.config_schema.length === 0) return null;

  return (
    <Card>
      <CardHeader>
        <CardTitle>Configure {task.name}</CardTitle>
      </CardHeader>
      <div className="space-y-4">
        {task.config_schema.map((field) => (
          <div key={field.key}>
            {field.field_type !== "toggle" && (
              <label className="block text-sm font-medium text-text-primary-light dark:text-text-primary-dark mb-1.5">
                {field.label}
                {field.required && <span className="text-red-500 ml-0.5">*</span>}
              </label>
            )}
            <FieldInput
              field={field}
              value={values[field.key] ?? ""}
              onChange={(val) => handleChange(field.key, val)}
            />
          </div>
        ))}
        <div className="flex justify-end gap-2 pt-2">
          <Button variant="ghost" size="sm" onClick={onCancel}>
            Cancel
          </Button>
          <Button size="sm" onClick={() => onSave(values)}>
            Save & Run
          </Button>
        </div>
      </div>
    </Card>
  );
}
