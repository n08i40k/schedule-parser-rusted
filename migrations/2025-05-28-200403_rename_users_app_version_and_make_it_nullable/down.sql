UPDATE users SET "android_version" = '' WHERE "android_version" IS NULL;
ALTER TABLE users ALTER COLUMN "android_version" SET NOT NULL;
ALTER TABLE users RENAME COLUMN android_version TO "version";