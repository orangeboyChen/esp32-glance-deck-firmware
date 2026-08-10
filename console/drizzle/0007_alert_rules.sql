CREATE TYPE "alert_operator" AS ENUM ('gt', 'gte', 'lt', 'lte', 'eq', 'neq', 'contains');
CREATE TABLE "alert_rules" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "name" varchar(128) NOT NULL,
  "source_id" uuid NOT NULL REFERENCES "usage_sources"("id") ON DELETE CASCADE,
  "field" varchar(32) NOT NULL,
  "operator" "alert_operator" NOT NULL,
  "threshold" text NOT NULL,
  "severity" varchar(16) NOT NULL DEFAULT 'warning',
  "message" varchar(256) NOT NULL,
  "device_ids" jsonb NOT NULL,
  "page_ids" jsonb NOT NULL,
  "enabled" boolean NOT NULL DEFAULT true,
  "test_only" boolean NOT NULL DEFAULT false,
  "active" boolean NOT NULL DEFAULT false,
  "last_value" jsonb,
  "last_evaluated_at" timestamp with time zone,
  "last_triggered_at" timestamp with time zone,
  "created_at" timestamp with time zone NOT NULL DEFAULT now()
);
CREATE INDEX "alert_rules_source_id_idx" ON "alert_rules" ("source_id");
