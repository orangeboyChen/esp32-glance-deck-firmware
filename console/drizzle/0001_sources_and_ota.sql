CREATE TYPE "source_status" AS ENUM ('active', 'paused', 'error');
CREATE TYPE "ota_job_status" AS ENUM ('queued', 'sent', 'downloading', 'verifying', 'rebooting', 'healthy', 'rolled_back', 'failed', 'cancelled');

CREATE TABLE "usage_sources" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "name" varchar(128) NOT NULL,
  "base_url" text NOT NULL,
  "request_path" text NOT NULL,
  "method" varchar(8) NOT NULL DEFAULT 'GET',
  "headers" jsonb NOT NULL DEFAULT '{}'::jsonb,
  "body_template" text,
  "secret_ciphertext" text NOT NULL,
  "mapper" jsonb NOT NULL,
  "refresh_interval_seconds" integer NOT NULL DEFAULT 900,
  "status" "source_status" NOT NULL DEFAULT 'active',
  "last_success_at" timestamp with time zone,
  "last_error" text,
  "created_at" timestamp with time zone NOT NULL DEFAULT now()
);
CREATE TABLE "source_snapshots" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "source_id" uuid NOT NULL REFERENCES "usage_sources"("id") ON DELETE CASCADE,
  "values" jsonb NOT NULL,
  "response_preview" text,
  "fetched_at" timestamp with time zone NOT NULL DEFAULT now()
);
CREATE TABLE "firmware_releases" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "version" varchar(64) NOT NULL,
  "board_model" varchar(64) NOT NULL,
  "channel" varchar(16) NOT NULL DEFAULT 'stable',
  "manifest_url" text NOT NULL,
  "image_url" text NOT NULL,
  "image_sha256" varchar(64) NOT NULL,
  "manifest_signature" text NOT NULL,
  "verified_at" timestamp with time zone NOT NULL DEFAULT now(),
  "created_at" timestamp with time zone NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX "firmware_releases_version_board_unique" ON "firmware_releases"("version", "board_model");
CREATE TABLE "ota_jobs" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "device_id" varchar(64) NOT NULL REFERENCES "devices"("id") ON DELETE CASCADE,
  "firmware_release_id" uuid NOT NULL REFERENCES "firmware_releases"("id"),
  "status" "ota_job_status" NOT NULL DEFAULT 'queued',
  "nonce" varchar(128) NOT NULL,
  "error_message" text,
  "created_at" timestamp with time zone NOT NULL DEFAULT now(),
  "completed_at" timestamp with time zone
);
ALTER TABLE "devices" ADD COLUMN "enrollment_code_hash" varchar(64);
ALTER TABLE "devices" ADD COLUMN "enrollment_expires_at" timestamp with time zone;
ALTER TABLE "devices" ADD COLUMN "mqtt_username" varchar(128);
ALTER TABLE "devices" ADD COLUMN "mqtt_password_ciphertext" text;
ALTER TABLE "webauthn_challenges" ALTER COLUMN "administrator_id" DROP NOT NULL;
