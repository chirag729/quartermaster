import { useState, useCallback } from "react";
import { NavLink, useNavigate } from "react-router-dom";
import {
  LayoutDashboard,
  Network,
  Server,
  Monitor,
  Plus,
  Layers,
  ListChecks,
  Shield,
  Settings,
  PanelLeftClose,
  PanelLeftOpen,
  type LucideIcon,
} from "lucide-react";
import clsx from "clsx";
import { useFleetStore } from "@/hooks/useStore";
import type { NodeStatus } from "@quartermaster/core";

const STORAGE_KEY = "sidebar-collapsed";

function useCollapsed() {
  const [collapsed, setCollapsed] = useState(() => {
    try {
      return localStorage.getItem(STORAGE_KEY) === "true";
    } catch {
      return false;
    }
  });

  const toggle = useCallback(() => {
    setCollapsed((prev) => {
      const next = !prev;
      try {
        localStorage.setItem(STORAGE_KEY, String(next));
      } catch {
        // storage unavailable
      }
      return next;
    });
  }, []);

  return [collapsed, toggle] as const;
}

function Tooltip({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="relative group/tooltip">
      {children}
      <div
        className="absolute left-full top-1/2 -translate-y-1/2 ml-2 px-2.5 py-1 rounded-md text-xs font-medium whitespace-nowrap
          bg-warm-800 text-white dark:bg-warm-200 dark:text-warm-900
          opacity-0 pointer-events-none group-hover/tooltip:opacity-100 transition-opacity z-50"
      >
        {label}
      </div>
    </div>
  );
}

function statusDotColor(status: NodeStatus): string {
  switch (status) {
    case "online":
      return "bg-green-500";
    case "connecting":
      return "bg-yellow-500";
    case "offline":
    case "error":
      return "bg-red-500";
    case "unknown":
    default:
      return "bg-gray-400";
  }
}

function NavItem({
  to,
  icon: Icon,
  label,
  collapsed,
}: {
  to: string;
  icon: LucideIcon;
  label: string;
  collapsed: boolean;
}) {
  const link = (
    <NavLink
      to={to}
      className={({ isActive }) =>
        clsx(
          "flex items-center py-2 rounded-lg text-sm font-medium transition-colors border-l-[3px]",
          collapsed ? "justify-center pl-0 border-l-0" : "gap-3 pl-[9px]",
          isActive
            ? clsx(
                "bg-warm-400/15 text-warm-700 dark:text-warm-300",
                !collapsed && "border-warm-500",
                collapsed && "border-l-0",
              )
            : clsx(
                "text-text-secondary-light dark:text-text-secondary-dark hover:bg-warm-100/30 dark:hover:bg-warm-900/30 hover:text-text-primary-light dark:hover:text-text-primary-dark",
                !collapsed && "border-transparent",
              ),
        )
      }
      end
    >
      <Icon size={16} className="shrink-0" />
      {!collapsed && <span>{label}</span>}
    </NavLink>
  );

  if (collapsed) {
    return <Tooltip label={label}>{link}</Tooltip>;
  }

  return link;
}

function SectionLabel({
  children,
  collapsed,
}: {
  children: React.ReactNode;
  collapsed: boolean;
}) {
  if (collapsed) {
    return (
      <div className="flex justify-center py-1">
        <div className="w-5 border-t border-text-secondary-light/20 dark:border-text-secondary-dark/20" />
      </div>
    );
  }

  return (
    <p className="text-[10px] font-bold uppercase tracking-widest text-text-secondary-light/60 dark:text-text-secondary-dark/60 px-3 mb-1.5">
      {children}
    </p>
  );
}

