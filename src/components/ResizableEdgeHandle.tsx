import type { PointerEvent as ReactPointerEvent } from "react";

interface ResizableEdgeHandleProps {
  label: string;
  value: number;
  min: number;
  max: number;
  onChange: (value: number) => void;
  side?: "left" | "right" | "top" | "bottom";
  className?: string;
}

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, value));
}

export function ResizableEdgeHandle({
  label,
  value,
  min,
  max,
  onChange,
  side = "left",
  className,
}: ResizableEdgeHandleProps) {
  function handlePointerDown(event: ReactPointerEvent<HTMLButtonElement>) {
    event.preventDefault();
    const handle = event.currentTarget;
    handle.setPointerCapture?.(event.pointerId);

    const startX = event.clientX;
    const startY = event.clientY;
    const startValue = value;

    function onPointerMove(moveEvent: PointerEvent) {
      const delta = (() => {
        if (side === "left") {
          return startX - moveEvent.clientX;
        }

        if (side === "right") {
          return moveEvent.clientX - startX;
        }

        if (side === "top") {
          return startY - moveEvent.clientY;
        }

        return moveEvent.clientY - startY;
      })();
      onChange(clamp(startValue + delta, min, max));
    }

    function onPointerUp() {
      if (handle.hasPointerCapture?.(event.pointerId)) {
        handle.releasePointerCapture(event.pointerId);
      }
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", onPointerUp);
    }

    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp);
  }

  return (
    <button
      type="button"
      className={`resize-handle resize-handle-${side}${className ? ` ${className}` : ""}`}
      aria-label={label}
      title={label}
      onPointerDown={handlePointerDown}
    >
      <span className="resize-handle-grip" aria-hidden="true">
        <span />
        <span />
        <span />
      </span>
    </button>
  );
}
