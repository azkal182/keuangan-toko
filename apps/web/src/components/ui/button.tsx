import { forwardRef, type ButtonHTMLAttributes } from "react";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "outline" | "ghost" | "destructive" | "plain";
  size?: "default" | "sm" | "lg" | "icon";
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  { className = "", variant = "primary", size = "default", type = "button", disabled, children, ...props },
  ref,
) {
  const baseClasses = "inline-flex items-center justify-center font-medium rounded-lg transition-all duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-emerald-700 focus-visible:ring-offset-1 disabled:opacity-50 disabled:pointer-events-none select-none cursor-pointer";

  const sizeClasses = {
    default: "h-9 px-4 py-2 text-sm gap-2",
    sm: "h-8 px-3 py-1.5 text-xs gap-1.5",
    lg: "h-11 px-6 py-2.5 text-base gap-2.5",
    icon: "h-9 w-9 p-0",
  }[size];

  const variantClasses = {
    primary: "bg-[#0f4c3a] text-white hover:bg-[#155e48] active:bg-[#0c3c2e] shadow-sm",
    secondary: "bg-slate-100 text-slate-800 hover:bg-slate-200 active:bg-slate-300",
    outline: "border border-slate-300 bg-white text-slate-700 hover:bg-slate-50 hover:border-slate-400 shadow-2xs",
    ghost: "text-slate-700 hover:bg-slate-100 hover:text-slate-900",
    plain: "text-emerald-800 hover:bg-emerald-50 hover:text-emerald-900 p-0 h-auto font-normal",
    destructive: "bg-rose-600 text-white hover:bg-rose-700 active:bg-rose-800 shadow-sm",
  }[variant];

  return (
    <button
      ref={ref}
      type={type}
      disabled={disabled}
      className={`${baseClasses} ${sizeClasses} ${variantClasses} ${className}`}
      {...props}
    >
      {children}
    </button>
  );
});
