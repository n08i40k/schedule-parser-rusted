CREATE TABLE users
(
    id           text PRIMARY KEY NOT NULL,
    username     text UNIQUE      NOT NULL,
    password     text             NOT NULL,
    vk_id        int4             NULL,
    access_token text UNIQUE      NOT NULL,
    "group"      text             NOT NULL,
    role         user_role        NOT NULL,
    version      text             NOT NULL
);