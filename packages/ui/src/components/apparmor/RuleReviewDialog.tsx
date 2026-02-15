import { useState, useEffect } from "react";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { ChevronDown, ChevronRight } from "lucide-react";
import type { ConsolidationResult } from "@quartermaster/core";

interface Props {
  open: boolean;
  consolidatedRules: ConsolidationResult[];
  mode?: "append" | "rewrite" | null;
  onClose: () => void;
  onApply: (editedRules: Map<string, string[]>) => void;
}

export function RuleReviewDialog({ open, consolidatedRules, mode = "append", onClose, onApply }: Props) {
  const [editedTexts, setEditedTexts] = useState<Map<string, string>>(new Map());
  const [expandedProfiles, setExpandedProfiles] = useState<Set<string>>(new Set());

  useEffect(() => {
    if (open && consolidatedRules.length > 0) {
      const initial = new Map<string, string>();
      for (const result of consolidatedRules) {
        const text = result.rules.map((r) => r.rule_text).join("\n");
        initial.set(result.profile, text);
      }
      setEditedTexts(initial);
      setExpandedProfiles(new Set());
    }
  }, [open, consolidatedRules]);

  const handleTextChange = (profile: string, value: string) => {
    setEditedTexts((prev) => {
      const next = new Map(prev);
      next.set(profile, value);
      return next;
    });
  };

  const toggleExpanded = (profile: string) => {
    setExpandedProfiles((prev) => {
      const next = new Set(prev);
      if (next.has(profile)) next.delete(profile);
      else next.add(profile);
      return next;
    });
  };

  const handleApply = () => {
    const parsed = new Map<string, string[]>();
    for (const [profile, text] of editedTexts) {
      const rules = text
        .split("\n")
        .map((l) => l.trimEnd())
        .filter((l) => l.trim().length > 0);
      if (rules.length > 0) {
        parsed.set(profile, rules);
      }
    }
    onApply(parsed);
  };

  const totalGlobs = consolidatedRules.reduce(
    (sum, r) => sum + r.rules.filter((rule) => rule.is_glob).length,
    0,
  );
  const totalOriginal = consolidatedRules.reduce(
    (sum, r) => sum + r.rules.reduce((s, rule) => s + rule.original_rules.length, 0),
    0,
  );
  const totalConsolidated = consolidatedRules.reduce((sum, r) => sum + r.rules.length, 0);

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title={mode === "rewrite" ? "Consolidate Existing Rules" : "Review Rules Before Applying"}
      className="max-w-2xl"
    >
      <div className="space-y-4 max-h-[60vh] overflow-y-auto">
        {totalGlobs > 0 && (
          <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark">
            Consolidated {totalOriginal} rules into {totalConsolidated} ({totalGlobs} glob pattern{totalGlobs > 1 ? "s" : ""})
          </p>
        )}

        {consolidatedRules.map((result) => {
          const isExpanded = expandedProfiles.has(result.profile);
          const globs = result.rules.filter((r) => r.is_glob);

          return (
            <div key={result.profile} className="space-y-2">
              <h4 className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark">
                {result.profile}
              </h4>
              <textarea
                value={editedTexts.get(result.profile) ?? ""}
                onChange={(e) => handleTextChange(result.profile, e.target.value)}
                rows={Math.min(12, (editedTexts.get(result.profile) ?? "").split("\n").length + 1)}
                spellCheck={false}
                className="w-full font-mono text-xs p-3 rounded-lg border border-border-light dark:border-border-dark bg-surface-light dark:bg-surface-dark text-text-primary-light dark:text-text-primary-dark focus:outline-none focus:ring-2 focus:ring-warm-300/50 resize-y"
              />
              {globs.length > 0 && (
                <div>
                  <button
                    onClick={() => toggleExpanded(result.profile)}
                    className="flex items-center gap-1 text-xs text-text-secondary-light dark:text-text-secondary-dark hover:text-text-primary-light dark:hover:text-text-primary-dark"
                  >
                    {isExpanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                    Show original rules ({globs.reduce((s, g) => s + g.original_rules.length, 0)} rules consolidated)
                  </button>
                  {isExpanded && (
                    <div className="mt-1 p-2 rounded-md bg-warm-50 dark:bg-warm-900/20 text-xs font-mono text-text-secondary-light dark:text-text-secondary-dark max-h-40 overflow-y-auto">
                      {globs.map((g, gi) => (
                        <div key={gi} className="mb-2">
                          <div className="text-text-primary-light dark:text-text-primary-dark">
                            {g.rule_text}
                          </div>
                          <div className="ml-4 opacity-70">
                            {g.original_rules.map((orig, oi) => (
                              <div key={oi}>{orig}</div>
                            ))}
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>
      <div className="flex justify-end gap-2 mt-4 pt-3 border-t border-border-light dark:border-border-dark">
        <Button variant="ghost" size="sm" onClick={onClose}>
          Cancel
        </Button>
        <Button variant="primary" size="sm" onClick={handleApply}>
          {mode === "rewrite" ? "Rewrite Profile" : "Apply Rules"}
        </Button>
      </div>
    </Dialog>
  );
}
