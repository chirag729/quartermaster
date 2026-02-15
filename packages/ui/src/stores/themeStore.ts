import { create } from "zustand";

type Theme = "light" | "dark" | "system";

interface ThemeState {
  theme: Theme;
  resolvedTheme: "light" | "dark";
  setTheme: (theme: Theme) => void;
  setResolvedTheme: (resolved: "light" | "dark") => void;
}

const getSystemTheme = (): "light" | "dark" => {
  if (typeof window !== "undefined" && window.matchMedia("(prefers-color-scheme: dark)").matches) {
    return "dark";
  }
  return "light";
};

export const useThemeStore = create<ThemeState>((set) => ({
  theme: "system",
  resolvedTheme: getSystemTheme(),
  setTheme: (theme) =>
    set({
      theme,
      resolvedTheme: theme === "system" ? getSystemTheme() : theme,
    }),
  setResolvedTheme: (resolved) => set({ resolvedTheme: resolved }),
}));
