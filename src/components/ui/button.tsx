import React from "react";
import { cn } from "../../lib/utils";

type Variant = "default" | "secondary" | "outline" | "ghost" | "destructive" | "success";
type Size = "sm" | "md" | "lg" | "icon";

const variants: Record<Variant, string> = {
  default:
    "bg-primary text-primary-foreground hover:bg-primary/90 shadow-sm disabled:bg-primary/50",
  secondary:
    "bg-secondary text-secondary-foreground hover:bg-secondary/80 disabled:opacity-50",
  outline:
    "border border-border bg-transparent hover:bg-accent hover:text-accent-foreground disabled:opacity-50",
  ghost: "bg-transparent hover:bg-accent hover:text-accent-foreground disabled:opacity-50",
  destructive:
    "bg-destructive text-destructive-foreground hover:bg-destructive/90 shadow-sm disabled:opacity-50",
  success: "bg-success text-primary-foreground hover:bg-success/90 shadow-sm disabled:opacity-50",
};

const sizes: Record<Size, string> = {
  sm: "h-8 px-3 text-xs gap-1.5",
  md: "h-9 px-4 text-sm gap-2",
  lg: "h-10 px-5 text-sm gap-2",
  icon: "h-8 w-8 p-0",
};

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant = "default", size = "md", type = "button", ...props }, ref) => (
    <button
      ref={ref}
      type={type}
      className={cn(
        "inline-flex items-center justify-center rounded-md font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:ring-offset-background disabled:cursor-not-allowed",
        variants[variant],
        sizes[size],
        className,
      )}
      {...props}
    />
  ),
);
Button.displayName = "Button";
