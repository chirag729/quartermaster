import { useEffect, useState, useCallback, useRef } from "react";
import { useParams, useNavigate, useSearchParams, useBlocker } from "react-router-dom";
import {
  ArrowLeft, Layers, ListChecks, Shield, Settings, Server, Monitor, Copy,
  Trash2, Download, GitBranch, Plus, X, Play, CheckCircle2, AlertTriangle,
  XCircle, Circle, ChevronUp, ChevronDown, Save, type LucideIcon,
} from "lucide-react";
import { useToastStore } from "../stores/toastStore";
import { formatError } from "../lib/formatError";
import { useBlueprintStore } from "../stores/blueprintStore";
import { useTauriEvent } from "../hooks/useTauriEvent";
import { Badge } from "../components/ui/Badge";
import { Button } from "../components/ui/Button";
import { Card, CardHeader, CardTitle, CardDescription } from "../components/ui/Card";
import { Skeleton } from "../components/ui/Skeleton";
import { Toggle } from "../components/ui/Toggle";
import { TaskConfigEditor } from "../components/blueprints/TaskConfigEditor";
import { AddTaskDialog } from "../components/blueprints/AddTaskDialog";
import { PreRunConfigDialog } from "../components/blueprints/PreRunConfigDialog";
import * as api from "../services/tauriCommands";
import type { Blueprint, BlueprintTaskEntry } from "../types/blueprint";
import type { TaskInfo, TaskStateInfo } from "../types/task";
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

type TaskStatusBadgeVariant = "success" | "warning" | "danger" | "default";

function taskStatusLabel(status: string, drifted: boolean, versionChanged: boolean): string {
  if (status === "completed" && !drifted && !versionChanged) return "Installed";
  if (drifted) return "Config drifted";
  if (versionChanged) return "Version changed";
  if (status === "failed") return "Failed";
  if (status === "in_progress") return "Running";
  return "Not installed";
}

function taskStatusBadgeVariant(status: string, drifted: boolean, versionChanged: boolean): TaskStatusBadgeVariant {
  if (status === "completed" && !drifted && !versionChanged) return "success";
  if (drifted || versionChanged) return "warning";
  if (status === "failed") return "danger";
  return "default";
}

function TaskStatusIcon({ status, drifted, versionChanged }: { status: string; drifted: boolean; versionChanged: boolean }) {
  if (status === "completed" && !drifted && !versionChanged) {
    return <CheckCircle2 size={14} className="text-green-500 shrink-0" />;
  }
  if (drifted || versionChanged) {
    return <AlertTriangle size={14} className="text-yellow-500 shrink-0" />;
  }
  if (status === "failed") {
    return <XCircle size={14} className="text-red-500 shrink-0" />;
  }
  return <Circle size={14} className="text-gray-400 dark:text-gray-600 shrink-0" />;
}

