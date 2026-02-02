import { useEffect, useState, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { Plus, RefreshCw } from "lucide-react";
import { useBlueprintStore } from "../stores/blueprintStore";
import { useToastStore } from "../stores/toastStore";
import { BlueprintGrid } from "../components/blueprints/BlueprintGrid";
import { CreateBlueprintDialog } from "../components/blueprints/CreateBlueprintDialog";
import { SearchInput } from "../components/ui/SearchInput";
import { Button } from "../components/ui/Button";
import { ToastContainer } from "../components/ui/Toast";
import * as api from "../services/tauriCommands";

export function BlueprintsPage() {
  const { blueprints, loading, setBlueprints, setLoading, createBlankBlueprint } = useBlueprintStore();
  const { addToast } = useToastStore();
  const navigate = useNavigate();
  const [search, setSearch] = useState("");
  const [showCreate, setShowCreate] = useState(false);
  const [creating, setCreating] = useState(false);

  const loadBlueprints = useCallback(async () => {
    setLoading(true);
    try {
      const result = await api.listBlueprints();
      setBlueprints(result);
    } catch (err) {
      addToast({ type: "error", title: "Failed to load blueprints", message: String(err) });
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
      addToast({ type: "error", title: "Failed to create blueprint", message: String(err) });
    } finally {
      setCreating(false);
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
      />
      <CreateBlueprintDialog
        open={showCreate}
        onClose={() => setShowCreate(false)}
        onSubmit={handleCreateBlueprint}
        loading={creating}
      />
      <ToastContainer />
    </div>
  );
}
