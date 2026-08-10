CREATE TYPE "device_status" AS ENUM ('enrolling', 'online', 'offline', 'error');
CREATE TYPE "command_status" AS ENUM ('queued', 'sent', 'confirmed', 'failed');

CREATE TABLE "administrators" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "email" varchar(320) NOT NULL,
  "password_hash" text NOT NULL,
  "created_at" timestamp with time zone NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX "administrators_email_unique" ON "administrators" ("email");
CREATE TABLE "sessions" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "administrator_id" uuid REFERENCES "administrators"("id") ON DELETE CASCADE,
  "token_hash" text NOT NULL,
  "expires_at" timestamp with time zone NOT NULL,
  "created_at" timestamp with time zone NOT NULL DEFAULT now()
);
CREATE TABLE "passkeys" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "administrator_id" uuid NOT NULL REFERENCES "administrators"("id") ON DELETE CASCADE,
  "credential_id" text NOT NULL,
  "public_key" text NOT NULL,
  "counter" integer NOT NULL DEFAULT 0,
  "transports" jsonb,
  "created_at" timestamp with time zone NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX "passkeys_credential_id_unique" ON "passkeys" ("credential_id");
CREATE TABLE "webauthn_challenges" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "administrator_id" uuid NOT NULL REFERENCES "administrators"("id") ON DELETE CASCADE,
  "challenge" text NOT NULL,
  "purpose" varchar(32) NOT NULL,
  "expires_at" timestamp with time zone NOT NULL,
  "created_at" timestamp with time zone NOT NULL DEFAULT now()
);
CREATE TABLE "display_releases" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "version" integer NOT NULL,
  "page_id" varchar(64) NOT NULL,
  "document" jsonb NOT NULL,
  "preview_svg" text NOT NULL,
  "content_sha256" varchar(64) NOT NULL,
  "created_at" timestamp with time zone NOT NULL DEFAULT now()
);
CREATE TABLE "devices" (
  "id" varchar(64) PRIMARY KEY,
  "name" varchar(128) NOT NULL,
  "board_model" varchar(64) NOT NULL DEFAULT 'ESP32-S3-RLCD-4.2',
  "status" device_status NOT NULL DEFAULT 'enrolling',
  "firmware_version" varchar(64),
  "wifi_rssi" integer,
  "active_page_id" varchar(64) NOT NULL DEFAULT 'system',
  "release_id" uuid REFERENCES "display_releases"("id"),
  "last_seen_at" timestamp with time zone,
  "created_at" timestamp with time zone NOT NULL DEFAULT now()
);
CREATE TABLE "device_commands" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "device_id" varchar(64) NOT NULL REFERENCES "devices"("id") ON DELETE CASCADE,
  "action" varchar(64) NOT NULL,
  "payload" jsonb NOT NULL,
  "status" command_status NOT NULL DEFAULT 'queued',
  "error_message" text,
  "created_at" timestamp with time zone NOT NULL DEFAULT now(),
  "confirmed_at" timestamp with time zone
);
CREATE TABLE "api_tokens" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "label" varchar(128) NOT NULL,
  "token_hash" text NOT NULL,
  "scopes" jsonb NOT NULL,
  "revoked_at" timestamp with time zone,
  "created_at" timestamp with time zone NOT NULL DEFAULT now()
);
CREATE TABLE "audit_events" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "actor" varchar(128) NOT NULL,
  "action" varchar(128) NOT NULL,
  "target" varchar(256) NOT NULL,
  "metadata" jsonb NOT NULL,
  "created_at" timestamp with time zone NOT NULL DEFAULT now()
);
