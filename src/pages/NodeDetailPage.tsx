import { useEffect, useState, useCallback } from "react";
import { useParams, useNavigate } from "react-router-dom";
import {
  ArrowLeft,
  Server,
  Monitor,
  Terminal,
  Layers,
  Play,
  RefreshCw,
  CheckCircle2,
  AlertTriangle,
  XCircle,
  Circle,
  Clock,
  Blocks,
} from "lucide-react";
import { useToastStore } from "../stores/toastStore";
import { useBlueprintStore } from "../stores/blueprintStore";
import { NodeStatusBadge } from "../components/fleet/NodeStatusBadge";
import { AssignBlueprintDialog } from "../components/blueprints/AssignBlueprintDialog";
import { PreRunConfigDialog } from "../components/blueprints/PreRunConfigDialog";
import { RunTaskDialog } from "../components/fleet/RunTaskDialog";
import { TaskExecutionDialog } from "../components/fleet/TaskExecutionDialog";
import { Badge } from "../components/ui/Badge";
import { Button } from "../components/ui/Button";
import { Card, CardHeader, CardTitle, CardDescription } from "../components/ui/Card";
import { Skeleton } from "../components/ui/Skeleton";
import { EmptyState } from "../components/ui/EmptyState";
import { Tabs } from "../components/ui/Tabs";
import * as api from "../services/tauriCommands";
import { formatError } from "../lib/formatError";
import { useTauriEvent } from "../hooks/useTauriEvent";
import type { Node } from "../types/node";
import type { Blueprint } from "../types/blueprint";
import type { TaskStateInfo, TaskInfo } from "../types/task";

const PAGE_TABS = [
  { id: "overview", label: "Overview" },
  { id: "tasks", label: "Tasks" },
  { id: "shell", label: "Shell" },
];