export function BlueprintDetailPage() {
  const { blueprintId } = useParams<{ blueprintId: string }>();
  const [searchParams] = useSearchParams();
  const nodeId = searchParams.get("nodeId");
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

  // Task status state (for showing installation status per task entry)
  const [taskStateMap, setTaskStateMap] = useState<Map<string, TaskStateInfo>>(new Map());

  // Pending edits: local copy of task_entries buffering ALL unsaved changes
  const [pendingEntries, setPendingEntries] = useState<BlueprintTaskEntry[] | null>(null);
  const [saving, setSaving] = useState(false);

  // Run blueprint state (only available when nodeId is present)
  const [showPreRun, setShowPreRun] = useState(false);
  const [applyingBlueprint, setApplyingBlueprint] = useState(false);
  const [applyProgress, setApplyProgress] = useState<string | null>(null);

  const isDirty = pendingEntries !== null;
  const backPath = nodeId ? `/fleet/${nodeId}` : "/blueprints";
  const backLabel = nodeId ? "Back to Node" : "Back to Blueprints";

  // ── Navigation guard ───────────────────────────────────────────────

  const blocker = useBlocker(isDirty);
  const blockerSavingRef = useRef(false);

  const handleBlockerSave = async () => {
    if (!blueprint || !pendingEntries) return;
    blockerSavingRef.current = true;
    setSaving(true);
    try {
      const updated = { ...blueprint, task_entries: pendingEntries };
      const result = await api.updateBlueprint(updated);
      setBlueprint(result);
      setPendingEntries(null);
      addToast({ type: "success", title: "Changes saved" });
      // proceed will be triggered by the useEffect below once isDirty becomes false
    } catch (err) {
      addToast({ type: "error", title: "Save failed", message: formatError(err) });
      blockerSavingRef.current = false;
    } finally {
      setSaving(false);
    }
  };

  const handleBlockerDiscard = () => {
    setPendingEntries(null);
    blocker.proceed?.();
  };

  // After a successful save during a blocked navigation, proceed once dirty state clears
  useEffect(() => {
    if (blockerSavingRef.current && !isDirty && blocker.state === "blocked") {
      blockerSavingRef.current = false;
      blocker.proceed?.();
    }
  }, [isDirty, blocker]);

  // ── Blueprint apply event listeners ─────────────────────────────────

  useTauriEvent<{ warning: string }>("blueprint-task-warning", (payload) => {
    addToast({ type: "warning", title: "Task skipped", message: payload.warning });
  });

  useTauriEvent<{ node_id: string; blueprint_id: string; completed: number; total: number; current_task_id: string }>(
    "blueprint-apply-progress",
    (payload) => {
      if (nodeId && payload.node_id === nodeId) {
        setApplyProgress(`Running task ${payload.completed + 1} of ${payload.total}`);
      }
    },
  );

  useTauriEvent<{ blueprint_id: string; node_id: string }>(
    "blueprint-apply-complete",
    (payload) => {
      if (nodeId && payload.node_id === nodeId) {
        setApplyProgress(null);
      }
    },
  );

  // ── Load blueprint ──────────────────────────────────────────────────

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

  // ── Load tasks and task states ──────────────────────────────────────

  useEffect(() => {
    api.listTasks().then((tasks) => {
      setAllTasks(tasks);
      const map: Record<string, TaskInfo> = {};
      for (const t of tasks) map[t.id] = t;
      setTaskInfoMap(map);

      // When no nodeId, use the local detection status from listTasks
      if (!nodeId) {
        const stateMap = new Map<string, TaskStateInfo>();
        for (const t of tasks) {
          stateMap.set(t.id, {
            ...t,
            config_drifted: false,
            version_changed: false,
          });
        }
        setTaskStateMap(stateMap);
      }
    }).catch(() => {});
  }, [nodeId]);

  // When nodeId is present, load per-node task states
  const loadTaskStates = useCallback(async () => {
    if (!nodeId) return;
    try {
      const states = await api.listTasksForNode(nodeId);
      const map = new Map<string, TaskStateInfo>();
      for (const s of states) map.set(s.id, s);
      setTaskStateMap(map);
    } catch (err) {
      addToast({ type: "error", title: "Failed to load task states", message: formatError(err) });
    }
  }, [nodeId, addToast]);

  useEffect(() => {
    if (nodeId) {
      loadTaskStates();
    }
  }, [nodeId, loadTaskStates]);

  // ── Helper: get current entries (pending or saved) ────────────────

  const currentEntries = useCallback(
    (): BlueprintTaskEntry[] => pendingEntries ?? blueprint?.task_entries ?? [],
    [pendingEntries, blueprint],
  );

  // ── Blueprint actions ───────────────────────────────────────────────

  const handleClone = async () => {
    if (!blueprint) return;
    const newName = prompt("Name for the cloned blueprint:", `${blueprint.name} (Copy)`);
    if (!newName) return;
    try {
      const cloned = await cloneBlueprint(blueprint.id, newName);
      addToast({ type: "success", title: "Blueprint cloned", message: `Created "${cloned.name}"` });
      setPendingEntries(null);
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
      setPendingEntries(null);
      navigate(backPath);
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

  // ── Task mutations (all buffer locally) ───────────────────────────

  const handleToggleTask = (taskId: string, currentEnabled: boolean) => {
    const entries = currentEntries();
    setPendingEntries(
      entries.map((e) => (e.task_id === taskId ? { ...e, enabled: !currentEnabled } : e)),
    );
  };

  const handleAddTask = (taskId: string) => {
    const entries = currentEntries();
    const maxOrder = entries.reduce((max, e) => Math.max(max, e.order), 0);
    setPendingEntries([
      ...entries,
      { task_id: taskId, enabled: true, config_overrides: {}, order: maxOrder + 1 },
    ]);
    const taskName = taskInfoMap[taskId]?.name ?? taskId;
    addToast({ type: "info", title: "Task added", message: `"${taskName}" added — save to persist` });
  };

  const handleRemoveTask = (taskId: string) => {
    const entries = currentEntries();
    setPendingEntries(entries.filter((e) => e.task_id !== taskId));
    const taskName = taskInfoMap[taskId]?.name ?? taskId;
    addToast({ type: "info", title: "Task removed", message: `"${taskName}" removed — save to persist` });
  };

  const handleConfigChange = (taskId: string, overrides: Record<string, unknown>) => {
    const entries = currentEntries();
    setPendingEntries(
      entries.map((e) => (e.task_id === taskId ? { ...e, config_overrides: overrides } : e)),
    );
  };

  // ── Task reordering ────────────────────────────────────────────────

  /** Check if moving taskId in the given direction would violate dependency ordering. */
  const canMoveTask = useCallback(
    (taskId: string, direction: "up" | "down", sorted: BlueprintTaskEntry[]): boolean => {
      const idx = sorted.findIndex((e) => e.task_id === taskId);
      if (idx < 0) return false;
      const swapIdx = direction === "up" ? idx - 1 : idx + 1;
      if (swapIdx < 0 || swapIdx >= sorted.length) return false;

      const movingTask = taskInfoMap[taskId];
      const otherTask = taskInfoMap[sorted[swapIdx].task_id];

      if (direction === "up") {
        // Can't move above a task we depend on
        if (movingTask?.depends_on?.includes(sorted[swapIdx].task_id)) return false;
      } else {
        // Can't move below a task that depends on us
        if (otherTask?.depends_on?.includes(taskId)) return false;
      }
      return true;
    },
    [taskInfoMap],
  );

  const handleMoveTask = (taskId: string, direction: "up" | "down") => {
    const entries = currentEntries();
    const sorted = [...entries].sort((a, b) => a.order - b.order);

    if (!canMoveTask(taskId, direction, sorted)) return;

    const idx = sorted.findIndex((e) => e.task_id === taskId);
    const swapIdx = direction === "up" ? idx - 1 : idx + 1;

    // Swap order values
    const temp = sorted[idx].order;
    sorted[idx] = { ...sorted[idx], order: sorted[swapIdx].order };
    sorted[swapIdx] = { ...sorted[swapIdx], order: temp };

    setPendingEntries(sorted);
  };

  // ── Save / Discard all changes ────────────────────────────────────

  const handleSaveAll = async () => {
    if (!blueprint || !pendingEntries) return;
    setSaving(true);
    try {
      const updated = { ...blueprint, task_entries: pendingEntries };
      const result = await api.updateBlueprint(updated);
      setBlueprint(result);
      setPendingEntries(null);
      addToast({ type: "success", title: "Changes saved" });
    } catch (err) {
      addToast({ type: "error", title: "Save failed", message: formatError(err) });
    } finally {
      setSaving(false);
    }
  };

  const handleDiscardAll = () => {
    setPendingEntries(null);
  };

  // ── Run blueprint (node context only) ───────────────────────────────

  const handleRunBlueprintConfirm = async () => {
    if (!nodeId || !blueprintId) return;
    setApplyingBlueprint(true);
    try {
      await api.applyBlueprint(nodeId, blueprintId);
      setShowPreRun(false);
      addToast({ type: "success", title: "Blueprint applied" });
      await loadTaskStates();
    } catch (err) {
      addToast({ type: "error", title: "Failed to apply blueprint", message: formatError(err) });
    } finally {
      setApplyingBlueprint(false);
    }
  };

  // ── Loading skeleton ────────────────────────────────────────────────

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
        <Button variant="ghost" size="sm" onClick={() => navigate(backPath)}>
          <ArrowLeft size={14} />
          {backLabel}
        </Button>
        <p className="mt-4 text-sm text-text-secondary-light dark:text-text-secondary-dark">
          Blueprint not found.
        </p>
      </div>
    );
  }

  const Icon = iconMap[blueprint.icon] || Layers;
  const displayEntries = pendingEntries ?? blueprint.task_entries;
  const sortedEntries = [...displayEntries].sort((a, b) => a.order - b.order);
  const existingTaskIds = displayEntries.map((e) => e.task_id);

  // Compute task completion stats
  const enabledEntries = sortedEntries.filter((e) => e.enabled);
  const completedCount = enabledEntries.filter((e) => {
    const ts = taskStateMap.get(e.task_id);
    return ts && ts.status === "completed" && !ts.config_drifted && !ts.version_changed;
  }).length;

  return (
    <div className="pb-16">
      <div className="flex items-center justify-between mb-4">
        <Button variant="ghost" size="sm" onClick={() => navigate(backPath)}>
          <ArrowLeft size={14} />
          {backLabel}
        </Button>
        {nodeId && (
          <Button size="sm" onClick={() => setShowPreRun(true)}>
            <Play size={14} />
            Run Blueprint
          </Button>
        )}
      </div>
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

      {/* Task completion bar */}
      {enabledEntries.length > 0 && taskStateMap.size > 0 && (
        <div className="mb-6 space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark">
              Task Completion
            </span>
            <span className="text-xs text-text-secondary-light dark:text-text-secondary-dark">
              {completedCount}/{enabledEntries.length} completed
            </span>
          </div>
          <div className="w-full h-2 rounded-full bg-warm-100 dark:bg-warm-900/30 overflow-hidden">
            <div
              className="h-full rounded-full bg-green-500 transition-all duration-300"
              style={{
                width: `${enabledEntries.length > 0 ? (completedCount / enabledEntries.length) * 100 : 0}%`,
              }}
            />
          </div>
        </div>
      )}

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
                const taskState = taskStateMap.get(entry.task_id);
                const status = taskState?.status ?? "not_started";
                const drifted = taskState?.config_drifted ?? false;
                const versionChanged = taskState?.version_changed ?? false;
                const hasStatus = taskStateMap.size > 0;

                return (
                  <div key={entry.task_id} className="space-y-0">
                    <div className="flex items-center justify-between p-3 rounded-lg border border-border-light dark:border-border-dark bg-surface-light dark:bg-surface-dark">
                      <div className="flex items-center gap-3 min-w-0">
                        <div className="flex flex-col shrink-0">
                          <button
                            type="button"
                            disabled={idx === 0 || !canMoveTask(entry.task_id, "up", sortedEntries)}
                            onClick={() => handleMoveTask(entry.task_id, "up")}
                            className="p-0.5 rounded hover:bg-warm-100 dark:hover:bg-warm-900/30 text-text-secondary-light dark:text-text-secondary-dark disabled:opacity-25 disabled:cursor-not-allowed transition-colors"
                            title={
                              idx > 0 && !canMoveTask(entry.task_id, "up", sortedEntries)
                                ? "Cannot move above a dependency"
                                : "Move up"
                            }
                          >
                            <ChevronUp size={14} />
                          </button>
                          <button
                            type="button"
                            disabled={idx === sortedEntries.length - 1 || !canMoveTask(entry.task_id, "down", sortedEntries)}
                            onClick={() => handleMoveTask(entry.task_id, "down")}
                            className="p-0.5 rounded hover:bg-warm-100 dark:hover:bg-warm-900/30 text-text-secondary-light dark:text-text-secondary-dark disabled:opacity-25 disabled:cursor-not-allowed transition-colors"
                            title={
                              idx < sortedEntries.length - 1 && !canMoveTask(entry.task_id, "down", sortedEntries)
                                ? "Cannot move below a dependent task"
                                : "Move down"
                            }
                          >
                            <ChevronDown size={14} />
                          </button>
                        </div>
                        <span className="text-xs font-mono text-text-secondary-light dark:text-text-secondary-dark w-6 text-center shrink-0">
                          {idx + 1}
                        </span>
                        <div className="p-1.5 rounded-md bg-warm-100 dark:bg-warm-900/30">
                          <TaskIcon size={14} className="text-warm-500 dark:text-warm-300" />
                        </div>
                        <div className="min-w-0">
                          <div className="flex items-center gap-2">
                            <p className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark truncate">
                              {taskInfo?.name || entry.task_id}
                            </p>
                            {hasStatus && (
                              <>
                                <TaskStatusIcon status={status} drifted={drifted} versionChanged={versionChanged} />
                                <Badge variant={taskStatusBadgeVariant(status, drifted, versionChanged)}>
                                  {taskStatusLabel(status, drifted, versionChanged)}
                                </Badge>
                              </>
                            )}
                          </div>
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
                          onSave={handleConfigChange}
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
              {nodeId && (
                <Button
                  size="sm"
                  variant="primary"
                  className="w-full"
                  onClick={() => setShowPreRun(true)}
                >
                  <Play size={14} />
                  Run Blueprint
                </Button>
              )}
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

      {/* Sticky save/discard bar */}
      {isDirty && (
        <div className="fixed bottom-0 left-0 right-0 z-50 border-t border-border-light dark:border-border-dark bg-surface-light/95 dark:bg-surface-dark/95 backdrop-blur-sm">
          <div className="max-w-5xl mx-auto px-6 py-3 flex items-center justify-between">
            <span className="text-sm text-text-secondary-light dark:text-text-secondary-dark">
              You have unsaved changes
            </span>
            <div className="flex items-center gap-2">
              <Button size="sm" variant="ghost" onClick={handleDiscardAll} disabled={saving}>
                Discard
              </Button>
              <Button size="sm" onClick={handleSaveAll} loading={saving}>
                <Save size={14} />
                Save Changes
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Navigation guard dialog */}
      {blocker.state === "blocked" && (
        <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/50">
          <div className="bg-surface-light dark:bg-surface-dark rounded-xl shadow-xl border border-border-light dark:border-border-dark p-6 max-w-sm w-full mx-4">
            <h3 className="text-lg font-semibold text-text-primary-light dark:text-text-primary-dark mb-2">
              Unsaved Changes
            </h3>
            <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark mb-4">
              You have unsaved changes to this blueprint. Would you like to save before leaving?
            </p>
            <div className="flex justify-end gap-2">
              <Button size="sm" variant="ghost" onClick={() => blocker.reset?.()}>
                Stay
              </Button>
              <Button size="sm" variant="danger" onClick={handleBlockerDiscard}>
                Discard
              </Button>
              <Button size="sm" onClick={handleBlockerSave} loading={saving}>
                <Save size={14} />
                Save
              </Button>
            </div>
          </div>
        </div>
      )}

      <AddTaskDialog
        open={showAddTask}
        onClose={() => setShowAddTask(false)}
        onAdd={handleAddTask}
        tasks={allTasks}
        existingTaskIds={existingTaskIds}
      />

      {nodeId && (
        <PreRunConfigDialog
          open={showPreRun}
          onClose={() => setShowPreRun(false)}
          onConfirm={handleRunBlueprintConfirm}
          blueprint={blueprint}
          tasks={allTasks}
          loading={applyingBlueprint}
          loadingMessage={applyProgress ?? undefined}
          nodeId={nodeId}
        />
      )}
    </div>
  );
}
