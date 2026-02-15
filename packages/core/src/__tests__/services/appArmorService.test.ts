import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { appArmorStore } from "../../stores/appArmorStore";
import { installMockIPC, type MockIPC } from "../helpers/mockIPC";
import type { DenialEvent, PermissionSuggestion, ProfileInfo, ConsolidationResult } from "../../types/apparmor";

import { appArmorService } from "../../services/appArmorService";

const makeDenial = (overrides: Partial<DenialEvent> = {}): DenialEvent => ({
  id: "denial-1",
  timestamp: "2026-01-01T00:00:00Z",
  profile: "test_profile",
  operation: "open",
  name: "/etc/passwd",
  requested_mask: "r",
  denied_mask: "r",
  pid: 1234,
  comm: "test-app",
  raw_log: "raw log line",
  ...overrides,
});

const makeSuggestion = (overrides: Partial<PermissionSuggestion> = {}): PermissionSuggestion => ({
  id: "sug-1",
  denial_ids: ["denial-1"],
  profile: "test_profile",
  description: "Allow read access",
  rule_text: "/etc/passwd r,",
  risk_level: "low",
  explanation: "Safe to allow",
  ...overrides,
});

const makeProfile = (overrides: Partial<ProfileInfo> = {}): ProfileInfo => ({
  name: "test_profile",
  mode: "enforce",
  pid_count: 1,
  ...overrides,
});

