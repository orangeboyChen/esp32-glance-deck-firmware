CREATE TABLE "display_release_pages" (
  "release_id" uuid NOT NULL REFERENCES "display_releases"("id") ON DELETE CASCADE,
  "page_id" varchar(64) NOT NULL,
  "position" integer NOT NULL,
  "document" jsonb NOT NULL,
  "preview_svg" text NOT NULL,
  "device_image" bytea NOT NULL,
  "image_format" varchar(32) NOT NULL DEFAULT 'mono1-msb',
  "image_width" integer NOT NULL DEFAULT 400,
  "image_height" integer NOT NULL DEFAULT 300,
  "content_sha256" varchar(64) NOT NULL,
  PRIMARY KEY ("release_id", "page_id")
);
CREATE UNIQUE INDEX "display_release_pages_position_unique" ON "display_release_pages" ("release_id", "position");

-- Preserve already-published one-page releases as page resources.
INSERT INTO "display_release_pages" (
  "release_id", "page_id", "position", "document", "preview_svg", "device_image",
  "image_format", "image_width", "image_height", "content_sha256"
)
SELECT
  "id", "page_id", 0, "document", "preview_svg", "device_image",
  "image_format", "image_width", "image_height", "content_sha256"
FROM "display_releases";
