import type { ReactNode } from "react";

interface SkeletonLoaderProps {
  rows?: number;
  height?: number;
  className?: string;
  children?: ReactNode;
}

/**
 * Skeleton loader with shimmer animation.
 * Renders rows of grey placeholder blocks while data is loading.
 * Respects reduced-motion via CSS -- the shimmer is disabled when
 * prefers-reduced-motion: reduce is set.
 */
export function SkeletonLoader({
  rows = 5,
  height = 48,
  className,
  children,
}: SkeletonLoaderProps) {
  return (
    <div className={`skeleton-loader${className ? ` ${className}` : ""}`}>
      {children ??
        Array.from({ length: rows }).map((_, i) => (
          <div
            key={i}
            className="skeleton-row"
            style={{ height }}
          />
        ))}
    </div>
  );
}
