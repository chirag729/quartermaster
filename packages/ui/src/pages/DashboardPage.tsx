import { useMemo } from "react";
import { useNavigate } from "react-router-dom";
import {
  Network,
  Server,
  Monitor,
  Layers,
  ListChecks,
  ChevronRight,
  Activity,
  AlertTriangle,
  CheckCircle2,
  Circle,
} from "lucide-react";
import { motion } from "motion/react";
import { useFleetStore, useBlueprintStore } from "@/hooks/useStore";
import { Card, CardHeader, CardTitle, CardDescription } from "../components/ui/Card";
import { ActivityFeed } from "../components/dashboard/ActivityFeed";
import { Badge } from "../components/ui/Badge";
import { Button } from "../components/ui/Button";
import { Skeleton } from "../components/ui/Skeleton";
import type { Node } from "@quartermaster/core";

const container = {
  hidden: { opacity: 0 },
  show: {
    opacity: 1,
    transition: { staggerChildren: 0.06 },
  },
};

const item = {
  hidden: { opacity: 0, y: 12 },
  show: { opacity: 1, y: 0 },
};

function statusBadgeVariant(status: Node["status"]) {
  switch (status) {
    case "online":
      return "success" as const;
    case "offline":
    case "error":
      return "danger" as const;
    default:
      return "default" as const;
  }
}

function statusLabel(status: Node["status"]) {
  switch (status) {
    case "online":
      return "Online";
    case "offline":
      return "Offline";
    case "connecting":
      return "Connecting";
    case "error":
      return "Error";
    default:
      return "Unknown";
  }
}

