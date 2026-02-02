import { useEffect, useState } from "react";
import { DashboardGrid } from "../components/dashboard/DashboardGrid";
import { RefreshCw } from "lucide-react";
import { Button } from "../components/ui/Button";
import { ToastContainer } from "../components/ui/Toast";
import { useToastStore } from "../stores/toastStore";
import * as api from "../services/tauriCommands";
import type { TaskInfo } from "../types/task";

export function TaskLibraryPage() {
  const [tasks, setTasks] = useState<TaskInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const { addToast } = useToastStore();

  const loadTasks = async () => {
    setLoading(true);
    try {
      const result = await api.listTasks();
      setTasks(result);
    } catch (err) {
      addToast({ type: "error", title: "Failed to load tasks", message: String(err) });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadTasks();
  }, []);

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-xl font-semibold text-text-primary-light dark:text-text-primary-dark">
            Task Library
          </h1>
          <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark mt-1">
            Browse available setup tasks
          </p>
        </div>
        <Button variant="ghost" size="sm" onClick={loadTasks} disabled={loading}>
          <RefreshCw size={14} className={loading ? "animate-spin" : ""} />
          Refresh
        </Button>
      </div>
      {loading ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {Array.from({ length: 6 }).map((_, i) => (
            <div key={i} className="h-48 rounded-xl bg-card-light dark:bg-card-dark border border-border-light dark:border-border-dark animate-pulse" />
          ))}
        </div>
      ) : (
        <DashboardGrid tasks={tasks} executing={null} onExecute={() => {}} readOnly />
      )}
      <ToastContainer />
    </div>
  );
}