export function NodeDetailPage() {
  const { nodeId } = useParams<{ nodeId: string }>();
  const navigate = useNavigate();
  const { addToast } = useToastStore();
  const { blueprints, setBlueprints, assignBlueprint, unassignBlueprint } = useBlueprintStore();

  const [node, setNode] = useState<Node | null>(null);
  const [loading, setLoading] = useState(true);
  const [blueprint, setBlueprint] = useState<Blueprint | null>(null);
  const [showAssign, setShowAssign] = useState(false);
  const [assigning, setAssigning] = useState(false);
  const [activeTab, setActiveTab] = useState("overview");

  // Tasks tab state
  const [taskStates, setTaskStates] = useState<TaskStateInfo[]>([]);
  const [tasksLoading, setTasksLoading] = useState(false);
  const [runningTaskId, setRunningTaskId] = useState<string | null>(null);
  const [syncingAll, setSyncingAll] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [showPreRun, setShowPreRun] = useState(false);
  const [applyingBlueprint, setApplyingBlueprint] = useState(false);
  const [allTasks, setAllTasks] = useState<TaskInfo[]>([]);
  const [applyProgress, setApplyProgress] = useState<string | null>(null);
  const [showRunTask, setShowRunTask] = useState(false);
  const [showExecution, setShowExecution] = useState(false);
  const [executionTaskId, setExecutionTaskId] = useState<string>("");
  const [executionTaskName, setExecutionTaskName] = useState<string>("");

  // ── Blueprint apply event listeners ─────────────────────────────────

  useTauriEvent<{ warning: string }>("blueprint-task-warning", (payload) => {
    addToast({ type: "warning", title: "Task skipped", message: payload.warning });
  });

  useTauriEvent<{ node_id: string; blueprint_id: string; completed: number; total: number; current_task_id: string }>(
    "blueprint-apply-progress",
    (payload) => {
      if (payload.node_id === nodeId) {
        setApplyProgress(`Running task ${payload.completed + 1} of ${payload.total}`);
      }
    },
  );

  useTauriEvent<{ blueprint_id: string; node_id: string }>(
    "blueprint-apply-complete",
    (payload) => {
      if (payload.node_id === nodeId) {
        setApplyProgress(null);
      }
    },
  );

  // ── Load node ──────────────────────────────────────────────────────

  const loadNode = useCallback(async () => {
    if (!nodeId) return;
    setLoading(true);
    try {
      const n = await api.getNode(nodeId);
      setNode(n);
      if (n.blueprint_id) {
        try {
          const bp = await api.getBlueprint(n.blueprint_id);
          setBlueprint(bp);
        } catch {
          setBlueprint(null);
        }
      } else {
        setBlueprint(null);
      }
    } catch (err) {
      addToast({ type: "error", title: "Failed to load node", message: formatError(err) });
    } finally {
      setLoading(false);
    }
  }, [nodeId, addToast]);

  useEffect(() => {
    loadNode();
  }, [loadNode]);

  // Load blueprints for the assign dialog and tasks for pre-run dialog lookups
  useEffect(() => {
    api.listBlueprints().then(setBlueprints).catch(() => {});
    api.listTasks().then(setAllTasks).catch(() => {});
  }, [setBlueprints]);

  // ── Load per-node task states ──────────────────────────────────────

  const loadTaskStates = useCallback(async () => {
    if (!nodeId) return;
    setTasksLoading(true);
    try {
      const states = await api.listTasksForNode(nodeId);
      setTaskStates(states);
    } catch (err) {
      addToast({ type: "error", title: "Failed to load task states", message: formatError(err) });
    } finally {
      setTasksLoading(false);
    }
  }, [nodeId, addToast]);

  // Load task states when overview/tasks tab is active, run-task dialog opens, or on initial mount
  useEffect(() => {
    if (nodeId && (activeTab === "tasks" || activeTab === "overview" || showRunTask)) {
      loadTaskStates();
    }
  }, [activeTab, showRunTask, nodeId, loadTaskStates]);

  // ── Refresh all data ───────────────────────────────────────────────

  const handleRefresh = async () => {
    setRefreshing(true);
    try {
      await loadNode();
      await loadTaskStates();
      addToast({ type: "success", title: "Refreshed" });
    } finally {
      setRefreshing(false);
    }
  };

  // ── Blueprint assign / unassign ────────────────────────────────────

  const handleAssign = async (blueprintId: string) => {
    if (!nodeId) return;
    setAssigning(true);
    try {
      await assignBlueprint(nodeId, blueprintId);
      addToast({ type: "success", title: "Blueprint assigned" });
      setShowAssign(false);
      await loadNode();
      // Fetch the newly assigned blueprint and open the pre-run dialog
      // so the user can review and run the tasks immediately
      try {
        const bp = await api.getBlueprint(blueprintId);
        setBlueprint(bp);
        await loadTaskStates();
        setShowPreRun(true);
      } catch {
        // Non-critical: blueprint was assigned, just couldn't open pre-run
      }
    } catch (err) {
      addToast({ type: "error", title: "Failed to assign blueprint", message: formatError(err) });
    } finally {
      setAssigning(false);
    }
  };

  const handleUnassign = async () => {
    if (!nodeId) return;
    try {
      await unassignBlueprint(nodeId);
      addToast({ type: "success", title: "Blueprint unassigned" });
      setBlueprint(null);
      await loadNode();
    } catch (err) {
      addToast({ type: "error", title: "Failed to unassign blueprint", message: formatError(err) });
    }
  };

  // ── Task execution ─────────────────────────────────────────────────

  const handleRunTask = async (taskId: string) => {
    if (!nodeId) return;
    // Look up the task name for the dialog title
    const taskInfo = allTasks.find((t) => t.id === taskId) ?? taskStates.find((t) => t.id === taskId);
    setExecutionTaskId(taskId);
    setExecutionTaskName(taskInfo?.name ?? taskId);
    setShowExecution(true);
    setRunningTaskId(taskId);
    try {
      await api.executeTask(taskId, nodeId, node?.blueprint_id ?? undefined);
      addToast({ type: "success", title: "Task completed", message: taskId });
      await loadTaskStates();
    } catch (err) {
      addToast({ type: "error", title: `Task failed: ${taskId}`, message: formatError(err) });
    } finally {
      setRunningTaskId(null);
    }
  };

  // Compute blueprint affiliation for each task: taskId -> blueprint names
  const taskBlueprintMap = (() => {
    const map = new Map<string, { id: string; name: string }[]>();
    for (const bp of blueprints) {
      for (const entry of bp.task_entries) {
        if (!entry.enabled) continue;
        const list = map.get(entry.task_id) ?? [];
        list.push({ id: bp.id, name: bp.name });
        map.set(entry.task_id, list);
      }
    }
    return map;
  })();

  // Filter to only tasks that are installed, in a blueprint, or have an install record
  const relevantTaskStates = taskStates.filter(
    (t) => t.status === "completed" || t.installed_at || taskBlueprintMap.has(t.id),
  );

  // Group filtered task states by category
  const groupedTaskStates = (() => {
    const map = new Map<string, TaskStateInfo[]>();
    for (const task of relevantTaskStates) {
      const list = map.get(task.category) ?? [];
      list.push(task);
      map.set(task.category, list);
    }
    return Array.from(map.entries()).sort(([a], [b]) => a.localeCompare(b));
  })();

  const handleSyncAll = async () => {
    if (!nodeId) return;
    const outOfSync = relevantTaskStates.filter(
      (t) => t.config_drifted || t.version_changed || t.status === "failed",
    );
    if (outOfSync.length === 0) {
      addToast({ type: "info", title: "All tasks are in sync" });
      return;
    }
    setSyncingAll(true);
    try {
      for (const task of outOfSync) {
        await api.executeTask(task.id, nodeId, node?.blueprint_id ?? undefined);
      }
      addToast({ type: "success", title: "All out-of-sync tasks completed" });
      await loadTaskStates();
    } catch (err) {
      addToast({ type: "error", title: "Sync failed", message: formatError(err) });
      await loadTaskStates();
    } finally {
      setSyncingAll(false);
    }
  };

  // ── Terminal ───────────────────────────────────────────────────────

  const handleOpenTerminal = async () => {
    if (!nodeId) return;
    try {
      await api.openNodeTerminal(nodeId);
    } catch (err) {
      addToast({ type: "error", title: "Failed to open terminal", message: formatError(err) });
    }
  };

  // ── Run blueprint ──────────────────────────────────────────────────

  const handleRunBlueprintClick = () => {
    if (!nodeId || !node?.blueprint_id || !blueprint) return;
    setShowPreRun(true);
  };

  const handleRunBlueprintConfirm = async () => {
    if (!nodeId || !node?.blueprint_id) return;
    setApplyingBlueprint(true);
    try {
      await api.applyBlueprint(nodeId, node.blueprint_id);
      setShowPreRun(false);
      addToast({ type: "success", title: "Blueprint applied" });
      await loadTaskStates();
    } catch (err) {
      addToast({ type: "error", title: "Failed to apply blueprint", message: formatError(err) });
    } finally {
      setApplyingBlueprint(false);
    }
  };

  // ── Helpers ────────────────────────────────────────────────────────

  function formatDate(iso?: string) {
    if (!iso) return "---";
    try {
      return new Date(iso).toLocaleString();
    } catch {
      return iso;
    }
  }

  function TaskStatusIcon({ task }: { task: TaskStateInfo }) {
    if (task.status === "completed" && !task.config_drifted && !task.version_changed) {
      return <CheckCircle2 size={16} className="text-green-500 shrink-0" />;
    }
    if (task.config_drifted || task.version_changed) {
      return <AlertTriangle size={16} className="text-yellow-500 shrink-0" />;
    }
    if (task.status === "failed") {
      return <XCircle size={16} className="text-red-500 shrink-0" />;
    }
    return <Circle size={16} className="text-gray-400 dark:text-gray-600 shrink-0" />;
  }

  function taskStatusLabel(task: TaskStateInfo): string {
    if (task.status === "completed" && !task.config_drifted && !task.version_changed) {
      return "Installed";
    }
    if (task.config_drifted) return "Config drifted";
    if (task.version_changed) return "Version changed";
    if (task.status === "failed") return "Failed";
    if (task.status === "in_progress") return "Running";
    return "Not installed";
  }

  function taskStatusBadgeVariant(task: TaskStateInfo): "success" | "warning" | "danger" | "default" {
    if (task.status === "completed" && !task.config_drifted && !task.version_changed) return "success";
    if (task.config_drifted || task.version_changed) return "warning";
    if (task.status === "failed") return "danger";
    return "default";
  }

  // ── Loading skeleton ───────────────────────────────────────────────

  if (loading) {
    return (
      <div className="space-y-4">
        <Skeleton className="h-8 w-48" rounded="lg" />
        <Skeleton className="h-12 w-full" rounded="xl" />
        <Skeleton className="h-64 w-full" rounded="xl" />
      </div>
    );
  }

  // ── Not found ──────────────────────────────────────────────────────

  if (!node) {
    return (
      <div>
        <Button variant="ghost" size="sm" onClick={() => navigate("/fleet")}>
          <ArrowLeft size={14} />
          Back to Fleet
        </Button>
        <p className="mt-4 text-sm text-text-secondary-light dark:text-text-secondary-dark">
          Node not found.
        </p>
      </div>
    );
  }

  const KindIcon = node.kind === "local" ? Monitor : Server;

  // Compute task completion stats from blueprint entries
  const enabledEntries = blueprint?.task_entries.filter((e) => e.enabled) || [];
  const taskStateMap = new Map(taskStates.map((t) => [t.id, t]));
  const completedCount = enabledEntries.filter((e) => {
    const ts = taskStateMap.get(e.task_id);
    return ts && ts.status === "completed" && !ts.config_drifted && !ts.version_changed;
  }).length;

  // ── Render ─────────────────────────────────────────────────────────

  return (
    <div>
      {/* ── Header ─────────────────────────────────────────────────── */}
      <div className="mb-6">
        <Button variant="ghost" size="sm" onClick={() => navigate("/fleet")} className="mb-4">
          <ArrowLeft size={14} />
          Back to Fleet
        </Button>

        <div className="flex items-center justify-between gap-4">
          <div className="flex items-center gap-3 min-w-0">
            <div className="p-2.5 rounded-lg bg-warm-100 dark:bg-warm-900/30 text-warm-600 dark:text-warm-400 shrink-0">
              <KindIcon size={22} />
            </div>
            <div className="min-w-0">
              <div className="flex items-center gap-2.5">
                <h1 className="text-xl font-semibold text-text-primary-light dark:text-text-primary-dark truncate">
                  {node.name}
                </h1>
                <NodeStatusBadge status={node.status} />
              </div>
              <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark truncate">
                {node.hostname}
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2 shrink-0">
            <Button variant="secondary" size="sm" onClick={handleOpenTerminal}>
              <Terminal size={14} />
              Terminal
            </Button>
            <Button variant="secondary" size="sm" onClick={() => setShowRunTask(true)}>
              <Play size={14} />
              Run Task
            </Button>
            {node.blueprint_id && (
              <Button variant="secondary" size="sm" onClick={handleRunBlueprintClick}>
                <Play size={14} />
                Run Blueprint
              </Button>
            )}
            <Button
              variant="ghost"
              size="sm"
              onClick={handleRefresh}
              loading={refreshing}
            >
              <RefreshCw size={14} />
              Refresh
            </Button>
          </div>
        </div>
      </div>

      {/* ── Tabs ───────────────────────────────────────────────────── */}
      <Tabs
        tabs={PAGE_TABS}
        activeTab={activeTab}
        onTabChange={setActiveTab}
        className="mb-6"
      />

      {/* ── Overview Tab ───────────────────────────────────────────── */}
      {activeTab === "overview" && (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          {/* Node metadata */}
          <Card>
            <CardHeader>
              <CardTitle>Node Details</CardTitle>
            </CardHeader>
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-sm text-text-secondary-light dark:text-text-secondary-dark">Status</span>
                <NodeStatusBadge status={node.status} />
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm text-text-secondary-light dark:text-text-secondary-dark">Kind</span>
                <Badge variant={node.kind === "local" ? "default" : "info"}>{node.kind}</Badge>
              </div>
              {node.os && (
                <div className="flex items-center justify-between">
                  <span className="text-sm text-text-secondary-light dark:text-text-secondary-dark">OS</span>
                  <span className="text-sm text-text-primary-light dark:text-text-primary-dark">{node.os}</span>
                </div>
              )}
              <div className="flex items-center justify-between">
                <span className="text-sm text-text-secondary-light dark:text-text-secondary-dark">Last Seen</span>
                <span className="text-sm text-text-primary-light dark:text-text-primary-dark">
                  {formatDate(node.last_seen)}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm text-text-secondary-light dark:text-text-secondary-dark">Created</span>
                <span className="text-sm text-text-primary-light dark:text-text-primary-dark">
                  {formatDate(node.created_at)}
                </span>
              </div>
            </div>
          </Card>

          {/* Tags */}
          <Card>
            <CardHeader>
              <CardTitle>Tags</CardTitle>
            </CardHeader>
            {node.tags.length > 0 ? (
              <div className="flex flex-wrap gap-2">
                {node.tags.map((tag) => (
                  <Badge key={tag} variant="default">{tag}</Badge>
                ))}
              </div>
            ) : (
              <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark">
                No tags assigned.
              </p>
            )}
          </Card>

          {/* Assigned blueprint */}
          <Card className="lg:col-span-2">
            <CardHeader>
              <CardTitle>Assigned Blueprint</CardTitle>
              <CardDescription>
                {node.blueprint_id
                  ? `Blueprint: ${blueprint?.name || node.blueprint_id}`
                  : "No blueprint assigned to this node yet."}
              </CardDescription>
            </CardHeader>
            {blueprint ? (
              <div className="space-y-3">
                <div
                  className="flex items-center gap-3 p-3 rounded-lg border border-border-light dark:border-border-dark bg-surface-light dark:bg-surface-dark cursor-pointer hover:bg-warm-50 dark:hover:bg-warm-900/10 transition-colors"
                  onClick={() => navigate(`/blueprints/${blueprint.id}`)}
                >
                  <div className="p-2 rounded-lg bg-warm-100 dark:bg-warm-900/30 text-warm-600 dark:text-warm-400">
                    <Layers size={16} />
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark">
                      {blueprint.name}
                    </p>
                    <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark truncate">
                      {blueprint.description}
                    </p>
                  </div>
                  <div className="flex items-center gap-2 shrink-0">
                    <Badge variant="info">v{blueprint.version}</Badge>
                    {node.applied_blueprint_version && node.applied_blueprint_version !== blueprint.version ? (
                      <Badge variant="danger">Outdated (v{node.applied_blueprint_version})</Badge>
                    ) : node.applied_blueprint_version ? (
                      <Badge variant="success">Up to date</Badge>
                    ) : null}
                    <Badge variant="default">{blueprint.task_entries.length} tasks</Badge>
                  </div>
                </div>

                {/* Task completion bar */}
                {enabledEntries.length > 0 && (
                  <div className="space-y-2">
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

                <div className="flex gap-2">
                  <Button variant="secondary" size="sm" onClick={() => setShowAssign(true)}>
                    Change Blueprint
                  </Button>
                  <Button variant="ghost" size="sm" onClick={handleUnassign}>
                    Unassign
                  </Button>
                </div>
              </div>
            ) : (
              <div className="border border-dashed border-border-light dark:border-border-dark rounded-lg p-6 text-center">
                <Layers
                  size={24}
                  className="mx-auto mb-2 text-text-secondary-light/40 dark:text-text-secondary-dark/40"
                />
                <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark mb-3">
                  No blueprint assigned to this node.
                </p>
                <Button variant="secondary" size="sm" onClick={() => setShowAssign(true)}>
                  Assign Blueprint
                </Button>
              </div>
            )}
          </Card>
        </div>
      )}

      {/* ── Tasks Tab ──────────────────────────────────────────────── */}
      {activeTab === "tasks" && (
        <div className="space-y-4">
          {/* Top actions */}
          <div className="flex items-center justify-between">
            <h2 className="text-base font-semibold text-text-primary-light dark:text-text-primary-dark">
              Task States
            </h2>
            <div className="flex items-center gap-2">
              <Button
                variant="secondary"
                size="sm"
                onClick={handleSyncAll}
                loading={syncingAll}
                disabled={syncingAll || tasksLoading}
              >
                <RefreshCw size={14} />
                Sync All
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={loadTaskStates}
                loading={tasksLoading}
              >
                <RefreshCw size={14} />
                Refresh
              </Button>
            </div>
          </div>

          {/* Task list */}
          {tasksLoading && taskStates.length === 0 ? (
            <div className="space-y-3">
              {[1, 2, 3].map((i) => (
                <Skeleton key={i} className="h-16 w-full" rounded="xl" />
              ))}
            </div>
          ) : taskStates.length === 0 ? (
            <EmptyState
              icon={<Layers size={32} />}
              title="No tasks found"
              description="No tasks are registered. Check that task definitions exist."
            />
          ) : (
            <div className="space-y-6">
              {groupedTaskStates.map(([category, categoryTasks]) => (
                <div key={category}>
                  <h3 className="text-xs font-semibold uppercase tracking-wider text-text-secondary-light dark:text-text-secondary-dark mb-2">
                    {category}
                  </h3>
                  <div className="space-y-2">
                    {categoryTasks.map((task) => {
                      const isDrifted = task.config_drifted || task.version_changed;
                      const isInstalled = task.status === "completed";
                      const isRunning = runningTaskId === task.id;
                      const affiliatedBlueprints = taskBlueprintMap.get(task.id) ?? [];

                      return (
                        <Card key={task.id} padding={false} className="p-4">
                          <div className="flex items-center gap-3">
                            <TaskStatusIcon task={task} />

                            <div className="flex-1 min-w-0">
                              <div className="flex items-center gap-2 flex-wrap">
                                <span className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark truncate">
                                  {task.name}
                                </span>
                                <Badge variant={taskStatusBadgeVariant(task)}>
                                  {taskStatusLabel(task)}
                                </Badge>
                                {affiliatedBlueprints.map((bp) => (
                                  <span
                                    key={bp.id}
                                    className="inline-flex items-center gap-1 text-[11px] px-1.5 py-0.5 rounded-md bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400 border border-blue-200 dark:border-blue-800"
                                  >
                                    <Blocks size={10} />
                                    {bp.name}
                                  </span>
                                ))}
                              </div>
                              <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark truncate mt-0.5">
                                {task.description}
                              </p>

                              {/* Drift / install details */}
                              <div className="flex items-center gap-3 mt-1.5 flex-wrap">
                                {task.installed_at && (
                                  <span className="inline-flex items-center gap-1 text-xs text-text-secondary-light dark:text-text-secondary-dark">
                                    <Clock size={11} />
                                    Installed {formatDate(task.installed_at)}
                                  </span>
                                )}
                                {task.installed_version && (
                                  <span className="text-xs text-text-secondary-light dark:text-text-secondary-dark">
                                    v{task.installed_version}
                                  </span>
                                )}
                                {task.config_drifted && (
                                  <span className="inline-flex items-center gap-1 text-xs text-yellow-600 dark:text-yellow-400">
                                    <AlertTriangle size={11} />
                                    Config drifted
                                  </span>
                                )}
                                {task.version_changed && (
                                  <span className="inline-flex items-center gap-1 text-xs text-yellow-600 dark:text-yellow-400">
                                    <AlertTriangle size={11} />
                                    Version changed
                                  </span>
                                )}
                              </div>
                            </div>

                            <Button
                              variant={isDrifted || task.status === "failed" ? "primary" : "secondary"}
                              size="sm"
                              onClick={() => handleRunTask(task.id)}
                              loading={isRunning}
                              disabled={isRunning || syncingAll}
                              className="shrink-0"
                            >
                              <Play size={12} />
                              {isInstalled && !isDrifted ? "Re-run" : "Run"}
                            </Button>
                          </div>
                        </Card>
                      );
                    })}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* ── Shell Tab ──────────────────────────────────────────────── */}
      {activeTab === "shell" && (
        <div className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>
                {node.kind === "local" ? "Local Terminal" : "Remote Shell"}
              </CardTitle>
              <CardDescription>
                {node.kind === "local"
                  ? "Open a local terminal session on this machine."
                  : "Open an SSH terminal session to this remote node."}
              </CardDescription>
            </CardHeader>

            {node.kind === "remote" && node.ssh_config && (
              <div className="space-y-3 mb-4">
                <div className="p-3 rounded-lg bg-surface-light dark:bg-surface-dark border border-border-light dark:border-border-dark space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-text-secondary-light dark:text-text-secondary-dark">Host</span>
                    <span className="text-sm font-mono text-text-primary-light dark:text-text-primary-dark">
                      {node.ssh_config.host}
                    </span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-text-secondary-light dark:text-text-secondary-dark">Port</span>
                    <span className="text-sm font-mono text-text-primary-light dark:text-text-primary-dark">
                      {node.ssh_config.port}
                    </span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-text-secondary-light dark:text-text-secondary-dark">Username</span>
                    <span className="text-sm font-mono text-text-primary-light dark:text-text-primary-dark">
                      {node.ssh_config.username}
                    </span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-text-secondary-light dark:text-text-secondary-dark">Auth</span>
                    <Badge variant="default">{node.ssh_config.auth_method.type}</Badge>
                  </div>
                </div>
              </div>
            )}

            <Button variant="primary" size="md" onClick={handleOpenTerminal}>
              <Terminal size={16} />
              {node.kind === "local" ? "Open Local Terminal" : "Open Terminal"}
            </Button>
          </Card>
        </div>
      )}

      {/* ── Dialogs ────────────────────────────────────────────────── */}
      <AssignBlueprintDialog
        open={showAssign}
        onClose={() => setShowAssign(false)}
        onSelect={handleAssign}
        blueprints={blueprints}
        currentBlueprintId={node.blueprint_id}
        loading={assigning}
      />
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
      <RunTaskDialog
        open={showRunTask}
        onClose={() => setShowRunTask(false)}
        onRun={handleRunTask}
        tasks={allTasks}
        taskStates={taskStates}
        runningTaskId={runningTaskId}
      />
      {nodeId && (
        <TaskExecutionDialog
          open={showExecution}
          onClose={() => setShowExecution(false)}
          nodeId={nodeId}
          taskId={executionTaskId}
          taskName={executionTaskName}
          isRunning={runningTaskId !== null}
        />
      )}
    </div>
  );
}
