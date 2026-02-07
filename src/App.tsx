import { HashRouter, Routes, Route, Navigate, useLocation } from "react-router-dom";
import { AppShell } from "./components/layout/AppShell";
import { CommandPalette } from "./components/layout/CommandPalette";
import { DashboardPage } from "./pages/DashboardPage";
import { FleetPage } from "./pages/FleetPage";
import { NodeDetailPage } from "./pages/NodeDetailPage";
import { TaskLibraryPage } from "./pages/TaskLibraryPage";
import { AppArmorPage } from "./pages/AppArmorPage";
import { SettingsPage } from "./pages/SettingsPage";
import { BlueprintsPage } from "./pages/BlueprintsPage";
import { BlueprintDetailPage } from "./pages/BlueprintDetailPage";
import { ErrorBoundary } from "./components/ui/ErrorBoundary";
import { useThemeStore } from "./stores/themeStore";
import { useEffect } from "react";
import { motion, AnimatePresence } from "motion/react";

function AnimatedRoutes() {
  const location = useLocation();

  return (
    <AnimatePresence mode="wait">
      <motion.div
        key={location.pathname}
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        exit={{ opacity: 0, y: -8 }}
        transition={{ duration: 0.2 }}
      >
        <ErrorBoundary key={location.pathname}>
          <Routes location={location}>
            <Route path="/" element={<Navigate to="/dashboard" replace />} />
            <Route path="/dashboard" element={<DashboardPage />} />
            <Route path="/fleet" element={<FleetPage />} />
            <Route path="/fleet/:nodeId" element={<NodeDetailPage />} />
            <Route path="/tasks" element={<TaskLibraryPage />} />
            <Route path="/blueprints" element={<BlueprintsPage />} />
            <Route path="/blueprints/:blueprintId" element={<BlueprintDetailPage />} />
            <Route path="/apparmor" element={<AppArmorPage />} />
            <Route path="/settings" element={<SettingsPage />} />
          </Routes>
        </ErrorBoundary>
      </motion.div>
    </AnimatePresence>
  );
}

export default function App() {
  const { theme, resolvedTheme } = useThemeStore();

  useEffect(() => {
    const root = document.documentElement;
    if (resolvedTheme === "dark") {
      root.classList.add("dark");
    } else {
      root.classList.remove("dark");
    }
  }, [resolvedTheme]);

  useEffect(() => {
    if (theme === "system") {
      const mq = window.matchMedia("(prefers-color-scheme: dark)");
      const handler = (e: MediaQueryListEvent) => {
        useThemeStore.getState().setResolvedTheme(e.matches ? "dark" : "light");
      };
      mq.addEventListener("change", handler);
      return () => mq.removeEventListener("change", handler);
    }
  }, [theme]);

  return (
    <HashRouter>
      <AppShell>
        <AnimatedRoutes />
      </AppShell>
      <CommandPalette />
    </HashRouter>
  );
}
