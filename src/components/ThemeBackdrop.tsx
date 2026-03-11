import { m, useReducedMotion } from "motion/react";
import { getBackdropMotion } from "../lib/motion";
import type { Screen, UiTheme } from "../lib/types";
import { backdropScreenLabel } from "../lib/uiLanguage";

interface ThemeBackdropProps {
  theme: UiTheme;
  screen: Screen;
}

export function ThemeBackdrop({ theme, screen }: ThemeBackdropProps) {
  const reducedMotion = useReducedMotion();
  const label = backdropScreenLabel(screen);
  const motionProfile = getBackdropMotion(theme);

  return (
    <div
      className={`theme-backdrop theme-backdrop-${theme}`}
      data-screen={screen}
      aria-hidden="true"
    >
      <m.div
        className="theme-backdrop-orb theme-backdrop-orb-a"
        animate={
          reducedMotion ? undefined : motionProfile.orbAAnimate
        }
        transition={motionProfile.orbATransition}
      />
      <m.div
        className="theme-backdrop-orb theme-backdrop-orb-b"
        animate={
          reducedMotion ? undefined : motionProfile.orbBAnimate
        }
        transition={motionProfile.orbBTransition}
      />
      <m.div
        className="theme-backdrop-band"
        animate={
          reducedMotion ? undefined : motionProfile.bandAnimate
        }
        transition={motionProfile.bandTransition}
      />
      <m.div
        className="theme-backdrop-pulse"
        animate={
          reducedMotion ? undefined : motionProfile.pulseAnimate
        }
        transition={motionProfile.pulseTransition}
      />
      <m.div
        className="theme-backdrop-trace"
        animate={reducedMotion ? undefined : motionProfile.traceAnimate}
        transition={motionProfile.traceTransition}
      />
      <div className="theme-backdrop-grid" />
      <div className="theme-backdrop-screen-tag">{label}</div>
    </div>
  );
}
