interface Props {
  original: string;
  modified: string;
}

export function ProfileDiff({ original, modified }: Props) {
  const origLines = original.split("\n");
  const modLines = modified.split("\n");

  const maxLen = Math.max(origLines.length, modLines.length);
  const lines: { type: "same" | "added" | "removed"; text: string }[] = [];

  for (let i = 0; i < maxLen; i++) {
    const orig = origLines[i];
    const mod = modLines[i];
    if (orig === mod) {
      lines.push({ type: "same", text: orig || "" });
    } else {
      if (orig !== undefined) lines.push({ type: "removed", text: orig });
      if (mod !== undefined) lines.push({ type: "added", text: mod });
    }
  }

  return (
    <pre className="text-xs font-mono p-3 rounded-lg bg-surface-light dark:bg-surface-dark overflow-x-auto">
      {lines.map((line, i) => (
        <div
          key={i}
          className={
            line.type === "added"
              ? "bg-green-100/50 dark:bg-green-900/20 text-green-700 dark:text-green-300"
              : line.type === "removed"
                ? "bg-red-100/50 dark:bg-red-900/20 text-red-700 dark:text-red-300"
                : "text-text-primary-light dark:text-text-primary-dark"
          }
        >
          <span className="inline-block w-6 text-right mr-2 text-text-secondary-light dark:text-text-secondary-dark select-none">
            {line.type === "added" ? "+" : line.type === "removed" ? "-" : " "}
          </span>
          {line.text}
        </div>
      ))}
    </pre>
  );
}
