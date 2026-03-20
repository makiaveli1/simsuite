import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { LibraryTopStrip } from "./LibraryTopStrip";

afterEach(() => {
  cleanup();
});

it("shows the common top filters in the main strip", () => {
  render(
    <LibraryTopStrip
      userView="standard"
      facets={{
        creators: ["Lumpinou"],
        kinds: ["Gameplay"],
        sources: ["mods"],
        subtypes: ["Romance"],
        taxonomyKinds: ["Gameplay"],
      }}
      search=""
      filters={{
        kind: "",
        creator: "",
        source: "",
        subtype: "",
        minConfidence: "",
        unsafeOnly: false,
      }}
      shownCount={100}
      totalCount={13010}
      activeFilterCount={0}
      moreFiltersOpen={false}
      onSearchChange={() => {}}
      onFilterChange={() => {}}
      onToggleMoreFilters={() => {}}
      onReset={() => {}}
    />,
  );

  expect(screen.getByLabelText(/search library/i)).toBeVisible();
  expect(screen.getByLabelText(/type/i)).toBeVisible();
  expect(screen.getByLabelText(/creator/i)).toBeVisible();
  expect(screen.getByLabelText(/folder/i)).toBeVisible();
  expect(screen.queryByLabelText(/subtype/i)).not.toBeInTheDocument();
});

it("shows the extra filter row only when more filters is open", () => {
  render(
    <LibraryTopStrip
      userView="power"
      facets={{
        creators: ["Lumpinou"],
        kinds: ["Gameplay"],
        sources: ["mods"],
        subtypes: ["Romance"],
        taxonomyKinds: ["Gameplay"],
      }}
      search=""
      filters={{
        kind: "",
        creator: "",
        source: "",
        subtype: "",
        minConfidence: "",
        unsafeOnly: false,
      }}
      shownCount={100}
      totalCount={13010}
      activeFilterCount={2}
      moreFiltersOpen
      onSearchChange={() => {}}
      onFilterChange={() => {}}
      onToggleMoreFilters={() => {}}
      onReset={() => {}}
    />,
  );

  expect(screen.getByLabelText(/subtype/i)).toBeVisible();
  expect(screen.getByLabelText(/confidence/i)).toBeVisible();
  expect(screen.getByRole("button", { name: /reset filters/i })).toBeVisible();
});

it("keeps extra filters hidden in creator view until asked for", () => {
  render(
    <LibraryTopStrip
      userView="power"
      facets={{
        creators: ["Lumpinou"],
        kinds: ["Gameplay"],
        sources: ["mods"],
        subtypes: ["Romance"],
        taxonomyKinds: ["Gameplay"],
      }}
      search=""
      filters={{
        kind: "",
        creator: "",
        source: "",
        subtype: "",
        minConfidence: "",
        unsafeOnly: false,
      }}
      shownCount={100}
      totalCount={13010}
      activeFilterCount={0}
      moreFiltersOpen={false}
      onSearchChange={() => {}}
      onFilterChange={() => {}}
      onToggleMoreFilters={() => {}}
      onReset={() => {}}
    />,
  );

  expect(screen.queryByLabelText(/subtype/i)).not.toBeInTheDocument();
  expect(screen.queryByLabelText(/confidence/i)).not.toBeInTheDocument();
});
