// Original Novastrap mark: a four-point nova star inside an orbit ring.
export function Logo({ size = 28 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 64 64" aria-hidden="true">
      <defs>
        <linearGradient id="nv-g" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#8b7bff" />
          <stop offset="1" stopColor="#3dd6f5" />
        </linearGradient>
      </defs>
      <rect x="2" y="2" width="60" height="60" rx="16" fill="#12131a" />
      <ellipse cx="32" cy="32" rx="22" ry="9" fill="none" stroke="url(#nv-g)" strokeWidth="2.5"
        opacity="0.55" transform="rotate(-28 32 32)" />
      <path d="M32 10 C34 26 38 30 54 32 C38 34 34 38 32 54 C30 38 26 34 10 32 C26 30 30 26 32 10Z"
        fill="url(#nv-g)" />
    </svg>
  );
}
