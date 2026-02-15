import { useState, useEffect } from "react";
import { Shield, X } from "lucide-react";
import { Button } from "../ui/Button";
import * as api from "../../services/tauriCommands";
import { useToastStore } from "../../stores/toastStore";
import { formatError } from "@quartermaster/core";
import { motion, AnimatePresence } from "motion/react";

export function PolkitBanner() {
  const [visible, setVisible] = useState(false);
  const [installing, setInstalling] = useState(false);
  const { addToast } = useToastStore();

  useEffect(() => {
    api.isPolkitPolicyInstalled().then((installed) => {
      if (!installed) setVisible(true);
    }).catch(() => {});
  }, []);

  const handleInstall = async () => {
    setInstalling(true);
    try {
      await api.installPolkitPolicy();
      addToast({ type: "success", title: "Policy installed", message: "Polkit policy has been installed." });
      setVisible(false);
    } catch (err) {
      addToast({ type: "error", title: "Installation failed", message: formatError(err) });
    } finally {
      setInstalling(false);
    }
  };

  return (
    <AnimatePresence>
      {visible && (
        <motion.div
          initial={{ opacity: 0, height: 0 }}
          animate={{ opacity: 1, height: "auto" }}
          exit={{ opacity: 0, height: 0 }}
          className="bg-warm-100 dark:bg-warm-900/30 border-b border-warm-200 dark:border-warm-800"
        >
          <div className="flex items-center gap-3 px-6 py-3">
            <Shield size={16} className="text-warm-500 shrink-0" />
            <p className="text-sm text-text-primary-light dark:text-text-primary-dark flex-1">
              Polkit policy not installed. Some operations (package installs, AppArmor management) require elevated privileges.
            </p>
            <Button size="sm" loading={installing} onClick={handleInstall}>
              Install Policy
            </Button>
            <button
              onClick={() => setVisible(false)}
              aria-label="Dismiss"
              className="p-1 rounded hover:bg-warm-200/50 dark:hover:bg-warm-800/50 focus:outline-none focus-visible:ring-2 focus-visible:ring-warm-300/50"
            >
              <X size={14} className="text-text-secondary-light dark:text-text-secondary-dark" />
            </button>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
