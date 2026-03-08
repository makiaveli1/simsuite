import { useEffect } from "react";
import { ResizableEdgeHandle } from "./ResizableEdgeHandle";
import { useUiPreferences } from "./UiPreferencesContext";
import type { Screen, UserView } from "../lib/types";

interface FieldGuideProps {
  open: boolean;
  screen: Screen;
  userView: UserView;
  onClose: () => void;
}

const VIEW_LABELS: Record<UserView, string> = {
  beginner: "Beginner view",
  standard: "Standard view",
  power: "Power view",
};

const GUIDE: Record<
  Screen,
  {
    title: string;
    intro: string;
    sections: Array<{ label: string; entries: string[] }>;
  }
> = {
  home: {
    title: "Home Guide",
    intro:
      "Pick your Mods and Tray folders, run a scan, then jump straight to anything that needs attention.",
    sections: [
      {
        label: "Start here",
        entries: [
          "Choose your Mods and Tray folders.",
          "Run a scan after adding or removing CC.",
          "Open Needs Attention if a lot of files get flagged.",
        ],
      },
      {
        label: "Safety",
        entries: [
          "Files only move from Tidy Up.",
          "A restore point is made before any batch move.",
          "Lots, rooms, and households never get mixed into Mods.",
        ],
      },
    ],
  },
  library: {
    title: "Library Guide",
    intro:
      "Use the filters to find a file fast, then check the right-hand panel before you change anything.",
    sections: [
      {
        label: "Focus",
        entries: [
          "Search by creator, type, or filename.",
          "Use the inspector to see warnings, tags, and file hints.",
          "Only move on to Tidy Up when the library looks right.",
        ],
      },
      {
        label: "What the tags mean",
        entries: [
          "Confidence shows how sure SimSuite is.",
          "Safety notes warn about risky placement.",
          "Bundle details mean files should stay together.",
        ],
      },
    ],
  },
  creatorAudit: {
    title: "Creator Audit Guide",
    intro:
      "This view groups files that probably came from the same creator, so you can fix names in batches.",
    sections: [
      {
        label: "How to use it",
        entries: [
          "Pick a group, confirm the creator name, then save it once for the whole batch.",
          "This works well for tags like [creator] or short prefixes.",
          "Future scans remember what you teach here.",
        ],
      },
      {
        label: "Safety",
        entries: [
          "This does not move anything.",
          "Your saved creator names win over automatic guesses later.",
          "Leave weak one-file cases alone if the match looks wrong.",
        ],
      },
    ],
  },
  categoryAudit: {
    title: "Category Audit Guide",
    intro:
      "This view groups files that look like the same type of CC or mod, so you can sort them in batches.",
    sections: [
      {
        label: "How to use it",
        entries: [
          "Pick a strong group with matching file names or folders.",
          "Choose the right type, then save it once for the whole batch.",
          "Skip mixed groups until you have better clues.",
        ],
      },
      {
        label: "Signals",
        entries: [
          "Groups use file names, folders, and package hints.",
          "Your saved type choices win over automatic guesses later.",
          "This changes library labels only and does not move files.",
        ],
      },
    ],
  },
  duplicates: {
    title: "Duplicates Guide",
    intro:
      "Use this view to check for exact copies, same-name lookalikes, and older versions before cleaning anything up.",
    sections: [
      {
        label: "Signals",
        entries: [
          "Exact means the files are byte-for-byte the same.",
          "Filename means the names match but the contents differ.",
          "Version means SimSuite found likely older or newer variants.",
        ],
      },
      {
        label: "Safety",
        entries: [
          "This view is check-only right now.",
          "Always compare the paths before deleting anything.",
          "Later cleanup should stay restore-point backed.",
        ],
      },
    ],
  },
  organize: {
    title: "Organize Guide",
    intro:
      "Only files that pass the safety check can be moved. Anything risky stays out of the batch.",
    sections: [
      {
        label: "How it works",
        entries: [
          "Pick a sorting style.",
          "Refresh the preview if your library changed.",
          "Only apply a batch when the safe count looks right.",
        ],
      },
      {
        label: "Undo",
        entries: [
          "Every approved batch creates a restore point.",
          "Use Snapshots to roll back if the result looks wrong.",
          "Script mods and Tray safety rules still win over folder plans.",
        ],
      },
    ],
  },
  review: {
    title: "Review Guide",
    intro:
      "This queue is for files SimSuite is not fully sure about, plus anything blocked by a safety rule.",
    sections: [
      {
        label: "Look for",
        entries: [
          "Low-confidence file names.",
          "Tray files sitting in the wrong folder.",
          "Items with repeated safety flags.",
        ],
      },
      {
        label: "What to do next",
        entries: [
          "Use the suggested path as the safe target when it looks right.",
          "Fix the file info or folder, then scan again.",
          "Go back to Tidy Up when the queue is under control.",
        ],
      },
    ],
  },
};

export function FieldGuide({ open, screen, userView, onClose }: FieldGuideProps) {
  const guide = GUIDE[screen];
  const { guideWidth, setGuideWidth } = useUiPreferences();

  useEffect(() => {
    if (!open) {
      return;
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        onClose();
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [open, onClose]);

  if (!open) {
    return null;
  }

  return (
    <div className="guide-shell" role="presentation" onClick={onClose}>
      <aside
        className="guide-drawer"
        role="dialog"
        aria-modal="true"
        aria-label={guide.title}
        onClick={(event) => event.stopPropagation()}
      >
        <ResizableEdgeHandle
          label="Resize guide drawer"
          value={guideWidth}
          min={320}
          max={620}
          onChange={setGuideWidth}
          side="left"
          className="guide-resize-handle"
        />
        <div className="guide-header">
          <div>
            <p className="eyebrow">Field guide</p>
            <h2>{guide.title}</h2>
          </div>
          <button type="button" className="secondary-action" onClick={onClose}>
            Close
          </button>
        </div>

        <p className="guide-intro">{guide.intro}</p>
        <div className="ghost-chip">{VIEW_LABELS[userView]}</div>

        <div className="guide-section-grid">
          {guide.sections.map((section) => (
            <section key={section.label} className="guide-section">
              <span className="section-label">{section.label}</span>
              <div className="guide-entry-stack">
                {section.entries.map((entry) => (
                  <div key={entry} className="guide-entry">
                    {entry}
                  </div>
                ))}
              </div>
            </section>
          ))}
        </div>
      </aside>
    </div>
  );
}
