export interface BlueprintTaskEntry {
  task_id: string;
  enabled: boolean;
  config_overrides: Record<string, unknown>;
  order: number;
}

export interface Blueprint {
  id: string;
  name: string;
  description: string;
  icon: string;
  is_builtin: boolean;
  version: string;
  extends?: string | null;
  task_entries: BlueprintTaskEntry[];
  created_at: string;
  updated_at: string;
}
