ALTER TABLE "display_releases" ADD COLUMN "device_image" bytea;
ALTER TABLE "display_releases" ADD COLUMN "image_format" varchar(32) NOT NULL DEFAULT 'mono1-msb';
ALTER TABLE "display_releases" ADD COLUMN "image_width" integer NOT NULL DEFAULT 400;
ALTER TABLE "display_releases" ADD COLUMN "image_height" integer NOT NULL DEFAULT 300;

-- Existing SVG-only releases cannot be safely displayed by firmware. Re-publish
-- them after this migration to create an immutable bitmap release.
UPDATE "display_releases" SET "device_image" = ''::bytea WHERE "device_image" IS NULL;
ALTER TABLE "display_releases" ALTER COLUMN "device_image" SET NOT NULL;
