import { useNavigate } from "react-router-dom";
import { Monitor, Server, MoreVertical, Trash2, Pencil } from "lucide-react";
import clsx from "clsx";
import type { Node } from "../../types/node";
import { NodeStatusBadge } from "./NodeStatusBadge";
import { Badge } from "../ui/Badge";
import { DropdownMenu } from "../ui/DropdownMenu";

interface NodeCardProps {
  node: Node;
  onRemove: (nodeId: string) => void;
}

export function NodeCard({ node, onRemove }: NodeCardProps) {
  const navigate = useNavigate();
  const KindIcon = node.kind === "local" ? Monitor : Server;

  return (
    <div
      onClick={() => navigate(`/fleet/${node.id}`)}
      className="bg-card-light dark:bg-card-dark rounded-xl border border-border-light dark:border-border-dark shadow-sm hover:shadow-md transition-shadow duration-200 p-5 cursor-pointer group"
    >
      <div className="flex items-start justify-between mb-3">
        <div className="flex items-center gap-2.5">
          <div
            className={clsx(
              "p-2 rounded-lg",
              node.kind === "local"
                ? "bg-warm-100 dark:bg-warm-900/30 text-warm-600 dark:text-warm-400"
                : "bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400",
            )}
          >
            <KindIcon size={16} />
          </div>
          <div className="min-w-0">
            <h3 className="text-sm font-semibold text-text-primary-light dark:text-text-primary-dark truncate">
              {node.name}
            </h3>
            <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark truncate">
              {node.hostname}
            </p>
          </div>
        </div>
        <div
          className="opacity-0 group-hover:opacity-100 transition-opacity"
          onClick={(e) => e.stopPropagation()}
        >
          <DropdownMenu
            trigger={
              <button className="p-1 rounded-md hover:bg-warm-100/50 dark:hover:bg-warm-900/20 text-text-secondary-light dark:text-text-secondary-dark">
                <MoreVertical size={14} />
              </button>
            }
            items={[
              {
                label: "Edit",
                icon: <Pencil size={14} />,
                onClick: () => navigate(`/fleet/${node.id}`),
              },
              {
                label: "Remove",
                icon: <Trash2 size={14} />,
                danger: true,
                onClick: () => onRemove(node.id),
              },
            ]}
          />
        </div>
      </div>
      <div className="flex items-center gap-2 flex-wrap">
        <NodeStatusBadge status={node.status} />
        <Badge variant={node.kind === "local" ? "default" : "info"}>
          {node.kind}
        </Badge>
        {node.tags.slice(0, 2).map((tag) => (
          <Badge key={tag} variant="default">
            {tag}
          </Badge>
        ))}
        {node.tags.length > 2 && (
          <span className="text-xs text-text-secondary-light dark:text-text-secondary-dark">
            +{node.tags.length - 2}
          </span>
        )}
      </div>
    </div>
  );
}
