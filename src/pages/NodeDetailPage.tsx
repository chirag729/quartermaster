import { useEffect, useState, useCallback } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { ArrowLeft, Server, Monitor, Layers, CheckCircle2, Circle, XCircle } from "lucide-react";
import { useToastStore } from "../stores/toastStore";
import { useBlueprintStore } from "../stores/blueprintStore";
import { NodeStatusBadge } from "../components/fleet/NodeStatusBadge";
import { AssignBlueprintDialog } from "../components/blueprints/AssignBlueprintDialog";
import { Badge } from "../components/ui/Badge";
import { Button } from "../components/ui/Button";
import { Card, CardHeader, CardTitle, CardDescription } from "../components/ui/Card";
import { Skeleton } from "../components/ui/Skeleton";
import { ToastContainer } from "../components/ui/Toast";
import * as api from "../services/tauriCommands";
import type { Node } from "../types/node";
import type { Blueprint } from "../types/blueprint";
import type { TaskStatus } from "../types/task";

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
  const [taskStates, setTaskStates] = useState<Record<string, TaskStatus>>({});

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
      addToast({ type: "error", title: "Failed to load node", message: String(err) });
    } finally {
      setLoading(false);
    }
  }, [nodeId, addToast]);

  useEffect(() => {
    loadNode();
  }, [loadNode]);

  // Load blueprints for the assign dialog
  useEffect(() => {
    api.listBlueprints().then(setBlueprints).catch(() => {});
  }, [setBlueprints]);

  // Detect task states for blueprint tasks when a blueprint is assigned
  useEffect(() => {
    if (!blueprint || !node) return;
    // Only detect for local nodes
    if (node.kind !== "local") return;

    api.detectAllStates().then((tasks) => {
      const stateMap: Record<string, TaskStatus> = {};
      for (const t of tasks) {
        stateMap[t.id] = t.status;
      }
      setTaskStates(stateMap);
    }).catch(() => {});
  }, [blueprint, node]);

  const handleAssign = async (blueprintId: string) => {
    if (!nodeId) return;
    setAssigning(true);
    try {
      await assignBlueprint(nodeId, blueprintId);
      addToast({ type: "success", title: "Blueprint assigned" });
      setShowAssign(false);
      loadNode();
    } catch (err) {
      addToast({ type: "error", title: "Failed to assign blueprint", message: String(err) });
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
      loadNode();
    } catch (err) {
      addToast({ type: "error", title: "Failed to unassign blueprint", message: String(err) });
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

  // Compute task completion stats
  const enabledEntries = blueprint?.task_entries.filter((e) => e.enabled) || [];
  const completedCount = enabledEntries.filter((e) => taskStates[e.task_id] === "completed").length;

  function StatusIcon({ status }: { status?: TaskStatus }) {
    if (status === "completed") return <CheckCircle2 size={14} className="text-green-500" />;
    if (status === "failed") return <XCircle size={14} className="text-red-500" />;
    return <Circle size={14} className="text-text-secondary-light/40 dark:text-text-secondary-dark/40" />;
  }

  return (
    <div>
      <Button variant="ghost" size="sm" onClick={() => navigate("/fleet")} className="mb-4">
        <ArrowLeft size={14} />
        Back to Fleet
      </Button>
      <div className="flex items-center gap-3 mb-6">
        <div className="p-2.5 rounded-lg bg-warm-100 dark:bg-warm-900/30 text-warm-600 dark:text-warm-400">
          <KindIcon size={20} />
        </div>
        <div>
          <h1 className="text-xl font-semibold text-text-primary-light dark:text-text-primary-dark">
            {node.name}
          </h1>
          <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark">
            {node.hostname}
          </p>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <Card>
          <CardHeader>
            <CardTitle>Status</CardTitle>
          </CardHeader>
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-sm text-text-secondary-light dark:text-text-secondary-dark">Connection</span>
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
            {node.last_seen && (
              <div className="flex items-center justify-between">
                <span className="text-sm text-text-secondary-light dark:text-text-secondary-dark">Last Seen</span>
                <span className="text-sm text-text-primary-light dark:text-text-primary-dark">{node.last_seen}</span>
              </div>
            )}
          </div>
        </Card>

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
            <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark">No tags assigned.</p>
          )}
        </Card>

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
                <Badge variant="default">{blueprint.task_entries.length} tasks</Badge>
              </div>

              {/* Task completion status */}
              {enabledEntries.length > 0 && Object.keys(taskStates).length > 0 && (
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
                      style={{ width: `${enabledEntries.length > 0 ? (completedCount / enabledEntries.length) * 100 : 0}%` }}
                    />
                  </div>
                  <div className="space-y-1">
                    {enabledEntries.map((entry) => (
                      <div key={entry.task_id} className="flex items-center gap-2">
                        <StatusIcon status={taskStates[entry.task_id]} />
                        <span className="text-xs text-text-primary-light dark:text-text-primary-dark">
                          {entry.task_id}
                        </span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              <div className="flex gap-2">
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => setShowAssign(true)}
                >
                  Change Blueprint
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={handleUnassign}
                >
                  Unassign
                </Button>
              </div>
            </div>
          ) : (
            <div className="border border-dashed border-border-light dark:border-border-dark rounded-lg p-6 text-center">
              <Layers size={24} className="mx-auto mb-2 text-text-secondary-light/40 dark:text-text-secondary-dark/40" />
              <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark mb-3">
                No blueprint assigned to this node.
              </p>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => setShowAssign(true)}
              >
                Assign Blueprint
              </Button>
            </div>
          )}
        </Card>
      </div>
      <AssignBlueprintDialog
        open={showAssign}
        onClose={() => setShowAssign(false)}
        onSelect={handleAssign}
        blueprints={blueprints}
        currentBlueprintId={node.blueprint_id}
        loading={assigning}
      />
      <ToastContainer />
    </div>
  );
}
