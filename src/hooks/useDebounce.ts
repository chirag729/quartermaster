import { useState, useEffect } from "react";

/**
 * Debounce a value by the specified delay.
 * Returns the debounced value that only updates after the delay has elapsed
 * since the last change.
 */
export function useDebounce<T>(value: T, delayMs: number): T {
  const [debounced, setDebounced] = useState(value);

  useEffect(() => {
    const timer = setTimeout(() => setDebounced(value), delayMs);
    return () => clearTimeout(timer);
  }, [value, delayMs]);

  return debounced;
}