export function Sidebar() {
  const navigate = useNavigate();
  const { nodes } = useFleetStore();
  const [collapsed, toggle] = useCollapsed();

  return (
    <aside
      className={clsx(
        "h-screen flex flex-col border-r border-border-light dark:border-border-dark bg-sidebar-light dark:bg-sidebar-dark transition-[width] duration-200 ease-in-out",
        collapsed ? "w-16" : "w-60",
      )}
    >
      {/* Header */}
      <div className={clsx("p-6", collapsed && "px-2 py-6 flex justify-center")}>
        {collapsed ? (
          <Tooltip label="Quartermaster">
            <span className="text-lg font-semibold text-text-primary-light dark:text-text-primary-dark select-none">
              Q
            </span>
          </Tooltip>
        ) : (
          <>
            <h1 className="text-lg font-semibold text-text-primary-light dark:text-text-primary-dark">
              Quartermaster
            </h1>
            <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark mt-1">
              Infrastructure Builder
            </p>
          </>
        )}
      </div>

      <nav className="flex-1 px-3 space-y-5 overflow-y-auto" aria-label="Main navigation">
        {/* OVERVIEW section */}
        <div className="space-y-0.5">
          <SectionLabel collapsed={collapsed}>Overview</SectionLabel>
          <NavItem to="/dashboard" icon={LayoutDashboard} label="Dashboard" collapsed={collapsed} />
        </div>

        {/* FLEET section */}
        <div className="space-y-0.5">
          <SectionLabel collapsed={collapsed}>Fleet</SectionLabel>
          <NavItem to="/fleet" icon={Network} label="Fleet Overview" collapsed={collapsed} />
          {nodes.map((node) => {
            const dotColor = statusDotColor(node.status);
            const NodeIcon = node.kind === "local" ? Monitor : Server;
            const nodeLabel = node.name;

            const link = (
              <NavLink
                key={node.id}
                to={`/fleet/${node.id}`}
                className={({ isActive }) =>
                  clsx(
                    "flex items-center py-1.5 rounded-lg text-xs font-medium transition-colors border-l-[3px]",
                    collapsed ? "justify-center pl-0 border-l-0" : "gap-3 pl-[9px]",
                    isActive
                      ? clsx(
                          "bg-warm-400/15 text-warm-700 dark:text-warm-300",
                          !collapsed && "border-warm-500",
                          collapsed && "border-l-0",
                        )
                      : clsx(
                          "text-text-secondary-light dark:text-text-secondary-dark hover:bg-warm-100/30 dark:hover:bg-warm-900/30 hover:text-text-primary-light dark:hover:text-text-primary-dark",
                          !collapsed && "border-transparent",
                        ),
                  )
                }
              >
                <span className="relative shrink-0">
                  <span
                    className={clsx(
                      "absolute -top-0.5 -left-0.5 h-2 w-2 rounded-full ring-1 ring-sidebar-light dark:ring-sidebar-dark",
                      dotColor,
                    )}
                  />
                  <NodeIcon size={14} />
                </span>
                {!collapsed && <span className="truncate">{nodeLabel}</span>}
              </NavLink>
            );

            if (collapsed) {
              return (
                <Tooltip key={node.id} label={nodeLabel}>
                  {link}
                </Tooltip>
              );
            }

            return <span key={node.id}>{link}</span>;
          })}
          {collapsed ? (
            <Tooltip label="Add Node">
              <button
                onClick={() => navigate("/fleet")}
                className="flex items-center justify-center py-1.5 rounded-lg text-xs font-medium text-text-secondary-light dark:text-text-secondary-dark hover:bg-warm-100/30 dark:hover:bg-warm-900/30 hover:text-text-primary-light dark:hover:text-text-primary-dark transition-colors w-full"
              >
                <Plus size={14} />
              </button>
            </Tooltip>
          ) : (
            <button
              onClick={() => navigate("/fleet")}
              className="flex items-center gap-3 py-1.5 pl-3 rounded-lg text-xs font-medium text-text-secondary-light dark:text-text-secondary-dark hover:bg-warm-100/30 dark:hover:bg-warm-900/30 hover:text-text-primary-light dark:hover:text-text-primary-dark transition-colors w-full"
            >
              <Plus size={14} />
              Add Node
            </button>
          )}
        </div>

        {/* MANAGE section */}
        <div className="space-y-0.5">
          <SectionLabel collapsed={collapsed}>Manage</SectionLabel>
          <NavItem to="/blueprints" icon={Layers} label="Blueprints" collapsed={collapsed} />
          <NavItem to="/tasks" icon={ListChecks} label="Task Library" collapsed={collapsed} />
        </div>

        {/* SECURITY section */}
        <div className="space-y-0.5">
          <SectionLabel collapsed={collapsed}>Security</SectionLabel>
          <NavItem to="/apparmor" icon={Shield} label="AppArmor" collapsed={collapsed} />
        </div>
      </nav>

      {/* Settings at bottom */}
      <div className="px-3 pb-2">
        <NavItem to="/settings" icon={Settings} label="Settings" collapsed={collapsed} />
      </div>

      {/* Footer with version + collapse toggle */}
      <div className="px-3 pb-3 border-t border-border-light dark:border-border-dark pt-3 flex items-center justify-between">
        {!collapsed && (
          <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark">
            v0.1.0
          </p>
        )}
        <Tooltip label={collapsed ? "Expand sidebar" : "Collapse sidebar"}>
          <button
            onClick={toggle}
            className={clsx(
              "p-1.5 rounded-md text-text-secondary-light dark:text-text-secondary-dark hover:bg-warm-100/30 dark:hover:bg-warm-900/30 hover:text-text-primary-light dark:hover:text-text-primary-dark transition-colors",
              collapsed && "mx-auto",
            )}
            aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          >
            {collapsed ? <PanelLeftOpen size={16} /> : <PanelLeftClose size={16} />}
          </button>
        </Tooltip>
      </div>
    </aside>
  );
}
