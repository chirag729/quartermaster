import { useEffect, useState, useCallback, useMemo } from "react";
import { useNavigate } from "react-router-dom";
import { Plus, RefreshCw, Terminal, Download, Server, Monitor, Circle, CheckSquare } from "lucide-react";
import { useFleetStore } from "@/hooks/useStore";
import { useToastStore } from "../stores/toastStore";
import { FleetOverview } from "../components/fleet/FleetOverview";
import { AddNodeDialog, type AddNodeFormData } from "../components/fleet/AddNodeDialog";
import { NodeTagFilter } from "../components/fleet/NodeTagFilter";
import { BulkActionBar } from "../components/fleet/BulkActionBar";
import { BulkBlueprintDialog } from "../components/fleet/BulkBlueprintDialog";
import { SearchInput } from "../components/ui/SearchInput";
import { Button } from "../components/ui/Button";
import { Badge } from "../components/ui/Badge";
import { Card } from "../components/ui/Card";
import { Dialog } from "../components/ui/Dialog";
import { formatError, fleetService } from "@quartermaster/core";
import { useDebounce } from "../hooks/useDebounce";
import type { SshHostEntry } from "@quartermaster/core";

type StatusFilter = "all" | "online" | "offline" | "unknown";

export function FleetPage() {
  const navigate = useNavigate();
  const {
    nodes,
    loading,
    selectedNodeIds,
    toggleNodeSelection,
    selectAllNodes,
    clearSelection,
  } = useFleetStore();
  const { addToast } = useToastStore();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [addingNode, setAddingNode] = useState(false);
  const [search, setSearch] = useState("");
  const debouncedSearch = useDebounce(search, 200);
  const [activeTags, setActiveTags] = useState<string[]>([]);
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("all");
  const [importDialogOpen, setImportDialogOpen] = useState(false);
  const [discoveredHosts, setDiscoveredHosts] = useState<SshHostEntry[]>([]);
  const [selectedHosts, setSelectedHosts] = useState<Set<string>>(new Set());
  const [discovering, setDiscovering] = useState(false);
  const [importing, setImporting] = useState(false);
  const [selectionMode, setSelectionMode] = useState(false);
  const [bulkDialogOpen, setBulkDialogOpen] = useState(false);
  const [refreshing, setRefreshing] = useState(false);

  const handleRefresh = useCallback(async () => {
    setRefreshing(true);
    try {
      await fleetService.loadNodes();
      await fleetService.pollStatuses();
    } catch (err) {
      addToast({ type: "error", title: "Failed to refresh", message: formatError(err) });
    } finally {
      setRefreshing(false);
    }
  }, [addToast]);

  // Clear selection when exiting selection mode
  useEffect(() => {
    if (!selectionMode) {
      clearSelection();
    }
  }, [selectionMode, clearSelection]);

  const handleAddNode = async (data: AddNodeFormData) => {
    setAddingNode(true);
    try {
      await fleetService.addNode({
        name: data.name,
        kind: data.kind,
        hostname: data.hostname,
        tags: [],
        sshConfig:
          data.kind === "remote"
            ? {
                host: data.sshHost || data.hostname,
                port: data.sshPort,
                username: data.sshUsername,
                auth_method: data.authMethod,
              }
            : undefined,
      });
      addToast({ type: "success", title: "Node added", message: `${data.name} was added to the fleet.` });
      setDialogOpen(false);
    } catch (err) {
      addToast({ type: "error", title: "Failed to add node", message: formatError(err) });
    } finally {
      setAddingNode(false);
    }
  };

  const handleRemoveNode = async (nodeId: string) => {
    const node = nodes.find((n) => n.id === nodeId);
    const name = node?.name ?? nodeId;
    if (!window.confirm(`Remove node "${name}"? This action cannot be undone.`)) {
      return;
    }
    try {
      await fleetService.removeNode(nodeId);
      addToast({ type: "success", title: "Node removed" });
    } catch (err) {
      addToast({ type: "error", title: "Failed to remove node", message: formatError(err) });
    }
  };

  const handleOpenTerminal = async (nodeId: string) => {
    try {
      await fleetService.openTerminal(nodeId);
    } catch (err) {
      addToast({ type: "error", title: "Failed to open terminal", message: formatError(err) });
    }
  };

  const handleRunBlueprint = (nodeId: string) => {
    navigate(`/fleet/${nodeId}`);
  };

  const handleToggleSelectionMode = () => {
    setSelectionMode((prev) => !prev);
  };

  const handleSelectAll = () => {
    selectAllNodes(filtered.map((n) => n.id));
  };

  const handleBulkRunBlueprint = () => {
    setBulkDialogOpen(true);
  };

  const handleBulkDialogClose = () => {
    setBulkDialogOpen(false);
  };

  // --- SSH Config Import ---

  const handleDiscoverHosts = async () => {
    setDiscovering(true);
    setDiscoveredHosts([]);
    setSelectedHosts(new Set());
    try {
      const hosts = await fleetService.discoverSshHosts();
      setDiscoveredHosts(hosts);
      setImportDialogOpen(true);
    } catch (err) {
      addToast({ type: "error", title: "Failed to discover SSH hosts", message: formatError(err) });
    } finally {
      setDiscovering(false);
    }
  };

  const toggleHostSelection = (alias: string) => {
    setSelectedHosts((prev) => {
      const next = new Set(prev);
      if (next.has(alias)) {
        next.delete(alias);
      } else {
        next.add(alias);
      }
      return next;
    });
  };

  const toggleAllHosts = () => {
    if (selectedHosts.size === discoveredHosts.length) {
      setSelectedHosts(new Set());
    } else {
      setSelectedHosts(new Set(discoveredHosts.map((h) => h.host_alias)));
    }
  };

  const handleImportSelected = async () => {
    setImporting(true);
    let imported = 0;
    try {
      for (const host of discoveredHosts) {
        if (!selectedHosts.has(host.host_alias)) continue;
        try {
          await fleetService.addNode({
            name: host.host_alias,
            kind: "remote",
            hostname: host.hostname,
            tags: ["ssh-import"],
            sshConfig: {
              host: host.hostname,
              port: host.port,
              username: host.username ?? "root",
              auth_method: host.identity_file
                ? { type: "key_file", private_key_path: host.identity_file }
                : { type: "agent" },
              proxy_jump: host.proxy_jump,
            },
          });
          imported++;
        } catch {
          // Skip duplicates or failures for individual hosts
        }
      }
      addToast({
        type: "success",
        title: "Import complete",
        message: `${imported} of ${selectedHosts.size} host(s) imported.`,
      });
      setImportDialogOpen(false);
    } catch (err) {
      addToast({ type: "error", title: "Import failed", message: formatError(err) });
    } finally {
      setImporting(false);
    }
  };

  // --- Status summary counts ---

  const statusCounts = useMemo(() => {
    let online = 0;
    let offline = 0;
    let unknown = 0;
    for (const node of nodes) {
      if (node.status === "online") online++;
      else if (node.status === "offline" || node.status === "error") offline++;
      else unknown++;
    }
    return { online, offline, unknown, total: nodes.length };
  }, [nodes]);

  // --- Filtering ---

  const filtered = useMemo(() => {
    return nodes.filter((n) => {
      // Text search filter (debounced for performance)
      if (debouncedSearch) {
        const lower = debouncedSearch.toLowerCase();
        const matchesSearch =
          n.name.toLowerCase().includes(lower) ||
          n.hostname.toLowerCase().includes(lower) ||
          n.tags.some((t) => t.toLowerCase().includes(lower));
        if (!matchesSearch) return false;
      }
      // Tag filter: node must have at least one of the active tags
      if (activeTags.length > 0) {
        const hasMatchingTag = activeTags.some((tag) => n.tags.includes(tag));
        if (!hasMatchingTag) return false;
      }
      // Status filter
      if (statusFilter !== "all") {
        if (statusFilter === "online" && n.status !== "online") return false;
        if (statusFilter === "offline" && n.status !== "offline" && n.status !== "error") return false;
        if (statusFilter === "unknown" && n.status !== "unknown" && n.status !== "connecting") return false;
      }
      return true;
    });
  }, [nodes, debouncedSearch, activeTags, statusFilter]);

  return (
    <div className={selectionMode && selectedNodeIds.length > 0 ? "pb-20" : ""}>
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-xl font-semibold text-text-primary-light dark:text-text-primary-dark">
            Fleet Overview
          </h1>
          <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark mt-1">
            Manage your nodes and infrastructure
          </p>
        </div>
        <div className="flex items-center gap-2">
          {nodes.length > 0 && (
            <Button
              variant={selectionMode ? "secondary" : "ghost"}
              size="sm"
              onClick={handleToggleSelectionMode}
            >
              <CheckSquare size={14} />
              {selectionMode ? "Exit Selection" : "Select"}
            </Button>
          )}
          {selectionMode && filtered.length > 0 && (
            <Button variant="ghost" size="sm" onClick={handleSelectAll}>
              Select All ({filtered.length})
            </Button>
          )}
          <Button variant="ghost" size="sm" onClick={handleDiscoverHosts} disabled={discovering}>
            <Download size={14} />
            Import from SSH Config
          </Button>
          <Button variant="ghost" size="sm" onClick={handleRefresh} disabled={loading || refreshing}>
            <RefreshCw size={14} className={loading || refreshing ? "animate-spin" : ""} />
            Refresh
          </Button>
          <Button size="sm" onClick={() => setDialogOpen(true)}>
            <Plus size={14} />
            Add Node
          </Button>
        </div>
      </div>

      {/* Status summary bar */}
      {nodes.length > 0 && (
        <Card padding={false} className="mb-4 px-4 py-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <span className="text-sm font-medium text-text-secondary-light dark:text-text-secondary-dark">
                {statusCounts.total} {statusCounts.total === 1 ? "node" : "nodes"}
              </span>
              <div className="h-4 w-px bg-border-light dark:bg-border-dark" />
              <button onClick={() => setStatusFilter(statusFilter === "online" ? "all" : "online")}>
                <Badge
                  variant="success"
                  className={statusFilter === "online" ? "ring-2 ring-green-400/50" : "cursor-pointer"}
                >
                  <Circle size={8} className="fill-green-500 text-green-500 mr-1" />
                  {statusCounts.online} Online
                </Badge>
              </button>
              <button onClick={() => setStatusFilter(statusFilter === "offline" ? "all" : "offline")}>
                <Badge
                  variant="danger"
                  className={statusFilter === "offline" ? "ring-2 ring-red-400/50" : "cursor-pointer"}
                >
                  <Circle size={8} className="fill-red-500 text-red-500 mr-1" />
                  {statusCounts.offline} Offline
                </Badge>
              </button>
              <button onClick={() => setStatusFilter(statusFilter === "unknown" ? "all" : "unknown")}>
                <Badge
                  variant="default"
                  className={statusFilter === "unknown" ? "ring-2 ring-warm-400/50" : "cursor-pointer"}
                >
                  <Circle size={8} className="fill-gray-400 text-gray-400 mr-1" />
                  {statusCounts.unknown} Unknown
                </Badge>
              </button>
              {statusFilter !== "all" && (
                <button
                  onClick={() => setStatusFilter("all")}
                  className="text-xs text-warm-500 hover:text-warm-600 dark:text-warm-400 dark:hover:text-warm-300 font-medium ml-1"
                >
                  Clear filter
                </button>
              )}
            </div>
            {refreshing && (
              <div className="flex items-center gap-1.5 text-text-secondary-light dark:text-text-secondary-dark">
                <RefreshCw size={12} className="animate-spin" />
                <span className="text-xs">Refreshing...</span>
              </div>
            )}
          </div>
        </Card>
      )}

      {/* Search, tag filter, and quick actions */}
      {nodes.length > 0 && (
        <div className="mb-4 space-y-3">
          <SearchInput
            placeholder="Search nodes..."
            onValueChange={setSearch}
            className="max-w-xs"
          />
          <NodeTagFilter
            nodes={nodes}
            activeTags={activeTags}
            onTagsChange={setActiveTags}
          />
        </div>
      )}

      {/* Quick actions row per visible node */}
      {filtered.length > 0 && !loading && !selectionMode && (
        <div className="mb-4 flex flex-wrap gap-2">
          {filtered.map((node) => (
            <div
              key={node.id}
              className="inline-flex items-center gap-1.5 bg-warm-50 dark:bg-warm-900/20 border border-border-light dark:border-border-dark rounded-lg px-2 py-1"
            >
              {node.kind === "local" ? <Monitor size={12} /> : <Server size={12} />}
              <span className="text-xs font-medium text-text-primary-light dark:text-text-primary-dark max-w-[120px] truncate">
                {node.name}
              </span>
              <button
                onClick={() => handleOpenTerminal(node.id)}
                title="Open Terminal"
                className="p-1 rounded hover:bg-warm-200/50 dark:hover:bg-warm-800/30 text-text-secondary-light dark:text-text-secondary-dark transition-colors"
              >
                <Terminal size={12} />
              </button>
              <button
                onClick={() => handleRunBlueprint(node.id)}
                title="Run Blueprint"
                className="p-1 rounded hover:bg-warm-200/50 dark:hover:bg-warm-800/30 text-text-secondary-light dark:text-text-secondary-dark transition-colors"
              >
                <Server size={12} />
              </button>
            </div>
          ))}
        </div>
      )}

      <FleetOverview
        nodes={filtered}
        loading={loading}
        onRemoveNode={handleRemoveNode}
        onAddNode={() => setDialogOpen(true)}
        selectionMode={selectionMode}
        selectedNodeIds={selectedNodeIds}
        onToggleSelection={toggleNodeSelection}
      />

      {/* Bulk action bar */}
      <BulkActionBar
        selectedCount={selectedNodeIds.length}
        onDeselectAll={clearSelection}
        onRunBlueprint={handleBulkRunBlueprint}
      />

      {/* Bulk blueprint dialog */}
      <BulkBlueprintDialog
        open={bulkDialogOpen}
        onClose={handleBulkDialogClose}
        selectedNodeIds={selectedNodeIds}
        nodes={nodes}
      />

      <AddNodeDialog
        open={dialogOpen}
        onClose={() => setDialogOpen(false)}
        onSubmit={handleAddNode}
        loading={addingNode}
      />

      {/* Import SSH Config Dialog */}
      <Dialog
        open={importDialogOpen}
        onClose={() => setImportDialogOpen(false)}
        title="Import from SSH Config"
      >
        <div className="space-y-4">
          {discoveredHosts.length === 0 ? (
            <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark">
              No hosts found in your SSH config.
            </p>
          ) : (
            <>
              <div className="flex items-center justify-between">
                <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark">
                  Found {discoveredHosts.length} host(s). Select which to import:
                </p>
                <button
                  onClick={toggleAllHosts}
                  className="text-xs text-warm-500 hover:text-warm-600 dark:text-warm-400 dark:hover:text-warm-300 font-medium"
                >
                  {selectedHosts.size === discoveredHosts.length ? "Deselect All" : "Select All"}
                </button>
              </div>
              <div className="max-h-64 overflow-y-auto space-y-1 border border-border-light dark:border-border-dark rounded-lg p-2">
                {discoveredHosts.map((host) => (
                  <label
                    key={host.host_alias}
                    className="flex items-center gap-3 p-2 rounded-lg hover:bg-warm-50 dark:hover:bg-warm-900/20 cursor-pointer"
                  >
                    <input
                      type="checkbox"
                      checked={selectedHosts.has(host.host_alias)}
                      onChange={() => toggleHostSelection(host.host_alias)}
                      className="rounded border-border-light dark:border-border-dark text-warm-500 focus:ring-warm-300/50"
                    />
                    <div className="min-w-0 flex-1">
                      <div className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark truncate">
                        {host.host_alias}
                      </div>
                      <div className="text-xs text-text-secondary-light dark:text-text-secondary-dark truncate">
                        {host.username ? `${host.username}@` : ""}{host.hostname}:{host.port}
                      </div>
                    </div>
                  </label>
                ))}
              </div>
            </>
          )}
          <div className="flex justify-end gap-2 pt-2">
            <Button variant="ghost" onClick={() => setImportDialogOpen(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleImportSelected}
              disabled={selectedHosts.size === 0}
              loading={importing}
            >
              Import {selectedHosts.size > 0 ? `(${selectedHosts.size})` : ""}
            </Button>
          </div>
        </div>
      </Dialog>

    </div>
  );
}