describe("appArmorService", () => {
  let mockIPC: MockIPC;

  beforeEach(() => {
    mockIPC = installMockIPC();
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

  afterEach(() => {
    appArmorService.destroy();
  });

  // --- loadData ---

  it("loadData fetches denials, suggestions, and profiles in parallel", async () => {
    const denials = [makeDenial()];
    const suggestions = [makeSuggestion()];
    const profiles = [makeProfile()];

    mockIPC.invoke
      .mockResolvedValueOnce({ denials, suggestions }) // get_denial_logs
      .mockResolvedValueOnce(profiles); // get_profiles

    await appArmorService.loadData();

    expect(mockIPC.invoke).toHaveBeenCalledWith("get_denial_logs");
    expect(mockIPC.invoke).toHaveBeenCalledWith("get_profiles");
    expect(appArmorStore.getState().denials).toHaveLength(1);
    expect(appArmorStore.getState().suggestions).toHaveLength(1);
    expect(appArmorStore.getState().profiles).toHaveLength(1);
    expect(appArmorStore.getState().loading).toBe(false);
  });

  it("loadData sets loading false even on error", async () => {
    mockIPC.invoke.mockRejectedValueOnce(new Error("fail"));

    await expect(appArmorService.loadData()).rejects.toThrow("fail");
    expect(appArmorStore.getState().loading).toBe(false);
  });

  // --- startMonitor / stopMonitor ---

  it("startMonitor invokes start_log_monitor and sets monitoring true", async () => {
    mockIPC.invoke.mockResolvedValueOnce(undefined);

    await appArmorService.startMonitor();

    expect(mockIPC.invoke).toHaveBeenCalledWith("start_log_monitor");
    expect(appArmorStore.getState().monitoring).toBe(true);
  });

  it("stopMonitor invokes stop_log_monitor and sets monitoring false", async () => {
    appArmorStore.getState().setMonitoring(true);
    mockIPC.invoke.mockResolvedValueOnce(undefined);

    await appArmorService.stopMonitor();

    expect(mockIPC.invoke).toHaveBeenCalledWith("stop_log_monitor");
    expect(appArmorStore.getState().monitoring).toBe(false);
  });

  // --- reviewSelected ---

  it("reviewSelected consolidates selected suggestions and sets review rules", async () => {
    const suggestion = makeSuggestion({ id: "sug-1" });
    appArmorStore.setState({
      suggestions: [suggestion, makeSuggestion({ id: "sug-2" })],
      selectedSuggestions: new Set(["sug-1"]),
    });

    const consolidated: ConsolidationResult[] = [
      { profile: "test_profile", rules: [{ rule_text: "/etc/passwd r,", original_rules: ["/etc/passwd r,"], is_glob: false }] },
    ];
    mockIPC.invoke.mockResolvedValueOnce(consolidated);

    await appArmorService.reviewSelected();

    expect(mockIPC.invoke).toHaveBeenCalledWith("consolidate_rules", {
      suggestions: [suggestion], // only the selected one
    });
    expect(appArmorStore.getState().reviewRules).toEqual(consolidated);
    expect(appArmorStore.getState().reviewMode).toBe("append");
  });

  it("reviewSelected does nothing when no suggestions are selected", async () => {
    appArmorStore.setState({
      suggestions: [makeSuggestion()],
      selectedSuggestions: new Set(),
    });

    await appArmorService.reviewSelected();

    expect(mockIPC.invoke).not.toHaveBeenCalled();
  });

  // --- consolidateProfile ---

  it("consolidateProfile sets review rules in rewrite mode when glob rules exist", async () => {
    const result: ConsolidationResult = {
      profile: "test_profile",
      rules: [{ rule_text: "/etc/** r,", original_rules: ["/etc/a r,", "/etc/b r,"], is_glob: true }],
    };
    mockIPC.invoke.mockResolvedValueOnce(result);

    const hasGlob = await appArmorService.consolidateProfile("test_profile");

    expect(hasGlob).toBe(true);
    expect(mockIPC.invoke).toHaveBeenCalledWith("consolidate_profile_rules", { profileName: "test_profile" });
    expect(appArmorStore.getState().reviewRules).toEqual([result]);
    expect(appArmorStore.getState().reviewMode).toBe("rewrite");
  });

  it("consolidateProfile returns false when no glob rules exist", async () => {
    const result: ConsolidationResult = {
      profile: "test_profile",
      rules: [{ rule_text: "/etc/passwd r,", original_rules: ["/etc/passwd r,"], is_glob: false }],
    };
    mockIPC.invoke.mockResolvedValueOnce(result);

    const hasGlob = await appArmorService.consolidateProfile("test_profile");

    expect(hasGlob).toBe(false);
    expect(appArmorStore.getState().reviewRules).toBeNull();
  });

  // --- confirmApply ---

  it("confirmApply in rewrite mode calls rewrite_profile_rules per profile", async () => {
    appArmorStore.setState({ reviewMode: "rewrite", reviewRules: [] });
    mockIPC.invoke.mockResolvedValue(undefined); // all invocations

    const editedRules = new Map([
      ["profile_a", ["/etc/** r,"]],
      ["profile_b", ["/tmp/** rw,"]],
    ]);

    // Mock the loadData calls at the end
    mockIPC.invoke
      .mockResolvedValueOnce(undefined) // rewrite profile_a
      .mockResolvedValueOnce(undefined) // rewrite profile_b
      .mockResolvedValueOnce({ denials: [], suggestions: [] }) // get_denial_logs
      .mockResolvedValueOnce([]); // get_profiles

    await appArmorService.confirmApply(editedRules);

    expect(mockIPC.invoke).toHaveBeenCalledWith("rewrite_profile_rules", {
      profileName: "profile_a",
      rules: ["/etc/** r,"],
    });
    expect(mockIPC.invoke).toHaveBeenCalledWith("rewrite_profile_rules", {
      profileName: "profile_b",
      rules: ["/tmp/** rw,"],
    });
    expect(appArmorStore.getState().reviewRules).toBeNull();
  });

  it("confirmApply in append mode calls apply_permission_rules_batch", async () => {
    appArmorStore.setState({ reviewMode: "append", reviewRules: [] });

    const editedRules = new Map([
      ["profile_a", ["/etc/passwd r,"]],
    ]);

    mockIPC.invoke
      .mockResolvedValueOnce(undefined) // apply_permission_rules_batch
      .mockResolvedValueOnce({ denials: [], suggestions: [] }) // get_denial_logs
      .mockResolvedValueOnce([]); // get_profiles

    await appArmorService.confirmApply(editedRules);

    expect(mockIPC.invoke).toHaveBeenCalledWith("apply_permission_rules_batch", {
      requests: [{ profile: "profile_a", rules: ["/etc/passwd r,"] }],
    });
    expect(appArmorStore.getState().reviewRules).toBeNull();
    expect(appArmorStore.getState().selectedSuggestions.size).toBe(0);
  });

  // --- cancelReview ---

  it("cancelReview clears review rules", () => {
    appArmorStore.setState({ reviewRules: [], reviewMode: "append" });

    appArmorService.cancelReview();

    expect(appArmorStore.getState().reviewRules).toBeNull();
  });

  // --- init / destroy ---

  it("init subscribes to apparmor-denial event without loading data", async () => {
    await appArmorService.init();

    expect(mockIPC.listen).toHaveBeenCalledWith("apparmor-denial", expect.any(Function));
    // Should NOT call get_denial_logs or get_profiles (they require pkexec)
    expect(mockIPC.invoke).not.toHaveBeenCalled();
  });

  it("apparmor-denial event handler adds denial to store", async () => {
    let denialHandler: ((payload: unknown) => void) | undefined;
    mockIPC.listen.mockImplementation(async (event: string, handler: (payload: unknown) => void) => {
      if (event === "apparmor-denial") denialHandler = handler;
      return () => {};
    });

    await appArmorService.init();

    const newDenial = makeDenial({ id: "denial-new" });
    denialHandler!({ denial: newDenial });

    expect(appArmorStore.getState().denials).toHaveLength(1);
    expect(appArmorStore.getState().denials[0].id).toBe("denial-new");
  });

  it("destroy calls unlisten", async () => {
    const unlisten = vi.fn();
    mockIPC.listen.mockResolvedValueOnce(unlisten);

    await appArmorService.init();
    appArmorService.destroy();

    expect(unlisten).toHaveBeenCalled();
  });
});
