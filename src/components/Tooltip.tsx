import { useState, useRef, type ReactNode } from "react";
import { AnimatePresence, m } from "motion/react";

interface TooltipProps {
  content: string;
  children: ReactNode;
  side?: "top" | "bottom" | "left" | "right";
}

export function Tooltip({ content, children, side = "top" }: TooltipProps) {
  const [visible, setVisible] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  const placementStyles: Record<typeof side, object> = {
    top: { bottom: "calc(100% + 6px)", left: "50%", x: "-50%" },
    bottom: { top: "calc(100% + 6px)", left: "50%", x: "-50%" },
    left: { right: "calc(100% + 6px)", top: "50%", y: "-50%" },
    right: { left: "calc(100% + 6px)", top: "50%", y: "-50%" },
  };

  return (
    <div
      ref={ref}
      className="tooltip-root"
      onMouseEnter={() => setVisible(true)}
      onMouseLeave={() => setVisible(false)}
      onFocus={() => setVisible(true)}
      onBlur={() => setVisible(false)}
      style={{ display: "inline-flex", position: "relative" }}
    >
      {children}
      <AnimatePresence>
        {visible && content && (
          <m.div
            role="tooltip"
            className="tooltip-box"
            initial={{ opacity: 0, y: side === "top" ? 4 : -4, scale: 0.96 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: side === "top" ? 4 : -4, scale: 0.96 }}
            transition={{ duration: 0.12, ease: "easeOut" }}
            style={{ position: "absolute", ...placementStyles[side], zIndex: 9999 }}
          >
            {content}
          </m.div>
        )}
      </AnimatePresence>
    </div>
  );
}
