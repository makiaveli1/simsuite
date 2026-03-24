import type { Screen, UiTheme } from "./types";

export const easeStandard: [number, number, number, number] = [0.22, 1, 0.36, 1];

export const panelSpring = {
  type: "spring" as const,
  stiffness: 280,
  damping: 32,
  mass: 0.88,
};

export const overlayTransition = {
  duration: 0.24,
  ease: easeStandard,
};

export const downloadsSelectionTransition = {
  duration: 0.2,
  ease: easeStandard,
};

export const downloadsSheetTransition = {
  duration: 0.22,
  ease: easeStandard,
};

export const downloadsDialogSpring = {
  type: "spring" as const,
  stiffness: 300,
  damping: 32,
  mass: 0.84,
};

const THEME_MOTION = {
  plumbob: {
    screenOffset: 12,
    screenExit: -10,
    screenDuration: 0.24,
    orbADrift: 14,
    orbBDrift: 12,
    orbVertical: 12,
    pulseDuration: 8.4,
    pulseScale: 1.055,
    bandDuration: 20,
    bandRotate: [-3, -2, -4, -3] as number[],
  },
  buildbuy: {
    screenOffset: 10,
    screenExit: -9,
    screenDuration: 0.22,
    orbADrift: 10,
    orbBDrift: 9,
    orbVertical: 9,
    pulseDuration: 9,
    pulseScale: 1.04,
    bandDuration: 18,
    bandRotate: [-4, -3, -5, -4] as number[],
  },
  cas: {
    screenOffset: 13,
    screenExit: -10,
    screenDuration: 0.25,
    orbADrift: 16,
    orbBDrift: 14,
    orbVertical: 13,
    pulseDuration: 8,
    pulseScale: 1.065,
    bandDuration: 21,
    bandRotate: [-2.4, -1.5, -3.2, -2.4] as number[],
  },
  neighborhood: {
    screenOffset: 14,
    screenExit: -11,
    screenDuration: 0.27,
    orbADrift: 18,
    orbBDrift: 16,
    orbVertical: 15,
    pulseDuration: 9.5,
    pulseScale: 1.07,
    bandDuration: 23,
    bandRotate: [-3.2, -2.3, -4.1, -3.2] as number[],
  },
  debuggrid: {
    screenOffset: 9,
    screenExit: -8,
    screenDuration: 0.2,
    orbADrift: 8,
    orbBDrift: 8,
    orbVertical: 7,
    pulseDuration: 6.6,
    pulseScale: 1.03,
    bandDuration: 16,
    bandRotate: [-2.2, -1.8, -2.8, -2.2] as number[],
  },
  sunroom: {
    screenOffset: 11,
    screenExit: -9,
    screenDuration: 0.25,
    orbADrift: 12,
    orbBDrift: 10,
    orbVertical: 9,
    pulseDuration: 10,
    pulseScale: 1.05,
    bandDuration: 24,
    bandRotate: [-2, -1.4, -2.8, -2] as number[],
  },
  patchday: {
    screenOffset: 8,
    screenExit: -8,
    screenDuration: 0.19,
    orbADrift: 8,
    orbBDrift: 9,
    orbVertical: 8,
    pulseDuration: 5.8,
    pulseScale: 1.08,
    bandDuration: 14,
    bandRotate: [-4.8, -3.6, -5.6, -4.8] as number[],
  },
  nightmarket: {
    screenOffset: 13,
    screenExit: -10,
    screenDuration: 0.26,
    orbADrift: 15,
    orbBDrift: 14,
    orbVertical: 12,
    pulseDuration: 8.8,
    pulseScale: 1.07,
    bandDuration: 22,
    bandRotate: [-3.4, -2.6, -4.2, -3.4] as number[],
  },
} satisfies Record<
  UiTheme,
  {
    screenOffset: number;
    screenExit: number;
    screenDuration: number;
    orbADrift: number;
    orbBDrift: number;
    orbVertical: number;
    pulseDuration: number;
    pulseScale: number;
    bandDuration: number;
    bandRotate: number[];
  }
>;

