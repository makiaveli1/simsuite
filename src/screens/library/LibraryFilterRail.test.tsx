import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { LibraryFilterRail } from "./LibraryFilterRail";

it("keeps only the core filters visible in casual mode", () => {
  render(
    <LibraryFilterRail
      userView="beginner"
      facets={{
        creators: ["Lumpinou"],
        kinds: ["CAS"],
        sources: ["mods"],
        subtypes: ["Hair"],
        taxonomyKinds: ["CAS"],
      }}
      filters={{
        kind: "",
        creator: "",
        source: "",
        subtype: "",
        minConfidence: "",
      }}
      watchFilter="all"
      activeFilterCount={1}
      resultsCount={12}
      isCollapsed={false}
      moreFiltersOpen={false}
      librarySummary={null}
      onToggleCollapsed={() => {}}
      onFiltersChange={() => {}}
      onResetFilters={() => {}}
      onToggleMoreFilters={() => {}}
      onWatchFilterChange={() => {}}
    />,
  );

  expect(screen.getByLabelText(/type/i)).toBeVisible();
  expect(screen.queryByLabelText(/confidence/i)).not.toBeInTheDocument();
});
