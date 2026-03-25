/**
 * guidedFlowStorage.ts
 *
 * Centralized localStorage utilities for SimSuite Casual guided flows.
 * All keys are namespaced under "simsuite:casual:" to avoid collisions.
 *
 * Design rules:
 * - Tour and lane explanations: permanently dismissed once dismissed
 * - Nudge: dismissed per-session (clears when user visits that lane)
 */

const PREFIX = "simsuite:casual:";

// ---------------------------------------------------------------------------
// Tour dismissal
// ---------------------------------------------------------------------------

/** True if the user has already completed/dismissed the downloads tour. */
export function isDownloadsTourDismissed(): boolean {
  return localStorage.getItem(`${PREFIX}downloads-tour-dismissed`) === "true";
}

/** Permanently dismiss the downloads tour. */
export function setDownloadsTourDismissed(): void {
  localStorage.setItem(`${PREFIX}downloads-tour-dismissed`, "true");
}

// ---------------------------------------------------------------------------
// Per-lane explanation banners
// ---------------------------------------------------------------------------

export type DownloadQueueLane =
  | "ready_now"
  | "waiting_on_you"
  | "special_setup"
  | "blocked"
  | "done"
  | "rejected";

/** True if the user has already seen the explanation for this lane. */
export function isLaneExplained(lane: DownloadQueueLane): boolean {
  return localStorage.getItem(`${PREFIX}lane-${lane}-explained`) === "true";
}

/** Permanently mark this lane as explained (banner won't show again). */
export function setLaneExplained(lane: DownloadQueueLane): void {
  localStorage.setItem(`${PREFIX}lane-${lane}-explained`, "true");
}

// ---------------------------------------------------------------------------
// Context-aware nudge
// ---------------------------------------------------------------------------

/** True if the nudge has been dismissed for the current session. */
export function isNudgeDismissed(): boolean {
  return localStorage.getItem(`${PREFIX}nudge-dismissed`) === "true";
}

// ---------------------------------------------------------------------------
// Keyboard shortcut hint — shown once, permanently dismissed after
// ---------------------------------------------------------------------------

const KEYBOARD_HINT_KEY = `${PREFIX}keyboard-hint-dismissed`;

export function isKeyboardHintDismissed(): boolean {
  return localStorage.getItem(KEYBOARD_HINT_KEY) === "true";
}

export function setKeyboardHintDismissed(): void {
  localStorage.setItem(KEYBOARD_HINT_KEY, "true");
}

/**
 * Dismiss the nudge for the current session.
 * The nudge will re-appear on next session if items are still waiting.
 */
export function setNudgeDismissed(): void {
  localStorage.setItem(`${PREFIX}nudge-dismissed`, "true");
}

/**
 * Clear the nudge dismissal so it can re-appear.
 * Called when the user switches lanes or navigates away.
 */
export function clearNudgeDismissed(): void {
  localStorage.removeItem(`${PREFIX}nudge-dismissed`);
}
