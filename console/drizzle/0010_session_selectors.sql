ALTER TABLE "sessions" ADD COLUMN "token_selector" varchar(32);
CREATE UNIQUE INDEX "sessions_token_selector_unique" ON "sessions" ("token_selector");
