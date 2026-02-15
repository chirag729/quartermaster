import type { ConsolidationResult, DenialEvent, PermissionSuggestion, ProfileInfo } from "@quartermaster/core";
import { DenialLogTable } from "./DenialLogTable";
import { ProfileList } from "./ProfileList";
import { PermissionSelector } from "./PermissionSelector";
import { LogMonitor } from "./LogMonitor";
import { RuleReviewDialog } from "./RuleReviewDialog";
import { Card, CardHeader, CardTitle } from "../ui/Card";
import { Button } from "../ui/Button";
import { Play, Square, RefreshCw } from "lucide-react";

interface Props {
  denials: DenialEvent[];
  suggestions: PermissionSuggestion[];
  profiles: ProfileInfo[];
  selectedSuggestions: Set<string>;
  monitoring: boolean;
  loading: boolean;
  reviewRules: ConsolidationResult[] | null;
  reviewMode: "append" | "rewrite" | null;
  toggleSuggestion: (id: string) => void;
  selectAllSuggestions: () => void;
  clearSelection: () => void;
  startMonitor: () => void;
  stopMonitor: () => void;
  reviewSelected: () => void;
  consolidateProfile: (profileName: string) => void;
  confirmApply: (editedRules: Map<string, string[]>) => void;
  cancelReview: () => void;
  refreshData: () => void;
}

export function AppArmorDashboard({
  denials,
  suggestions,
  profiles,
  selectedSuggestions,
  monitoring,
  loading,
  reviewRules,
  reviewMode,
  toggleSuggestion,
  selectAllSuggestions,
  clearSelection,
  startMonitor,
  stopMonitor,
  reviewSelected,
  consolidateProfile,
  confirmApply,
  cancelReview,
  refreshData,
}: Props) {
  if (loading) {
    return (
      <div className="space-y-4">
        {Array.from({ length: 3 }).map((_, i) => (
          <div key={i} className="h-32 rounded-xl bg-card-light dark:bg-card-dark border border-border-light dark:border-border-dark animate-pulse" />
        ))}
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-3">
        <Button
          variant={monitoring ? "danger" : "primary"}
          size="sm"
          onClick={monitoring ? stopMonitor : startMonitor}
        >
          {monitoring ? <Square size={14} /> : <Play size={14} />}
          {monitoring ? "Stop Monitor" : "Start Monitor"}
        </Button>
        <Button variant="ghost" size="sm" onClick={refreshData}>
          <RefreshCw size={14} />
          Refresh
        </Button>
        {selectedSuggestions.size > 0 && (
          <Button variant="primary" size="sm" onClick={reviewSelected}>
            Apply {selectedSuggestions.size} Rule{selectedSuggestions.size > 1 ? "s" : ""}
          </Button>
        )}
      </div>

      {monitoring && <LogMonitor denials={denials} />}

      <Card>
        <CardHeader>
          <CardTitle>Profiles ({profiles.length})</CardTitle>
        </CardHeader>
        <ProfileList profiles={profiles} onConsolidate={consolidateProfile} />
      </Card>

      {suggestions.length > 0 && (
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle>Permission Suggestions ({suggestions.length})</CardTitle>
              <div className="flex gap-2">
                <Button variant="ghost" size="sm" onClick={selectAllSuggestions}>
                  Select All
                </Button>
                <Button variant="ghost" size="sm" onClick={clearSelection}>
                  Clear
                </Button>
              </div>
            </div>
          </CardHeader>
          <PermissionSelector
            suggestions={suggestions}
            selected={selectedSuggestions}
            onToggle={toggleSuggestion}
          />
        </Card>
      )}

      <Card>
        <CardHeader>
          <CardTitle>Recent Denials ({denials.length})</CardTitle>
        </CardHeader>
        <DenialLogTable denials={denials} />
      </Card>

      <RuleReviewDialog
        open={reviewRules !== null}
        consolidatedRules={reviewRules ?? []}
        mode={reviewMode}
        onClose={cancelReview}
        onApply={confirmApply}
      />
    </div>
  );
}
