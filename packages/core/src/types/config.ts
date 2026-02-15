export interface AppConfig {
  theme: "light" | "dark" | "system";
  task_configs: Record<string, Record<string, unknown>>;
  completed_tasks: string[];
}
