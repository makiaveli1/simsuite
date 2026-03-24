import { m } from "motion/react";
import { overlayTransition } from "../lib/motion";

interface KeyboardShortcut {
  key: string;
  description: string;
}

const SHORTCUTS: KeyboardShortcut[] = [
  { key: "J / ↓", description: "Select next item" },
  { key: "K / ↑", description: "Select previous item" },
  { key: "Enter", description: "Toggle inspector panel" },
  { key: "A", description: "Apply selected item" },
  { key: "I", description: "Ignore selected item" },
  { key: "R", description: "Refresh downloads" },
  { key: "?", description: "Show this help" },
  { key: "Esc", description: "Close / deselect" },
];

interface KeyboardShortcutsDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

export function KeyboardShortcutsDialog({
  isOpen,
  onClose,
}: KeyboardShortcutsDialogProps) {
  if (!isOpen) return null;

  return (
    <div
      className="shortcuts-backdrop"
      role="dialog"
      aria-modal="true"
      aria-label="Keyboard shortcuts"
      onClick={onClose}
    >
      <m.div
        className="shortcuts-card"
        initial={{ opacity: 0, scale: 0.95 }}
        animate={{ opacity: 1, scale: 1 }}
        exit={{ opacity: 0, scale: 0.95 }}
        transition={overlayTransition}
        onClick={(e) => e.stopPropagation()}
      >
        <h2 className="shortcuts-title">Keyboard Shortcuts</h2>
        <div className="shortcuts-list">
          {SHORTCUTS.map((shortcut) => (
            <div key={shortcut.key} className="shortcuts-row">
              <kbd className="shortcuts-key">{shortcut.key}</kbd>
              <span className="shortcuts-desc">{shortcut.description}</span>
            </div>
          ))}
        </div>
        <button
          type="button"
          className="shortcuts-close"
          onClick={onClose}
          aria-label="Close shortcuts help"
        >
          Press <kbd>Esc</kbd> to close
        </button>
      </m.div>
    </div>
  );
}
