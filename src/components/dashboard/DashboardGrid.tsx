import { ModuleCard } from "./ModuleCard";
import type { TaskInfo } from "../../types/task";
import { motion, AnimatePresence } from "motion/react";

interface Props {
  tasks: TaskInfo[];
  executing: string | null;
  onExecute: (taskId: string) => void;
  readOnly?: boolean;
}

export function DashboardGrid({ tasks, executing, onExecute, readOnly }: Props) {
  const categories = Array.from(new Set(tasks.map((t) => t.category)));

  return (
    <div className="space-y-8">
      {categories.map((category, catIdx) => (
        <motion.section
          key={category}
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.3, delay: catIdx * 0.1 }}
        >
          <h2 className="text-xs font-bold text-text-primary-light/70 dark:text-text-primary-dark/70 uppercase tracking-widest mb-4">
            {category}
          </h2>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            <AnimatePresence>
              {tasks
                .filter((t) => t.category === category)
                .map((task, idx) => (
                  <motion.div
                    key={task.id}
                    initial={{ opacity: 0, y: 16 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ duration: 0.3, delay: catIdx * 0.1 + idx * 0.05 }}
                  >
                    <ModuleCard
                      task={task}
                      executing={executing === task.id}
                      onExecute={() => onExecute(task.id)}
                      readOnly={readOnly}
                    />
                  </motion.div>
                ))}
            </AnimatePresence>
          </div>
        </motion.section>
      ))}
    </div>
  );
}
