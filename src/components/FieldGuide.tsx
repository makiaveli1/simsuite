import { useEffect, useMemo, useState } from "react";
import type { LucideIcon } from "lucide-react";
import { AnimatePresence, m } from "motion/react";
import {
  BookOpen,
  Copy,
  Download,
  Fingerprint,
  FolderTree,
  House,
  ListChecks,
  RefreshCw,
  ShieldAlert,
  ShieldCheck,
  Shapes,
  SlidersHorizontal,
  Workflow,
} from "lucide-react";
import { ResizableEdgeHandle } from "./ResizableEdgeHandle";
import { useUiPreferences } from "./UiPreferencesContext";
import type { ExperienceMode, Screen, UserView } from "../lib/types";
import {
  experienceModeToLegacyView,
  getExperienceModeProfile,
} from "../lib/experienceMode";
import { overlayTransition, panelSpring } from "../lib/motion";
import { viewModeLabel } from "../lib/uiLanguage";

interface FieldGuideProps {
  open: boolean;
  screen: Screen;
  experienceMode: ExperienceMode;
  onClose: () => void;
}

type GuideTopicId = Screen | "workflow" | "safety";

interface GuideSection {
  label: string;
  tone?: "accent" | "warn" | "neutral";
  items: string[];
}

interface GuideFact {
  label: string;
  value: string;
}

interface GuideTopic {
  id: GuideTopicId;
  navLabel: string;
  title: string;
  intro: string;
  purpose: string;
  nextStep: string;
  status: string;
  icon: LucideIcon;
  sections: GuideSection[];
  facts: GuideFact[];
}

const VIEW_SUMMARIES: Record<UserView, string> = {
  beginner: "Keeps the next safe step up front, hides the extra clutter, and uses friendlier Sims-style help.",
  standard: "Keeps the main action easy to spot, while leaving the important clues open as you sort.",
  power: "Opens the full receipts: raw paths, deeper evidence, and denser checks when you want them.",
};

const TOPIC_ORDER: GuideTopicId[] = [
  "workflow",
  "safety",
  "home",
  "settings",
  "downloads",
  "library",
  "creatorAudit",
  "categoryAudit",
  "duplicates",
  "organize",
  "review",
];

function viewCopy(
  userView: UserView,
  copy: { beginner: string; standard: string; power: string },
) {
  return copy[userView];
}

