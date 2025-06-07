UPDATE users SET "group" = '' WHERE "group" IS NULL;
ALTER TABLE users ALTER COLUMN "group" SET NOT NULL;