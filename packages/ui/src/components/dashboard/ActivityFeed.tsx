import { useState, useEffect, useCallback } from "react";
import { RefreshCw, Trash2, CheckCircle2, XCircle } from "lucide-react";
import { Button } from "../ui/Button";
import { Card, CardHeader, CardTitle } from "../ui/Card";
import { useToastStore } from "../../stores/toastStore";
import { formatError } from "@quartermaster/core";
import * as api from "../../services/tauriCommands";

interface ActivityFeedProps {
  maxEntries?: number;
}

function timeAgo(timestamp: string): string {
  const now = Date.now();
  const then = new Date(timestamp).getTime();
  const seconds = Math.floor((now - then) / 1000);

  if (seconds < 60) return "just now";

  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} ${minutes === 1 ? "minute" : "minutes"} ago`;

  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} ${hours === 1 ? "hour" : "hours"} ago`;

  const days = Math.floor(hours / 24);
  if (days < 30) return `${days} ${days === 1 ? "day" : "days"} ago`;

  const months = Math.floor(days / 30);
  return `${months} ${months === 1 ? "month" : "months"} ago`;
}

function formatAction(action: string): string {
  const words = action.replace(/_/g, " ");
  return words.charAt(0).toUpperCase() + words.slice(1);
}

export function ActivityFeed({ maxEntries = 20 }: ActivityFeedProps) {
  const [entries, setEntries] = useState<api.ActivityEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const { addToast } = useToastStore();

  const loadActivity = useCallback(async () => {
    setLoading(true);
    try {
      const result = await api.getActivityLog(maxEntries);
      setEntries(result);
    } catch (err) {
      addToast({ type: "error", title: "Failed to load activity log", message: formatError(err) });
    } finally {
      setLoading(false);
    }
  }, [maxEntries, addToast]);

  useEffect(() => {
    loadActivity();
  }, [loadActivity]);

  const handleClear = useCallback(async () => {
    if (!window.confirm("Clear all activity log entries?")) return;
    try {
      await api.clearActivityLog();
      setEntries([]);
      addToast({ type: "success", title: "Activity log cleared" });
    } catch (err) {
      addToast({ type: "error", title: "Failed to clear activity log", message: formatError(err) });
    }
  }, [addToast]);

  return (
    <Card padding={false}>
      <CardHeader className="px-5 pt-5 pb-0">
        <div className="flex items-center justify-between">
          <CardTitle>Recent Activity</CardTitle>
          <div className="flex items-center gap-1">
            <Button variant="ghost" size="sm" onClick={loadActivity} disabled={loading}>
              <RefreshCw size={14} className={loading ? "animate-spin" : ""} />
              Refresh
            </Button>
            {entries.length > 0 && (
              <Button variant="ghost" size="sm" onClick={handleClear}>
                <Trash2 size={14} />
                Clear
              </Button>
            )}
          </div>
        </div>
      </CardHeader>
      <div className="px-5 pb-5">
        {entries.length === 0 ? (
          <div className="py-8 text-center">
            <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark">
              No activity recorded yet.
            </p>
          </div>
        ) : (
          <div className="space-y-0">
            {entries.map((entry) => (
              <div
                key={entry.id}
                className="flex items-start gap-3 py-3 border-b border-border-light dark:border-border-dark last:border-b-0"
              >
                {/* Status dot */}
                <div className="mt-1.5 shrink-0">
                  <div
                    className={`w-2.5 h-2.5 rounded-full ${
                      entry.success ? "bg-green-500" : "bg-red-500"
                    }`}
                  />
                </div>

                {/* Content */}
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    {entry.success ? (
                      <CheckCircle2 size={14} className="text-green-500 shrink-0" />
                    ) : (
                      <XCircle size={14} className="text-red-500 shrink-0" />
                    )}
                    <span className="text-sm text-text-primary-light dark:text-text-primary-dark">
                      {formatAction(entry.action)}
                    </span>
                    <span className="text-sm font-bold text-text-primary-light dark:text-text-primary-dark truncate">
                      {entry.target}
                    </span>
                  </div>
                  {entry.detail && (
                    <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark mt-0.5 ml-[22px]">
                      {entry.detail}
                    </p>
                  )}
                </div>

                {/* Timestamp */}
                <span className="text-xs text-text-secondary-light dark:text-text-secondary-dark whitespace-nowrap shrink-0 mt-0.5">
                  {timeAgo(entry.timestamp)}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
    </Card>
  );
}
