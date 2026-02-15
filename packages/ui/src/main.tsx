import React from "react";
import ReactDOM from "react-dom/client";
import { setIPCClient } from "@quartermaster/core";
import { tauriIPCClient } from "./ipc/tauriAdapter";
import App from "./App";
import "./index.css";

// Wire up the Tauri IPC adapter before anything else
setIPCClient(tauriIPCClient);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
