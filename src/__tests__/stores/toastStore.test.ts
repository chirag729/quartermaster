import { describe, it, expect, beforeEach, vi } from "vitest";
import { useToastStore } from "../../stores/toastStore";

describe("toastStore", () => {
  beforeEach(() => {
    useToastStore.setState({ toasts: [] });
    vi.useFakeTimers();
  });

  it("starts with no toasts", () => {
    expect(useToastStore.getState().toasts).toHaveLength(0);
  });

  it("addToast adds a toast with id", () => {
    useToastStore.getState().addToast({ type: "success", title: "Done" });
    const toasts = useToastStore.getState().toasts;
    expect(toasts).toHaveLength(1);
    expect(toasts[0].type).toBe("success");
    expect(toasts[0].title).toBe("Done");
    expect(toasts[0].id).toBeDefined();
  });

  it("removeToast removes by id", () => {
    useToastStore.getState().addToast({ type: "info", title: "Info" });
    const id = useToastStore.getState().toasts[0].id;
    useToastStore.getState().removeToast(id);
    expect(useToastStore.getState().toasts).toHaveLength(0);
  });

  it("auto-removes toast after 5 seconds", () => {
    useToastStore.getState().addToast({ type: "error", title: "Error" });
    expect(useToastStore.getState().toasts).toHaveLength(1);
    vi.advanceTimersByTime(5000);
    expect(useToastStore.getState().toasts).toHaveLength(0);
  });

  it("can add multiple toasts", () => {
    useToastStore.getState().addToast({ type: "success", title: "A" });
    useToastStore.getState().addToast({ type: "warning", title: "B" });
    expect(useToastStore.getState().toasts).toHaveLength(2);
  });
});