const SCREEN_TEMPO = {
  home: 1.08,
  downloads: 0.96,
  library: 0.94,
  updates: 0.94,
  creatorAudit: 0.98,
  categoryAudit: 0.98,
  organize: 0.9,
  review: 0.92,
  duplicates: 0.94,
  settings: 1,
  staging: 0.94,
} satisfies Record<Screen, number>;

export const screenTransition = {
  duration: 0.26,
  ease: easeStandard,
};

export const hoverLift = {
  y: -1.5,
  transition: {
    duration: 0.18,
    ease: easeStandard,
  },
};

export const tapPress = {
  scale: 0.992,
  transition: {
    duration: 0.14,
    ease: easeStandard,
  },
};

export const rowHover = {
  x: 2,
  transition: {
    duration: 0.18,
    ease: easeStandard,
  },
};

export const rowPress = {
  scale: 0.995,
  transition: {
    duration: 0.12,
  },
};

export function stagedListItem(index: number) {
  return {
    initial: { opacity: 0, y: 6 },
    animate: { opacity: 1, y: 0 },
    exit: { opacity: 0, y: -4 },
    transition: {
      duration: 0.22,
      ease: easeStandard,
      delay: Math.min(index * 0.022, 0.16),
    },
  };
}

export function getScreenFrameMotion(theme: UiTheme, screen: Screen) {
  const themeProfile = THEME_MOTION[theme];
  const tempo = SCREEN_TEMPO[screen];
  const offset = Math.round(themeProfile.screenOffset * tempo);
  const exit = Math.round(themeProfile.screenExit * tempo);

  return {
    initial: { opacity: 0, y: offset, scale: 0.998 },
    animate: { opacity: 1, y: 0, scale: 1 },
    exit: { opacity: 0, y: exit, scale: 0.996 },
    transition: {
      duration: Number((themeProfile.screenDuration * tempo).toFixed(3)),
      ease: easeStandard,
    },
  };
}

export function getBackdropMotion(theme: UiTheme) {
  const profile = THEME_MOTION[theme];

  return {
    orbAAnimate: {
      x: [-Math.round(profile.orbADrift * 0.7), profile.orbADrift, -Math.round(profile.orbADrift * 0.55), -Math.round(profile.orbADrift * 0.7)],
      y: [0, profile.orbVertical, -Math.round(profile.orbVertical * 0.5), 0],
      scale: [1, 1.04, 0.985, 1],
    },
    orbATransition: {
      duration: Number((profile.bandDuration - 2).toFixed(2)),
      repeat: Number.POSITIVE_INFINITY,
      ease: "easeInOut" as const,
    },
    orbBAnimate: {
      x: [Math.round(profile.orbBDrift * 0.7), -profile.orbBDrift, Math.round(profile.orbBDrift * 0.5), Math.round(profile.orbBDrift * 0.7)],
      y: [0, -Math.round(profile.orbVertical * 1.1), Math.round(profile.orbVertical * 0.66), 0],
      scale: [1, 0.975, 1.05, 1],
    },
    orbBTransition: {
      duration: Number((profile.bandDuration + 2).toFixed(2)),
      repeat: Number.POSITIVE_INFINITY,
      ease: "easeInOut" as const,
    },
    bandAnimate: {
      x: ["-2%", "3%", "-1%", "-2%"],
      rotate: profile.bandRotate,
    },
    bandTransition: {
      duration: profile.bandDuration,
      repeat: Number.POSITIVE_INFINITY,
      ease: "easeInOut" as const,
    },
    pulseAnimate: {
      opacity: [0.22, 0.42, 0.22],
      scale: [1, profile.pulseScale, 1],
    },
    pulseTransition: {
      duration: profile.pulseDuration,
      repeat: Number.POSITIVE_INFINITY,
      ease: "easeInOut" as const,
    },
    traceAnimate: {
      x: ["-1%", "2.5%", "-0.5%", "-1%"],
      opacity: [0.24, 0.54, 0.28, 0.24],
    },
    traceTransition: {
      duration: Number((profile.bandDuration * 0.72).toFixed(2)),
      repeat: Number.POSITIVE_INFINITY,
      ease: "easeInOut" as const,
    },
  };
}
