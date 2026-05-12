import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { useState, type ComponentProps } from "react";
import type { LibraryFacets, LibrarySummary, UserView } from "../../lib/types";
import { LibraryTopStrip } from "./LibraryTopStrip";

const emptyFacets: LibraryFacets = {
  creators: [],
  kinds: [],
  subtypes: [],
  sources: [],
  taxonomyKinds: [],
};

const facetsWithKinds: LibraryFacets = {
  ...emptyFacets,
  kinds: ["Gameplay", "ScriptMods"],
  creators: ["Deaderpool"],
  subtypes: ["Utilities"],
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

function renderTopStrip(userView: UserView, overrides: Partial<ComponentProps<typeof LibraryTopStrip>> = {}) {
  const props: ComponentProps<typeof LibraryTopStrip> = {
    userView,
    activeFilterCount: 0,
    search: "",
    sortBy: "name",
    watchFilter: "all",
    filters: {
      kind: "",
      creator: "",
      source: "",
      subtype: "",
      minConfidence: "",
    },
    facets: emptyFacets,
    drawerOpen: false,
    librarySummary: summaryWithUpdateLeads,
    viewMode: "list",
    pageSize: 100,
    densityValue: 50,
    filtersCollapsed: false,
    atmosphereEnabled: false,
    onPageSizeChange: vi.fn(),
    onDensityChange: vi.fn(),
    onFiltersCollapsedChange: vi.fn(),
    onAtmosphereChange: vi.fn(),
    onSearchChange: vi.fn(),
    onSortByChange: vi.fn(),
    onResetSort: vi.fn(),
    onWatchFilterChange: vi.fn(),
    onFiltersChange: vi.fn(),
    onDrawerToggle: vi.fn(),
    onResetFilters: vi.fn(),
    onViewModeChange: vi.fn(),
    ...overrides,
  };

  return {
    ...render(
    <LibraryTopStrip
      {...props}
    />,
    ),
    props,
  };
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
    expect(screen.getByRole("button", { name: /hide filters/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /advanced/i })).toBeInTheDocument();
  },
);

it("shows grouped type and signal filters with selected chip state", () => {
  renderTopStrip("standard", {
    facets: facetsWithKinds,
    filters: {
      kind: "Gameplay",
      creator: "",
      source: "",
      subtype: "",
      minConfidence: "",
    },
    watchFilter: "needs_attention",
    activeFilterCount: 2,
  });

  expect(screen.getByText("Types")).toBeInTheDocument();
  expect(screen.getByText("Signals")).toBeInTheDocument();
  expect(screen.getAllByRole("button", { name: /gameplay/i }).some((button) => button.getAttribute("aria-pressed") === "true")).toBe(true);
  expect(screen.getAllByRole("button", { name: /needs review/i }).some((button) => button.getAttribute("aria-pressed") === "true")).toBe(true);
});

it("shows removable active filter pills and clears only narrowing filters", () => {
  const onSearchChange = vi.fn();
  const onFiltersChange = vi.fn();
  const onWatchFilterChange = vi.fn();
  const onResetFilters = vi.fn();
  const onResetSort = vi.fn();
  const onPageSizeChange = vi.fn();
  const onDensityChange = vi.fn();
  const onViewModeChange = vi.fn();

  renderTopStrip("standard", {
    activeFilterCount: 4,
    search: "mccc",
    sortBy: "creator",
    watchFilter: "not_tracked",
    filters: {
      kind: "ScriptMods",
      creator: "Deaderpool",
      source: "",
      subtype: "",
      minConfidence: "",
    },
    facets: facetsWithKinds,
    onSearchChange,
    onFiltersChange,
    onWatchFilterChange,
    onResetFilters,
    onResetSort,
    onPageSizeChange,
    onDensityChange,
    onViewModeChange,
  });

  expect(screen.getByRole("button", { name: /clear search: mccc/i })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /clear type: script mods/i })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /clear signal: no update source/i })).toBeInTheDocument();
  expect(screen.getByText(/sorted: creator/i)).toBeInTheDocument();

  fireEvent.click(screen.getByRole("button", { name: /clear filters/i }));
  expect(onResetFilters).toHaveBeenCalledTimes(1);
  expect(onResetSort).not.toHaveBeenCalled();
  expect(onPageSizeChange).not.toHaveBeenCalled();
  expect(onDensityChange).not.toHaveBeenCalled();
  expect(onViewModeChange).not.toHaveBeenCalled();

  fireEvent.click(screen.getByRole("button", { name: /reset sort/i }));
  expect(onResetSort).toHaveBeenCalledTimes(1);
});

it("clears individual active filter pills without using unsafe wording", () => {
  const onSearchChange = vi.fn();
  const onFiltersChange = vi.fn();
  const onWatchFilterChange = vi.fn();

  renderTopStrip("power", {
    activeFilterCount: 5,
    search: "lot51",
    watchFilter: "has_updates",
    filters: {
      kind: "Gameplay",
      creator: "Deaderpool",
      source: "mods",
      subtype: "Utilities",
      minConfidence: "0.55",
    },
    facets: facetsWithKinds,
    onSearchChange,
    onFiltersChange,
    onWatchFilterChange,
  });

  fireEvent.click(screen.getByRole("button", { name: /clear search: lot51/i }));
  expect(onSearchChange).toHaveBeenCalledWith("");

  fireEvent.click(screen.getByRole("button", { name: /clear type: gameplay/i }));
  expect(onFiltersChange).toHaveBeenCalledWith({ kind: "" });

  fireEvent.click(screen.getByRole("button", { name: /clear signal: possible updates/i }));
  expect(onWatchFilterChange).toHaveBeenCalledWith("all");

  const text = document.body.textContent ?? "";
  expect(text).not.toMatch(/safe to delete|confirmed duplicate|definitely outdated|official source found/i);
});

