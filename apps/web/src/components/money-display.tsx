import { formatRupiah } from "../money";

export function MoneyDisplay({
  amount,
  variant = "neutral",
  size = "md",
  className = "",
}: {
  amount: string | bigint | number | undefined | null;
  variant?: "neutral" | "debit" | "credit" | "danger" | "success";
  size?: "xs" | "sm" | "md" | "lg" | "xl";
  className?: string;
}) {
  if (amount === undefined || amount === null || amount === "") {
    return <span className={`text-slate-400 ${className}`}>—</span>;
  }

  const strAmount = String(amount);
  const formatted = formatRupiah(strAmount);

  const sizeClasses = {
    xs: "text-xs",
    sm: "text-sm",
    md: "text-sm font-medium",
    lg: "text-lg font-semibold tracking-tight",
    xl: "text-2xl font-bold tracking-tight",
  }[size];

  const colorClasses = {
    neutral: "text-slate-900",
    debit: "text-amber-800",
    credit: "text-emerald-700",
    danger: "text-rose-700 font-semibold",
    success: "text-emerald-700 font-semibold",
  }[variant];

  return (
    <span className={`font-mono tabular-nums ${sizeClasses} ${colorClasses} ${className}`}>
      {formatted}
    </span>
  );
}
