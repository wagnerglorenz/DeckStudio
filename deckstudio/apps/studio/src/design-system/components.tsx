import React from "react";

export function DeckLogo({ size = 26 }: { size?: number }) {
  const uid = React.useId().replace(/:/g, "");
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 512 512"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      style={{ flexShrink: 0, display: "block" }}
      aria-label="DeckStudio Logo"
    >
      <defs>
        <linearGradient id={`bg_${uid}`} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%" stopColor="#0D1117" />
          <stop offset="55%" stopColor="#111C2E" />
          <stop offset="100%" stopColor="#162032" />
        </linearGradient>
        <linearGradient id={`bd_${uid}`} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%" stopColor="#22D3EE" />
          <stop offset="55%" stopColor="#38BDF8" />
          <stop offset="100%" stopColor="#22C55E" />
        </linearGradient>
        <linearGradient id={`st_${uid}`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#22C55E" />
          <stop offset="100%" stopColor="#22D3EE" />
        </linearGradient>
      </defs>
      <rect x="24" y="24" width="464" height="464" rx="108" fill={`url(#bg_${uid})`} stroke={`url(#bd_${uid})`} strokeWidth="20" />
      <rect x="104" y="108" width="84" height="84" rx="20" fill="#12233B" stroke="#22D3EE" strokeWidth="8" />
      <circle cx="146" cy="150" r="18" fill="none" stroke="#22D3EE" strokeWidth="7" strokeDasharray="78 36" strokeLinecap="round" />
      <rect x="206" y="108" width="84" height="84" rx="20" fill="#12233B" stroke="#38BDF8" strokeWidth="8" />
      <path d="M226 164 L240 144 L252 154 L270 130" stroke="#22C55E" strokeWidth="7" strokeLinecap="round" strokeLinejoin="round" />
      <rect x="104" y="210" width="84" height="84" rx="20" fill="#12233B" stroke="#38BDF8" strokeWidth="8" />
      <rect x="126" y="244" width="40" height="16" rx="8" fill="#22D3EE" />
      <rect x="206" y="210" width="84" height="84" rx="20" fill="#13233A" stroke="#22D3EE" strokeWidth="8" />
      <circle cx="248" cy="252" r="15" fill="#22C55E" />
      <rect x="104" y="312" width="84" height="84" rx="20" fill="#12233B" stroke="#22D3EE" strokeWidth="8" />
      <path d="M126 364 L166 364" stroke="#38BDF8" strokeWidth="8" strokeLinecap="round" />
      <path d="M126 346 L152 346" stroke="#22C55E" strokeWidth="8" strokeLinecap="round" />
      <rect x="206" y="312" width="84" height="84" rx="20" fill="#12233B" stroke="#38BDF8" strokeWidth="8" />
      <path d="M238 338 L262 354 L238 370 Z" fill="#22D3EE" />
      <path
        d="M 314 108 H 346 C 386 108 408 134 408 176 V 328 C 408 370 386 396 346 396 H 314 A 18 18 0 0 1 296 378 V 126 A 18 18 0 0 1 314 108 Z"
        fill="#102136"
        stroke={`url(#st_${uid})`}
        strokeWidth="9"
      />
      <rect x="322" y="138" width="58" height="22" rx="6" fill="#22D3EE" />
      <circle cx="351" cy="252" r="22" fill="none" stroke="#22C55E" strokeWidth="8" />
      <circle cx="351" cy="252" r="8" fill="#22C55E" />
      <rect x="322" y="336" width="58" height="24" rx="7" fill="#38BDF8" />
    </svg>
  );
}

export type ButtonVariant = "default" | "primary" | "success" | "danger";

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
}

export function Button({ variant = "default", className = "", children, ...rest }: ButtonProps) {
  const vCls = variant === "default" ? "btn" : `btn ${variant}`;
  return (
    <button className={`${vCls} ${className}`.trim()} {...rest}>
      {children}
    </button>
  );
}

export function StatusPill({
  dot,
  accent,
  children,
}: {
  dot?: "green" | "cyan" | "red" | "gray";
  accent?: string;
  children: React.ReactNode;
}) {
  return (
    <span className="pill" style={accent ? { borderColor: accent, color: accent } : undefined}>
      {dot && <span className={`dot ${dot}`} />}
      {children}
    </span>
  );
}

export function Card({
  title,
  action,
  children,
  style,
}: {
  title?: React.ReactNode;
  action?: React.ReactNode;
  children: React.ReactNode;
  style?: React.CSSProperties;
}) {
  return (
    <div className="card" style={style}>
      {(title || action) && (
        <h2>
          <span>{title}</span>
          {action}
        </h2>
      )}
      {children}
    </div>
  );
}

export function KpiCard({
  label,
  value,
  hint,
  accent,
}: {
  label: string;
  value: React.ReactNode;
  hint?: string;
  accent?: string;
}) {
  return (
    <div className="kpi">
      <div className="kpi-lbl">{label}</div>
      <div className="kpi-val" style={accent ? { color: accent } : undefined}>
        {value}
      </div>
      {hint && <div className="kpi-hint">{hint}</div>}
    </div>
  );
}

export function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="field">
      <label>{label}</label>
      {children}
    </div>
  );
}
