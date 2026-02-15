import { describe, it, expect, beforeEach } from "vitest";
import { appArmorStore } from "@quartermaster/core";
import type {
  DenialEvent,
  PermissionSuggestion,
  ProfileInfo,
  ConsolidationResult,
} from "@quartermaster/core";

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
    appArmorStore.setState({
      denials: [],
      suggestions: [],
      profiles: [],
      selectedSuggestions: new Set(),
      monitoring: false,
      loading: false,
      reviewRules: null,
      reviewMode: null,
    });
  });

  // --- Initial state ---

  it("starts with correct initial state", () => {
    const state = appArmorStore.getState();
    expect(state.denials).toHaveLength(0);
    expect(state.suggestions).toHaveLength(0);
    expect(state.profiles).toHaveLength(0);
    expect(state.selectedSuggestions.size).toBe(0);
    expect(state.monitoring).toBe(false);
    expect(state.loading).toBe(false);
    expect(state.reviewRules).toBeNull();
    expect(state.reviewMode).toBeNull();
  });

  // --- setDenials ---

  it("setDenials replaces the denials list", () => {
    const denials = [makeDenial({ id: "d-1" }), makeDenial({ id: "d-2" })];
    appArmorStore.getState().setDenials(denials);
    expect(appArmorStore.getState().denials).toHaveLength(2);
  });

  it("setDenials with an empty array clears all denials", () => {
    appArmorStore.getState().setDenials([makeDenial()]);
    appArmorStore.getState().setDenials([]);
    expect(appArmorStore.getState().denials).toHaveLength(0);
  });

  // --- setSuggestions ---

  it("setSuggestions replaces the suggestions list", () => {
    const suggestions = [makeSuggestion({ id: "s-1" }), makeSuggestion({ id: "s-2" })];
    appArmorStore.getState().setSuggestions(suggestions);
    expect(appArmorStore.getState().suggestions).toHaveLength(2);
  });

  it("setSuggestions with an empty array clears all suggestions", () => {
    appArmorStore.getState().setSuggestions([makeSuggestion()]);
    appArmorStore.getState().setSuggestions([]);
    expect(appArmorStore.getState().suggestions).toHaveLength(0);
  });

  // --- setProfiles ---

  it("setProfiles replaces the profiles list", () => {
    const profiles = [makeProfile({ name: "p1" }), makeProfile({ name: "p2" })];
    appArmorStore.getState().setProfiles(profiles);
    expect(appArmorStore.getState().profiles).toHaveLength(2);
  });

  it("setProfiles with an empty array clears all profiles", () => {
    appArmorStore.getState().setProfiles([makeProfile()]);
    appArmorStore.getState().setProfiles([]);
    expect(appArmorStore.getState().profiles).toHaveLength(0);
  });

  // --- toggleSuggestion ---

  it("toggleSuggestion adds an id to the selection", () => {
    appArmorStore.getState().toggleSuggestion("sug-1");
    expect(appArmorStore.getState().selectedSuggestions.has("sug-1")).toBe(true);
    expect(appArmorStore.getState().selectedSuggestions.size).toBe(1);
  });

  it("toggleSuggestion removes an already-selected id", () => {
    appArmorStore.getState().toggleSuggestion("sug-1");
    appArmorStore.getState().toggleSuggestion("sug-1");
    expect(appArmorStore.getState().selectedSuggestions.has("sug-1")).toBe(false);
    expect(appArmorStore.getState().selectedSuggestions.size).toBe(0);
  });

  it("toggleSuggestion can manage multiple selections", () => {
    appArmorStore.getState().toggleSuggestion("sug-1");
    appArmorStore.getState().toggleSuggestion("sug-2");
    appArmorStore.getState().toggleSuggestion("sug-3");

    const selected = appArmorStore.getState().selectedSuggestions;
    expect(selected.size).toBe(3);
    expect(selected.has("sug-1")).toBe(true);
    expect(selected.has("sug-2")).toBe(true);
    expect(selected.has("sug-3")).toBe(true);
  });

  it("toggleSuggestion removes one without affecting others", () => {
    appArmorStore.getState().toggleSuggestion("sug-1");
    appArmorStore.getState().toggleSuggestion("sug-2");
    appArmorStore.getState().toggleSuggestion("sug-1"); // remove sug-1

    const selected = appArmorStore.getState().selectedSuggestions;
    expect(selected.size).toBe(1);
    expect(selected.has("sug-1")).toBe(false);
    expect(selected.has("sug-2")).toBe(true);
  });

  // --- selectAllSuggestions ---

  it("selectAllSuggestions selects all suggestion ids", () => {
    appArmorStore.getState().setSuggestions([
      makeSuggestion({ id: "sug-1" }),
      makeSuggestion({ id: "sug-2" }),
      makeSuggestion({ id: "sug-3" }),
    ]);
    appArmorStore.getState().selectAllSuggestions();

    const selected = appArmorStore.getState().selectedSuggestions;
    expect(selected.size).toBe(3);
    expect(selected.has("sug-1")).toBe(true);
    expect(selected.has("sug-2")).toBe(true);
    expect(selected.has("sug-3")).toBe(true);
  });

  it("selectAllSuggestions with no suggestions results in empty set", () => {
    appArmorStore.getState().selectAllSuggestions();
    expect(appArmorStore.getState().selectedSuggestions.size).toBe(0);
  });

  it("selectAllSuggestions replaces any existing selection", () => {
    appArmorStore.getState().toggleSuggestion("old-id");
    appArmorStore.getState().setSuggestions([makeSuggestion({ id: "sug-1" })]);
    appArmorStore.getState().selectAllSuggestions();

    const selected = appArmorStore.getState().selectedSuggestions;
    expect(selected.size).toBe(1);
    expect(selected.has("sug-1")).toBe(true);
    expect(selected.has("old-id")).toBe(false);
  });

  // --- clearSelection ---

  it("clearSelection empties the selected suggestions set", () => {
    appArmorStore.getState().toggleSuggestion("sug-1");
    appArmorStore.getState().toggleSuggestion("sug-2");
    appArmorStore.getState().clearSelection();
    expect(appArmorStore.getState().selectedSuggestions.size).toBe(0);
  });

  it("clearSelection on an already empty set does nothing", () => {
    appArmorStore.getState().clearSelection();
    expect(appArmorStore.getState().selectedSuggestions.size).toBe(0);
  });

  // --- setMonitoring ---

  it("setMonitoring enables monitoring", () => {
    appArmorStore.getState().setMonitoring(true);
    expect(appArmorStore.getState().monitoring).toBe(true);
  });

  it("setMonitoring disables monitoring", () => {
    appArmorStore.getState().setMonitoring(true);
    appArmorStore.getState().setMonitoring(false);
    expect(appArmorStore.getState().monitoring).toBe(false);
  });

  // --- setLoading ---

  it("setLoading updates loading state", () => {
    appArmorStore.getState().setLoading(false);
    expect(appArmorStore.getState().loading).toBe(false);
  });

  // --- addDenial ---

  it("addDenial prepends a denial to the list", () => {
    appArmorStore.getState().setDenials([makeDenial({ id: "old" })]);
    appArmorStore.getState().addDenial(makeDenial({ id: "new" }));

    const denials = appArmorStore.getState().denials;
    expect(denials).toHaveLength(2);
    expect(denials[0].id).toBe("new");
    expect(denials[1].id).toBe("old");
  });

  it("addDenial to an empty list creates a single-element list", () => {
    appArmorStore.getState().addDenial(makeDenial({ id: "first" }));
    expect(appArmorStore.getState().denials).toHaveLength(1);
    expect(appArmorStore.getState().denials[0].id).toBe("first");
  });

  it("addDenial maintains prepend order with multiple additions", () => {
    appArmorStore.getState().addDenial(makeDenial({ id: "d-1" }));
    appArmorStore.getState().addDenial(makeDenial({ id: "d-2" }));
    appArmorStore.getState().addDenial(makeDenial({ id: "d-3" }));

    const denials = appArmorStore.getState().denials;
    expect(denials[0].id).toBe("d-3");
    expect(denials[1].id).toBe("d-2");
    expect(denials[2].id).toBe("d-1");
  });

  // --- setReviewRules ---

  it("setReviewRules sets rules and defaults mode to append", () => {
    const rules = [makeConsolidationResult()];
    appArmorStore.getState().setReviewRules(rules);

    const state = appArmorStore.getState();
    expect(state.reviewRules).toEqual(rules);
    expect(state.reviewMode).toBe("append");
  });

  it("setReviewRules sets rules with explicit rewrite mode", () => {
    const rules = [makeConsolidationResult()];
    appArmorStore.getState().setReviewRules(rules, "rewrite");

    const state = appArmorStore.getState();
    expect(state.reviewRules).toEqual(rules);
    expect(state.reviewMode).toBe("rewrite");
  });

  it("setReviewRules with null clears rules and sets mode to null", () => {
    appArmorStore.getState().setReviewRules([makeConsolidationResult()], "rewrite");
    appArmorStore.getState().setReviewRules(null);

    const state = appArmorStore.getState();
    expect(state.reviewRules).toBeNull();
    expect(state.reviewMode).toBeNull();
  });

  it("setReviewRules with null ignores the mode parameter", () => {
    appArmorStore.getState().setReviewRules(null, "rewrite");

    const state = appArmorStore.getState();
    expect(state.reviewRules).toBeNull();
    expect(state.reviewMode).toBeNull();
  });

  it("setReviewRules replaces previous review rules", () => {
    const first = [makeConsolidationResult({ profile: "profile-a" })];
    const second = [makeConsolidationResult({ profile: "profile-b" })];

    appArmorStore.getState().setReviewRules(first, "append");
    appArmorStore.getState().setReviewRules(second, "rewrite");

    const state = appArmorStore.getState();
    expect(state.reviewRules).toEqual(second);
    expect(state.reviewMode).toBe("rewrite");
  });
});
