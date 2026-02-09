import { useEffect, useState } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { ArrowLeft, Shield, Globe, Link2, Trash2, type LucideIcon } from "lucide-react";
import * as Icons from "lucide-react";
import { Card, CardHeader, CardTitle } from "../components/ui/Card";
import { Badge } from "../components/ui/Badge";
import { Button } from "../components/ui/Button";
import { Skeleton } from "../components/ui/Skeleton";
import { StatusIndicator } from "../components/layout/StatusIndicator";
import { useToastStore } from "../stores/toastStore";
import { useTaskStore } from "../stores/taskStore";
import { useTauriEvent } from "../hooks/useTauriEvent";
import { formatError } from "../lib/formatError";
import * as api from "../services/tauriCommands";
import type { TaskInfo } from "../types/task";

function getIcon(iconName: string): LucideIcon {
  const icons = Icons as unknown as Record<string, LucideIcon>;
  return icons[iconName] || Icons.Box;
}

const targetLabels: Record<string, string> = {
  local_only: "Local only",
  remote_only: "Remote only",
  any: "Local & Remote",
};

export function TaskDetailPage() {
  const { taskId } = useParams<{ taskId: string }>();
  const navigate = useNavigate();
  const { addToast } = useToastStore();
  const { uninstalling, setUninstalling } = useTaskStore();
  const [task, setTask] = useState<TaskInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [confirmUninstall, setConfirmUninstall] = useState(false);
  const [uninstallProgress, setUninstallProgress] = useState<string | null>(null);

  const loadTask = () => {
    if (!taskId) return;
    setLoading(true);
    api.listTasks()
      .then((tasks) => {
        const found = tasks.find((t) => t.id === taskId);
        setTask(found ?? null);
      })
      .catch((err) => {
        addToast({ type: "error", title: "Failed to load task", message: formatError(err) });
      })
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    loadTask();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [taskId]);

  // Listen for task progress during uninstall
  useTauriEvent<{ task_id: string; progress: number; message: string }>(
    "task-progress",
    (payload) => {
      if (payload.task_id === taskId && uninstalling === taskId) {
        setUninstallProgress(payload.message);
      }
    },
  );

  const handleUninstall = async () => {
    if (!taskId) return;
    setConfirmUninstall(false);
    setUninstalling(taskId);
    setUninstallProgress("Starting uninstall...");
    try {
      await api.uninstallTask(taskId);
      addToast({ type: "success", title: "Task uninstalled", message: `${task?.name ?? taskId} has been uninstalled.` });
      loadTask();
    } catch (err) {
      addToast({ type: "error", title: "Uninstall failed", message: formatError(err) });
    } finally {
      setUninstalling(null);
      setUninstallProgress(null);
    }
  };

  const isInstalled = task?.status === "completed";
  const canUninstall = task?.supports_uninstall && isInstalled && uninstalling !== taskId;

  if (loading) {
    return (
      <div className="space-y-4">
        <Skeleton className="h-8 w-48" rounded="lg" />
        <Skeleton className="h-64 w-full" rounded="xl" />
      </div>
    );
  }

  if (!task) {
    return (
      <div>
        <Button variant="ghost" size="sm" onClick={() => navigate("/tasks")}>
          <ArrowLeft size={14} />
          Back to Task Library
        </Button>
        <p className="mt-4 text-sm text-text-secondary-light dark:text-text-secondary-dark">
          Task not found.
        </p>
      </div>
    );
  }

  const Icon = getIcon(task.icon);
  const steps = task.steps ?? [];
  const uninstallSteps = task.uninstall_steps ?? [];
  const hasConfig = task.config_schema.length > 0;
  const hasSteps = steps.length > 0;
  const hasUninstallSteps = uninstallSteps.length > 0;
  const hasDependencies = task.depends_on.length > 0;
  const hasAppArmor = !!task.apparmor;
  const hasDownload = !!task.download;
  const hasDesktop = !!task.desktop;
  const hasVariables = (task.variables ?? []).length > 0;

  return (
    <div>
      <Button variant="ghost" size="sm" onClick={() => navigate("/tasks")} className="mb-4">
        <ArrowLeft size={14} />
        Back to Task Library
      </Button>

      {/* Header */}
      <div className="flex items-start gap-3 mb-6">
        <div className="p-2.5 rounded-lg bg-warm-100 dark:bg-warm-900/30 text-warm-600 dark:text-warm-400">
          <Icon size={24} />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            <h1 className="text-xl font-semibold text-text-primary-light dark:text-text-primary-dark">
              {task.name}
            </h1>
            <Badge>{task.category}</Badge>
            {task.privilege_level === "admin" && <Badge variant="warning">Admin</Badge>}
            {task.execution_target !== "any" && (
              <Badge variant="info">{targetLabels[task.execution_target]}</Badge>
            )}
            {task.tags.map((tag) => (
              <Badge key={tag} variant="default">{tag}</Badge>
            ))}
          </div>
          <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark mt-1">
            {task.description}
          </p>
        </div>
      </div>

      {/* 3-column grid */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        {/* Main content - spans 2 columns */}
        <div className="lg:col-span-2 space-y-4">
          {/* Installation Steps */}
          {hasSteps && (
            <Card>
              <CardHeader>
                <CardTitle>Installation Steps</CardTitle>
              </CardHeader>
              <div className="space-y-2">
                {steps.map((step, idx) => (
                  <div
                    key={idx}
                    className="flex items-center gap-3 p-3 rounded-lg border border-border-light dark:border-border-dark bg-surface-light dark:bg-surface-dark"
                  >
                    <span className="text-xs font-mono text-text-secondary-light dark:text-text-secondary-dark w-6 text-center shrink-0">
                      {idx + 1}
                    </span>
                    <span className="text-sm text-text-primary-light dark:text-text-primary-dark flex-1">
                      {step.name}
                    </span>
                    <div className="flex items-center gap-2 shrink-0">
                      <div className="w-16 h-1.5 rounded-full bg-warm-100 dark:bg-warm-900/30 overflow-hidden">
                        <div
                          className="h-full rounded-full bg-warm-400"
                          style={{ width: `${step.progress}%` }}
                        />
                      </div>
                      <span className="text-xs font-mono text-text-secondary-light dark:text-text-secondary-dark w-8 text-right">
                        {step.progress}%
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            </Card>
          )}

          {/* Uninstall Steps */}
          {hasUninstallSteps && (
            <Card>
              <CardHeader>
                <CardTitle>Uninstall Steps</CardTitle>
              </CardHeader>
              <div className="space-y-2">
                {uninstallSteps.map((step, idx) => (
                  <div
                    key={idx}
                    className="flex items-center gap-3 p-3 rounded-lg border border-border-light dark:border-border-dark bg-surface-light dark:bg-surface-dark"
                  >
                    <span className="text-xs font-mono text-text-secondary-light dark:text-text-secondary-dark w-6 text-center shrink-0">
                      {idx + 1}
                    </span>
                    <span className="text-sm text-text-primary-light dark:text-text-primary-dark flex-1">
                      {step.name}
                    </span>
                    <div className="flex items-center gap-2 shrink-0">
                      <div className="w-16 h-1.5 rounded-full bg-warm-100 dark:bg-warm-900/30 overflow-hidden">
                        <div
                          className="h-full rounded-full bg-warm-400"
                          style={{ width: `${step.progress}%` }}
                        />
                      </div>
                      <span className="text-xs font-mono text-text-secondary-light dark:text-text-secondary-dark w-8 text-right">
                        {step.progress}%
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            </Card>
          )}

          {/* Configuration Schema */}
          {hasConfig && (
            <Card>
              <CardHeader>
                <CardTitle>Configuration</CardTitle>
              </CardHeader>
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-border-light dark:border-border-dark">
                      <th className="text-left py-2 pr-4 font-medium text-text-secondary-light dark:text-text-secondary-dark">Key</th>
                      <th className="text-left py-2 pr-4 font-medium text-text-secondary-light dark:text-text-secondary-dark">Label</th>
                      <th className="text-left py-2 pr-4 font-medium text-text-secondary-light dark:text-text-secondary-dark">Type</th>
                      <th className="text-left py-2 pr-4 font-medium text-text-secondary-light dark:text-text-secondary-dark">Default</th>
                      <th className="text-left py-2 font-medium text-text-secondary-light dark:text-text-secondary-dark">Required</th>
                    </tr>
                  </thead>
                  <tbody>
                    {task.config_schema.map((field) => (
                      <tr key={field.key} className="border-b border-border-light/50 dark:border-border-dark/50 last:border-0">
                        <td className="py-2 pr-4 font-mono text-xs text-text-primary-light dark:text-text-primary-dark">{field.key}</td>
                        <td className="py-2 pr-4 text-text-primary-light dark:text-text-primary-dark">{field.label}</td>
                        <td className="py-2 pr-4">
                          <Badge>{field.field_type}</Badge>
                        </td>
                        <td className="py-2 pr-4 font-mono text-xs text-text-secondary-light dark:text-text-secondary-dark">
                          {field.default_value || "\u2014"}
                        </td>
                        <td className="py-2">
                          {field.required ? (
                            <Badge variant="warning">Required</Badge>
                          ) : (
                            <span className="text-text-secondary-light dark:text-text-secondary-dark">Optional</span>
                          )}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </Card>
          )}

          {/* Variables */}
          {hasVariables && (
            <Card>
              <CardHeader>
                <CardTitle>Variables</CardTitle>
              </CardHeader>
              <div className="flex flex-wrap gap-2">
                {(task.variables ?? []).map((v) => (
                  <Badge key={v} variant="info">{v}</Badge>
                ))}
              </div>
            </Card>
          )}

          {/* Download Info */}
          {hasDownload && task.download && (
            <Card>
              <CardHeader>
                <CardTitle>Download</CardTitle>
              </CardHeader>
              <div className="space-y-2 text-sm">
                <div className="flex items-start gap-2">
                  <span className="text-text-secondary-light dark:text-text-secondary-dark shrink-0 w-20">URL</span>
                  <span className="font-mono text-xs text-text-primary-light dark:text-text-primary-dark break-all">
                    {task.download.url}
                  </span>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-text-secondary-light dark:text-text-secondary-dark shrink-0 w-20">Extract</span>
                  <Badge>{task.download.extract}</Badge>
                </div>
                {task.download.checksum_sha256 && (
                  <div className="flex items-start gap-2">
                    <span className="text-text-secondary-light dark:text-text-secondary-dark shrink-0 w-20">SHA-256</span>
                    <span className="font-mono text-xs text-text-primary-light dark:text-text-primary-dark break-all">
                      {task.download.checksum_sha256}
                    </span>
                  </div>
                )}
              </div>
            </Card>
          )}

          {/* Desktop Entry */}
          {hasDesktop && task.desktop && (
            <Card>
              <CardHeader>
                <CardTitle>Desktop Entry</CardTitle>
              </CardHeader>
              <div className="space-y-2 text-sm">
                <div className="flex items-center gap-2">
                  <span className="text-text-secondary-light dark:text-text-secondary-dark shrink-0 w-24">Name</span>
                  <span className="text-text-primary-light dark:text-text-primary-dark">{task.desktop.name}</span>
                </div>
                <div className="flex items-start gap-2">
                  <span className="text-text-secondary-light dark:text-text-secondary-dark shrink-0 w-24">Exec</span>
                  <span className="font-mono text-xs text-text-primary-light dark:text-text-primary-dark break-all">
                    {task.desktop.exec}
                  </span>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-text-secondary-light dark:text-text-secondary-dark shrink-0 w-24">Categories</span>
                  <div className="flex flex-wrap gap-1">
                    {task.desktop.categories.map((cat) => (
                      <Badge key={cat}>{cat}</Badge>
                    ))}
                  </div>
                </div>
              </div>
            </Card>
          )}
        </div>

        {/* Sidebar */}
        <div className="space-y-4">
          {/* Properties */}
          <Card>
            <CardHeader>
              <CardTitle>Properties</CardTitle>
            </CardHeader>
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-sm text-text-secondary-light dark:text-text-secondary-dark">Status</span>
                <StatusIndicator status={task.status} />
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm text-text-secondary-light dark:text-text-secondary-dark">Privilege</span>
                <div className="flex items-center gap-1.5">
                  <Shield size={14} className={task.privilege_level === "admin" ? "text-yellow-500" : "text-text-secondary-light dark:text-text-secondary-dark"} />
                  <span className="text-sm text-text-primary-light dark:text-text-primary-dark capitalize">
                    {task.privilege_level}
                  </span>
                </div>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm text-text-secondary-light dark:text-text-secondary-dark">Target</span>
                <div className="flex items-center gap-1.5">
                  <Globe size={14} className="text-text-secondary-light dark:text-text-secondary-dark" />
                  <span className="text-sm text-text-primary-light dark:text-text-primary-dark">
                    {targetLabels[task.execution_target]}
                  </span>
                </div>
              </div>
              {task.version && (
                <div className="flex items-center justify-between">
                  <span className="text-sm text-text-secondary-light dark:text-text-secondary-dark">Version</span>
                  <span className="text-sm font-mono text-text-primary-light dark:text-text-primary-dark">
                    {task.version}
                  </span>
                </div>
              )}
              {task.installed_version && (
                <div className="flex items-center justify-between">
                  <span className="text-sm text-text-secondary-light dark:text-text-secondary-dark">Installed</span>
                  <span className="text-sm font-mono text-text-primary-light dark:text-text-primary-dark">
                    {task.installed_version}
                  </span>
                </div>
              )}
              <div className="flex items-center justify-between">
                <span className="text-sm text-text-secondary-light dark:text-text-secondary-dark">Uninstall</span>
                <span className="text-sm text-text-primary-light dark:text-text-primary-dark">
                  {task.supports_uninstall ? "Supported" : "Not supported"}
                </span>
              </div>
            </div>

            {/* Uninstall action */}
            {task.supports_uninstall && (
              <div className="pt-3 border-t border-border-light dark:border-border-dark">
                {uninstalling === task.id ? (
                  <div className="space-y-2">
                    <Button variant="danger" size="sm" className="w-full" disabled>
                      Uninstalling...
                    </Button>
                    {uninstallProgress && (
                      <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark text-center">
                        {uninstallProgress}
                      </p>
                    )}
                  </div>
                ) : confirmUninstall ? (
                  <div className="space-y-2">
                    <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark">
                      This will reverse the installation and remove all files added by this task.
                    </p>
                    <div className="flex gap-2">
                      <Button variant="danger" size="sm" className="flex-1" onClick={handleUninstall}>
                        Confirm
                      </Button>
                      <Button variant="ghost" size="sm" className="flex-1" onClick={() => setConfirmUninstall(false)}>
                        Cancel
                      </Button>
                    </div>
                  </div>
                ) : (
                  <Button
                    variant="danger"
                    size="sm"
                    className="w-full"
                    disabled={!canUninstall}
                    onClick={() => setConfirmUninstall(true)}
                  >
                    <Trash2 size={14} />
                    {isInstalled ? "Uninstall" : "Not installed"}
                  </Button>
                )}
              </div>
            )}
          </Card>

          {/* Dependencies */}
          {hasDependencies && (
            <Card>
              <CardHeader>
                <CardTitle>Dependencies</CardTitle>
              </CardHeader>
              <div className="space-y-2">
                {task.depends_on.map((dep) => (
                  <button
                    key={dep}
                    onClick={() => navigate(`/tasks/${dep}`)}
                    className="flex items-center gap-2 w-full p-2 rounded-lg border border-border-light dark:border-border-dark bg-surface-light dark:bg-surface-dark hover:bg-warm-50 dark:hover:bg-warm-900/20 transition-colors text-left"
                  >
                    <Link2 size={14} className="text-warm-500 dark:text-warm-300 shrink-0" />
                    <span className="text-sm text-text-primary-light dark:text-text-primary-dark truncate">
                      {dep}
                    </span>
                  </button>
                ))}
              </div>
            </Card>
          )}

          {/* AppArmor */}
          {hasAppArmor && task.apparmor && (
            <Card>
              <CardHeader>
                <CardTitle>AppArmor</CardTitle>
              </CardHeader>
              <div className="space-y-3">
                <div>
                  <span className="text-xs font-medium text-text-secondary-light dark:text-text-secondary-dark uppercase tracking-wider">
                    Profile
                  </span>
                  <p className="text-sm font-mono text-text-primary-light dark:text-text-primary-dark mt-0.5">
                    {task.apparmor.profile}
                  </p>
                </div>
                {task.apparmor.abstractions.length > 0 && (
                  <div>
                    <span className="text-xs font-medium text-text-secondary-light dark:text-text-secondary-dark uppercase tracking-wider">
                      Abstractions
                    </span>
                    <div className="flex flex-wrap gap-1 mt-1">
                      {task.apparmor.abstractions.map((abs) => (
                        <Badge key={abs} variant="info">{abs}</Badge>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            </Card>
          )}
        </div>
      </div>
    </div>
  );
}
