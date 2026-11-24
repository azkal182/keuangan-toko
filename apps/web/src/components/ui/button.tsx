import type { ComponentProps } from "react";

// Local Tailwind button in the editable shadcn/ui component style.
export function Button({
  className = "", variant = "primary", type = "button", ...props
}: ComponentProps<"button"> & { variant?: "primary" | "outline" | "plain" }) {
  return (
    <button
      type={type}
      className={`button button-${variant} ${className}`}
      {...props}
    />
  );
}
