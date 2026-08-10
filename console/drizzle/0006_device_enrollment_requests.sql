CREATE TABLE "device_enrollment_requests" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  "pairing_code_hash" varchar(64) NOT NULL,
  "claim_secret_hash" varchar(64) NOT NULL,
  "board_model" varchar(64) NOT NULL,
  "claimed_device_id" varchar(64) REFERENCES "devices"("id") ON DELETE SET NULL,
  "expires_at" timestamp with time zone NOT NULL,
  "created_at" timestamp with time zone NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX "device_enrollment_requests_pairing_code_unique" ON "device_enrollment_requests"("pairing_code_hash");
