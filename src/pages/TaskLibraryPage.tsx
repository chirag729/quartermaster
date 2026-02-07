import { useEffect, useMemo, useState } from "react";
import { DashboardGrid } from "../components/dashboard/DashboardGrid";
import { RefreshCw } from "lucide-react";
import { Button } from "../components/ui/Button";
import { SearchInput } from "../components/ui/SearchInput";
import { Badge } from "../components/ui/Badge";
import { useToastStore } from "../stores/toastStore";
import * as api from "../services/tauriCommands";
import { formatError } from "../lib/formatError";
import { useDebounce } from "../hooks/useDebounce";
import type { TaskInfo } from "../types/task";

const ALL_CATEGORY = "All";

export function TaskLibraryPage() {
  const [tasks, setTasks] = useState<TaskInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchQuery, setSearchQuery] = useState("");
  const debouncedSearch = useDebounce(searchQuery, 200);
  const [activeCategory, setActiveCategory] = useState<string>(ALL_CATEGORY);
  const { addToast } = useToastStore();

  const loadTasks = async () => {
    setLoading(true);
    try {
      const result = await api.listTasks();
      setTasks(result);
    } catch (err) {
      addToast({ type: "error", title: "Failed to load tasks", message: formatError(err) });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadTasks();
  }, []);

  const categories = useMemo(
    () => [ALL_CATEGORY, ...Array.from(new Set(tasks.map((t) => t.category))).sort()],
    [tasks],
  );

  const filteredTasks = useMemo(() => {
    const query = debouncedSearch.toLowerCase().trim();

    return tasks.filter((task) => {
      // Category filter
      if (activeCategory !== ALL_CATEGORY && task.category !== activeCategory) {
        return false;
      }

      // Search filter — match against name, description, or tags (debounced)
      if (query) {
        const matchesName = task.name.toLowerCase().includes(query);
        const matchesDescription = task.description.toLowerCase().includes(query);
        const matchesTags = task.tags.some((tag) => tag.toLowerCase().includes(query));
        if (!matchesName && !matchesDescription && !matchesTags) {
          return false;
        }
      }

      return true;
    });
  }, [tasks, debouncedSearch, activeCategory]);

  const handleCategoryClick = (category: string) => {
    setActiveCategory(category);
  };

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

      {!loading && (
        <div className="space-y-4 mb-6">
          <SearchInput
            placeholder="Search tasks by name, description, or tags..."
            onValueChange={setSearchQuery}
            className="max-w-md"
          />

          <div className="flex flex-wrap items-center gap-2">
            {categories.map((category) => {
              const isActive = category === activeCategory;
              return (
                <Badge
                  key={category}
                  variant="default"
                  className={
                    isActive
                      ? "bg-warm-500 text-white dark:bg-warm-500 dark:text-white cursor-pointer select-none"
                      : "cursor-pointer select-none hover:bg-warm-200 dark:hover:bg-warm-800/40"
                  }
                  onClick={() => handleCategoryClick(category)}
                >
                  {category}
                </Badge>
              );
            })}
          </div>

          <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark">
            Showing {filteredTasks.length} of {tasks.length} tasks
          </p>
        </div>
      )}

      {loading ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {Array.from({ length: 6 }).map((_, i) => (
            <div key={i} className="h-48 rounded-xl bg-card-light dark:bg-card-dark border border-border-light dark:border-border-dark animate-pulse" />
          ))}
        </div>
      ) : filteredTasks.length === 0 ? (
        <div className="text-center py-16">
          <p className="text-text-secondary-light dark:text-text-secondary-dark">
            No tasks match your filters.
          </p>
        </div>
      ) : (
        <DashboardGrid tasks={filteredTasks} executing={null} onExecute={() => {}} readOnly />
      )}
    </div>
  );
}
