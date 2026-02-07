import { describe, it, expect, beforeEach } from "vitest";
import { useAppArmorStore } from "../../stores/appArmorStore";
import type {
  DenialEvent,
  PermissionSuggestion,
  ProfileInfo,
  ConsolidationResult,
} from "../../types/apparmor";

const makeDenial = (overrides: Partial<DenialEvent> = {}): DenialEvent => ({
  id: "denial-1",
  timestamp: "2026-01-01T00:00:00Z",
  profile: "test-profile",
  operation: "open",
  name: "/etc/passwd",
  requested_mask: "r",
  denied_mask: "r",
  pid: 1234,
  comm: "test-app",
  raw_log: "audit: denied ...",
  ...overrides,
});

const makeSuggestion = (overrides: Partial<PermissionSuggestion> = {}): PermissionSuggestion => ({
  id: "sug-1",
  denial_ids: ["denial-1"],
  profile: "test-profile",
  description: "Allow read access to /etc/passwd",
  rule_text: "/etc/passwd r,",
  risk_level: "low",
  explanation: "This file is commonly read by applications.",
  ...overrides,
});

const makeProfile = (overrides: Partial<ProfileInfo> = {}): ProfileInfo => ({
  name: "test-profile",
  mode: "enforce",
  pid_count: 2,
  ...overrides,
});

const makeConsolidationResult = (overrides: Partial<ConsolidationResult> = {}): ConsolidationResult => ({
  profile: "test-profile",
  rules: [
    {
      rule_text: "/etc/** r,",
      original_rules: ["/etc/passwd r,", "/etc/hostname r,"],
      is_glob: true,
    },
  ],
  ...overrides,
});

