import { NavLink, useNavigate } from "react-router-dom";
import {
  Network,
  Server,
  Monitor,
  Plus,
  Layers,
  ListChecks,
  Shield,
  Settings,
  type LucideIcon,
} from "lucide-react";
import clsx from "clsx";
import { useFleetStore } from "../../stores/fleetStore";

function NavItem({ to, icon: Icon, label }: { to: string; icon: LucideIcon; label: string }) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) =>
        clsx(
          "flex items-center gap-3 py-2 rounded-lg text-sm font-medium transition-colors border-l-[3px]",
          isActive
            ? "border-warm-500 pl-[9px] bg-warm-400/15 text-warm-700 dark:text-warm-300"
            : "border-transparent pl-[9px] text-text-secondary-light dark:text-text-secondary-dark hover:bg-warm-100/30 dark:hover:bg-warm-900/30 hover:text-text-primary-light dark:hover:text-text-primary-dark",
        )
      }
      end
    >
      <Icon size={16} />
      {label}
    </NavLink>
  );
}

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <p className="text-[10px] font-bold uppercase tracking-widest text-text-secondary-light/60 dark:text-text-secondary-dark/60 px-3 mb-1.5">
      {children}
    </p>
  );
}

export function Sidebar() {
  const navigate = useNavigate();
  const { nodes } = useFleetStore();

  return (
    <aside className="w-60 h-screen flex flex-col border-r border-border-light dark:border-border-dark bg-sidebar-light dark:bg-sidebar-dark">
      <div className="p-6">
        <h1 className="text-lg font-semibold text-text-primary-light dark:text-text-primary-dark">
          Anvil
        </h1>
        <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark mt-1">
          Infrastructure Builder
        </p>
      </div>

      <nav className="flex-1 px-3 space-y-5 overflow-y-auto" aria-label="Main navigation">
        {/* FLEET section */}
        <div className="space-y-0.5">
          <SectionLabel>Fleet</SectionLabel>
          <NavItem to="/fleet" icon={Network} label="Fleet Overview" />
          {nodes.map((node) => (
            <NavLink
              key={node.id}
              to={`/fleet/${node.id}`}
              className={({ isActive }) =>
                clsx(
                  "flex items-center gap-3 py-1.5 rounded-lg text-xs font-medium transition-colors border-l-[3px] pl-[9px]",
                  isActive
                    ? "border-warm-500 bg-warm-400/15 text-warm-700 dark:text-warm-300"
                    : "border-transparent text-text-secondary-light dark:text-text-secondary-dark hover:bg-warm-100/30 dark:hover:bg-warm-900/30 hover:text-text-primary-light dark:hover:text-text-primary-dark",
                )
              }
            >
              {node.kind === "local" ? <Monitor size={14} /> : <Server size={14} />}
              <span className="truncate">{node.name}</span>
            </NavLink>
          ))}
          <button
            onClick={() => navigate("/fleet")}
            className="flex items-center gap-3 py-1.5 pl-3 rounded-lg text-xs font-medium text-text-secondary-light dark:text-text-secondary-dark hover:bg-warm-100/30 dark:hover:bg-warm-900/30 hover:text-text-primary-light dark:hover:text-text-primary-dark transition-colors w-full"
          >
            <Plus size={14} />
            Add Node
          </button>
        </div>

        {/* MANAGE section */}
        <div className="space-y-0.5">
          <SectionLabel>Manage</SectionLabel>
          <NavItem to="/blueprints" icon={Layers} label="Blueprints" />
          <NavItem to="/tasks" icon={ListChecks} label="Task Library" />
        </div>

        {/* SECURITY section */}
        <div className="space-y-0.5">
          <SectionLabel>Security</SectionLabel>
          <NavItem to="/apparmor" icon={Shield} label="AppArmor" />
        </div>
      </nav>

      {/* Settings at bottom */}
      <div className="px-3 pb-2">
        <NavItem to="/settings" icon={Settings} label="Settings" />
      </div>
      <div className="p-4 border-t border-border-light dark:border-border-dark">
        <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark">
          v0.1.0
        </p>
      </div>
    </aside>
  );
}
