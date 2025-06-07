ALTER TABLE users ADD telegram_id int8 NULL;
ALTER TABLE users ADD CONSTRAINT users_telegram_id_key UNIQUE (telegram_id);