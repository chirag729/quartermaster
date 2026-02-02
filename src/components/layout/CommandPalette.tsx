import { useEffect, useState, useRef, useCallback, useMemo } from "react";
import { useNavigate } from "react-router-dom";
import {
  Search,
  Network,
  Layers,
  ListChecks,
  Shield,
  Settings,
  Plus,
  RefreshCw,
  type LucideIcon,
} from "lucide-react";
import clsx from "clsx";
import { AnimatePresence, motion } from "motion/react";

interface CommandItem {
  id: string;
  label: string;
  category: "Pages" | "Actions";
  icon: LucideIcon;
  action: () => void;
}

export function CommandPalette() {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const navigate = useNavigate();

  const close = useCallback(() => {
    setOpen(false);
    setQuery("");
    setSelectedIndex(0);
  }, []);

  const items: CommandItem[] = useMemo(
    () => [
      { id: "page-fleet", label: "Fleet Overview", category: "Pages", icon: Network, action: () => { navigate("/fleet"); close(); } },
      { id: "page-blueprints", label: "Blueprints", category: "Pages", icon: Layers, action: () => { navigate("/blueprints"); close(); } },
      { id: "page-tasks", label: "Task Library", category: "Pages", icon: ListChecks, action: () => { navigate("/tasks"); close(); } },
      { id: "page-apparmor", label: "AppArmor", category: "Pages", icon: Shield, action: () => { navigate("/apparmor"); close(); } },
      { id: "page-settings", label: "Settings", category: "Pages", icon: Settings, action: () => { navigate("/settings"); close(); } },
      { id: "action-add-node", label: "Add Node", category: "Actions", icon: Plus, action: () => { navigate("/fleet"); close(); } },
      { id: "action-create-blueprint", label: "Create Blueprint", category: "Actions", icon: Layers, action: () => { navigate("/blueprints"); close(); } },
      { id: "action-refresh-fleet", label: "Refresh Fleet", category: "Actions", icon: RefreshCw, action: () => { navigate("/fleet"); close(); } },
    ],
    [navigate, close],
  );

  const filtered = useMemo(() => {
    if (!query.trim()) return items;
    const lower = query.toLowerCase();
    return items.filter(
      (item) =>
        item.label.toLowerCase().includes(lower) ||
        item.category.toLowerCase().includes(lower),
    );
  }, [query, items]);

  const grouped = useMemo(() => {
    const groups: { category: string; items: CommandItem[] }[] = [];
    const seen = new Set<string>();
    for (const item of filtered) {
      if (!seen.has(item.category)) {
        seen.add(item.category);
        groups.push({ category: item.category, items: [] });
      }
      groups.find((g) => g.category === item.category)!.items.push(item);
    }
    return groups;
  }, [filtered]);

  // Flatten for keyboard navigation
  const flatFiltered = useMemo(() => filtered, [filtered]);

  // Global keyboard shortcut
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setOpen((prev) => !prev);
        if (!open) {
          setQuery("");
          setSelectedIndex(0);
        }
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open]);

  // Focus input when opened
  useEffect(() => {
    if (open) {
      requestAnimationFrame(() => inputRef.current?.focus());
    }
  }, [open]);

  // Reset selection when filtered list changes
  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  // Scroll selected item into view
  useEffect(() => {
    if (!listRef.current) return;
    const selected = listRef.current.querySelector("[data-selected=\"true\"]");
    if (selected) {
      selected.scrollIntoView({ block: "nearest" });
    }
  }, [selectedIndex]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        setSelectedIndex((prev) => (prev + 1) % flatFiltered.length);
        break;
      case "ArrowUp":
        e.preventDefault();
        setSelectedIndex((prev) => (prev - 1 + flatFiltered.length) % flatFiltered.length);
        break;
      case "Enter":
        e.preventDefault();
        if (flatFiltered[selectedIndex]) {
          flatFiltered[selectedIndex].action();
        }
        break;
      case "Escape":
        e.preventDefault();
        close();
        break;
    }
  };

  // Track the flat index across grouped rendering
  let flatIndex = -1;

  return (
    <AnimatePresence>
      {open && (
        <motion.div
          className="fixed inset-0 z-50 flex items-start justify-center pt-[20vh]"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.15 }}
        >
          {/* Backdrop */}
          <div
            className="absolute inset-0 bg-black/50 backdrop-blur-sm"
            onClick={close}
          />

          {/* Palette */}
          <motion.div
            className="relative w-full max-w-lg bg-card-light dark:bg-card-dark rounded-xl border border-border-light dark:border-border-dark shadow-2xl overflow-hidden"
            initial={{ opacity: 0, scale: 0.95, y: -10 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.95, y: -10 }}
            transition={{ duration: 0.15 }}
            onKeyDown={handleKeyDown}
          >
            {/* Search input */}
            <div className="flex items-center gap-3 px-4 border-b border-border-light dark:border-border-dark">
              <Search
                size={16}
                className="text-text-secondary-light/50 dark:text-text-secondary-dark/50 shrink-0"
              />
              <input
                ref={inputRef}
                type="text"
                placeholder="Search commands..."
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                className="flex-1 py-3 text-sm bg-transparent text-text-primary-light dark:text-text-primary-dark placeholder:text-text-secondary-light/50 dark:placeholder:text-text-secondary-dark/50 focus:outline-none"
              />
              <kbd className="hidden sm:inline-flex items-center gap-1 px-1.5 py-0.5 text-[10px] font-medium text-text-secondary-light dark:text-text-secondary-dark bg-surface-light dark:bg-surface-dark rounded border border-border-light dark:border-border-dark">
                ESC
              </kbd>
            </div>

            {/* Results */}
            <div ref={listRef} className="max-h-72 overflow-y-auto p-2">
              {grouped.length === 0 ? (
                <p className="py-8 text-center text-sm text-text-secondary-light dark:text-text-secondary-dark">
                  No results found.
                </p>
              ) : (
                grouped.map((group) => (
                  <div key={group.category} className="mb-2 last:mb-0">
                    <p className="text-[10px] font-bold uppercase tracking-widest text-text-secondary-light/60 dark:text-text-secondary-dark/60 px-3 py-1.5">
                      {group.category}
                    </p>
                    {group.items.map((item) => {
                      flatIndex++;
                      const isSelected = flatIndex === selectedIndex;
                      const currentFlatIndex = flatIndex;
                      return (
                        <button
                          key={item.id}
                          data-selected={isSelected}
                          onClick={item.action}
                          onMouseEnter={() => setSelectedIndex(currentFlatIndex)}
                          className={clsx(
                            "flex items-center gap-3 w-full px-3 py-2 rounded-lg text-sm transition-colors",
                            isSelected
                              ? "bg-warm-400/15 text-warm-700 dark:text-warm-300"
                              : "text-text-primary-light dark:text-text-primary-dark hover:bg-warm-100/30 dark:hover:bg-warm-900/30",
                          )}
                        >
                          <item.icon size={16} className={clsx(isSelected ? "text-warm-500" : "text-text-secondary-light dark:text-text-secondary-dark")} />
                          <span className="font-medium">{item.label}</span>
                        </button>
                      );
                    })}
                  </div>
                ))
              )}
            </div>

            {/* Footer hint */}
            <div className="flex items-center gap-4 px-4 py-2 border-t border-border-light dark:border-border-dark text-[10px] text-text-secondary-light/60 dark:text-text-secondary-dark/60">
              <span className="flex items-center gap-1">
                <kbd className="px-1 py-0.5 rounded border border-border-light dark:border-border-dark bg-surface-light dark:bg-surface-dark font-mono">
                  &uarr;&darr;
                </kbd>
                navigate
              </span>
              <span className="flex items-center gap-1">
                <kbd className="px-1 py-0.5 rounded border border-border-light dark:border-border-dark bg-surface-light dark:bg-surface-dark font-mono">
                  &crarr;
                </kbd>
                select
              </span>
              <span className="flex items-center gap-1">
                <kbd className="px-1 py-0.5 rounded border border-border-light dark:border-border-dark bg-surface-light dark:bg-surface-dark font-mono">
                  esc
                </kbd>
                close
              </span>
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
