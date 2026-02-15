import { getIPCClient } from "../ipc/provider";
import { appArmorStore } from "../stores/appArmorStore";
import type { DenialEvent, PermissionSuggestion, ProfileInfo, ConsolidationResult } from "../types/apparmor";

let unlistenDenial: (() => void) | null = null;

async function loadData(): Promise<void> {
  const ipc = getIPCClient();
  const { setDenials, setSuggestions, setProfiles, setLoading } = appArmorStore.getState();
  setLoading(true);
  try {
    const [logs, profiles] = await Promise.all([
      ipc.invoke<{ denials: DenialEvent[]; suggestions: PermissionSuggestion[] }>("get_denial_logs"),
      ipc.invoke<ProfileInfo[]>("get_profiles"),
    ]);
    setDenials(logs.denials);
    setSuggestions(logs.suggestions);
    setProfiles(profiles);
  } finally {
    setLoading(false);
  }
}

async function startMonitor(): Promise<void> {
  const ipc = getIPCClient();
  await ipc.invoke<void>("start_log_monitor");
  appArmorStore.getState().setMonitoring(true);
}

async function stopMonitor(): Promise<void> {
  const ipc = getIPCClient();
  await ipc.invoke<void>("stop_log_monitor");
  appArmorStore.getState().setMonitoring(false);
}

async function reviewSelected(): Promise<void> {
  const ipc = getIPCClient();
  const { suggestions, selectedSuggestions, setReviewRules } = appArmorStore.getState();
  const selected = suggestions.filter((s) => selectedSuggestions.has(s.id));
  if (selected.length === 0) return;
  const consolidated = await ipc.invoke<ConsolidationResult[]>("consolidate_rules", { suggestions: selected });
  setReviewRules(consolidated, "append");
}

async function consolidateProfile(profileName: string): Promise<boolean> {
  const ipc = getIPCClient();
  const { setReviewRules } = appArmorStore.getState();
  const result = await ipc.invoke<ConsolidationResult>("consolidate_profile_rules", { profileName });
  if (result.rules.some((r) => r.is_glob)) {
    setReviewRules([result], "rewrite");
    return true;
  }
  return false;
}

async function confirmApply(editedRules: Map<string, string[]>): Promise<void> {
  const ipc = getIPCClient();
  const { reviewMode, setReviewRules, clearSelection } = appArmorStore.getState();
  const isRewrite = reviewMode === "rewrite";

  if (isRewrite) {
    for (const [profile, rules] of editedRules) {
      await ipc.invoke<void>("rewrite_profile_rules", { profileName: profile, rules });
    }
  } else {
    const requests = Array.from(editedRules.entries()).map(([profile, rules]) => ({
      profile,
      rules,
    }));
    await ipc.invoke<void>("apply_permission_rules_batch", { requests });
  }

  setReviewRules(null);
  clearSelection();
  await loadData();
}

function cancelReview(): void {
  appArmorStore.getState().setReviewRules(null);
}

async function init(): Promise<void> {
  const ipc = getIPCClient();
  // Do NOT call loadData() here — it invokes get_profiles / get_denial_logs
  // which require pkexec (root), triggering a password prompt on startup.
  // The AppArmor page calls loadData() when mounted instead.

  unlistenDenial = await ipc.listen<{ denial: DenialEvent }>(
    "apparmor-denial",
    (payload) => {
      appArmorStore.getState().addDenial(payload.denial);
    },
  );
}

function destroy(): void {
  unlistenDenial?.();
  unlistenDenial = null;
}

export const appArmorService = {
  init,
  destroy,
  loadData,
  startMonitor,
  stopMonitor,
  reviewSelected,
  consolidateProfile,
  confirmApply,
  cancelReview,
};
