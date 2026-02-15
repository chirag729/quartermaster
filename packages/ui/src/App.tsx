import { createHashRouter, RouterProvider, Outlet, Navigate, useLocation } from "react-router-dom";
import { AppShell } from "./components/layout/AppShell";
import { CommandPalette } from "./components/layout/CommandPalette";
import { DashboardPage } from "./pages/DashboardPage";
import { FleetPage } from "./pages/FleetPage";
import { NodeDetailPage } from "./pages/NodeDetailPage";
import { TaskLibraryPage } from "./pages/TaskLibraryPage";
import { TaskDetailPage } from "./pages/TaskDetailPage";
import { AppArmorPage } from "./pages/AppArmorPage";
import { SettingsPage } from "./pages/SettingsPage";
import { BlueprintsPage } from "./pages/BlueprintsPage";
import { BlueprintDetailPage } from "./pages/BlueprintDetailPage";
import { ErrorBoundary } from "./components/ui/ErrorBoundary";
import { useThemeStore } from "./stores/themeStore";
import { useEffect } from "react";
import { motion, AnimatePresence } from "motion/react";
import { initializeServices, destroyServices } from "@quartermaster/core";

function AnimatedOutlet() {
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
          <Outlet />
        </ErrorBoundary>
      </motion.div>
    </AnimatePresence>
  );
}

function RootLayout() {
  return (
    <>
      <AppShell>
        <AnimatedOutlet />
      </AppShell>
      <CommandPalette />
    </>
  );
}

const router = createHashRouter([
  {
    element: <RootLayout />,
    children: [
      { index: true, element: <Navigate to="dashboard" replace /> },
      { path: "dashboard", element: <DashboardPage /> },
      { path: "fleet", element: <FleetPage /> },
      { path: "fleet/:nodeId", element: <NodeDetailPage /> },
      { path: "tasks", element: <TaskLibraryPage /> },
      { path: "tasks/:taskId", element: <TaskDetailPage /> },
      { path: "blueprints", element: <BlueprintsPage /> },
      { path: "blueprints/:blueprintId", element: <BlueprintDetailPage /> },
      { path: "apparmor", element: <AppArmorPage /> },
      { path: "settings", element: <SettingsPage /> },
    ],
  },
]);

export default function App() {
  const { theme, resolvedTheme } = useThemeStore();

  // Initialize all app services on mount
  useEffect(() => {
    initializeServices();
    return () => { destroyServices(); };
  }, []);

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

  return <RouterProvider router={router} />;
}
