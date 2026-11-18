import React from "react";
import { ShieldAlert, ShieldCheck, AlertCircle } from "lucide-react";
import type { PitfallDimensionStatus } from "../../types/terminal";

const SHORT_LABELS: Record<PitfallDimensionStatus["dimension"], string> = {
  INSIDER_REDUCTION: "减持",
  NEGATIVE_NEWS: "舆情",
  HOLDER_SURGE: "股东数",
  MAIN_OUTFLOW: "主力流",
  TECH_BREAKDOWN: "破位",
};

export interface PitfallAuditBadgesProps {
  pitfalls: PitfallDimensionStatus[];
  compact?: boolean;
}

export const PitfallAuditBadges: React.FC<PitfallAuditBadgesProps> = ({
  pitfalls,
  compact = true,
}) => {
  return (
    <div
      style={{
        display: "flex",
        flexWrap: "wrap",
        gap: compact ? "5px" : "8px",
        minWidth: 0,
      }}
    >
      {pitfalls.map((item) => {
        const pillTone =
          item.severity === "VETO"
            ? "pill-risk-veto"
            : item.severity === "WARN"
              ? "pill-risk-stoploss"
              : "pill-risk-clear";
        const shortName = SHORT_LABELS[item.dimension] ?? item.labelZh;

        return (
          <span
            key={item.dimension}
            data-pitfall-severity={item.severity}
            title={`${item.labelZh} (${item.severity}): ${item.detailZh}`}
            className={`glass-pill ${pillTone}`}
            style={{
              paddingInline: compact ? "7px" : "10px",
              fontSize: compact ? "11.5px" : "12px",
            }}
          >
            {item.severity === "VETO" ? (
              <ShieldAlert size={12} aria-hidden="true" />
            ) : item.severity === "WARN" ? (
              <AlertCircle size={12} aria-hidden="true" />
            ) : (
              <ShieldCheck size={12} aria-hidden="true" />
            )}
            <span>{shortName}</span>
            {!compact && <span style={{ opacity: 0.9 }}>· {item.detailZh}</span>}
          </span>
        );
      })}
    </div>
  );
};
