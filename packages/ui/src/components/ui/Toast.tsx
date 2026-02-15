import { X, CheckCircle, AlertCircle, Info, AlertTriangle } from "lucide-react";
import { useToastStore, type Toast as ToastType } from "../../stores/toastStore";
import { motion, AnimatePresence } from "motion/react";

const iconMap = {
  success: CheckCircle,
  error: AlertCircle,
  info: Info,
  warning: AlertTriangle,
};

const colorMap = {
  success: "text-green-600 dark:text-green-400",
  error: "text-red-600 dark:text-red-400",
  info: "text-blue-600 dark:text-blue-400",
  warning: "text-yellow-600 dark:text-yellow-400",
};

function ToastItem({ toast }: { toast: ToastType }) {
  const { removeToast } = useToastStore();
  const Icon = iconMap[toast.type];

  return (
    <motion.div
      initial={{ opacity: 0, y: 16, scale: 0.95 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, y: 16, scale: 0.95 }}
      transition={{ type: "spring", stiffness: 500, damping: 35 }}
      className="bg-card-light dark:bg-card-dark border border-border-light dark:border-border-dark rounded-lg shadow-lg p-4 flex items-start gap-3 min-w-[320px]"
    >
      <Icon size={18} className={colorMap[toast.type]} />
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark">{toast.title}</p>
        {toast.message && (
          <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark mt-0.5 truncate">{toast.message}</p>
        )}
      </div>
      <button onClick={() => removeToast(toast.id)} aria-label="Dismiss notification" className="p-0.5 rounded hover:bg-warm-100/50 dark:hover:bg-warm-900/20">
        <X size={14} className="text-text-secondary-light dark:text-text-secondary-dark" />
      </button>
    </motion.div>
  );
}

export function ToastContainer() {
  const { toasts } = useToastStore();

  return (
    <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2" role="status" aria-live="polite">
      <AnimatePresence>
        {toasts.map((toast) => (
          <ToastItem key={toast.id} toast={toast} />
        ))}
      </AnimatePresence>
    </div>
  );
}
