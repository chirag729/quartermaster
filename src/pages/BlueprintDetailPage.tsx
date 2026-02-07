import { useEffect, useState, useCallback } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { ArrowLeft, Layers, ListChecks, Shield, Settings, Server, Monitor, Copy, Trash2, Download, GitBranch, Plus, X, type LucideIcon } from "lucide-react";
import { useToastStore } from "../stores/toastStore";
import { formatError } from "../lib/formatError";
import { useBlueprintStore } from "../stores/blueprintStore";
import { Badge } from "../components/ui/Badge";
import { Button } from "../components/ui/Button";
import { Card, CardHeader, CardTitle, CardDescription } from "../components/ui/Card";
import { Skeleton } from "../components/ui/Skeleton";
import { Toggle } from "../components/ui/Toggle";
import { TaskConfigEditor } from "../components/blueprints/TaskConfigEditor";
import { AddTaskDialog } from "../components/blueprints/AddTaskDialog";
import * as api from "../services/tauriCommands";
import type { Blueprint } from "../types/blueprint";
import type { TaskInfo } from "../types/task";
import * as TaskIcons from "lucide-react";

const iconMap: Record<string, LucideIcon> = {
  Layers,
  ListChecks,
  Shield,
  Settings,
  Server,
  Monitor,
};

function getTaskIcon(iconName: string) {
  const icons = TaskIcons as unknown as Record<string, LucideIcon>;
  return icons[iconName] || TaskIcons.Box;
}

