import { useMemo } from "react";
import clsx from "clsx";
import type { Node } from "@quartermaster/core";

interface NodeTagFilterProps {
  nodes: Node[];
  activeTags: string[];
  onTagsChange: (tags: string[]) => void;
}

export function NodeTagFilter({ nodes, activeTags, onTagsChange }: NodeTagFilterProps) {
  const allTags = useMemo(() => {
    const tagSet = new Set<string>();
    for (const node of nodes) {
      for (const tag of node.tags) {
        tagSet.add(tag);
      }
    }
    return Array.from(tagSet).sort();
  }, [nodes]);

  if (allTags.length === 0) return null;

  const handleTagClick = (tag: string) => {
    if (activeTags.includes(tag)) {
      onTagsChange(activeTags.filter((t) => t !== tag));
    } else {
      onTagsChange([...activeTags, tag]);
    }
  };

  const handleClear = () => {
    onTagsChange([]);
  };

  return (
    <div className="flex items-center gap-2 flex-wrap">
      <button
        onClick={handleClear}
        className={clsx(
          "inline-flex items-center px-2.5 py-1 rounded-md text-xs font-medium transition-colors",
          activeTags.length === 0
            ? "bg-warm-500 text-white"
            : "bg-warm-100 text-warm-700 dark:bg-warm-900/30 dark:text-warm-300 hover:bg-warm-200 dark:hover:bg-warm-900/50",
        )}
      >
        All
      </button>
      {allTags.map((tag) => {
        const isActive = activeTags.includes(tag);
        return (
          <button
            key={tag}
            onClick={() => handleTagClick(tag)}
            className={clsx(
              "inline-flex items-center px-2.5 py-1 rounded-md text-xs font-medium transition-colors",
              isActive
                ? "bg-warm-500 text-white"
                : "bg-warm-100 text-warm-700 dark:bg-warm-900/30 dark:text-warm-300 hover:bg-warm-200 dark:hover:bg-warm-900/50",
            )}
          >
            {tag}
          </button>
        );
      })}
    </div>
  );
}
