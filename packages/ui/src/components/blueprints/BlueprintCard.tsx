import { useNavigate } from "react-router-dom";
import { Layers, ListChecks, Shield, Settings, Server, Monitor, Users, type LucideIcon } from "lucide-react";
import { motion } from "motion/react";
import type { Blueprint } from "@quartermaster/core";
import { Badge } from "../ui/Badge";

const iconMap: Record<string, LucideIcon> = {
  Layers,
  ListChecks,
  Shield,
  Settings,
  Server,
  Monitor,
};

interface BlueprintCardProps {
  blueprint: Blueprint;
  assignedNodeCount?: number;
}

export function BlueprintCard({ blueprint, assignedNodeCount = 0 }: BlueprintCardProps) {
  const navigate = useNavigate();
  const Icon = iconMap[blueprint.icon] || Layers;
  const taskCount = blueprint.task_entries.length;

  return (
    <motion.div
      whileHover={{ y: -2 }}
      transition={{ duration: 0.15 }}
      onClick={() => navigate(`/blueprints/${blueprint.id}`)}
      className="bg-card-light dark:bg-card-dark rounded-xl border border-border-light dark:border-border-dark shadow-sm hover:shadow-md transition-shadow duration-200 p-5 cursor-pointer group"
    >
      <div className="flex items-start justify-between mb-3">
        <div className="p-2 rounded-lg bg-warm-100 dark:bg-warm-900/30 text-warm-600 dark:text-warm-400">
          <Icon size={18} />
        </div>
        {blueprint.is_builtin && (
          <Badge variant="info">Built-in</Badge>
        )}
      </div>
      <h3 className="text-sm font-semibold text-text-primary-light dark:text-text-primary-dark mb-1">
        {blueprint.name}
      </h3>
      <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark line-clamp-2 mb-3">
        {blueprint.description}
      </p>
      <div className="flex items-center gap-3 text-xs text-text-secondary-light dark:text-text-secondary-dark">
        <span className="flex items-center gap-1.5">
          v{blueprint.version}
        </span>
        <span className="flex items-center gap-1.5">
          <ListChecks size={12} />
          {taskCount} {taskCount === 1 ? "task" : "tasks"}
        </span>
        {assignedNodeCount > 0 && (
          <span className="flex items-center gap-1.5">
            <Users size={12} />
            {assignedNodeCount} {assignedNodeCount === 1 ? "node" : "nodes"}
          </span>
        )}
      </div>
    </motion.div>
  );
}
