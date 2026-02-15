import { motion, AnimatePresence } from "motion/react";
import { X, Play } from "lucide-react";
import { Button } from "../ui/Button";

interface BulkActionBarProps {
  selectedCount: number;
  onDeselectAll: () => void;
  onRunBlueprint: () => void;
}

export function BulkActionBar({ selectedCount, onDeselectAll, onRunBlueprint }: BulkActionBarProps) {
  return (
    <AnimatePresence>
      {selectedCount > 0 && (
        <motion.div
          initial={{ opacity: 0, y: 40 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: 40 }}
          transition={{ duration: 0.2 }}
          className="fixed bottom-6 left-1/2 -translate-x-1/2 z-50"
        >
          <div className="flex items-center gap-3 bg-card-light dark:bg-card-dark border border-border-light dark:border-border-dark rounded-xl shadow-lg px-5 py-3">
            <span className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark whitespace-nowrap">
              {selectedCount} {selectedCount === 1 ? "node" : "nodes"} selected
            </span>

            <div className="h-5 w-px bg-border-light dark:bg-border-dark" />

            <Button variant="ghost" size="sm" onClick={onDeselectAll}>
              <X size={14} />
              Deselect All
            </Button>

            <Button variant="primary" size="sm" onClick={onRunBlueprint}>
              <Play size={14} />
              Run Blueprint
            </Button>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
