CREATE TABLE IF NOT EXISTS map_mobs (
    id BIGSERIAL PRIMARY KEY,
    map_id SMALLINT NOT NULL,
    channel SMALLINT NOT NULL DEFAULT 0,
    handler INT NOT NULL,
    type_id INT NOT NULL,
    model INT NOT NULL,
    name VARCHAR(64) NOT NULL,
    level SMALLINT NOT NULL DEFAULT 1,
    x INT NOT NULL DEFAULT 0,
    y INT NOT NULL DEFAULT 0,
    previous_x INT NOT NULL DEFAULT 0,
    previous_y INT NOT NULL DEFAULT 0,
    current_hp INT NOT NULL DEFAULT 100,
    max_hp INT NOT NULL DEFAULT 100,
    current_ds INT NOT NULL DEFAULT 100,
    max_ds INT NOT NULL DEFAULT 100,
    alive BOOLEAN NOT NULL DEFAULT TRUE,
    respawn BOOLEAN NOT NULL DEFAULT TRUE,
    active_debuffs JSONB NOT NULL DEFAULT '[]',
    UNIQUE(map_id, channel, handler)
);

CREATE TABLE IF NOT EXISTS map_drops (
    id BIGSERIAL PRIMARY KEY,
    map_id SMALLINT NOT NULL,
    channel SMALLINT NOT NULL DEFAULT 0,
    handler INT NOT NULL,
    owner_id BIGINT NOT NULL DEFAULT 0,
    owner_handler INT NOT NULL DEFAULT 0,
    item_id INT NOT NULL,
    amount INT NOT NULL DEFAULT 1,
    x INT NOT NULL DEFAULT 0,
    y INT NOT NULL DEFAULT 0,
    owner_expires_at_unix BIGINT NOT NULL DEFAULT 0,
    expires_at_unix BIGINT NOT NULL DEFAULT 0,
    bits_drop BOOLEAN NOT NULL DEFAULT FALSE,
    no_owner BOOLEAN NOT NULL DEFAULT FALSE,
    collected BOOLEAN NOT NULL DEFAULT FALSE,
    UNIQUE(map_id, channel, handler)
);

CREATE TABLE IF NOT EXISTS server_config (
    key VARCHAR(64) PRIMARY KEY,
    value TEXT NOT NULL
);