function buildGuideTopics(userView: UserView): Record<GuideTopicId, GuideTopic> {
  return {
    workflow: {
      id: "workflow",
      navLabel: "Workflow",
      title: "How SimSuite Works",
      intro:
        "Every safe action in SimSuite follows the same chain so the app stays predictable on big libraries.",
      purpose: "Use this when you want the big picture before jumping into a screen.",
      nextStep: "Start on Home, scan the library, then use Inbox, Review, or Tidy Up depending on what you see.",
      status: "Core workflow",
      icon: ListChecks,
      sections: [
        {
          label: "Safety chain",
          tone: "accent",
          items: [
            "Scan files first.",
            "Parse names and file clues.",
            "Build a rule-based destination idea.",
            "Validate that idea against safety rules.",
            "Show a preview before any move.",
            "Wait for your approval.",
            "Create a restore point, then move only the approved safe files.",
          ],
        },
        {
          label: "What screens are for",
          items: [
            "Home sets folders, shows totals, and starts scans.",
            "Downloads handles new files and archives before they enter your main library.",
            "Library is the read-only file desk for inspection and learning fixes.",
            "Review holds files that still need a person.",
            "Tidy Up applies the approved move plan.",
          ],
        },
        {
          label: "What SimSuite will not do",
          tone: "warn",
          items: [
            "It will not move files straight from AI guesses.",
            "It will not auto-fix risky items without showing you a preview.",
            "It will not mix Tray content into Mods folders.",
          ],
        },
      ],
      facts: [
        { label: "Move rule", value: "Approval-first only" },
        { label: "Undo model", value: "Restore points" },
        { label: "Best first stop", value: "Home" },
      ],
    },
    safety: {
      id: "safety",
      navLabel: "Safety",
      title: "Safety Rules",
      intro:
        "These rules override speed, convenience, and folder preference. If a plan conflicts with them, the plan loses.",
      purpose: "Use this when you want to understand why a file stayed in review or why a preview path changed.",
      nextStep: "If a file keeps getting blocked, inspect it in Library or Review and save a creator or type fix first.",
      status: "Always enforced",
      icon: ShieldCheck,
      sections: [
        {
          label: "Always true",
          tone: "accent",
          items: [
            "Batch moves create restore points first.",
            "Moves must be reversible.",
            "Anything uncertain stays visible instead of moving silently.",
          ],
        },
        {
          label: "Placement rules",
          items: [
            "Script mods never go deeper than one subfolder.",
            "Tray files stay out of Mods.",
            "Bundle-related files should move together when SimSuite can detect the bundle.",
          ],
        },
        {
          label: "When something is blocked",
          tone: "warn",
          items: [
            "Low confidence keeps files in Review or Inbox.",
            "Path collisions force a human check.",
            "Validator corrections can change a rule-engine path before you approve it.",
          ],
        },
      ],
      facts: [
        { label: "Snapshots", value: "Before batch moves" },
        { label: "Script depth", value: "1 subfolder max" },
        { label: "Tray rule", value: "Never sort into Mods" },
      ],
    },
    home: makeScreenTopic(
      "home",
      House,
      "Home",
      "Set your folders, check the library totals, and jump to the right workspace.",
      viewCopy(userView, {
        beginner: "Use Home as your starting screen whenever you add or remove CC.",
        standard: "Use Home to verify folders, scan state, and the quickest path to the next task.",
        power: "Use Home as your control surface for roots, totals, and scan entry.",
      }),
      "Start here",
      "After a scan, go to Inbox for new downloads, Review for blocked files, or Tidy Up for safe moves.",
      [
        section("What matters here", [
          "Folder paths must be correct before the rest of the app can help.",
          "The totals show what SimSuite has already indexed.",
          "Shortcut cards take you straight to the next useful screen.",
        ]),
        section("Good habits", [
          "Rescan after installing or removing content.",
          "Keep Downloads configured so new files land in the inbox workflow.",
          "Treat Home as the health check before cleanup sessions.",
        ]),
      ],
      [
        { label: "Folders", value: "Mods, Tray, Downloads" },
        { label: "Main action", value: "Scan library" },
        { label: "Best after scan", value: "Inbox or Review" },
      ],
    ),
    settings: makeScreenTopic(
      "settings",
      SlidersHorizontal,
      "Settings",
      "Change the look, density, and detail level without cluttering the work screens.",
      viewCopy(userView, {
        beginner: "Use this when you want SimSuite to feel simpler, roomier, or easier to read.",
        standard: "Use this to tune the app's appearance and workspace feel without touching your files.",
        power: "Use this for density, skin, and layout reset changes while keeping the operational screens focused.",
      }),
      "Personal setup",
      "After changing these options, jump back to Home, Library, or Tidy Up and keep working.",
      [
        section("What lives here", [
          "View mode changes how much detail the app shows.",
          "Skins change color, mood, and motion feel.",
          "Workspace size changes how tightly panels and rows are packed.",
          "Reset panels restores saved widths, heights, and dock layouts.",
        ]),
        section("What does not change", [
          "Your files do not move from this screen.",
          "Scan results, creator learning, and review queues stay intact.",
          "Safety rules stay the same no matter which skin or density you pick.",
        ]),
      ],
      [
        { label: "Saved", value: "Locally on this PC" },
        { label: "Affects", value: "Appearance and layout only" },
        { label: "Best for", value: "Personalizing the workspace" },
      ],
    ),
    downloads: makeScreenTopic(
      "downloads",
      Download,
      "Downloads Inbox",
      "Review newly downloaded files and archives before they join your library.",
      viewCopy(userView, {
        beginner: "Use this like an intake desk for new CC and mod downloads.",
        standard: "Use this to triage direct downloads, unpacked archives, and safe hand-off previews.",
        power: "Use this as the staging desk before files enter Mods or Tray through the validated pipeline.",
      }),
      "New arrivals",
      "Apply the safe part of a batch, or leave uncertain files in the inbox for another pass.",
      [
        section("What you can do", [
          "See newly indexed direct downloads and supported archives.",
          "Preview where safe files would go under the current preset.",
          "Ignore a batch if you do not want it in the active intake queue.",
        ]),
        section("What stays blocked", [
          "Review-required files stay in the inbox.",
          "Archive errors stay visible instead of being hidden.",
          "Moves still wait for approval and create restore points first.",
        ], "warn"),
      ],
      [
        { label: "Sources", value: "File and archive intake" },
        { label: "Safe action", value: "Apply safe batch" },
        { label: "Not moved yet", value: "Review leftovers" },
      ],
    ),
    library: makeScreenTopic(
      "library",
      FolderTree,
      "Library",
      "Inspect indexed files, warnings, bundle clues, and saved learning data.",
      viewCopy(userView, {
        beginner: "Use Library when you want to check one file without moving anything.",
        standard: "Use Library to inspect file details before saving creators or types.",
        power: "Use Library for the full indexed view, source paths, confidence, and metadata hints.",
      }),
      "Inspect files",
      "If the name or type is wrong, save a creator or type fix here or jump to a batch audit.",
      [
        section("What the details panel is for", [
          "Fix creators one file at a time.",
          "Correct types without moving the file.",
          "Inspect parser warnings, internal file hints, and full paths.",
        ]),
        section("What the tags mean", [
          "Confidence shows how sure SimSuite is.",
          "Safety notes show why a file should pause or reroute.",
          "Bundle details warn when files should stay together.",
        ]),
      ],
      [
        { label: "Moves allowed", value: "No" },
        { label: "Best use", value: "Inspect and save" },
        { label: "Related screens", value: "Creators, Types" },
      ],
    ),
    updates: makeScreenTopic(
      "updates",
      RefreshCw,
      "Updates",
      "Track mods for updates, set up new watch sources, and check for new versions.",
      viewCopy(userView, {
        beginner: "Check which of your tracked mods have updates available.",
        standard: "Track mods for updates, set up new sources, and refresh all tracked items.",
        power: "Full watch management with source configuration, review, and bulk refresh.",
      }),
      "Watch center",
      "Start by checking for updates, then set up any mods that need watching.",
      [
        section("What you can do", [
          "Check tracked mods for available updates.",
          "Set up watch sources for mods that need tracking.",
          "Review uncertain watch sources before they become active.",
        ]),
        section("Modes", [
          "Tracked: See all watched mods and their update status.",
          "Setup: Set up new watch sources for mods.",
          "Review: Check sources that need human verification.",
        ]),
      ],
      [
        { label: "Focus", value: "Mod updates" },
        { label: "Safe action", value: "Check for updates" },
        { label: "Not moved", value: "Read-only tracking" },
      ],
    ),
    creatorAudit: makeScreenTopic(
      "creatorAudit",
      Fingerprint,
      "Creators",
      "Fix repeated unknown creators in batches instead of one file at a time.",
      viewCopy(userView, {
        beginner: "Use this when many files clearly look like they came from the same creator.",
        standard: "Use grouped clues like prefixes, tags, and folder patterns to batch-save creators.",
        power: "Use this to batch-save creator names across unresolved groups without touching file placement.",
      }),
      "Batch saving",
      "Run another scan later and SimSuite should reuse the creator you saved here.",
      [
        section("Strong groups", [
          "Bracket tags like [creator] often group well.",
          "Repeated prefixes and folder names are useful clues.",
          "Leave weak one-file cases alone if the match looks off.",
        ]),
        section("What saving changes", [
          "Future scans remember the creator.",
          "Review items tied to missing creator identity can clear out.",
          "Nothing moves from this screen.",
        ]),
      ],
      [
        { label: "Batch action", value: "Save creator" },
        { label: "Affects", value: "Future scans and previews" },
        { label: "Moves allowed", value: "No" },
      ],
    ),
    categoryAudit: makeScreenTopic(
      "categoryAudit",
      Shapes,
      "Types",
      "Fix repeated type guesses in batches when files obviously belong together.",
      viewCopy(userView, {
        beginner: "Use this when several files are clearly the same type of CC or mod.",
        standard: "Use grouped keyword and folder clues to save a shared type once.",
        power: "Use this to batch-lock type and subtype decisions over heuristic guesses.",
      }),
      "Batch saving",
      "Rescan later and SimSuite should keep using the saved type instead of the old guess.",
      [
        section("Useful clues", [
          "Shared file words can suggest hair, skin, build items, presets, or gameplay mods.",
          "Folder patterns and package hints strengthen the group.",
          "Mixed groups are better left for later than forced into the wrong type.",
        ]),
        section("What saving changes", [
          "The saved type and subtype override the automatic label later.",
          "Review items tied to weak category detection can clear out.",
          "This still does not move files.",
        ]),
      ],
      [
        { label: "Batch action", value: "Save type" },
        { label: "Affects", value: "Labels and future previews" },
        { label: "Moves allowed", value: "No" },
      ],
    ),
    duplicates: makeScreenTopic(
      "duplicates",
      Copy,
      "Duplicates",
      "Compare exact duplicates, same-name lookalikes, and likely version pairs.",
      viewCopy(userView, {
        beginner: "Use this to check whether you have the same mod twice before deleting anything elsewhere.",
        standard: "Use this to compare duplicate paths and identify which copy looks newer or safer.",
        power: "Use this to inspect exact, filename, and version matches before future cleanup actions exist.",
      }),
      "Inspect only",
      "Use it as an inspection desk for now. Snapshot-backed duplicate actions are still a later phase.",
      [
        section("What the match types mean", [
          "Exact means byte-for-byte the same file.",
          "Filename means the names match even if the contents differ.",
          "Version means SimSuite found likely older or newer variants.",
        ]),
        section("Important caution", [
          "This screen is inspection-only right now.",
          "Do not delete just from the label alone; always compare paths and dates.",
          "Archive and downloads paths can reveal the newer source copy.",
        ], "warn"),
      ],
      [
        { label: "Current phase", value: "Inspect only" },
        { label: "Best check", value: "Compare both paths" },
        { label: "Future work", value: "Safe cleanup actions" },
      ],
    ),
    organize: makeScreenTopic(
      "organize",
      Workflow,
      "Tidy Up",
      "Preview safe moves, apply approved batches, and roll back with restore points.",
      viewCopy(userView, {
        beginner: "Use this after the library looks right and you want SimSuite to tidy safe files.",
        standard: "Use presets, preview counts, and restore points to apply only the validated part of the plan.",
        power: "Use this to inspect rule output, validator corrections, snapshots, and approved move batches.",
      }),
      "Apply safe moves",
      "If the preview looks wrong, go fix names, types, or review issues first instead of forcing the batch.",
      [
        section("What moves from here", [
          "Only validated safe files move.",
          "Review-required rows stay out of the batch.",
          "Snapshots are created before approved moves.",
        ], "accent"),
        section("What can change a path", [
          "The chosen preset builds the first path idea.",
          "The validator can flatten script depth or reroute Tray content.",
          "Locked creator routes can override the preset output.",
        ]),
      ],
      [
        { label: "Safe action", value: "Apply safe batch" },
        { label: "Undo", value: "Restore points" },
        { label: "Best before move", value: "Check preview rows" },
      ],
    ),
    review: makeScreenTopic(
      "review",
      ShieldAlert,
      "Review",
      "Handle files that still need a person because a name, type, or safety rule is not settled yet.",
      viewCopy(userView, {
        beginner: "Use this when SimSuite says a file needs review.",
        standard: "Use this queue to see why a file was blocked and where it would safely go.",
        power: "Use this as the hold queue for low confidence, validator conflicts, and path-risk cases.",
      }),
      "Needs review",
      "Go back to Tidy Up only after the queue is under control.",
      [
        section("Common reasons", [
          "Low-confidence creator or type detection.",
          "Tray content found outside the Tray folder.",
          "Unsafe script depth or collisions.",
        ]),
        section("Good next moves", [
          "Use Library for one-file fixes.",
          "Use Creators or Types for batch fixes.",
          "Rescan after changing the file info or the folder layout.",
        ]),
      ],
      [
        { label: "Moves allowed", value: "No" },
        { label: "Best use", value: "Resolve blockers" },
        { label: "Usually leads to", value: "Library or audits" },
      ],
    ),
  };
}

