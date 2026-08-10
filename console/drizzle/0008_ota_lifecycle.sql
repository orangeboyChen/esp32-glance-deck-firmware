ALTER TABLE "devices" ADD COLUMN "last_good_firmware_release_id" uuid REFERENCES "firmware_releases"("id");
CREATE INDEX "devices_last_good_firmware_release_id_idx" ON "devices" ("last_good_firmware_release_id");
