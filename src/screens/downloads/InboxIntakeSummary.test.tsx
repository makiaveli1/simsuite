import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { InboxIntakeSummary } from "./InboxIntakeSummary";

it("explains Inbox intake review without exposing file-changing actions", () => {
  const onNavigate = vi.fn();

  render(
    <InboxIntakeSummary
      userView="standard"
      totalItems={4}
      readyCount={2}
      reviewCount={1}
      blockedCount={1}
      watchedPath="C:\\Users\\Player\\Downloads"
      lastCheckLabel="Last check 5/14/2026, 10:00 PM"
      onNavigate={onNavigate}
    />,
  );

  expect(
    screen.getByRole("heading", {
      name: /Review new downloads and imported batches/i,
    }),
  ).toBeVisible();
  expect(screen.getByText(/No files changed/i)).toBeVisible();
  expect(screen.getByText(/before content becomes part of Library or Organize planning/i)).toBeVisible();
  const technicalDetails = screen
    .getByText(/Technical details/i)
    .closest("details") as HTMLDetailsElement;
  const watchedFolderValue = Array.from(
    technicalDetails.querySelectorAll("code"),
  ).find((node) => /Users.*Player.*Downloads/.test(node.textContent ?? ""));

  expect(watchedFolderValue).toBeTruthy();
  expect(technicalDetails.open).toBe(false);
  expect(watchedFolderValue).not.toBeVisible();

  fireEvent.click(screen.getByText(/Technical details/i));
  expect(technicalDetails.open).toBe(true);
  expect(watchedFolderValue).toBeVisible();

  fireEvent.click(screen.getByRole("button", { name: /Create preview plan/i }));
  expect(onNavigate).toHaveBeenCalledWith("organize");

  const enabledLabels = screen
    .getAllByRole("button")
    .filter((button) => !(button as HTMLButtonElement).disabled)
    .map((button) => button.textContent ?? "");
  expect(enabledLabels).not.toEqual(
    expect.arrayContaining([
      expect.stringMatching(
        /apply|move files|delete|clean up|cleanup|quarantine|commit|reject|fix/i,
      ),
    ]),
  );
});