describe("appArmorStore", () => {
  beforeEach(() => {
    useAppArmorStore.setState({
      denials: [],
      suggestions: [],
      profiles: [],
      selectedSuggestions: new Set(),
      monitoring: false,
      loading: true,
      reviewRules: null,
      reviewMode: null,
    });
  });

  // --- Initial state ---

  it("starts with correct initial state", () => {
    const state = useAppArmorStore.getState();
    expect(state.denials).toHaveLength(0);
    expect(state.suggestions).toHaveLength(0);
    expect(state.profiles).toHaveLength(0);
    expect(state.selectedSuggestions.size).toBe(0);
    expect(state.monitoring).toBe(false);
    expect(state.loading).toBe(true);
    expect(state.reviewRules).toBeNull();
    expect(state.reviewMode).toBeNull();
  });

  // --- setDenials ---

  it("setDenials replaces the denials list", () => {
    const denials = [makeDenial({ id: "d-1" }), makeDenial({ id: "d-2" })];
    useAppArmorStore.getState().setDenials(denials);
    expect(useAppArmorStore.getState().denials).toHaveLength(2);
  });

  it("setDenials with an empty array clears all denials", () => {
    useAppArmorStore.getState().setDenials([makeDenial()]);
    useAppArmorStore.getState().setDenials([]);
    expect(useAppArmorStore.getState().denials).toHaveLength(0);
  });

  // --- setSuggestions ---

  it("setSuggestions replaces the suggestions list", () => {
    const suggestions = [makeSuggestion({ id: "s-1" }), makeSuggestion({ id: "s-2" })];
    useAppArmorStore.getState().setSuggestions(suggestions);
    expect(useAppArmorStore.getState().suggestions).toHaveLength(2);
  });

  it("setSuggestions with an empty array clears all suggestions", () => {
    useAppArmorStore.getState().setSuggestions([makeSuggestion()]);
    useAppArmorStore.getState().setSuggestions([]);
    expect(useAppArmorStore.getState().suggestions).toHaveLength(0);
  });

  // --- setProfiles ---

  it("setProfiles replaces the profiles list", () => {
    const profiles = [makeProfile({ name: "p1" }), makeProfile({ name: "p2" })];
    useAppArmorStore.getState().setProfiles(profiles);
    expect(useAppArmorStore.getState().profiles).toHaveLength(2);
  });

  it("setProfiles with an empty array clears all profiles", () => {
    useAppArmorStore.getState().setProfiles([makeProfile()]);
    useAppArmorStore.getState().setProfiles([]);
    expect(useAppArmorStore.getState().profiles).toHaveLength(0);
  });

  // --- toggleSuggestion ---

  it("toggleSuggestion adds an id to the selection", () => {
    useAppArmorStore.getState().toggleSuggestion("sug-1");
    expect(useAppArmorStore.getState().selectedSuggestions.has("sug-1")).toBe(true);
    expect(useAppArmorStore.getState().selectedSuggestions.size).toBe(1);
  });

  it("toggleSuggestion removes an already-selected id", () => {
    useAppArmorStore.getState().toggleSuggestion("sug-1");
    useAppArmorStore.getState().toggleSuggestion("sug-1");
    expect(useAppArmorStore.getState().selectedSuggestions.has("sug-1")).toBe(false);
    expect(useAppArmorStore.getState().selectedSuggestions.size).toBe(0);
  });

  it("toggleSuggestion can manage multiple selections", () => {
    useAppArmorStore.getState().toggleSuggestion("sug-1");
    useAppArmorStore.getState().toggleSuggestion("sug-2");
    useAppArmorStore.getState().toggleSuggestion("sug-3");

    const selected = useAppArmorStore.getState().selectedSuggestions;
    expect(selected.size).toBe(3);
    expect(selected.has("sug-1")).toBe(true);
    expect(selected.has("sug-2")).toBe(true);
    expect(selected.has("sug-3")).toBe(true);
  });

  it("toggleSuggestion removes one without affecting others", () => {
    useAppArmorStore.getState().toggleSuggestion("sug-1");
    useAppArmorStore.getState().toggleSuggestion("sug-2");
    useAppArmorStore.getState().toggleSuggestion("sug-1"); // remove sug-1

    const selected = useAppArmorStore.getState().selectedSuggestions;
    expect(selected.size).toBe(1);
    expect(selected.has("sug-1")).toBe(false);
    expect(selected.has("sug-2")).toBe(true);
  });

  // --- selectAllSuggestions ---

  it("selectAllSuggestions selects all suggestion ids", () => {
    useAppArmorStore.getState().setSuggestions([
      makeSuggestion({ id: "sug-1" }),
      makeSuggestion({ id: "sug-2" }),
      makeSuggestion({ id: "sug-3" }),
    ]);
    useAppArmorStore.getState().selectAllSuggestions();

    const selected = useAppArmorStore.getState().selectedSuggestions;
    expect(selected.size).toBe(3);
    expect(selected.has("sug-1")).toBe(true);
    expect(selected.has("sug-2")).toBe(true);
    expect(selected.has("sug-3")).toBe(true);
  });

  it("selectAllSuggestions with no suggestions results in empty set", () => {
    useAppArmorStore.getState().selectAllSuggestions();
    expect(useAppArmorStore.getState().selectedSuggestions.size).toBe(0);
  });

  it("selectAllSuggestions replaces any existing selection", () => {
    useAppArmorStore.getState().toggleSuggestion("old-id");
    useAppArmorStore.getState().setSuggestions([makeSuggestion({ id: "sug-1" })]);
    useAppArmorStore.getState().selectAllSuggestions();

    const selected = useAppArmorStore.getState().selectedSuggestions;
    expect(selected.size).toBe(1);
    expect(selected.has("sug-1")).toBe(true);
    expect(selected.has("old-id")).toBe(false);
  });

  // --- clearSelection ---

  it("clearSelection empties the selected suggestions set", () => {
    useAppArmorStore.getState().toggleSuggestion("sug-1");
    useAppArmorStore.getState().toggleSuggestion("sug-2");
    useAppArmorStore.getState().clearSelection();
    expect(useAppArmorStore.getState().selectedSuggestions.size).toBe(0);
  });

  it("clearSelection on an already empty set does nothing", () => {
    useAppArmorStore.getState().clearSelection();
    expect(useAppArmorStore.getState().selectedSuggestions.size).toBe(0);
  });

  // --- setMonitoring ---

  it("setMonitoring enables monitoring", () => {
    useAppArmorStore.getState().setMonitoring(true);
    expect(useAppArmorStore.getState().monitoring).toBe(true);
  });

  it("setMonitoring disables monitoring", () => {
    useAppArmorStore.getState().setMonitoring(true);
    useAppArmorStore.getState().setMonitoring(false);
    expect(useAppArmorStore.getState().monitoring).toBe(false);
  });

  // --- setLoading ---

  it("setLoading updates loading state", () => {
    useAppArmorStore.getState().setLoading(false);
    expect(useAppArmorStore.getState().loading).toBe(false);
  });

  // --- addDenial ---

  it("addDenial prepends a denial to the list", () => {
    useAppArmorStore.getState().setDenials([makeDenial({ id: "old" })]);
    useAppArmorStore.getState().addDenial(makeDenial({ id: "new" }));

    const denials = useAppArmorStore.getState().denials;
    expect(denials).toHaveLength(2);
    expect(denials[0].id).toBe("new");
    expect(denials[1].id).toBe("old");
  });

  it("addDenial to an empty list creates a single-element list", () => {
    useAppArmorStore.getState().addDenial(makeDenial({ id: "first" }));
    expect(useAppArmorStore.getState().denials).toHaveLength(1);
    expect(useAppArmorStore.getState().denials[0].id).toBe("first");
  });

  it("addDenial maintains prepend order with multiple additions", () => {
    useAppArmorStore.getState().addDenial(makeDenial({ id: "d-1" }));
    useAppArmorStore.getState().addDenial(makeDenial({ id: "d-2" }));
    useAppArmorStore.getState().addDenial(makeDenial({ id: "d-3" }));

    const denials = useAppArmorStore.getState().denials;
    expect(denials[0].id).toBe("d-3");
    expect(denials[1].id).toBe("d-2");
    expect(denials[2].id).toBe("d-1");
  });

  // --- setReviewRules ---

  it("setReviewRules sets rules and defaults mode to append", () => {
    const rules = [makeConsolidationResult()];
    useAppArmorStore.getState().setReviewRules(rules);

    const state = useAppArmorStore.getState();
    expect(state.reviewRules).toEqual(rules);
    expect(state.reviewMode).toBe("append");
  });

  it("setReviewRules sets rules with explicit rewrite mode", () => {
    const rules = [makeConsolidationResult()];
    useAppArmorStore.getState().setReviewRules(rules, "rewrite");

    const state = useAppArmorStore.getState();
    expect(state.reviewRules).toEqual(rules);
    expect(state.reviewMode).toBe("rewrite");
  });

  it("setReviewRules with null clears rules and sets mode to null", () => {
    useAppArmorStore.getState().setReviewRules([makeConsolidationResult()], "rewrite");
    useAppArmorStore.getState().setReviewRules(null);

    const state = useAppArmorStore.getState();
    expect(state.reviewRules).toBeNull();
    expect(state.reviewMode).toBeNull();
  });

  it("setReviewRules with null ignores the mode parameter", () => {
    useAppArmorStore.getState().setReviewRules(null, "rewrite");

    const state = useAppArmorStore.getState();
    expect(state.reviewRules).toBeNull();
    expect(state.reviewMode).toBeNull();
  });

  it("setReviewRules replaces previous review rules", () => {
    const first = [makeConsolidationResult({ profile: "profile-a" })];
    const second = [makeConsolidationResult({ profile: "profile-b" })];

    useAppArmorStore.getState().setReviewRules(first, "append");
    useAppArmorStore.getState().setReviewRules(second, "rewrite");

    const state = useAppArmorStore.getState();
    expect(state.reviewRules).toEqual(second);
    expect(state.reviewMode).toBe("rewrite");
  });
});
