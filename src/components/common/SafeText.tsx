import React, { useState } from "react";
import { createPortal } from "react-dom";
import { clampPopoverPosition } from "../../domain/uiInvariants";

export interface SafeTextProps {
  text: string;
  maxLines?: 1 | 2 | 3 | 4;
  className?: string;
  showPopover?: boolean;
  as?: "span" | "div" | "p" | "strong";
}

export const SafeText: React.FC<SafeTextProps> = ({
  text,
  maxLines = 1,
  className = "",
  showPopover = false,
  as: Tag = "span",
}) => {
  const [popoverCoords, setPopoverCoords] = useState<{ x: number; y: number } | null>(null);

  const shouldOfferPopover = showPopover || text.length >= 22;

  const handleOpen = (clientX: number, clientY: number) => {
    if (!shouldOfferPopover || typeof window === "undefined") return;
    const clamped = clampPopoverPosition({
      anchorX: clientX + 10,
      anchorY: clientY + 14,
      popoverWidth: 300,
      popoverHeight: 96,
      paneWidth: window.innerWidth || 1280,
      paneHeight: window.innerHeight || 800,
      margin: 12,
    });
    setPopoverCoords(clamped);
  };

  const baseClass =
    maxLines === 1 ? "safe-text-single" : "safe-text-clamp";

  const clampStyle: React.CSSProperties | undefined =
    maxLines > 1 ? { WebkitLineClamp: maxLines } : undefined;

  return (
    <>
      <Tag
        data-safe-text=""
        title={text}
        className={`${baseClass} ${className}`.trim()}
        style={clampStyle}
        onMouseOver={(e: React.MouseEvent) => handleOpen(e.clientX, e.clientY)}
        onMouseOut={() => setPopoverCoords(null)}
        onFocus={(e: React.FocusEvent<HTMLElement>) => {
          const rect = e.currentTarget.getBoundingClientRect();
          handleOpen(rect.left, rect.bottom);
        }}
        onBlur={() => setPopoverCoords(null)}
      >
        {text}
      </Tag>
      {popoverCoords &&
        typeof document !== "undefined" &&
        createPortal(
          <div
            data-safe-text-popover=""
            role="tooltip"
            className="glass-tier-3 lh-prose"
            style={{
              position: "fixed",
              left: `${popoverCoords.x}px`,
              top: `${popoverCoords.y}px`,
              maxWidth: "300px",
              padding: "10px 14px",
              fontSize: "12.5px",
              zIndex: 90,
              pointerEvents: "none",
              wordBreak: "break-word",
            }}
          >
            {text}
          </div>,
          document.body
        )}
    </>
  );
};
