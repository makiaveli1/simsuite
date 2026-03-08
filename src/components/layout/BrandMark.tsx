interface BrandMarkProps {
  className?: string;
}

export function BrandMark({ className }: BrandMarkProps) {
  return (
    <svg
      viewBox="0 0 64 64"
      className={`brand-glyph${className ? ` ${className}` : ""}`}
      aria-hidden="true"
    >
      <rect
        x="1.5"
        y="1.5"
        width="61"
        height="61"
        className="brand-glyph-shell"
      />
      <rect
        x="9"
        y="9"
        width="46"
        height="46"
        className="brand-glyph-frame"
      />
      <path
        d="M15 15H26V17H17V26H15V15Z"
        className="brand-glyph-corner"
      />
      <path
        d="M49 49H38V47H47V38H49V49Z"
        className="brand-glyph-corner brand-glyph-corner-soft"
      />
      <path
        d="M32 12L46 28L32 52L18 28Z"
        className="brand-glyph-plumbob"
      />
      <path
        d="M32 20L39 28L32 40L25 28Z"
        className="brand-glyph-core"
      />
      <path
        d="M19 31H45"
        className="brand-glyph-scan"
      />
      <circle
        cx="47.5"
        cy="15.5"
        r="2.5"
        className="brand-glyph-beacon"
      />
    </svg>
  );
}
