import { useMemo, useState } from "react";
import { Play, Settings2, ListChecks, Eye, CheckCircle2, SkipForward, Terminal, FolderPlus, FileEdit } from "lucide-react";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { Badge } from "../ui/Badge";
import { ExecutionOutputPanel } from "../ui/ExecutionOutputPanel";
import * as api from "../../services/tauriCommands";
import { formatError } from "@quartermaster/core";
import type { Blueprint, BlueprintTaskEntry } from "@quartermaster/core";
import type { TaskInfo } from "@quartermaster/core";
import type { DryRunResult, DryRunTaskResult, DryRunAction } from "../../services/tauriCommands";

interface PreRunConfigDialogProps {
  open: boolean;
  onClose: () => void;
  onConfirm: () => void;
  blueprint: Blueprint | null;
  tasks: TaskInfo[];
  loading?: boolean;
  loadingMessage?: string;
  nodeId?: string;
}

export function PreRunConfigDialog({
  open,
  onClose,
  onConfirm,
  blueprint,
  tasks,
  loading,
  loadingMessage,
  nodeId,
}: PreRunConfigDialogProps) {
  const [dryRunResult, setDryRunResult] = useState<DryRunResult | null>(null);
  const [dryRunLoading, setDryRunLoading] = useState(false);
  const [dryRunError, setDryRunError] = useState<string | null>(null);

  const enabledEntries = useMemo(() => {
    if (!blueprint) return [];
    return [...blueprint.task_entries]
      .filter((entry) => entry.enabled)
      .sort((a, b) => a.order - b.order);
  }, [blueprint]);

  const taskLookup = useMemo(() => {
    const map = new Map<string, TaskInfo>();
    for (const t of tasks) {
      map.set(t.id, t);
    }
    return map;
  }, [tasks]);

  const handleDryRun = async () => {
    if (!nodeId || !blueprint) return;
    setDryRunLoading(true);
    setDryRunError(null);
    setDryRunResult(null);
    try {
      const result = await api.dryRunBlueprint(nodeId, blueprint.id);
      setDryRunResult(result);
    } catch (err) {
      setDryRunError(formatError(err));
    } finally {
      setDryRunLoading(false);
    }
  };

  const handleClose = () => {
    setDryRunResult(null);
    setDryRunError(null);
    onClose();
  };

  if (!blueprint) return null;

  // Dry-run summary stats
  const wouldRun = dryRunResult?.task_actions.filter((t) => t.status === "would_run").length ?? 0;
  const alreadyDone = dryRunResult?.task_actions.filter((t) => t.status === "already_completed").length ?? 0;
  const skipped = dryRunResult?.task_actions.filter((t) => t.status === "skipped").length ?? 0;

  return (
    <Dialog open={open} onClose={handleClose} title="Run Blueprint" className={loading ? "max-w-3xl" : undefined}>
      {/* Blueprint header */}
      <div className="mb-4">
        <div className="flex items-center gap-2">
          <h3 className="text-sm font-semibold text-text-primary-light dark:text-text-primary-dark">
            {blueprint.name}
          </h3>
          <Badge variant="info">v{blueprint.version}</Badge>
        </div>
        {blueprint.description && (
          <p className="mt-1 text-xs text-text-secondary-light dark:text-text-secondary-dark">
            {blueprint.description}
          </p>
        )}
      </div>

      {/* Task count summary */}
      <div className="flex items-center gap-1.5 mb-3 text-xs text-text-secondary-light dark:text-text-secondary-dark">
        <ListChecks size={14} />
        <span>
          {enabledEntries.length} {enabledEntries.length === 1 ? "task" : "tasks"} will be executed
        </span>
      </div>

      {/* Dry-run result */}
      {dryRunResult && (
        <div className="mb-4 space-y-3">
          {/* Summary badges */}
          <div className="flex items-center gap-2 flex-wrap">
            {wouldRun > 0 && (
              <Badge variant="info">{wouldRun} will run</Badge>
            )}
            {alreadyDone > 0 && (
              <Badge variant="success">{alreadyDone} already done</Badge>
            )}
            {skipped > 0 && (
              <Badge variant="default">{skipped} skipped</Badge>
            )}
          </div>

          {/* Per-task actions */}
          <div className="space-y-2 max-h-[40vh] overflow-y-auto">
            {dryRunResult.task_actions.map((taskResult) => (
              <DryRunTaskRow key={taskResult.task_id} result={taskResult} />
            ))}
          </div>
        </div>
      )}

      {/* Dry-run error */}
      {dryRunError && (
        <div className="mb-4 p-3 rounded-lg bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800">
          <p className="text-xs text-red-700 dark:text-red-300">{dryRunError}</p>
        </div>
      )}

      {/* Task list (only show when no dry-run result) */}
      {!dryRunResult && (
        <div className="space-y-2 max-h-[50vh] overflow-y-auto mb-4">
          {enabledEntries.map((entry) => (
            <TaskEntryRow
              key={entry.task_id}
              entry={entry}
              taskInfo={taskLookup.get(entry.task_id)}
            />
          ))}
          {enabledEntries.length === 0 && (
            <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark text-center py-4">
              No enabled tasks in this blueprint.
            </p>
          )}
        </div>
      )}

      {/* Loading message + real-time output */}
      {loading && loadingMessage && (
        <div className="mb-3 text-xs text-text-secondary-light dark:text-text-secondary-dark animate-pulse">
          {loadingMessage}
        </div>
      )}
      {loading && nodeId && (
        <div className="mb-3">
          <ExecutionOutputPanel
            nodeId={nodeId}
            isActive={!!loading}
          />
        </div>
      )}

      {/* Actions */}
      <div className="flex justify-end gap-2">
        <Button type="button" variant="ghost" onClick={handleClose}>
          Cancel
        </Button>
        {nodeId && !dryRunResult && (
          <Button
            variant="secondary"
            onClick={handleDryRun}
            disabled={enabledEntries.length === 0}
            loading={dryRunLoading}
          >
            <Eye size={14} />
            Preview Changes
          </Button>
        )}
        <Button
          onClick={onConfirm}
          disabled={enabledEntries.length === 0}
          loading={loading}
        >
          <Play size={14} />
          Run Blueprint
        </Button>
      </div>
    </Dialog>
  );
}

