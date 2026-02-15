import clsx from "clsx";

interface SkeletonProps {
  className?: string;
  rounded?: "md" | "lg" | "xl" | "full";
}

export function Skeleton({ className, rounded = "lg" }: SkeletonProps) {
  return (
    <div
      className={clsx(
        "animate-pulse bg-warm-100 dark:bg-warm-900/20",
        rounded === "md" && "rounded-md",
        rounded === "lg" && "rounded-lg",
        rounded === "xl" && "rounded-xl",
        rounded === "full" && "rounded-full",
        className,
      )}
    />
  );
}
