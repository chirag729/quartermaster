import { DashboardGrid } from "../components/dashboard/DashboardGrid";
import { useTasks } from "../hooks/useTasks";
import { RefreshCw } from "lucide-react";
import { Button } from "../components/ui/Button";
import { ToastContainer } from "../components/ui/Toast";

export function DashboardPage() {
  const { tasks, loading, executing, executeTask, refreshTasks } = useTasks();

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-xl font-semibold text-text-primary-light dark:text-text-primary-dark">
            Task Library
          </h1>
          <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark mt-1">
            Configure and set up your development environment
          </p>
        </div>
        <Button variant="ghost" size="sm" onClick={refreshTasks} disabled={loading}>
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
        <DashboardGrid tasks={tasks} executing={executing} onExecute={executeTask} />
      )}
      <ToastContainer />
    </div>
  );
}
