import { create } from "zustand";
import type { ConsolidationResult, DenialEvent, PermissionSuggestion, ProfileInfo } from "../types/apparmor";

interface AppArmorState {
  denials: DenialEvent[];
  suggestions: PermissionSuggestion[];
  profiles: ProfileInfo[];
  selectedSuggestions: Set<string>;
  monitoring: boolean;
  loading: boolean;
  setDenials: (denials: DenialEvent[]) => void;
  setSuggestions: (suggestions: PermissionSuggestion[]) => void;
  setProfiles: (profiles: ProfileInfo[]) => void;
  toggleSuggestion: (id: string) => void;
  selectAllSuggestions: () => void;
  clearSelection: () => void;
  setMonitoring: (monitoring: boolean) => void;
  setLoading: (loading: boolean) => void;
  addDenial: (denial: DenialEvent) => void;
  reviewRules: ConsolidationResult[] | null;
  reviewMode: "append" | "rewrite" | null;
  setReviewRules: (rules: ConsolidationResult[] | null, mode?: "append" | "rewrite") => void;
}

export const useAppArmorStore = create<AppArmorState>((set) => ({
  denials: [],
  suggestions: [],
  profiles: [],
  selectedSuggestions: new Set(),
  monitoring: false,
  loading: true,
  setDenials: (denials) => set({ denials }),
  setSuggestions: (suggestions) => set({ suggestions }),
  setProfiles: (profiles) => set({ profiles }),
  toggleSuggestion: (id) =>
    set((state) => {
      const next = new Set(state.selectedSuggestions);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return { selectedSuggestions: next };
    }),
  selectAllSuggestions: () =>
    set((state) => ({
      selectedSuggestions: new Set(state.suggestions.map((s) => s.id)),
    })),
  clearSelection: () => set({ selectedSuggestions: new Set() }),
  setMonitoring: (monitoring) => set({ monitoring }),
  setLoading: (loading) => set({ loading }),
  addDenial: (denial) =>
    set((state) => ({ denials: [denial, ...state.denials] })),
  reviewRules: null,
  reviewMode: null,
  setReviewRules: (rules, mode = "append") => set({ reviewRules: rules, reviewMode: rules ? mode : null }),
}));
