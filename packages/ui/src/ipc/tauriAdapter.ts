import { invoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";
import type { IPCClient } from "@quartermaster/core";

export const tauriIPCClient: IPCClient = {
  invoke: <T>(command: string, args?: Record<string, unknown>) =>
    invoke<T>(command, args),

  listen: <T>(event: string, handler: (payload: T) => void) =>
    tauriListen<T>(event, (e) => handler(e.payload)).then(
      (unlisten) => unlisten,
    ),
};
