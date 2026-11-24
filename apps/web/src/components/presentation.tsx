import { AlertCircle, RotateCcw, Store } from "lucide-react";
import { Button } from "./ui/button";

export function Brand({ compact = false }: { compact?: boolean }) {
  return (
    <div className="flex items-center gap-2.5">
      <div className="flex items-center justify-center h-9 w-9 rounded-xl bg-gradient-to-br from-[#0f4c3a] to-[#0a3528] text-white shadow-sm ring-1 ring-white/20">
        <Store className="h-5 w-5" />
      </div>
      {!compact && (
        <div className="flex flex-col">
          <span className="font-bold text-sm tracking-tight text-slate-900 leading-none">Keuangan Toko</span>
          <span className="text-[11px] text-slate-500 font-medium">Buku Piutang & Kas</span>
        </div>
      )}
    </div>
  );
}

export function ErrorNotice({ message, retry }: { message: string; retry?: () => void }) {
  return (
    <div
      role="alert"
      className="flex items-start justify-between gap-3 p-4 rounded-xl border border-rose-200 bg-rose-50/80 text-rose-900 text-sm shadow-2xs"
    >
      <div className="flex items-start gap-2.5 min-w-0">
        <AlertCircle className="h-5 w-5 text-rose-600 shrink-0 mt-0.5" />
        <p className="text-sm font-medium text-rose-900 leading-snug">{message}</p>
      </div>
      {retry && (
        <Button variant="outline" size="sm" onClick={retry} className="shrink-0 gap-1.5 border-rose-300 bg-white text-rose-900 hover:bg-rose-100">
          <RotateCcw className="h-3.5 w-3.5" />
          Coba lagi
        </Button>
      )}
    </div>
  );
}

export function EmptyState({
  title,
  description,
  action,
}: {
  title: string;
  description?: string;
  action?: React.ReactNode;
}) {
  return (
    <div className="flex flex-col items-center justify-center p-10 text-center rounded-xl border border-dashed border-slate-200 bg-slate-50/50">
      <p className="text-sm font-semibold text-slate-700">{title}</p>
      {description && <p className="text-xs text-slate-500 mt-1 max-w-sm">{description}</p>}
      {action && <div className="mt-4">{action}</div>}
    </div>
  );
}
