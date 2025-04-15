CREATE TABLE fcm
(
    user_id text PRIMARY KEY NOT NULL REFERENCES users (id),
    token   text             NOT NULL,
    topics  text[]           NOT NULL CHECK ( array_position(topics, null) is null )
);