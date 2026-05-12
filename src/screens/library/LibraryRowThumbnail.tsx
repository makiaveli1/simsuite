import { ImageOff } from "lucide-react";
import type { LibraryRowModel } from "./libraryDisplay";

interface LibraryRowThumbnailProps {
  model: Pick<LibraryRowModel, "thumbnailPreview" | "previewSource" | "typeColor" | "typeLabel">;
}

function previewSourceLabel(source: LibraryRowModel["previewSource"]) {
  if (source === "embedded") return "Embedded preview";
  if (source === "cache") return "Cached preview";
  if (source === "external") return "External preview";
  return "Preview";
}

export function LibraryRowThumbnail({ model }: LibraryRowThumbnailProps) {
  const hasPreview = Boolean(model.thumbnailPreview);
  const sourceLabel = previewSourceLabel(model.previewSource);
  const title = hasPreview ? `${model.typeLabel} - ${sourceLabel}` : `${model.typeLabel} - no preview available`;

  return (
    <div
      className={[
        "library-row-thumb-frame",
        hasPreview ? "library-row-thumb-real" : "library-row-thumb-fallback",
        `library-row-thumb-frame--${model.typeColor}`,
        !hasPreview ? `library-row-thumb-fallback--${model.typeColor}` : "",
      ]
        .filter(Boolean)
        .join(" ")}
      title={title}
      aria-label={title}
    >
      {hasPreview ? (
        <>
          <img
            src={`data:image/png;base64,${model.thumbnailPreview}`}
            alt=""
            className="library-row-thumb-img"
            loading="lazy"
            decoding="async"
          />
          {model.previewSource && model.previewSource !== "fallback" ? (
            <span
              className={`library-row-thumb-source-dot library-row-thumb-source-dot--${model.previewSource}`}
              aria-hidden="true"
            />
          ) : null}
        </>
      ) : (
        <span className="library-row-thumb-fallback-icon" aria-hidden="true">
          <ImageOff size={15} strokeWidth={2} />
        </span>
      )}
    </div>
  );
}