export function DashboardPage() {
  const navigate = useNavigate();
  const { nodes, loading: nodesLoading } = useFleetStore();
  const { blueprints, loading: blueprintsLoading } = useBlueprintStore();

  const loading = nodesLoading || blueprintsLoading;

  // --- Derived stats ---

  const statusCounts = useMemo(() => {
    let online = 0;
    let offline = 0;
    for (const node of nodes) {
      if (node.status === "online") online++;
      else if (node.status === "offline" || node.status === "error") offline++;
    }
    return { online, offline };
  }, [nodes]);

  const uniqueTaskCount = useMemo(() => {
    const ids = new Set<string>();
    for (const bp of blueprints) {
      for (const entry of bp.task_entries) {
        ids.add(entry.task_id);
      }
    }
    return ids.size;
  }, [blueprints]);

  const healthPercent = useMemo(() => {
    if (nodes.length === 0) return 0;
    return Math.round((statusCounts.online / nodes.length) * 100);
  }, [nodes.length, statusCounts.online]);

  const recentNodes = useMemo(() => {
    return [...nodes]
      .sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
      .slice(0, 5);
  }, [nodes]);

  const nodeCountByBlueprint = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const node of nodes) {
      if (node.blueprint_id) {
        counts[node.blueprint_id] = (counts[node.blueprint_id] || 0) + 1;
      }
    }
    return counts;
  }, [nodes]);

  // --- Render ---

  if (loading) {
    return (
      <div>
        <div className="mb-6">
          <Skeleton className="h-7 w-36 mb-2" />
          <Skeleton className="h-4 w-64" />
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
          {Array.from({ length: 4 }).map((_, i) => (
            <Skeleton key={i} className="h-28" rounded="xl" />
          ))}
        </div>
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          <Skeleton className="h-72" rounded="xl" />
          <Skeleton className="h-72" rounded="xl" />
        </div>
      </div>
    );
  }

  return (
    <div>
      {/* Header */}
      <div className="mb-6">
        <h1 className="text-xl font-semibold text-text-primary-light dark:text-text-primary-dark">
          Dashboard
        </h1>
        <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark mt-1">
          Overview of your infrastructure
        </p>
      </div>

      <motion.div variants={container} initial="hidden" animate="show">
        {/* Stats Row */}
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
          {/* Total Nodes */}
          <motion.div variants={item}>
            <Card className="relative overflow-hidden">
              <div className="flex items-start justify-between">
                <div>
                  <p className="text-sm font-medium text-text-secondary-light dark:text-text-secondary-dark">
                    Total Nodes
                  </p>
                  <p className="text-2xl font-bold text-text-primary-light dark:text-text-primary-dark mt-1">
                    {nodes.length}
                  </p>
                  <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark mt-1">
                    {statusCounts.online} online, {statusCounts.offline} offline
                  </p>
                </div>
                <div className="p-2 rounded-lg bg-warm-100 dark:bg-warm-900/30">
                  <Network size={20} className="text-warm-600 dark:text-warm-400" />
                </div>
              </div>
            </Card>
          </motion.div>

          {/* Blueprints */}
          <motion.div variants={item}>
            <Card className="relative overflow-hidden">
              <div className="flex items-start justify-between">
                <div>
                  <p className="text-sm font-medium text-text-secondary-light dark:text-text-secondary-dark">
                    Blueprints
                  </p>
                  <p className="text-2xl font-bold text-text-primary-light dark:text-text-primary-dark mt-1">
                    {blueprints.length}
                  </p>
                  <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark mt-1">
                    {blueprints.filter((b) => b.is_builtin).length} built-in, {blueprints.filter((b) => !b.is_builtin).length} custom
                  </p>
                </div>
                <div className="p-2 rounded-lg bg-warm-100 dark:bg-warm-900/30">
                  <Layers size={20} className="text-warm-600 dark:text-warm-400" />
                </div>
              </div>
            </Card>
          </motion.div>

          {/* Tasks Available */}
          <motion.div variants={item}>
            <Card className="relative overflow-hidden">
              <div className="flex items-start justify-between">
                <div>
                  <p className="text-sm font-medium text-text-secondary-light dark:text-text-secondary-dark">
                    Tasks Available
                  </p>
                  <p className="text-2xl font-bold text-text-primary-light dark:text-text-primary-dark mt-1">
                    {uniqueTaskCount}
                  </p>
                  <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark mt-1">
                    Across {blueprints.length} {blueprints.length === 1 ? "blueprint" : "blueprints"}
                  </p>
                </div>
                <div className="p-2 rounded-lg bg-warm-100 dark:bg-warm-900/30">
                  <ListChecks size={20} className="text-warm-600 dark:text-warm-400" />
                </div>
              </div>
            </Card>
          </motion.div>

          {/* Node Health */}
          <motion.div variants={item}>
            <Card className="relative overflow-hidden">
              <div className="flex items-start justify-between">
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium text-text-secondary-light dark:text-text-secondary-dark">
                    Node Health
                  </p>
                  <p className="text-2xl font-bold text-text-primary-light dark:text-text-primary-dark mt-1">
                    {nodes.length > 0 ? `${healthPercent}%` : "--"}
                  </p>
                  <div className="mt-2 w-full h-2 rounded-full bg-warm-100 dark:bg-warm-900/30 overflow-hidden">
                    <div
                      className={`h-full rounded-full transition-all duration-500 ${
                        healthPercent >= 75
                          ? "bg-green-500"
                          : healthPercent >= 50
                            ? "bg-yellow-500"
                            : healthPercent > 0
                              ? "bg-red-500"
                              : "bg-gray-300 dark:bg-gray-700"
                      }`}
                      style={{ width: `${nodes.length > 0 ? healthPercent : 0}%` }}
                    />
                  </div>
                </div>
                <div className="p-2 rounded-lg bg-warm-100 dark:bg-warm-900/30 ml-3">
                  <Activity size={20} className="text-warm-600 dark:text-warm-400" />
                </div>
              </div>
            </Card>
          </motion.div>
        </div>

        {/* Bottom row: Recent Nodes + Blueprints Summary */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          {/* Recent Nodes */}
          <motion.div variants={item}>
            <Card padding={false}>
              <CardHeader className="px-5 pt-5 pb-0">
                <div className="flex items-center justify-between">
                  <div>
                    <CardTitle>Recent Nodes</CardTitle>
                    <CardDescription>Latest machines added to your fleet</CardDescription>
                  </div>
                  <Button variant="ghost" size="sm" onClick={() => navigate("/fleet")}>
                    View All
                    <ChevronRight size={14} />
                  </Button>
                </div>
              </CardHeader>
              <div className="px-5 pb-5">
                {recentNodes.length === 0 ? (
                  <div className="py-8 text-center">
                    <Network size={32} className="mx-auto text-text-secondary-light/30 dark:text-text-secondary-dark/30 mb-2" />
                    <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark">
                      No nodes yet.{" "}
                      <button
                        onClick={() => navigate("/fleet")}
                        className="text-warm-500 hover:text-warm-600 dark:text-warm-400 dark:hover:text-warm-300 font-medium"
                      >
                        Add your first node
                      </button>
                    </p>
                  </div>
                ) : (
                  <div className="divide-y divide-border-light dark:divide-border-dark">
                    {recentNodes.map((node) => (
                      <div
                        key={node.id}
                        className="flex items-center justify-between py-3 first:pt-4"
                      >
                        <div className="flex items-center gap-3 min-w-0">
                          <div className="p-1.5 rounded-md bg-warm-100 dark:bg-warm-900/30">
                            {node.kind === "local" ? (
                              <Monitor size={16} className="text-warm-600 dark:text-warm-400" />
                            ) : (
                              <Server size={16} className="text-warm-600 dark:text-warm-400" />
                            )}
                          </div>
                          <div className="min-w-0">
                            <p className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark truncate">
                              {node.name}
                            </p>
                            <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark truncate">
                              {node.hostname}
                            </p>
                          </div>
                        </div>
                        <div className="flex items-center gap-2 shrink-0 ml-3">
                          <Badge variant={statusBadgeVariant(node.status)}>
                            {node.status === "online" ? (
                              <CheckCircle2 size={10} className="mr-1" />
                            ) : node.status === "error" || node.status === "offline" ? (
                              <AlertTriangle size={10} className="mr-1" />
                            ) : (
                              <Circle size={10} className="mr-1" />
                            )}
                            {statusLabel(node.status)}
                          </Badge>
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => navigate(`/fleet/${node.id}`)}
                          >
                            View
                          </Button>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </Card>
          </motion.div>

          {/* Blueprints Summary */}
          <motion.div variants={item}>
            <Card padding={false}>
              <CardHeader className="px-5 pt-5 pb-0">
                <div className="flex items-center justify-between">
                  <div>
                    <CardTitle>Blueprints</CardTitle>
                    <CardDescription>Setup configurations and assigned nodes</CardDescription>
                  </div>
                  <Button variant="ghost" size="sm" onClick={() => navigate("/blueprints")}>
                    View All
                    <ChevronRight size={14} />
                  </Button>
                </div>
              </CardHeader>
              <div className="px-5 pb-5">
                {blueprints.length === 0 ? (
                  <div className="py-8 text-center">
                    <Layers size={32} className="mx-auto text-text-secondary-light/30 dark:text-text-secondary-dark/30 mb-2" />
                    <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark">
                      No blueprints yet.{" "}
                      <button
                        onClick={() => navigate("/blueprints")}
                        className="text-warm-500 hover:text-warm-600 dark:text-warm-400 dark:hover:text-warm-300 font-medium"
                      >
                        Create one
                      </button>
                    </p>
                  </div>
                ) : (
                  <div className="divide-y divide-border-light dark:divide-border-dark">
                    {blueprints.map((bp) => {
                      const assignedCount = nodeCountByBlueprint[bp.id] || 0;
                      return (
                        <div
                          key={bp.id}
                          className="flex items-center justify-between py-3 first:pt-4"
                        >
                          <div className="min-w-0 flex-1">
                            <p className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark truncate">
                              {bp.name}
                            </p>
                            <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark truncate mt-0.5">
                              {bp.description}
                            </p>
                          </div>
                          <div className="flex items-center gap-2 shrink-0 ml-3">
                            <Badge variant="default">v{bp.version}</Badge>
                            <Badge variant="info">
                              {bp.task_entries.length} {bp.task_entries.length === 1 ? "task" : "tasks"}
                            </Badge>
                            <span className="text-xs text-text-secondary-light dark:text-text-secondary-dark whitespace-nowrap">
                              {assignedCount} {assignedCount === 1 ? "node" : "nodes"}
                            </span>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            </Card>
          </motion.div>
        </div>

        {/* Activity Feed */}
        <motion.div variants={item} className="mt-4">
          <ActivityFeed />
        </motion.div>
      </motion.div>
    </div>
  );
}
