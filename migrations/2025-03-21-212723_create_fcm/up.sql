CREATE TABLE fcm
(
    user_id text PRIMARY KEY NOT NULL,
    token   text             NOT NULL,
    topics  text[]           NULL
);

CREATE UNIQUE INDEX fcm_user_id_key ON fcm USING btree (user_id);

ALTER TABLE fcm
    ADD CONSTRAINT fcm_user_id_fkey FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE RESTRICT ON UPDATE CASCADE;