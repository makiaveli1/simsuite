import type { Screen, WatchListFilter } from "./types";

export interface UpdatesNavigationParams {
  mode?: "tracked" | "setup" | "review";
  filter?: WatchListFilter;
  fileId?: number;
}

export function resolveUpdatesNavigationParams(search: string): UpdatesNavigationParams {
  const params = new URLSearchParams(search);
  const mode = params.get("mode") as "tracked" | "setup" | "review" | null;
  const filter = params.get("filter") as WatchListFilter | null;
  const fileIdValue = params.get("fileId");
  const fileId = fileIdValue ? Number(fileIdValue) : Number.NaN;

  return {
    mode:
      mode === "tracked" || mode === "setup" || mode === "review"
        ? mode
        : undefined,
    filter:
      filter === "attention" ||
      filter === "exact_updates" ||
      filter === "possible_updates" ||
      filter === "unclear" ||
      filter === "all"
        ? filter
        : undefined,
    fileId: Number.isFinite(fileId) ? fileId : undefined,
  };
}

export function createUpdatesNavigationParams(
  mode?: UpdatesNavigationParams["mode"],
  filter?: WatchListFilter,
  fileId?: number,
): UpdatesNavigationParams {
  return { mode, filter, fileId };
}

export function nextUpdatesParamsForPlainNavigation(
  targetScreen: Screen,
  current: UpdatesNavigationParams,
): UpdatesNavigationParams {
  return targetScreen === "updates" ? {} : current;
}