function makeScreenTopic(
  id: Screen,
  icon: LucideIcon,
  title: string,
  intro: string,
  purpose: string,
  status: string,
  nextStep: string,
  sections: GuideSection[],
  facts: GuideFact[],
): GuideTopic {
  return {
    id,
    navLabel: title,
    title,
    intro,
    purpose,
    status,
    nextStep,
    icon,
    sections,
    facts,
  };
}

function section(
  label: string,
  items: string[],
  tone: GuideSection["tone"] = "neutral",
): GuideSection {
  return { label, items, tone };
}

export function FieldGuide({
  open,
  screen,
  experienceMode,
  onClose,
}: FieldGuideProps) {
  const userView = experienceModeToLegacyView(experienceMode);
  const modeProfile = getExperienceModeProfile(experienceMode);
  const topics = useMemo(() => buildGuideTopics(userView), [userView]);
  const [activeTopicId, setActiveTopicId] = useState<GuideTopicId>(screen);
  const { guideWidth, setGuideWidth } = useUiPreferences();

  useEffect(() => {
    if (open) {
      setActiveTopicId(screen);
    }
  }, [open, screen]);

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

  const activeTopic = topics[activeTopicId];

  return (
    <AnimatePresence>
      {open ? (
        <m.div
          className="guide-shell"
          role="presentation"
          onClick={onClose}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={overlayTransition}
        >
          <m.aside
            className="guide-drawer"
            role="dialog"
            aria-modal="true"
            aria-label={activeTopic.title}
            onClick={(event) => event.stopPropagation()}
            initial={{ opacity: 0, x: 28 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: 24 }}
            transition={panelSpring}
          >
            <ResizableEdgeHandle
              label="Resize guide drawer"
              value={guideWidth}
              min={460}
              max={1040}
              onChange={setGuideWidth}
              side="left"
              className="guide-resize-handle"
            />

            <div className="guide-header">
              <div className="guide-header-main">
                <p className="eyebrow">Field guide</p>
                <h2>{activeTopic.title}</h2>
                <p className="guide-intro">{activeTopic.intro}</p>
                <p className="workspace-toolbar-copy">{modeProfile.workspaceSummary}</p>
              </div>
              <div className="guide-header-actions">
                <span className="ghost-chip">{viewModeLabel(experienceMode)} mode</span>
                <button type="button" className="secondary-action" onClick={onClose}>
                  Close
                </button>
              </div>
            </div>

            <div className="guide-workbench">
              <div className="guide-nav-panel">
                <div className="guide-nav-summary">
                  <strong>Guide mode</strong>
                  <span>{VIEW_SUMMARIES[userView]}</span>
                  <span>{modeProfile.summary}</span>
                </div>

                <div className="guide-topic-list" aria-label="Guide topics">
                  {TOPIC_ORDER.map((topicId) => {
                    const topic = topics[topicId];
                    const Icon = topic.icon;
                    const isActive = activeTopicId === topicId;

                    return (
                      <button
                        key={topicId}
                        type="button"
                        className={`guide-topic-button ${isActive ? "is-active" : ""}`}
                        onClick={() => setActiveTopicId(topicId)}
                        title={topic.title}
                      >
                        <span className="guide-topic-icon">
                          <Icon size={15} strokeWidth={2} />
                        </span>
                        <span className="guide-topic-copy">
                          <strong>{topic.navLabel}</strong>
                          <span>{topic.status}</span>
                        </span>
                      </button>
                    );
                  })}
                </div>

                <div className="guide-nav-note">
                  <BookOpen size={15} strokeWidth={2} />
                  <span>
                    Press Esc to close. Drag the left edge to resize this help desk.
                  </span>
                </div>
              </div>

              <div className="guide-content-panel">
                <div className="guide-hero-grid">
                  <div className="guide-hero-card guide-hero-card-primary">
                    <span className="section-label">Use this when</span>
                    <strong>{activeTopic.purpose}</strong>
                  </div>
                  <div className="guide-hero-card">
                    <span className="section-label">Best next step</span>
                    <strong>{activeTopic.nextStep}</strong>
                  </div>
                </div>

                <div className="guide-fact-grid">
                  {activeTopic.facts.map((fact) => (
                    <div key={fact.label} className="guide-fact-card">
                      <span>{fact.label}</span>
                      <strong>{fact.value}</strong>
                    </div>
                  ))}
                </div>

                <div className="guide-section-grid">
                  {activeTopic.sections.map((section) => (
                    <section
                      key={section.label}
                      className={`guide-section guide-section-${section.tone ?? "neutral"}`}
                    >
                      <span className="section-label">{section.label}</span>
                      <div className="guide-entry-stack">
                        {section.items.map((entry) => (
                          <div key={entry} className="guide-entry">
                            {entry}
                          </div>
                        ))}
                      </div>
                    </section>
                  ))}
                </div>
              </div>
            </div>
          </m.aside>
        </m.div>
      ) : null}
    </AnimatePresence>
  );
}
