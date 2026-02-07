import { useEffect, useState, useCallback, useMemo } from "react";
import { useNavigate } from "react-router-dom";
import { Plus, RefreshCw, Upload } from "lucide-react";
import { useBlueprintStore } from "../stores/blueprintStore";
import { useFleetStore } from "../stores/fleetStore";
import { useToastStore } from "../stores/toastStore";
import { BlueprintGrid } from "../components/blueprints/BlueprintGrid";
import { CreateBlueprintDialog } from "../components/blueprints/CreateBlueprintDialog";
import { ImportBlueprintDialog } from "../components/blueprints/ImportBlueprintDialog";
import { SearchInput } from "../components/ui/SearchInput";
import { Button } from "../components/ui/Button";
import { ToastContainer } from "../components/ui/Toast";
import * as api from "../services/tauriCommands";
import { formatError } from "../lib/formatError";

export function BlueprintsPage() {
  const { blueprints, loading, setBlueprints, setLoading, createBlankBlueprint, importBlueprint } = useBlueprintStore();
  const { nodes } = useFleetStore();
  const { addToast } = useToastStore();
  const navigate = useNavigate();
  const [search, setSearch] = useState("");
  const [showCreate, setShowCreate] = useState(false);
  const [showImport, setShowImport] = useState(false);
  const [creating, setCreating] = useState(false);
  const [importing, setImporting] = useState(false);

  const nodeCountByBlueprint = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const node of nodes) {
      if (node.blueprint_id) {
        counts[node.blueprint_id] = (counts[node.blueprint_id] || 0) + 1;
      }
    }
    return counts;
  }, [nodes]);

  const loadBlueprints = useCallback(async () => {
    setLoading(true);
    try {
      const result = await api.listBlueprints();
      setBlueprints(result);
    } catch (err) {
      addToast({ type: "error", title: "Failed to load blueprints", message: formatError(err) });
    } finally {
      setLoading(false);
    }
  }, [setBlueprints, setLoading, addToast]);

  useEffect(() => {
    loadBlueprints();
  }, [loadBlueprints]);

  const handleCreateBlueprint = async (name: string, description: string) => {
    setCreating(true);
    try {
      const bp = await createBlankBlueprint(name, description);
      setShowCreate(false);
      addToast({ type: "success", title: "Blueprint created", message: `Created "${bp.name}"` });
      navigate(`/blueprints/${bp.id}`);
    } catch (err) {
      addToast({ type: "error", title: "Failed to create blueprint", message: formatError(err) });
    } finally {
      setCreating(false);
    }
  };

  const handleImportBlueprint = async (path: string) => {
    setImporting(true);
    try {
      const bp = await importBlueprint(path);
      setShowImport(false);
      addToast({ type: "success", title: "Blueprint imported", message: `Imported "${bp.name}"` });
      navigate(`/blueprints/${bp.id}`);
    } catch (err) {
      addToast({ type: "error", title: "Failed to import blueprint", message: formatError(err) });
      throw err;
    } finally {
      setImporting(false);
    }
  };

  const filtered = search
    ? blueprints.filter(
        (b) =>
          b.name.toLowerCase().includes(search.toLowerCase()) ||
          b.description.toLowerCase().includes(search.toLowerCase()),
      )
    : blueprints;

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-xl font-semibold text-text-primary-light dark:text-text-primary-dark">
            Blueprints
          </h1>
          <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark mt-1">
            Reusable setup configurations for your nodes
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="sm" onClick={loadBlueprints} disabled={loading}>
            <RefreshCw size={14} className={loading ? "animate-spin" : ""} />
            Refresh
          </Button>
          <Button variant="secondary" size="sm" onClick={() => setShowImport(true)}>
            <Upload size={14} />
            Import
          </Button>
          <Button size="sm" onClick={() => setShowCreate(true)}>
            <Plus size={14} />
            Create Blueprint
          </Button>
        </div>
      </div>
      {blueprints.length > 0 && (
        <div className="mb-4">
          <SearchInput
            placeholder="Search blueprints..."
            onValueChange={setSearch}
            className="max-w-xs"
          />
        </div>
      )}
      <BlueprintGrid
        blueprints={filtered}
        loading={loading}
        onCreateBlueprint={() => setShowCreate(true)}
        nodeCountByBlueprint={nodeCountByBlueprint}
      />
      <CreateBlueprintDialog
        open={showCreate}
        onClose={() => setShowCreate(false)}
        onSubmit={handleCreateBlueprint}
        loading={creating}
      />
      <ImportBlueprintDialog
        open={showImport}
        onClose={() => setShowImport(false)}
        onImport={handleImportBlueprint}
        loading={importing}
      />
      <ToastContainer />
    </div>
  );
}
