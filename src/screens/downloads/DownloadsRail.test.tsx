import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { DownloadsRail } from "./DownloadsRail";

it("shows compact lane counts and hides advanced filters in casual mode", () => {
  render(
    <DownloadsRail
      userView="beginner"
      watcherLabel="Watching"
      watchedPath="C:\\Users\\Player\\Downloads"
      lastCheckLabel="Last check 3/8/2026, 4:12 AM"
      activeItemsLabel="7 active item(s)"
      activeLane="ready_now"
      laneCounts={{
        ready_now: 1,
        special_setup: 2,
        waiting_on_you: 1,
        blocked: 3,
        done: 0,
      }}
      search=""
      statusFilter=""
      selectedPreset="Category First"
      presetOptions={["Category First"]}
      onLaneChange={() => {}}
      onSearchChange={() => {}}
      onStatusFilterChange={() => {}}
      onPresetChange={() => {}}
      onToggleFilters={() => {}}
    />,
  );

  expect(screen.getByRole("button", { name: /ready now/i })).toBeVisible();
  expect(screen.queryByRole("button", { name: /more filters/i })).not.toBeInTheDocument();
});
