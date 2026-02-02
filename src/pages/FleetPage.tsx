import { useEffect, useState, useCallback } from "react";
import { Plus, RefreshCw } from "lucide-react";
import { useFleetStore } from "../stores/fleetStore";
import { useToastStore } from "../stores/toastStore";
import { FleetOverview } from "../components/fleet/FleetOverview";
import { AddNodeDialog, type AddNodeFormData } from "../components/fleet/AddNodeDialog";
import { NodeTagFilter } from "../components/fleet/NodeTagFilter";
import { SearchInput } from "../components/ui/SearchInput";
import { Button } from "../components/ui/Button";
import { ToastContainer } from "../components/ui/Toast";
import * as api from "../services/tauriCommands";

export function FleetPage() {
  const { nodes, loading, setNodes, setLoading } = useFleetStore();
  const { addToast } = useToastStore();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [addingNode, setAddingNode] = useState(false);
  const [search, setSearch] = useState("");
  const [activeTags, setActiveTags] = useState<string[]>([]);

  const loadNodes = useCallback(async () => {
    setLoading(true);
    try {
      const result = await api.listNodes();
      setNodes(result);
    } catch (err) {
      addToast({ type: "error", title: "Failed to load nodes", message: String(err) });
    } finally {
      setLoading(false);
    }
  }, [setNodes, setLoading, addToast]);

  useEffect(() => {
    loadNodes();
  }, [loadNodes]);

  const handleAddNode = async (data: AddNodeFormData) => {
    setAddingNode(true);
    try {
      await api.addNode({
        name: data.name,
        kind: data.kind,
        hostname: data.hostname,
        tags: [],
        ssh_config:
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
      await loadNodes();
    } catch (err) {
      addToast({ type: "error", title: "Failed to add node", message: String(err) });
    } finally {
      setAddingNode(false);
    }
  };

  const handleRemoveNode = async (nodeId: string) => {
    try {
      await api.removeNode(nodeId);
      addToast({ type: "success", title: "Node removed" });
      await loadNodes();
    } catch (err) {
      addToast({ type: "error", title: "Failed to remove node", message: String(err) });
    }
  };

  const filtered = nodes.filter((n) => {
    // Text search filter
    if (search) {
      const lower = search.toLowerCase();
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
    return true;
  });

  return (
    <div>
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
          <Button variant="ghost" size="sm" onClick={loadNodes} disabled={loading}>
            <RefreshCw size={14} className={loading ? "animate-spin" : ""} />
            Refresh
          </Button>
          <Button size="sm" onClick={() => setDialogOpen(true)}>
            <Plus size={14} />
            Add Node
          </Button>
        </div>
      </div>
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
      <FleetOverview
        nodes={filtered}
        loading={loading}
        onRemoveNode={handleRemoveNode}
        onAddNode={() => setDialogOpen(true)}
      />
      <AddNodeDialog
        open={dialogOpen}
        onClose={() => setDialogOpen(false)}
        onSubmit={handleAddNode}
        loading={addingNode}
      />
      <ToastContainer />
    </div>
  );
}
