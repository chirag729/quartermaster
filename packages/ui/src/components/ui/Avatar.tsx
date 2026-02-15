import clsx from "clsx";

interface AvatarProps {
  name: string;
  size?: "sm" | "md" | "lg";
  className?: string;
}

const sizeStyles = {
  sm: "h-7 w-7 text-xs",
  md: "h-9 w-9 text-sm",
  lg: "h-12 w-12 text-base",
};

function getInitials(name: string): string {
  return name
    .split(/[\s-]+/)
    .slice(0, 2)
    .map((w) => w[0]?.toUpperCase() ?? "")
    .join("");
}

function getColor(name: string): string {
  const colors = [
    "bg-warm-200 text-warm-700 dark:bg-warm-800 dark:text-warm-200",
    "bg-blue-200 text-blue-700 dark:bg-blue-800 dark:text-blue-200",
    "bg-green-200 text-green-700 dark:bg-green-800 dark:text-green-200",
    "bg-purple-200 text-purple-700 dark:bg-purple-800 dark:text-purple-200",
    "bg-rose-200 text-rose-700 dark:bg-rose-800 dark:text-rose-200",
  ];
  let hash = 0;
  for (const ch of name) hash = (hash * 31 + ch.charCodeAt(0)) | 0;
  return colors[Math.abs(hash) % colors.length];
}

export function Avatar({ name, size = "md", className }: AvatarProps) {
  return (
    <div
      className={clsx(
        "inline-flex items-center justify-center rounded-full font-medium shrink-0",
        sizeStyles[size],
        getColor(name),
        className,
      )}
      title={name}
    >
      {getInitials(name)}
    </div>
  );
}
