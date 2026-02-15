import { useState, useEffect, useCallback } from "react";
import { Layers, CheckCircle2, XCircle, Loader2 } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import type { Node } from "@quartermaster/core";
import type { Blueprint } from "@quartermaster/core";
import type { BulkApplyResult } from "../../services/tauriCommands";
import { applyBlueprintBulk, listBlueprints } from "../../services/tauriCommands";
import { formatError } from "@quartermaster/core";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { Badge } from "../ui/Badge";

interface BulkBlueprintDialogProps {
  open: boolean;
  onClose: () => void;
  selectedNodeIds: string[];
  nodes: Node[];
}

type Phase = "pick" | "running" | "done";

interface ProgressPayload {
  completed: number;
  total: number;
  current_node_id: string | null;
  current_node_name: string | null;
}

export function BulkBlueprintDialog({ open, onClose, selectedNodeIds }: BulkBlueprintDialogProps) {
  const [blueprints, setBlueprints] = useState<Blueprint[]>([]);
  const [loadingBlueprints, setLoadingBlueprints] = useState(false);
  const [phase, setPhase] = useState<Phase>("pick");
  const [selectedBlueprintId, setSelectedBlueprintId] = useState<string | null>(null);
  const [progress, setProgress] = useState<ProgressPayload>({ completed: 0, total: 0, current_node_id: null, current_node_name: null });
  const [results, setResults] = useState<BulkApplyResult[]>([]);
  const [error, setError] = useState<string | null>(null);

  // Load blueprints when dialog opens
  useEffect(() => {
    if (!open) return;
    setPhase("pick");
    setSelectedBlueprintId(null);
    setResults([]);
    setError(null);
    setProgress({ completed: 0, total: 0, current_node_id: null, current_node_name: null });

    setLoadingBlueprints(true);
    listBlueprints()
      .then(setBlueprints)
      .catch(() => {
        setBlueprints([]);
        setError("Failed to load blueprints");
      })
      .finally(() => setLoadingBlueprints(false));
  }, [open]);

  // Listen for bulk progress events
  useEffect(() => {
    if (!open || phase !== "running") return;

    let cancelled = false;
    let unlistenFn: (() => void) | null = null;

    listen<ProgressPayload>("bulk-blueprint-progress", (event) => {
      if (!cancelled) {
        setProgress(event.payload);
      }
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlistenFn = fn;
      }
    });

    return () => {
      cancelled = true;
      if (unlistenFn) unlistenFn();
    };
  }, [open, phase]);

  const handleRunBlueprint = useCallback(async () => {
    if (!selectedBlueprintId) return;
    setPhase("running");
    setError(null);
    setProgress({ completed: 0, total: selectedNodeIds.length, current_node_id: null, current_node_name: null });

    try {
      const bulkResults = await applyBlueprintBulk(selectedNodeIds, selectedBlueprintId);
      setResults(bulkResults);
      setPhase("done");
    } catch (err) {
      setError(formatError(err));
      setPhase("done");
    }
  }, [selectedBlueprintId, selectedNodeIds]);

  const selectedBlueprint = blueprints.find((b) => b.id === selectedBlueprintId);
  const succeededCount = results.filter((r) => r.success).length;
  const failedCount = results.filter((r) => !r.success).length;

  const handleClose = () => {
    if (phase === "running") return; // Prevent closing while running
    onClose();
  };

  return (
    <Dialog open={open} onClose={handleClose} title="Run Blueprint on Selected Nodes" className="max-w-xl">
      {/* Phase 1: Pick a blueprint */}
      {phase === "pick" && (
        <div className="space-y-4">
          <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark">
            Select a blueprint to apply to {selectedNodeIds.length} {selectedNodeIds.length === 1 ? "node" : "nodes"}.
          </p>

          {loadingBlueprints ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 size={20} className="animate-spin text-text-secondary-light dark:text-text-secondary-dark" />
            </div>
          ) : blueprints.length === 0 ? (
            <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark py-4 text-center">
              No blueprints available. Create one first.
            </p>
          ) : (
            <div className="max-h-64 overflow-y-auto space-y-1 border border-border-light dark:border-border-dark rounded-lg p-2">
              {blueprints.map((bp) => (
                <button
                  key={bp.id}
                  onClick={() => setSelectedBlueprintId(bp.id)}
                  className={`w-full flex items-center gap-3 p-3 rounded-lg text-left transition-colors ${
                    selectedBlueprintId === bp.id
                      ? "bg-warm-100 dark:bg-warm-900/40 ring-2 ring-warm-400/50"
                      : "hover:bg-warm-50 dark:hover:bg-warm-900/20"
                  }`}
                >
                  <div className="p-2 rounded-lg bg-warm-100 dark:bg-warm-900/30 text-warm-600 dark:text-warm-400">
                    <Layers size={16} />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark truncate">
                      {bp.name}
                    </div>
                    <div className="text-xs text-text-secondary-light dark:text-text-secondary-dark truncate">
                      {bp.description || `${bp.task_entries.length} tasks`}
                    </div>
                  </div>
                  {bp.is_builtin && (
                    <Badge variant="info">Built-in</Badge>
                  )}
                </button>
              ))}
            </div>
          )}

          <div className="flex justify-end gap-2 pt-2">
            <Button variant="ghost" onClick={handleClose}>
              Cancel
            </Button>
            <Button
              onClick={handleRunBlueprint}
              disabled={!selectedBlueprintId}
            >
              Apply Blueprint
            </Button>
          </div>
        </div>
      )}

      {/* Phase 2: Running -- show progress */}
      {phase === "running" && (
        <div className="space-y-4">
          <div className="text-center py-4">
            <Loader2 size={24} className="animate-spin text-warm-500 mx-auto mb-3" />
            <p className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark">
              Applying "{selectedBlueprint?.name}" to node {Math.min(progress.completed + 1, progress.total)} of {progress.total}...
            </p>
            {progress.current_node_name && (
              <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark mt-1">
                Current: {progress.current_node_name}
              </p>
            )}
          </div>

          {/* Progress bar */}
          <div className="w-full bg-warm-100 dark:bg-warm-900/30 rounded-full h-2 overflow-hidden">
            <div
              className="bg-warm-400 h-2 rounded-full transition-all duration-300"
              style={{ width: `${progress.total > 0 ? (progress.completed / progress.total) * 100 : 0}%` }}
            />
          </div>

          <p className="text-xs text-center text-text-secondary-light dark:text-text-secondary-dark">
            {progress.completed} of {progress.total} completed
          </p>
        </div>
      )}

      {/* Phase 3: Done -- show results */}
      {phase === "done" && (
        <div className="space-y-4">
          {error ? (
            <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-3">
              <p className="text-sm text-red-700 dark:text-red-300">{error}</p>
            </div>
          ) : (
            <div className="flex items-center justify-center gap-4 py-3">
              {succeededCount > 0 && (
                <div className="flex items-center gap-1.5">
                  <CheckCircle2 size={16} className="text-green-500" />
                  <span className="text-sm font-medium text-green-700 dark:text-green-300">
                    {succeededCount} succeeded
                  </span>
                </div>
              )}
              {failedCount > 0 && (
                <div className="flex items-center gap-1.5">
                  <XCircle size={16} className="text-red-500" />
                  <span className="text-sm font-medium text-red-700 dark:text-red-300">
                    {failedCount} failed
                  </span>
                </div>
              )}
            </div>
          )}

          {/* Per-node results table */}
          {results.length > 0 && (
            <div className="max-h-64 overflow-y-auto border border-border-light dark:border-border-dark rounded-lg">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-border-light dark:border-border-dark">
                    <th className="text-left py-2 px-3 text-text-secondary-light dark:text-text-secondary-dark font-medium">Node</th>
                    <th className="text-left py-2 px-3 text-text-secondary-light dark:text-text-secondary-dark font-medium">Status</th>
                    <th className="text-left py-2 px-3 text-text-secondary-light dark:text-text-secondary-dark font-medium">Detail</th>
                  </tr>
                </thead>
                <tbody>
                  {results.map((result) => (
                    <tr key={result.node_id} className="border-b border-border-light/50 dark:border-border-dark/50 last:border-b-0">
                      <td className="py-2 px-3 text-text-primary-light dark:text-text-primary-dark truncate max-w-[140px]">
                        {result.node_name}
                      </td>
                      <td className="py-2 px-3">
                        {result.success ? (
                          <Badge variant="success">Success</Badge>
                        ) : (
                          <Badge variant="danger">Failed</Badge>
                        )}
                      </td>
                      <td className="py-2 px-3 text-xs text-text-secondary-light dark:text-text-secondary-dark truncate max-w-[200px]">
                        {result.error || "--"}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}

          <div className="flex justify-end pt-2">
            <Button onClick={handleClose}>
              Close
            </Button>
          </div>
        </div>
      )}
    </Dialog>
  );
}
