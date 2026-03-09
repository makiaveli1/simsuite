import type { UiTheme } from "./types";

export interface ThemeDefinition {
  id: UiTheme;
  label: string;
  hint: string;
  mood: string;
  signature: string;
  swatch: [string, string, string];
}

export const UI_THEMES: ThemeDefinition[] = [
  {
    id: "plumbob",
    label: "Plumbob",
    hint: "Classic SimSuite green with calm dark chrome.",
    mood: "Steady everyday sorting",
    signature: "Calm glides with soft console signals",
    swatch: ["#78f0a1", "#0d1a1f", "#102028"],
  },
  {
    id: "buildbuy",
    label: "Build/Buy",
    hint: "Warm catalog brass with workshop-style contrast.",
    mood: "Floor-plan and object catalog work",
    signature: "Heavier movements and warm catalog glow",
    swatch: ["#f0c879", "#1d1512", "#241b17"],
  },
  {
    id: "cas",
    label: "CAS",
    hint: "Cool studio blue for hair, skin, and outfit work.",
    mood: "Creator studio and CAS cleanup",
    signature: "Cool studio drifts with cleaner hover light",
    swatch: ["#84cfff", "#10202d", "#162534"],
  },
  {
    id: "neighborhood",
    label: "Neighborhood",
    hint: "Dusk map colors with coral signals and teal glass.",
    mood: "Soft survey mode for mixed libraries",
    signature: "Longer fades and map-room ambience",
    swatch: ["#f09a72", "#142027", "#1a2428"],
  },
  {
    id: "debuggrid",
    label: "Debug Grid",
    hint: "Industrial slate and safety orange for power sorting.",
    mood: "High-contrast technical passes",
    signature: "Sharper responses with gridline punch",
    swatch: ["#f4873b", "#181c20", "#202428"],
  },
  {
    id: "sunroom",
    label: "Sunroom",
    hint: "Bright parchment panels with teal controls and softer glare.",
    mood: "Daylight browsing without harsh contrast",
    signature: "Slow daylight motion with light paper surfaces",
    swatch: ["#0f9e8d", "#f5efe1", "#e4d9c7"],
  },
  {
    id: "patchday",
    label: "Patch Day",
    hint: "Alert amber, backup red, and dark command-center chrome.",
    mood: "Recovery sweeps and emergency checks",
    signature: "Brisk alerts with stronger recovery pulses",
    swatch: ["#ffb44c", "#21130f", "#3a1b15"],
  },
  {
    id: "nightmarket",
    label: "Night Market",
    hint: "Ink-dark shell with jade, ember, and signage glow.",
    mood: "Late-night curation with richer neon accents",
    signature: "Glowing signage, deeper shadows, slower reveal",
    swatch: ["#49ddb8", "#130f17", "#281924"],
  },
];

export const UI_THEME_IDS = UI_THEMES.map((theme) => theme.id);

export function getThemeDefinition(theme: UiTheme) {
  return UI_THEMES.find((item) => item.id === theme) ?? UI_THEMES[0];
}
