import { forwardRef, type InputHTMLAttributes } from "react";

export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  error?: boolean;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(function Input(
  { className = "", error = false, ...props },
  ref,
) {
  return (
    <input
      ref={ref}
      className={`flex h-9 w-full rounded-lg border bg-white px-3 py-1.5 text-sm text-slate-900 shadow-2xs transition-colors placeholder:text-slate-400 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-emerald-700 focus-visible:border-emerald-700 disabled:cursor-not-allowed disabled:opacity-50 ${
        error || props["aria-invalid"] === "true" || props["aria-invalid"] === true
          ? "border-rose-500 focus-visible:ring-rose-500 focus-visible:border-rose-500"
          : "border-slate-300"
      } ${className}`}
      {...props}
    />
  );
});
