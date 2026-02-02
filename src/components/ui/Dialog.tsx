import { useEffect, useRef, type ReactNode } from "react";
import { X } from "lucide-react";
import clsx from "clsx";

interface DialogProps {
  open: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  className?: string;
}

export function Dialog({ open, onClose, title, children, className }: DialogProps) {
  const dialogRef = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (open && !dialog.open) {
      dialog.showModal();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  }, [open]);

  return (
    <dialog
      ref={dialogRef}
      onClose={onClose}
      onClick={(e) => { if (e.target === dialogRef.current) onClose(); }}
      className={clsx(
        "backdrop:bg-black/50 backdrop:backdrop-blur-sm bg-card-light dark:bg-card-dark rounded-xl border border-border-light dark:border-border-dark shadow-xl p-0 max-w-lg w-full",
        className,
      )}
    >
      <div className="flex items-center justify-between p-4 border-b border-border-light dark:border-border-dark">
        <h2 className="text-base font-semibold text-text-primary-light dark:text-text-primary-dark">{title}</h2>
        <button onClick={onClose} aria-label="Close dialog" className="p-1 rounded-md hover:bg-warm-100/50 dark:hover:bg-warm-900/20 text-text-secondary-light dark:text-text-secondary-dark focus:outline-none focus-visible:ring-2 focus-visible:ring-warm-300/50">
          <X size={16} />
        </button>
      </div>
      <div className="p-4">{children}</div>
    </dialog>
  );
}
