import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import type { LibraryFacets, LibrarySummary, UserView } from "../../lib/types";
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

afterEach(() => {
  cleanup();
});

function renderTopStrip(userView: UserView) {
  return render(
    <LibraryTopStrip
      userView={userView}
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
}

it("uses trust-first update wording in Library filters and summary", () => {
  renderTopStrip("standard");

  expect(screen.getByRole("button", { name: /all types/i })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /all signals/i })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /possible updates/i })).toBeInTheDocument();
  expect(screen.getAllByText(/update leads/i).length).toBeGreaterThan(0);
  expect(screen.queryByText(/has updates/i)).toBeNull();
});

it.each<UserView>(["beginner", "standard", "power"])(
  "keeps the main Library toolbar controls accessible in %s mode",
  (userView) => {
    renderTopStrip(userView);

    expect(screen.getByRole("textbox", { name: /search library/i })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: /sort by/i })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: /items per page/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /list view/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /grid view/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /folders view/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /advanced/i })).toBeInTheDocument();
  },
);
