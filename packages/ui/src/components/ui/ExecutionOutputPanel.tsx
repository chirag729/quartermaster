import { useState, useEffect, useRef, useCallback } from "react";
import { ChevronDown, ChevronRight, CheckCircle2, XCircle, Clock } from "lucide-react";
import { useTauriEvent } from "../../hooks/useTauriEvent";
import type { TaskOutputEvent } from "@quartermaster/core";

interface StepEntry {
  step_index: number;
  step_total: number;
  step_name: string;
  command: string;
  stdout: string;
  stderr: string;
  exit_code: number;
  duration_ms: number;
  task_id: string;
}

interface ExecutionOutputPanelProps {
  nodeId: string;
  taskIdFilter?: string;
  isActive: boolean;
}

export function ExecutionOutputPanel({
  nodeId,
  taskIdFilter,
  isActive,
}: ExecutionOutputPanelProps) {
  const [steps, setSteps] = useState<StepEntry[]>([]);
  const [expandedSteps, setExpandedSteps] = useState<Set<string>>(new Set());
  const scrollRef = useRef<HTMLDivElement>(null);

  // Reset when activity starts fresh
  useEffect(() => {
    if (isActive) {
      setSteps([]);
      setExpandedSteps(new Set());
    }
  }, [isActive, nodeId, taskIdFilter]);

  useTauriEvent<TaskOutputEvent>("task-output", useCallback((payload: TaskOutputEvent) => {
    if (payload.node_id !== nodeId) return;
    if (taskIdFilter && payload.task_id !== taskIdFilter) return;

    const entry: StepEntry = {
      step_index: payload.step_index,
      step_total: payload.step_total,
      step_name: payload.step_name,
      command: payload.command,
      stdout: payload.stdout,
      stderr: payload.stderr,
      exit_code: payload.exit_code,
      duration_ms: payload.duration_ms,
      task_id: payload.task_id,
    };

    setSteps((prev) => [...prev, entry]);

    // Auto-expand failed steps
    if (payload.exit_code !== 0) {
      const key = `${payload.task_id}-${payload.step_index}`;
      setExpandedSteps((prev) => new Set(prev).add(key));
    }
  }, [nodeId, taskIdFilter]));

  // Auto-scroll to bottom
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [steps]);

  const toggleStep = (key: string) => {
    setExpandedSteps((prev) => {
      const next = new Set(prev);
      if (next.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
      }
      return next;
    });
  };

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(1)}s`;
  };

  return (
    <div
      ref={scrollRef}
      className="bg-gray-950 rounded-lg border border-gray-800 max-h-[50vh] overflow-y-auto font-mono text-xs"
    >
      {steps.length === 0 && isActive && (
        <div className="p-4 text-gray-500 animate-pulse">
          Waiting for output...
        </div>
      )}
      {steps.length === 0 && !isActive && (
        <div className="p-4 text-gray-600">
          No output captured.
        </div>
      )}
      {steps.map((step) => {
        const key = `${step.task_id}-${step.step_index}`;
        const isExpanded = expandedSteps.has(key);
        const failed = step.exit_code !== 0;
        const hasOutput = step.stdout.trim() || step.stderr.trim();

        return (
          <div key={key} className="border-b border-gray-800/50 last:border-b-0">
            {/* Step header */}
            <button
              onClick={() => hasOutput && toggleStep(key)}
              className="w-full flex items-center gap-2 px-3 py-2 hover:bg-gray-900/50 text-left"
            >
              {hasOutput ? (
                isExpanded ? (
                  <ChevronDown size={12} className="text-gray-500 shrink-0" />
                ) : (
                  <ChevronRight size={12} className="text-gray-500 shrink-0" />
                )
              ) : (
                <span className="w-3 shrink-0" />
              )}

              {failed ? (
                <XCircle size={12} className="text-red-400 shrink-0" />
              ) : (
                <CheckCircle2 size={12} className="text-green-400 shrink-0" />
              )}

              {/* Show task_id prefix when not filtered to a single task */}
              {!taskIdFilter && (
                <span className="text-blue-400 shrink-0">[{step.task_id}]</span>
              )}

              <span className="text-gray-300 truncate flex-1">
                {step.step_name}
              </span>

              <span className="text-gray-600 shrink-0 flex items-center gap-1">
                <Clock size={10} />
                {formatDuration(step.duration_ms)}
              </span>

              <span className="text-gray-600 shrink-0">
                [{step.step_index + 1}/{step.step_total}]
              </span>
            </button>

            {/* Expanded output */}
            {isExpanded && hasOutput && (
              <div className="px-3 pb-3 space-y-2">
                {/* Command */}
                <div className="text-gray-500">
                  $ <span className="text-gray-400">{step.command}</span>
                </div>

                {/* stdout */}
                {step.stdout.trim() && (
                  <pre className="text-green-300/80 whitespace-pre-wrap break-all leading-relaxed">
                    {step.stdout.trim()}
                  </pre>
                )}

                {/* stderr */}
                {step.stderr.trim() && (
                  <pre className={`whitespace-pre-wrap break-all leading-relaxed ${failed ? "text-red-400" : "text-yellow-400/70"}`}>
                    {step.stderr.trim()}
                  </pre>
                )}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
