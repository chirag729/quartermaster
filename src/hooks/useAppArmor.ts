import { useEffect, useCallback } from "react";
import { useAppArmorStore } from "../stores/appArmorStore";
import { useToastStore } from "../stores/toastStore";
import { useTauriEvent } from "./useTauriEvent";
import * as api from "../services/tauriCommands";
import { formatError } from "../lib/formatError";
import type { AppArmorDenialEvent } from "../types/events";

export function useAppArmor() {
  const store = useAppArmorStore();
  const { addToast } = useToastStore();

  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    store.setLoading(true);
    try {
      const [logs, profiles] = await Promise.all([
        api.getDenialLogs(),
        api.getProfiles(),
      ]);
      store.setDenials(logs.denials);
      store.setSuggestions(logs.suggestions);
      store.setProfiles(profiles);
    } catch (err) {
      addToast({ type: "error", title: "Failed to load AppArmor data", message: formatError(err) });
    } finally {
      store.setLoading(false);
    }
  };

  const startMonitor = async () => {
    try {
      await api.startLogMonitor();
      store.setMonitoring(true);
    } catch (err) {
      addToast({ type: "error", title: "Failed to start monitor", message: formatError(err) });
    }
  };

  const stopMonitor = async () => {
    try {
      await api.stopLogMonitor();
      store.setMonitoring(false);
    } catch (err) {
      addToast({ type: "error", title: "Failed to stop monitor", message: formatError(err) });
    }
  };

  const reviewSelected = async () => {
    const selected = store.suggestions.filter((s) => store.selectedSuggestions.has(s.id));
    if (selected.length === 0) return;
    try {
      const consolidated = await api.consolidateRules(selected);
      store.setReviewRules(consolidated, "append");
    } catch (err) {
      addToast({ type: "error", title: "Failed to consolidate rules", message: formatError(err) });
    }
  };

  const consolidateProfile = async (profileName: string) => {
    try {
      const result = await api.consolidateProfileRules(profileName);
      if (result.rules.some((r) => r.is_glob)) {
        store.setReviewRules([result], "rewrite");
      } else {
        addToast({ type: "info", title: "No consolidation needed", message: `${profileName} rules are already optimal` });
      }
    } catch (err) {
      addToast({ type: "error", title: "Failed to consolidate profile", message: formatError(err) });
    }
  };

  const confirmApply = async (editedRules: Map<string, string[]>) => {
    const isRewrite = store.reviewMode === "rewrite";
    if (isRewrite) {
      // Rewrite mode: replace entire rules section
      for (const [profile, rules] of editedRules) {
        try {
          await api.rewriteProfileRules(profile, rules);
          addToast({ type: "success", title: "Profile consolidated", message: `Rewrote rules for: ${profile}` });
        } catch (err) {
          addToast({ type: "error", title: "Failed to rewrite profile", message: formatError(err) });
        }
      }
    } else {
      // Append mode: batch append new rules
      const requests = Array.from(editedRules.entries()).map(([profile, rules]) => ({
        profile,
        rules,
      }));
      try {
        await api.applyPermissionRulesBatch(requests);
        const profileNames = requests.map((r) => r.profile).join(", ");
        addToast({ type: "success", title: "Rules applied", message: `Updated profiles: ${profileNames}` });
      } catch (err) {
        addToast({ type: "error", title: "Failed to apply rules", message: formatError(err) });
      }
    }
    store.setReviewRules(null);
    store.clearSelection();
    await loadData();
  };

  const cancelReview = () => {
    store.setReviewRules(null);
  };

  const onDenial = useCallback((payload: AppArmorDenialEvent) => {
    store.addDenial(payload.denial);
  }, [store.addDenial]);

  useTauriEvent("apparmor-denial", onDenial);

  return {
    ...store,
    startMonitor,
    stopMonitor,
    reviewSelected,
    consolidateProfile,
    confirmApply,
    cancelReview,
    refreshData: loadData,
  };
}
