import React from "react";

interface FriedChipLogoProps {
  size?: number;
  className?: string;
  variant?: "monochrome" | "accent";
}

export const FriedChipLogo: React.FC<FriedChipLogoProps> = ({
  size = 28,
  className = "",
  variant = "accent",
}) => {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 48 48"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      style={{ display: "inline-block", verticalAlign: "middle", flexShrink: 0 }}
      aria-label="FriedChip+ Logo"
    >
      <defs>
        <linearGradient id="fcDieGrad" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="var(--fc-die-start, #1f2430)" />
          <stop offset="100%" stopColor="var(--fc-die-end, #10131a)" />
        </linearGradient>
        <linearGradient id="fcPlusGrad" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="var(--fc-plus-start, #ffffff)" />
          <stop offset="100%" stopColor="var(--fc-plus-end, #cbd5e1)" />
        </linearGradient>
      </defs>

      {/* Outer Ceramic Carrier */}
      <rect
        x="4"
        y="4"
        width="40"
        height="40"
        rx="10"
        fill="url(#fcDieGrad)"
        stroke="var(--fc-carrier-stroke, #374151)"
        strokeWidth="1.5"
      />

      {/* Perimeter Pins (2 per side) */}
      {/* Top Pins */}
      <rect x="15" y="1.5" width="4" height="4" rx="1" fill="var(--fc-pin, #64748b)" />
      <rect x="29" y="1.5" width="4" height="4" rx="1" fill="var(--fc-pin, #64748b)" />
      {/* Bottom Pins */}
      <rect x="15" y="42.5" width="4" height="4" rx="1" fill="var(--fc-pin, #64748b)" />
      <rect x="29" y="42.5" width="4" height="4" rx="1" fill="var(--fc-pin, #64748b)" />
      {/* Left Pins */}
      <rect x="1.5" y="15" width="4" height="4" rx="1" fill="var(--fc-pin, #64748b)" />
      <rect x="1.5" y="29" width="4" height="4" rx="1" fill="var(--fc-pin, #64748b)" />
      {/* Right Pins */}
      <rect x="42.5" y="15" width="4" height="4" rx="1" fill="var(--fc-pin, #64748b)" />
      <rect x="42.5" y="29" width="4" height="4" rx="1" fill="var(--fc-pin, #64748b)" />

      {/* Silicon Die Inset */}
      <rect
        x="10"
        y="10"
        width="28"
        height="28"
        rx="6"
        fill="var(--fc-inner-die, #0a0d14)"
        stroke="var(--fc-inner-stroke, #232938)"
        strokeWidth="1"
      />

      {/* Pin 1 Index Dot */}
      <circle
        cx="14"
        cy="14"
        r="1.2"
        fill={variant === "accent" ? "var(--fc-accent-dot, #f59e0b)" : "var(--fc-pin, #94a3b8)"}
      />

      {/* Micro-Die Corner Tick Marks */}
      <line x1="12" y1="36" x2="14" y2="36" stroke="var(--fc-trace, #334155)" strokeWidth="0.8" />
      <line x1="34" y1="12" x2="36" y2="12" stroke="var(--fc-trace, #334155)" strokeWidth="0.8" />
      <line x1="34" y1="36" x2="36" y2="36" stroke="var(--fc-trace, #334155)" strokeWidth="0.8" />

      {/* Central Geometric Plus (+) Sign */}
      {/* Vertical bar */}
      <rect
        x="22"
        y="16"
        width="4"
        height="16"
        rx="1.5"
        fill="url(#fcPlusGrad)"
      />
      {/* Horizontal bar */}
      <rect
        x="16"
        y="22"
        width="16"
        height="4"
        rx="1.5"
        fill="url(#fcPlusGrad)"
      />

      {/* Center Silicon Micro-Core */}
      <rect
        x="22.5"
        y="22.5"
        width="3"
        height="3"
        rx="0.8"
        fill={variant === "accent" ? "var(--fc-accent-dot, #f59e0b)" : "#94a3b8"}
      />
    </svg>
  );
};
