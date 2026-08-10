ALTER TABLE "devices" ADD COLUMN "enabled_page_ids" jsonb;
ALTER TABLE "devices" ADD COLUMN "desired_page_id" varchar(64);
ALTER TABLE "devices" ADD COLUMN "power_source" varchar(16);
ALTER TABLE "devices" ADD COLUMN "charging" boolean;
ALTER TABLE "devices" ADD COLUMN "battery_percent" integer;
ALTER TABLE "devices" ADD COLUMN "battery_mv" integer;
ALTER TABLE "devices" ADD COLUMN "power_updated_at" timestamp with time zone;

-- Existing releases are one-page releases. Preserve their only page as enabled.
UPDATE "devices" SET "enabled_page_ids" = jsonb_build_array("active_page_id")
WHERE "enabled_page_ids" IS NULL;

-- active_page_id remains the device-confirmed value. Existing devices retain their
-- current page as the desired target until the first console configuration update.
UPDATE "devices" SET "desired_page_id" = "active_page_id"
WHERE "desired_page_id" IS NULL;
