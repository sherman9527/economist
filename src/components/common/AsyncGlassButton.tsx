import React, { useEffect, useRef, useState } from "react";
import { Check, Loader2, AlertTriangle } from "lucide-react";
import type { AsyncActionState } from "../../types/ui";

export interface AsyncGlassButtonProps {
  label: string;
  loadingLabel?: string;
  successLabel?: string;
  icon?: React.ReactNode;
  variant?: "primary" | "ghost";
  size?: "md" | "sm";
  disabled?: boolean;
  title?: string;
  className?: string;
  dataTestId?: string;
  onClick: () => Promise<unknown> | void;
}

export const AsyncGlassButton: React.FC<AsyncGlassButtonProps> = ({
  label,
  loadingLabel,
  successLabel,
  icon,
  variant = "primary",
  size = "md",
  disabled = false,
  title,
  className = "",
  dataTestId,
  onClick,
}) => {
  const [state, setState] = useState<AsyncActionState>("idle");
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, []);

  const handlePress = async () => {
    if (disabled || state === "loading") return;
    if (timerRef.current) clearTimeout(timerRef.current);
    setState("loading");
    try {
      await onClick();
      setState("success");
      timerRef.current = setTimeout(() => setState("idle"), 1600);
    } catch {
      setState("error");
      timerRef.current = setTimeout(() => setState("idle"), 2200);
    }
  };

  const variantCls = variant === "ghost" ? "glass-btn glass-btn-ghost" : "glass-btn";
  const sizeCls = size === "sm" ? "glass-btn-sm" : "";

  const displayText =
    state === "loading"
      ? loadingLabel ?? label
      : state === "success"
        ? successLabel ?? label
        : state === "error"
          ? "操作受阻"
          : label;

  return (
    <button
      type="button"
      data-testid={dataTestId}
      data-async-state={state}
      disabled={disabled || state === "loading"}
      title={title}
      onClick={() => void handlePress()}
      className={`${variantCls} ${sizeCls} ${className}`.trim()}
    >
      {state === "loading" ? (
        <Loader2 size={15} className="icon-spin" aria-hidden="true" />
      ) : state === "success" ? (
        <Check size={15} aria-hidden="true" />
      ) : state === "error" ? (
        <AlertTriangle size={15} aria-hidden="true" />
      ) : (
        icon
      )}
      <span className="min-w-zero">{displayText}</span>
    </button>
  );
};
