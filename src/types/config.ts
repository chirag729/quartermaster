export interface AppConfig {
  theme: "light" | "dark" | "system";
  task_configs: Record<string, Record<string, string>>;
  completed_tasks: string[];
}
