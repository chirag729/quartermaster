import { useCallback, useEffect } from "react";
import { useAppArmorStore } from "@/hooks/useStore";
import { useToastStore } from "../stores/toastStore";
import { AppArmorDashboard } from "../components/apparmor/AppArmorDashboard";
import { formatError, appArmorService } from "@quartermaster/core";

export function AppArmorPage() {
  const store = useAppArmorStore();
  const { addToast } = useToastStore();

  // Load AppArmor data on mount (requires elevated privileges)
  useEffect(() => {
    appArmorService.loadData().catch((err) => {
      addToast({ type: "error", title: "Failed to load AppArmor data", message: formatError(err) });
    });
  }, [addToast]);

  const startMonitor = useCallback(async () => {
    try {
      await appArmorService.startMonitor();
    } catch (err) {
      addToast({ type: "error", title: "Failed to start monitor", message: formatError(err) });
    }
  }, [addToast]);

  const stopMonitor = useCallback(async () => {
    try {
      await appArmorService.stopMonitor();
    } catch (err) {
      addToast({ type: "error", title: "Failed to stop monitor", message: formatError(err) });
    }
  }, [addToast]);

  const reviewSelected = useCallback(async () => {
    try {
      await appArmorService.reviewSelected();
    } catch (err) {
      addToast({ type: "error", title: "Failed to consolidate rules", message: formatError(err) });
    }
  }, [addToast]);

  const consolidateProfile = useCallback(async (profileName: string) => {
    try {
      const hadChanges = await appArmorService.consolidateProfile(profileName);
      if (!hadChanges) {
        addToast({ type: "info", title: "No consolidation needed", message: `${profileName} rules are already optimal` });
      }
    } catch (err) {
      addToast({ type: "error", title: "Failed to consolidate profile", message: formatError(err) });
    }
  }, [addToast]);

  const confirmApply = useCallback(async (editedRules: Map<string, string[]>) => {
    const isRewrite = store.reviewMode === "rewrite";
    try {
      await appArmorService.confirmApply(editedRules);
      if (isRewrite) {
        const profiles = Array.from(editedRules.keys()).join(", ");
        addToast({ type: "success", title: "Profile consolidated", message: `Rewrote rules for: ${profiles}` });
      } else {
        const profiles = Array.from(editedRules.keys()).join(", ");
        addToast({ type: "success", title: "Rules applied", message: `Updated profiles: ${profiles}` });
      }
    } catch (err) {
      addToast({ type: "error", title: "Failed to apply rules", message: formatError(err) });
    }
  }, [addToast, store.reviewMode]);

  const cancelReview = useCallback(() => {
    appArmorService.cancelReview();
  }, []);

  const refreshData = useCallback(async () => {
    try {
      await appArmorService.loadData();
    } catch (err) {
      addToast({ type: "error", title: "Failed to load AppArmor data", message: formatError(err) });
    }
  }, [addToast]);

  return (
    <div>
      <div className="mb-6">
        <h1 className="text-xl font-semibold text-text-primary-light dark:text-text-primary-dark">
          AppArmor Manager
        </h1>
        <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark mt-1">
          Monitor and manage AppArmor profiles and permissions
        </p>
      </div>
      <AppArmorDashboard
        {...store}
        startMonitor={startMonitor}
        stopMonitor={stopMonitor}
        reviewSelected={reviewSelected}
        consolidateProfile={consolidateProfile}
        confirmApply={confirmApply}
        cancelReview={cancelReview}
        refreshData={refreshData}
      />
    </div>
  );
}
