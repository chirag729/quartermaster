/** Structured error returned by Tauri IPC when a command fails. */
interface TauriError {
  kind: string;
  message: string;
  hint?: string | null;
}

function isTauriError(err: unknown): err is TauriError {
  return (
    typeof err === "object" &&
    err !== null &&
    "message" in err &&
    typeof (err as TauriError).message === "string"
  );
}

/**
 * Extract a user-friendly message from a Tauri command error.
 * Handles both the structured `{kind, message, hint}` format
 * and plain string errors.
 */
export function formatError(err: unknown): string {
  if (isTauriError(err)) {
    if (err.hint) {
      return `${err.message}. ${err.hint}`;
    }
    return err.message;
  }
  if (err instanceof Error) {
    return err.message;
  }
  return String(err);
}
