import { describe, it, expect, beforeEach } from "vitest";
import { useThemeStore } from "../../stores/themeStore";

describe("themeStore", () => {
  beforeEach(() => {
    useThemeStore.setState({ theme: "system", resolvedTheme: "light" });
  });

  it("defaults to system theme", () => {
    const state = useThemeStore.getState();
    expect(state.theme).toBe("system");
  });

  it("sets theme to dark", () => {
    useThemeStore.getState().setTheme("dark");
    const state = useThemeStore.getState();
    expect(state.theme).toBe("dark");
    expect(state.resolvedTheme).toBe("dark");
  });

  it("sets theme to light", () => {
    useThemeStore.getState().setTheme("light");
    const state = useThemeStore.getState();
    expect(state.theme).toBe("light");
    expect(state.resolvedTheme).toBe("light");
  });

  it("setResolvedTheme updates resolved only", () => {
    useThemeStore.getState().setTheme("system");
    useThemeStore.getState().setResolvedTheme("dark");
    const state = useThemeStore.getState();
    expect(state.theme).toBe("system");
    expect(state.resolvedTheme).toBe("dark");
  });
});
