import { useEffect, useRef, type ReactNode } from "react";

export function Dialog({ open, onOpenChange, title, children }: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  children: ReactNode;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const dialog = ref.current;
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    if (!open && dialog.open) dialog.close();
  }, [open]);
  return <dialog ref={ref} className="review-dialog" aria-label={title} onClose={() => onOpenChange(false)}>
    <div className="review-dialog-heading"><h2>{title}</h2><button className="dialog-close" type="button" aria-label="Tutup" onClick={() => onOpenChange(false)}>×</button></div>
    <div className="review-dialog-body">{children}</div>
  </dialog>;
}