it("keeps Advanced accessible and shows precision filter count", () => {
  const onDrawerToggle = vi.fn();

  renderTopStrip("standard", {
    activeFilterCount: 1,
    filters: {
      kind: "",
      creator: "Deaderpool",
      source: "",
      subtype: "",
      minConfidence: "",
    },
    facets: facetsWithKinds,
    onDrawerToggle,
  });

  const advanced = screen.getByRole("button", { name: /advanced/i });
  expect(advanced).toHaveAttribute("aria-expanded", "false");
  expect(advanced).toHaveTextContent("1");

  fireEvent.click(advanced);
  expect(onDrawerToggle).toHaveBeenCalledTimes(1);
});

it("collapses the filter deck while keeping command controls available", () => {
  const onDrawerToggle = vi.fn();
  const props: ComponentProps<typeof LibraryTopStrip> = {
    userView: "standard",
    activeFilterCount: 4,
    search: "mccc",
    sortBy: "name",
    watchFilter: "not_tracked",
    filters: {
      kind: "ScriptMods",
      creator: "Deaderpool",
      source: "",
      subtype: "",
      minConfidence: "",
    },
    facets: facetsWithKinds,
    drawerOpen: true,
    librarySummary: summaryWithUpdateLeads,
    viewMode: "list",
    pageSize: 100,
    densityValue: 50,
    atmosphereEnabled: false,
    onPageSizeChange: vi.fn(),
    onDensityChange: vi.fn(),
    onAtmosphereChange: vi.fn(),
    onSearchChange: vi.fn(),
    onSortByChange: vi.fn(),
    onResetSort: vi.fn(),
    onWatchFilterChange: vi.fn(),
    onFiltersChange: vi.fn(),
    onResetFilters: vi.fn(),
    onViewModeChange: vi.fn(),
    onDrawerToggle,
    filtersCollapsed: false,
    onFiltersCollapsedChange: vi.fn(),
  };

  function ControlledTopStrip() {
    const [collapsed, setCollapsed] = useState(false);
    const [drawerOpen, setDrawerOpen] = useState(true);

    return (
      <LibraryTopStrip
        {...props}
        drawerOpen={drawerOpen}
        filtersCollapsed={collapsed}
        onDrawerToggle={() => {
          onDrawerToggle();
          setDrawerOpen((current) => !current);
        }}
        onFiltersCollapsedChange={setCollapsed}
      />
    );
  }

  render(<ControlledTopStrip />);

  const panel = document.getElementById("library-filter-panel");
  const toggle = screen.getByRole("button", { name: /hide filters/i });
  expect(panel).toHaveAttribute("aria-hidden", "false");
  expect(toggle).toHaveTextContent("4");
  expect(screen.getByRole("button", { name: /advanced/i })).toBeInTheDocument();

  fireEvent.click(toggle);

  expect(onDrawerToggle).toHaveBeenCalledTimes(1);
  expect(toggle).toHaveAttribute("aria-expanded", "false");
  expect(panel).toHaveAttribute("aria-hidden", "true");
  expect(screen.getByRole("textbox", { name: /search library/i })).toBeInTheDocument();
  expect(screen.getByRole("combobox", { name: /sort by/i })).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: /advanced/i })).toBeNull();
  expect(screen.getByRole("button", { name: /show filters, 4 active/i })).toBeInTheDocument();

  fireEvent.click(screen.getByRole("button", { name: /show filters/i }));
  expect(panel).toHaveAttribute("aria-hidden", "false");
  expect(screen.getByRole("button", { name: /advanced/i })).toBeInTheDocument();
});

it("offers a persisted cozy Library glow control in Advanced", () => {
  const onAtmosphereChange = vi.fn();

  renderTopStrip("standard", {
    drawerOpen: true,
    atmosphereEnabled: false,
    onAtmosphereChange,
  });

  const cozyToggle = screen.getByRole("button", { name: /turn on cozy library glow/i });
  expect(cozyToggle).toHaveAttribute("aria-pressed", "false");
  expect(cozyToggle).toHaveTextContent("Cozy glow");

  fireEvent.click(cozyToggle);
  expect(onAtmosphereChange).toHaveBeenCalledWith(true);
});

it("shows the cozy Library glow control as pressed when atmosphere is on", () => {
  renderTopStrip("standard", {
    drawerOpen: true,
    atmosphereEnabled: true,
  });

  const cozyToggle = screen.getByRole("button", { name: /turn off cozy library glow/i });
  expect(cozyToggle).toHaveAttribute("aria-pressed", "true");
});

it("gives grid card sizing a continuous slider with an obvious handle", () => {
  const onDensityChange = vi.fn();

  renderTopStrip("standard", {
    viewMode: "grid",
    densityValue: 50,
    onDensityChange,
  });

  expect(screen.getByRole("group", { name: /grid card size/i })).toBeInTheDocument();
  expect(screen.getByText("Size")).toBeInTheDocument();

  const slider = screen.getByRole("slider", { name: /grid card size/i });
  expect(slider).toHaveAttribute("aria-valuetext", "50 percent card size");
  expect(screen.queryByRole("button", { name: /smaller grid cards/i })).toBeNull();
  expect(screen.queryByRole("button", { name: /larger grid cards/i })).toBeNull();

  fireEvent.change(slider, { target: { value: "75" } });
  expect(onDensityChange).toHaveBeenCalledWith(75);
});
