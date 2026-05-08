import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import type { LibraryFacets, LibrarySummary } from "../../lib/types";
import { LibraryTopStrip } from "./LibraryTopStrip";

const emptyFacets: LibraryFacets = {
  creators: [],
  kinds: [],
  subtypes: [],
  sources: [],
  taxonomyKinds: [],
};

const summaryWithUpdateLeads: LibrarySummary = {
  total: 12,
  tracked: 3,
  notTracked: 9,
  hasUpdates: 2,
  needsReview: 1,
  duplicates: 0,
  disabled: 0,
};

it("uses trust-first update wording in Library filters and summary", () => {
  render(
    <LibraryTopStrip
      userView="standard"
      activeFilterCount={0}
      search=""
      sortBy="name"
      watchFilter="all"
      filters={{
        kind: "",
        creator: "",
        source: "",
        subtype: "",
        minConfidence: "",
      }}
      facets={emptyFacets}
      drawerOpen={false}
      librarySummary={summaryWithUpdateLeads}
      viewMode="list"
      pageSize={100}
      densityValue={50}
      onPageSizeChange={() => {}}
      onDensityChange={() => {}}
      onSearchChange={() => {}}
      onSortByChange={() => {}}
      onWatchFilterChange={() => {}}
      onFiltersChange={() => {}}
      onDrawerToggle={() => {}}
      onResetFilters={() => {}}
      onViewModeChange={() => {}}
    />,
  );

  expect(screen.getByRole("button", { name: /possible updates/i })).toBeInTheDocument();
  expect(screen.getAllByText(/update leads/i).length).toBeGreaterThan(0);
  expect(screen.queryByText(/has updates/i)).toBeNull();
});
