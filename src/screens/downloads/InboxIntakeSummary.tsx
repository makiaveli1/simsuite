import { FolderSearch, Library, ListChecks, ShieldCheck } from "lucide-react";
import type { Screen, UserView } from "../../lib/types";

interface InboxIntakeSummaryProps {
  userView: UserView;
  totalItems: number;
  readyCount: number;
  reviewCount: number;
  blockedCount: number;
  watchedPath: string | null;
  lastCheckLabel: string;
  onNavigate: (screen: Screen) => void;
}

export function InboxIntakeSummary({
  userView,
  totalItems,
  readyCount,
  reviewCount,
  blockedCount,
  watchedPath,
  lastCheckLabel,
  onNavigate,
}: InboxIntakeSummaryProps) {
  return (
    <section className="inbox-intake-summary" aria-label="Inbox intake summary">
      <div className="inbox-intake-copy">
        <span className="eyebrow">Inbox</span>
        <h2>Review new downloads and imported batches.</h2>
        <p>
          Inbox is the intake area before content becomes part of Library or
          Organize planning. Use it to understand what arrived, what needs
          review, and what can become a preview plan later.
        </p>
      </div>

      <div className="inbox-intake-safety" aria-label="Inbox safety boundary">
        <ShieldCheck size={16} />
        <strong>No files changed</strong>
        <span>Review here first. File-changing actions are not available from this pass.</span>
      </div>

      <div className="inbox-intake-metrics" aria-label="Inbox batch totals">
        <span>
          <strong>{totalItems.toLocaleString()}</strong>
          <small>{totalItems === 1 ? "Item" : "Items"} in intake</small>
        </span>
        <span>
          <strong>{readyCount.toLocaleString()}</strong>
          <small>Ready for review</small>
        </span>
        <span>
          <strong>{reviewCount.toLocaleString()}</strong>
          <small>Need attention</small>
        </span>
        <span>
          <strong>{blockedCount.toLocaleString()}</strong>
          <small>Blocked</small>
        </span>
      </div>

      <div className="inbox-intake-actions" aria-label="Inbox safe next steps">
        <button
          type="button"
          className="primary-action"
          onClick={() => onNavigate("organize")}
        >
          <ListChecks size={16} />
          Create preview plan
        </button>
        <button
          type="button"
          className="secondary-action"
          onClick={() => onNavigate("organize")}
        >
          <FolderSearch size={16} />
          Open Organize
        </button>
        <button
          type="button"
          className="secondary-action"
          onClick={() => onNavigate("library")}
        >
          <Library size={16} />
          Open Library
        </button>
      </div>

      <details className="inbox-intake-technical">
        <summary>Technical details</summary>
        <div>
          <span>Watched folder</span>
          <code>{watchedPath ?? "Not configured"}</code>
        </div>
        <div>
          <span>Last check</span>
          <code>{lastCheckLabel}</code>
        </div>
        <div>
          <span>View mode</span>
          <code>{userView}</code>
        </div>
      </details>
    </section>
  );
}
