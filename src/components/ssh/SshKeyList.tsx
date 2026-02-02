import { useCallback, useEffect, useState } from "react";
import { Key, Plus, Shield } from "lucide-react";
import { Badge } from "../ui/Badge";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { Skeleton } from "../ui/Skeleton";
import { EmptyState } from "../ui/EmptyState";
import { listSshKeys, generateSshKey, type SshKeyInfo } from "../../services/tauriCommands";

type GenerateMode = "ed25519" | "ed25519-sk" | null;

export function SshKeyList() {
  const [keys, setKeys] = useState<SshKeyInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [generateMode, setGenerateMode] = useState<GenerateMode>(null);
  const [comment, setComment] = useState("");
  const [generating, setGenerating] = useState(false);
  const [generateError, setGenerateError] = useState<string | null>(null);

  const loadKeys = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await listSshKeys();
      setKeys(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const result = await listSshKeys();
        if (!cancelled) setKeys(result);
      } catch (err) {
        if (!cancelled) setError(String(err));
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    load();
    return () => {
      cancelled = true;
    };
  }, []);

  const handleGenerate = async () => {
    if (!generateMode || !comment.trim()) return;
    setGenerating(true);
    setGenerateError(null);
    try {
      await generateSshKey(generateMode, comment.trim());
      setGenerateMode(null);
      setComment("");
      await loadKeys();
    } catch (err) {
      setGenerateError(String(err));
    } finally {
      setGenerating(false);
    }
  };

  const handleCancel = () => {
    setGenerateMode(null);
    setComment("");
    setGenerateError(null);
  };

  return (
    <div className="space-y-4">
      {/* Generate Key Section */}
      <div className="space-y-3">
        <h4 className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark">
          Generate New Key
        </h4>
        {generateMode === null ? (
          <div className="flex items-center gap-2">
            <Button
              variant="secondary"
              size="sm"
              onClick={() => setGenerateMode("ed25519")}
            >
              <Plus size={14} />
              Generate ed25519 Key
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => setGenerateMode("ed25519-sk")}
            >
              <Shield size={14} />
              Generate YubiKey Key
            </Button>
          </div>
        ) : (
          <div className="p-3 rounded-lg border border-border-light dark:border-border-dark bg-surface-light dark:bg-surface-dark space-y-3">
            <div className="flex items-center gap-2">
              <Badge variant={generateMode === "ed25519-sk" ? "warning" : "info"}>
                {generateMode}
              </Badge>
              {generateMode === "ed25519-sk" && (
                <span className="text-xs text-text-secondary-light dark:text-text-secondary-dark">
                  Requires YubiKey or FIDO2 security key
                </span>
              )}
            </div>
            <Input
              label="Comment"
              placeholder="e.g. work-laptop, deploy-key"
              value={comment}
              onChange={(e) => setComment(e.target.value)}
              disabled={generating}
            />
            {generateError && (
              <p className="text-xs text-red-600 dark:text-red-400">
                {generateError}
              </p>
            )}
            <div className="flex items-center gap-2">
              <Button
                variant="primary"
                size="sm"
                onClick={handleGenerate}
                disabled={!comment.trim()}
                loading={generating}
              >
                Generate
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={handleCancel}
                disabled={generating}
              >
                Cancel
              </Button>
            </div>
          </div>
        )}
      </div>

      {/* Key List Section */}
      {loading ? (
        <div className="space-y-2">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} className="h-10" rounded="lg" />
          ))}
        </div>
      ) : error ? (
        <p className="text-sm text-red-600 dark:text-red-400">{error}</p>
      ) : keys.length === 0 ? (
        <EmptyState
          icon={<Key size={32} />}
          title="No SSH keys found"
          description="No SSH keys were detected in your ~/.ssh directory."
        />
      ) : (
        <div className="space-y-2">
          <h4 className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark mb-2">
            Available SSH Keys
          </h4>
          <ul className="space-y-1.5">
            {keys.map((key) => (
              <li
                key={key.path}
                className="flex items-center gap-2 px-3 py-2 rounded-lg bg-surface-light dark:bg-surface-dark border border-border-light dark:border-border-dark text-sm"
              >
                <Key
                  size={14}
                  className="text-text-secondary-light dark:text-text-secondary-dark shrink-0"
                />
                <span className="text-text-primary-light dark:text-text-primary-dark font-medium truncate">
                  {key.name}
                </span>
                <Badge variant="info">{key.key_type}</Badge>
                {key.is_fido2 && (
                  <Badge variant="warning">
                    <Shield size={10} className="inline mr-0.5" />
                    FIDO2
                  </Badge>
                )}
                <span className="ml-auto text-xs text-text-secondary-light dark:text-text-secondary-dark truncate max-w-[200px]">
                  {key.path}
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
