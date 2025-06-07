UPDATE users SET "access_token" = '' WHERE "access_token" IS NULL;
ALTER TABLE users ALTER COLUMN "access_token" SET NOT NULL;