function formatDate(dateStr: string) {
  return new Date(dateStr).toLocaleDateString("en-US", {
    year: "numeric",
    month: "long",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function BlueprintDetailPage() {
  const { blueprintId } = useParams<{ blueprintId: string }>();
  const navigate = useNavigate();
  const { addToast } = useToastStore();
  const { cloneBlueprint, deleteBlueprint, exportBlueprint } = useBlueprintStore();
  const [blueprint, setBlueprint] = useState<Blueprint | null>(null);
  const [loading, setLoading] = useState(true);
  const [allTasks, setAllTasks] = useState<TaskInfo[]>([]);
  const [taskInfoMap, setTaskInfoMap] = useState<Record<string, TaskInfo>>({});
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [parentBlueprintName, setParentBlueprintName] = useState<string | null>(null);
  const [showAddTask, setShowAddTask] = useState(false);

  const loadBlueprint = useCallback(async () => {
    if (!blueprintId) return;
    setLoading(true);
    try {
      const bp = await api.getBlueprint(blueprintId);
      setBlueprint(bp);

      // Fetch parent blueprint name if this blueprint extends another
      if (bp.extends) {
        try {
          const parent = await api.getBlueprint(bp.extends);
          setParentBlueprintName(parent.name);
        } catch {
          setParentBlueprintName(bp.extends);
        }
      } else {
        setParentBlueprintName(null);
      }
    } catch (err) {
      addToast({ type: "error", title: "Failed to load blueprint", message: formatError(err) });
    } finally {
      setLoading(false);
    }
  }, [blueprintId, addToast]);

  useEffect(() => {
    loadBlueprint();
  }, [loadBlueprint]);

  useEffect(() => {
    api.listTasks().then((tasks) => {
      setAllTasks(tasks);
      const map: Record<string, TaskInfo> = {};
      for (const t of tasks) map[t.id] = t;
      setTaskInfoMap(map);
    }).catch(() => {});
  }, []);

  const handleClone = async () => {
    if (!blueprint) return;
    const newName = prompt("Name for the cloned blueprint:", `${blueprint.name} (Copy)`);
    if (!newName) return;
    try {
      const cloned = await cloneBlueprint(blueprint.id, newName);
      addToast({ type: "success", title: "Blueprint cloned", message: `Created "${cloned.name}"` });
      navigate(`/blueprints/${cloned.id}`);
    } catch (err) {
      addToast({ type: "error", title: "Clone failed", message: formatError(err) });
    }
  };

  const handleDelete = async () => {
    if (!blueprint) return;
    try {
      await deleteBlueprint(blueprint.id);
      addToast({ type: "success", title: "Blueprint deleted", message: `"${blueprint.name}" has been deleted.` });
      navigate("/blueprints");
    } catch (err) {
      addToast({ type: "error", title: "Delete failed", message: formatError(err) });
    }
  };

  const handleExport = async () => {
    if (!blueprint) return;
    try {
      const homeDir = "~";
      const outputPath = await exportBlueprint(blueprint.id, homeDir);
      addToast({ type: "success", title: "Blueprint exported", message: `Exported to ${outputPath}` });
    } catch (err) {
      addToast({ type: "error", title: "Export failed", message: formatError(err) });
    }
  };

  const handleToggleTask = async (taskId: string, currentEnabled: boolean) => {
    if (!blueprint) return;
    const updated = { ...blueprint };
    updated.task_entries = updated.task_entries.map((e) =>
      e.task_id === taskId ? { ...e, enabled: !currentEnabled } : e,
    );
    try {
      const result = await api.updateBlueprint(updated);
      setBlueprint(result);
    } catch (err) {
      addToast({ type: "error", title: "Toggle failed", message: formatError(err) });
    }
  };

  const handleAddTask = async (taskId: string) => {
    if (!blueprint) return;
    const maxOrder = blueprint.task_entries.reduce((max, e) => Math.max(max, e.order), 0);
    const updated = { ...blueprint };
    updated.task_entries = [
      ...updated.task_entries,
      { task_id: taskId, enabled: true, config_overrides: {}, order: maxOrder + 1 },
    ];
    try {
      const result = await api.updateBlueprint(updated);
      setBlueprint(result);
      const taskName = taskInfoMap[taskId]?.name ?? taskId;
      addToast({ type: "success", title: "Task added", message: `Added "${taskName}" to blueprint` });
    } catch (err) {
      addToast({ type: "error", title: "Add failed", message: formatError(err) });
    }
  };

  const handleRemoveTask = async (taskId: string) => {
    if (!blueprint) return;
    const updated = { ...blueprint };
    updated.task_entries = updated.task_entries.filter((e) => e.task_id !== taskId);
    try {
      const result = await api.updateBlueprint(updated);
      setBlueprint(result);
      const taskName = taskInfoMap[taskId]?.name ?? taskId;
      addToast({ type: "success", title: "Task removed", message: `Removed "${taskName}" from blueprint` });
    } catch (err) {
      addToast({ type: "error", title: "Remove failed", message: formatError(err) });
    }
  };

  const handleConfigSave = async (taskId: string, overrides: Record<string, unknown>) => {
    if (!blueprint) return;
    const updated = { ...blueprint };
    updated.task_entries = updated.task_entries.map((e) =>
      e.task_id === taskId ? { ...e, config_overrides: overrides } : e,
    );

    // Path cascading: if this is the SDK folder task and path changed,
    // propagate to flutter_sdk and android_sdk as sdk_base_path
    if (taskId === "create-sdk-folder" && overrides.path) {
      const sdkPath = String(overrides.path);
      updated.task_entries = updated.task_entries.map((e) => {
        if (e.task_id === "flutter-sdk" || e.task_id === "android-sdk") {
          return {
            ...e,
            config_overrides: { ...e.config_overrides, sdk_base_path: sdkPath },
          };
        }
        return e;
      });
    }

    try {
      const result = await api.updateBlueprint(updated);
      setBlueprint(result);
      addToast({ type: "success", title: "Configuration saved" });
    } catch (err) {
      addToast({ type: "error", title: "Save failed", message: formatError(err) });
    }
  };

  if (loading) {
    return (
      <div className="space-y-4">
        <Skeleton className="h-8 w-48" rounded="lg" />
        <Skeleton className="h-64 w-full" rounded="xl" />
      </div>
    );
  }

  if (!blueprint) {
    return (
      <div>
        <Button variant="ghost" size="sm" onClick={() => navigate("/blueprints")}>
          <ArrowLeft size={14} />
          Back to Blueprints
        </Button>
        <p className="mt-4 text-sm text-text-secondary-light dark:text-text-secondary-dark">
          Blueprint not found.
        </p>
      </div>
    );
  }

  const Icon = iconMap[blueprint.icon] || Layers;
  const sortedEntries = [...blueprint.task_entries].sort((a, b) => a.order - b.order);

  // Get SDK folder path for cascading
  const sdkFolderEntry = blueprint.task_entries.find((e) => e.task_id === "create-sdk-folder");
  const sdkBasePath = sdkFolderEntry?.config_overrides?.path
    ? String(sdkFolderEntry.config_overrides.path)
    : undefined;

  const existingTaskIds = blueprint.task_entries.map((e) => e.task_id);

  return (
    <div>
      <Button variant="ghost" size="sm" onClick={() => navigate("/blueprints")} className="mb-4">
        <ArrowLeft size={14} />
        Back to Blueprints
      </Button>
      <div className="flex items-center gap-3 mb-6">
        <div className="p-2.5 rounded-lg bg-warm-100 dark:bg-warm-900/30 text-warm-600 dark:text-warm-400">
          <Icon size={20} />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <h1 className="text-xl font-semibold text-text-primary-light dark:text-text-primary-dark">
              {blueprint.name}
            </h1>
            {blueprint.is_builtin && <Badge variant="info">Built-in</Badge>}
            {blueprint.extends && parentBlueprintName && (
              <Badge>
                <GitBranch size={12} className="inline mr-1" />
                Extends: {parentBlueprintName}
              </Badge>
            )}
          </div>
          <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark mt-0.5">
            {blueprint.description}
          </p>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        <Card className="lg:col-span-2">
          <CardHeader>
            <div className="flex items-center justify-between">
              <div>
                <CardTitle>Task Entries</CardTitle>
                <CardDescription>
                  {sortedEntries.length} {sortedEntries.length === 1 ? "task" : "tasks"} in this blueprint
                </CardDescription>
              </div>
              <Button size="sm" onClick={() => setShowAddTask(true)}>
                <Plus size={14} />
                Add Task
              </Button>
            </div>
          </CardHeader>
          {sortedEntries.length === 0 ? (
            <div className="text-center py-8">
              <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark mb-3">
                No tasks configured in this blueprint.
              </p>
              <Button size="sm" variant="secondary" onClick={() => setShowAddTask(true)}>
                <Plus size={14} />
                Add your first task
              </Button>
            </div>
          ) : (
            <div className="space-y-2">
              {sortedEntries.map((entry, idx) => {
                const taskInfo = taskInfoMap[entry.task_id];
                const TaskIcon = taskInfo ? getTaskIcon(taskInfo.icon) : TaskIcons.Box;
                return (
                  <div key={entry.task_id} className="space-y-0">
                    <div className="flex items-center justify-between p-3 rounded-lg border border-border-light dark:border-border-dark bg-surface-light dark:bg-surface-dark">
                      <div className="flex items-center gap-3 min-w-0">
                        <span className="text-xs font-mono text-text-secondary-light dark:text-text-secondary-dark w-6 text-center shrink-0">
                          {idx + 1}
                        </span>
                        <div className="p-1.5 rounded-md bg-warm-100 dark:bg-warm-900/30">
                          <TaskIcon size={14} className="text-warm-500 dark:text-warm-300" />
                        </div>
                        <div className="min-w-0">
                          <p className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark truncate">
                            {taskInfo?.name || entry.task_id}
                          </p>
                          <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark truncate">
                            {taskInfo?.description || "Unknown task"}
                          </p>
                        </div>
                      </div>
                      <div className="flex items-center gap-2 shrink-0">
                        {taskInfo?.category && <Badge>{taskInfo.category}</Badge>}
                        {taskInfo?.privilege_level === "admin" && <Badge variant="warning">Admin</Badge>}
                        <Toggle
                          checked={entry.enabled}
                          onChange={() => handleToggleTask(entry.task_id, entry.enabled)}
                        />
                        <button
                          type="button"
                          onClick={() => handleRemoveTask(entry.task_id)}
                          className="p-1 rounded-md hover:bg-red-100 dark:hover:bg-red-900/20 text-text-secondary-light dark:text-text-secondary-dark hover:text-red-600 dark:hover:text-red-400 transition-colors"
                          title="Remove task"
                        >
                          <X size={14} />
                        </button>
                      </div>
                    </div>
                    {taskInfo && taskInfo.config_schema.length > 0 && (
                      <div className="ml-9 mt-1">
                        <TaskConfigEditor
                          taskId={entry.task_id}
                          taskName={taskInfo.name}
                          configSchema={taskInfo.config_schema}
                          entry={entry}
                          onSave={handleConfigSave}
                          sdkBasePath={
                            (entry.task_id === "flutter-sdk" || entry.task_id === "android-sdk")
                              ? sdkBasePath
                              : undefined
                          }
                        />
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          )}
        </Card>

        <div className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>Actions</CardTitle>
            </CardHeader>
            <div className="space-y-2">
              <Button
                size="sm"
                variant="secondary"
                className="w-full"
                onClick={handleExport}
              >
                <Download size={14} />
                Export Blueprint
              </Button>
              <Button
                size="sm"
                variant="secondary"
                className="w-full"
                onClick={handleClone}
              >
                <Copy size={14} />
                Clone Blueprint
              </Button>
              {!blueprint.is_builtin && (
                <>
                  {confirmDelete ? (
                    <div className="flex gap-2">
                      <Button
                        variant="danger"
                        size="sm"
                        className="flex-1"
                        onClick={handleDelete}
                      >
                        Confirm
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="flex-1"
                        onClick={() => setConfirmDelete(false)}
                      >
                        Cancel
                      </Button>
                    </div>
                  ) : (
                    <Button
                      variant="danger"
                      size="sm"
                      className="w-full"
                      onClick={() => setConfirmDelete(true)}
                    >
                      <Trash2 size={14} />
                      Delete Blueprint
                    </Button>
                  )}
                </>
              )}
            </div>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Details</CardTitle>
            </CardHeader>
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-sm text-text-secondary-light dark:text-text-secondary-dark">Version</span>
                <span className="text-sm text-text-primary-light dark:text-text-primary-dark">
                  {blueprint.version}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm text-text-secondary-light dark:text-text-secondary-dark">Created</span>
                <span className="text-sm text-text-primary-light dark:text-text-primary-dark">
                  {formatDate(blueprint.created_at)}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm text-text-secondary-light dark:text-text-secondary-dark">Updated</span>
                <span className="text-sm text-text-primary-light dark:text-text-primary-dark">
                  {formatDate(blueprint.updated_at)}
                </span>
              </div>
            </div>
          </Card>
        </div>
      </div>

      <AddTaskDialog
        open={showAddTask}
        onClose={() => setShowAddTask(false)}
        onAdd={handleAddTask}
        tasks={allTasks}
        existingTaskIds={existingTaskIds}
      />
    </div>
  );
}
