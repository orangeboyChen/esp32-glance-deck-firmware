CREATE TABLE "display_bindings" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "source_id" uuid NOT NULL REFERENCES "usage_sources"("id") ON DELETE CASCADE,
  "page_id" varchar(64) NOT NULL,
  "document_template" jsonb NOT NULL,
  "device_ids" jsonb NOT NULL,
  "created_at" timestamp with time zone NOT NULL DEFAULT now()
);
