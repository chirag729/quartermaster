import { motion, AnimatePresence } from "motion/react";
import { Layers } from "lucide-react";
import type { Blueprint } from "../../types/blueprint";
import { BlueprintCard } from "./BlueprintCard";
import { Skeleton } from "../ui/Skeleton";
import { EmptyState } from "../ui/EmptyState";
import { Button } from "../ui/Button";

interface BlueprintGridProps {
  blueprints: Blueprint[];
  loading: boolean;
  onCreateBlueprint: () => void;
}

export function BlueprintGrid({ blueprints, loading, onCreateBlueprint }: BlueprintGridProps) {
  if (loading) {
    return (
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {Array.from({ length: 6 }).map((_, i) => (
          <Skeleton key={i} className="h-[156px]" rounded="xl" />
        ))}
      </div>
    );
  }

  if (blueprints.length === 0) {
    return (
      <EmptyState
        icon={<Layers size={40} />}
        title="No blueprints yet"
        description="Create your first blueprint to define reusable setup configurations."
        action={
          <Button size="md" onClick={onCreateBlueprint}>
            Create Blueprint
          </Button>
        }
      />
    );
  }

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      <AnimatePresence>
        {blueprints.map((blueprint, idx) => (
          <motion.div
            key={blueprint.id}
            initial={{ opacity: 0, y: 16 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.95 }}
            transition={{ duration: 0.25, delay: idx * 0.04 }}
          >
            <BlueprintCard blueprint={blueprint} />
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  );
}
