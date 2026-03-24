import { useState } from "react";
import { AnimatePresence, m } from "motion/react";
import {
  Check,
  Inbox,
  Shield,
  type LucideProps,
} from "lucide-react";
import { overlayTransition } from "../lib/motion";
import { setDownloadsTourDismissed } from "../lib/guidedFlowStorage";

interface CasualGuidedToursProps {
  onComplete: () => void;
}

type StepIcon = "inbox" | "lanes" | "apply" | "control";

interface TourStep {
  title: string;
  body: string;
  icon: StepIcon;
}

const STEPS: TourStep[] = [
  {
    title: "Your Downloads inbox",
    body: "New mods land here before they go to your game. SimSuite checks each one to make sure everything is safe.",
    icon: "inbox",
  },
  {
    title: "Five waiting lanes",
    body: "**Ready to go** = safe to add to your game\n**Needs your input** = waiting for a choice\n**Needs setup** = has special rules to follow\n**Needs a look** = something might need attention\n**All done** = already handled",
    icon: "lanes",
  },
  {
    title: "How to add a mod to your game",
    body: "Click any item to see what it is. Hit the big green button when you're ready. Nothing gets added without you saying so.",
    icon: "apply",
  },
  {
    title: "You're always in charge",
    body: "Nothing moves without your permission. If something looks wrong, hit Ignore. You can always undo it later.",
    icon: "control",
  },
];

function StepIcon({ icon, size = 40 }: { icon: StepIcon; size?: number }) {
  const props: LucideProps = { size, strokeWidth: 1.75 };
  switch (icon) {
    case "inbox":
      return <Inbox {...props} />;
    case "lanes":
      return <LanesIcon size={size} />;
    case "apply":
      return <Check {...props} />;
    case "control":
      return <Shield {...props} />;
  }
}

function LanesIcon({ size = 40 }: { size?: number }) {
  const strokeWidth = 1.75;
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <line x1="3" y1="7" x2="21" y2="7" />
      <line x1="3" y1="12" x2="15" y2="12" />
      <line x1="3" y1="17" x2="18" y2="17" />
    </svg>
  );
}

function renderBody(body: string): string {
  return body;
}

export function CasualGuidedTours({ onComplete }: CasualGuidedToursProps) {
  const [step, setStep] = useState(0);

  function handleComplete() {
    setDownloadsTourDismissed();
    onComplete();
  }

  function handleSkip() {
    setDownloadsTourDismissed();
    onComplete();
  }

  function handleBack() {
    setStep((s) => Math.max(0, s - 1));
  }

  function handleNext() {
    if (step < STEPS.length - 1) {
      setStep((s) => s + 1);
    } else {
      handleComplete();
    }
  }

  const currentStep = STEPS[step];
  const isLast = step === STEPS.length - 1;

  return (
    <div className="casual-tour-backdrop" role="dialog" aria-modal="true" aria-label="Downloads tour">
      <div className="casual-tour-card">
        {/* Step dots */}
        <div className="casual-tour-step-dot-row" role="tablist" aria-label="Tour progress">
          {STEPS.map((_, i) => (
            <span
              key={i}
              role="tab"
              aria-selected={i === step}
              aria-label={`Step ${i + 1} of ${STEPS.length}`}
              className={`casual-tour-step-dot${i === step ? " is-active" : ""}`}
            />
          ))}
        </div>

        {/* Animated step content */}
        <AnimatePresence mode="wait" initial={false}>
          <m.div
            key={step}
            className="casual-tour-step-content"
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -8 }}
            transition={overlayTransition}
          >
            {/* Icon */}
            <div className="casual-tour-icon">
              <StepIcon icon={currentStep.icon} size={40} />
            </div>

            {/* Title */}
            <h2 className="casual-tour-title">{currentStep.title}</h2>

            {/* Body — render markdown-like bold syntax */}
            <p className="casual-tour-body">
              {renderBody(currentStep.body)
                .split("**")
                .map((segment, i) =>
                  i % 2 === 1 ? <strong key={i}>{segment}</strong> : segment,
                )}
            </p>
          </m.div>
        </AnimatePresence>

        {/* Navigation */}
        <div className="casual-tour-nav">
          <button
            type="button"
            className="ghost-chip"
            onClick={handleBack}
            disabled={step === 0}
            style={{ visibility: step === 0 ? "hidden" : undefined }}
          >
            Back
          </button>

          <div className="casual-tour-btn-group">
            <button
              type="button"
              className="casual-tour-skip"
              onClick={handleSkip}
            >
              Skip tour
            </button>
            <button
              type="button"
              className="primary-action"
              onClick={handleNext}
            >
              {isLast ? "Got it" : "Next"}
              {!isLast && <span aria-hidden="true"> →</span>}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
