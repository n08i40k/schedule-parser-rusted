ALTER TABLE users RENAME COLUMN "version" TO android_version;
ALTER TABLE users ALTER COLUMN android_version DROP NOT NULL;