function DryRunTaskRow({ result }: { result: DryRunTaskResult }) {
  const statusIcon = result.status === "already_completed" ? (
    <CheckCircle2 size={14} className="text-green-500 shrink-0" />
  ) : result.status === "skipped" ? (
    <SkipForward size={14} className="text-gray-400 shrink-0" />
  ) : (
    <Play size={14} className="text-blue-500 shrink-0" />
  );

  const statusLabel = result.status === "already_completed"
    ? "Already done"
    : result.status === "skipped"
      ? "Skipped"
      : "Will run";

  return (
    <div className="p-3 rounded-lg border border-border-light dark:border-border-dark bg-surface-light dark:bg-surface-dark space-y-2">
      <div className="flex items-center gap-2">
        {statusIcon}
        <span className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark truncate">
          {result.task_name}
        </span>
        <Badge
          variant={
            result.status === "already_completed"
              ? "success"
              : result.status === "skipped"
                ? "default"
                : "info"
          }
        >
          {statusLabel}
        </Badge>
      </div>

      {result.actions.length > 0 && (
        <div className="ml-6 space-y-1">
          {result.actions.map((action, i) => (
            <ActionRow key={i} action={action} />
          ))}
        </div>
      )}
    </div>
  );
}

function ActionRow({ action }: { action: DryRunAction }) {
  if (action.type === "run_command") {
    return (
      <div className="flex items-start gap-1.5 text-xs text-text-secondary-light dark:text-text-secondary-dark">
        <Terminal size={11} className="mt-0.5 shrink-0" />
        <span className="font-mono truncate">
          {action.command} {action.args?.join(" ")}
        </span>
      </div>
    );
  }
  if (action.type === "write_file") {
    return (
      <div className="flex items-start gap-1.5 text-xs text-text-secondary-light dark:text-text-secondary-dark">
        <FileEdit size={11} className="mt-0.5 shrink-0" />
        <span className="font-mono truncate">
          Write {action.path} ({action.content_length} bytes)
        </span>
      </div>
    );
  }
  if (action.type === "create_dir") {
    return (
      <div className="flex items-start gap-1.5 text-xs text-text-secondary-light dark:text-text-secondary-dark">
        <FolderPlus size={11} className="mt-0.5 shrink-0" />
        <span className="font-mono truncate">
          Create directory {action.path}
        </span>
      </div>
    );
  }
  return null;
}

function TaskEntryRow({
  entry,
  taskInfo,
}: {
  entry: BlueprintTaskEntry;
  taskInfo?: TaskInfo;
}) {
  const overrideKeys = Object.keys(entry.config_overrides);
  const name = taskInfo?.name ?? entry.task_id;
  const description = taskInfo?.description ?? "";

  return (
    <div className="flex flex-col gap-1.5 p-3 rounded-lg border border-border-light dark:border-border-dark bg-surface-light dark:bg-surface-dark">
      <div className="flex items-center gap-2">
        <div className="p-1.5 rounded-md bg-warm-100 dark:bg-warm-900/30 text-warm-600 dark:text-warm-400">
          <Settings2 size={14} />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark truncate">
              {name}
            </span>
            {!entry.enabled && <Badge variant="default">Disabled</Badge>}
          </div>
          {description && (
            <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark truncate mt-0.5">
              {description}
            </p>
          )}
        </div>
      </div>

      {overrideKeys.length > 0 && (
        <div className="ml-8 flex flex-wrap gap-1.5">
          {overrideKeys.map((key) => (
            <span
              key={key}
              className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] bg-warm-100/60 dark:bg-warm-900/20 text-text-secondary-light dark:text-text-secondary-dark"
            >
              <span className="font-medium">{key}:</span>
              <span>{String(entry.config_overrides[key])}</span>
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
