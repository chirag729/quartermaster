import { motion, AnimatePresence } from "motion/react";
import { Server } from "lucide-react";
import type { Node } from "../../types/node";
import { NodeCard } from "./NodeCard";
import { Skeleton } from "../ui/Skeleton";
import { EmptyState } from "../ui/EmptyState";
import { Button } from "../ui/Button";

interface FleetOverviewProps {
  nodes: Node[];
  loading: boolean;
  onRemoveNode: (nodeId: string) => void;
  onAddNode: () => void;
}

export function FleetOverview({ nodes, loading, onRemoveNode, onAddNode }: FleetOverviewProps) {
  if (loading) {
    return (
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {Array.from({ length: 6 }).map((_, i) => (
          <Skeleton key={i} className="h-[132px]" rounded="xl" />
        ))}
      </div>
    );
  }

  if (nodes.length === 0) {
    return (
      <EmptyState
        icon={<Server size={40} />}
        title="No nodes yet"
        description="Add your first node to start managing your fleet."
        action={
          <Button size="md" onClick={onAddNode}>
            Add Node
          </Button>
        }
      />
    );
  }

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      <AnimatePresence>
        {nodes.map((node, idx) => (
          <motion.div
            key={node.id}
            initial={{ opacity: 0, y: 16 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.95 }}
            transition={{ duration: 0.25, delay: idx * 0.04 }}
          >
            <NodeCard node={node} onRemove={onRemoveNode} />
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  );
}
