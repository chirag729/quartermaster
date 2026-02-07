import type { ReactNode } from "react";
import { Sidebar } from "./Sidebar";
import { TopBar } from "./TopBar";
import { PolkitBanner } from "./PolkitBanner";
import { ToastContainer } from "../ui/Toast";

interface Props {
  children: ReactNode;
}

export function AppShell({ children }: Props) {
  return (
    <div className="flex h-screen overflow-hidden bg-surface-light dark:bg-surface-dark">
      <Sidebar />
      <div className="flex-1 flex flex-col overflow-hidden">
        <TopBar />
        <PolkitBanner />
        <main className="flex-1 overflow-auto p-6">
          {children}
        </main>
      </div>
      <ToastContainer />
    </div>
  );
}
