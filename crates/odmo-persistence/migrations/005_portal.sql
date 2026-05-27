-- Portal bridge tables: replace JSON file-based persistence with PostgreSQL.

-- Transfer tickets (account → character server handoff)
CREATE TABLE IF NOT EXISTS transfer_tickets (
    account_id    BIGINT PRIMARY KEY,
    token         TEXT NOT NULL,
    server_id     INTEGER NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Game session tickets (character → game server handoff)
CREATE TABLE IF NOT EXISTS game_session_tickets (
    account_id    BIGINT PRIMARY KEY,
    token         TEXT NOT NULL,
    character_id  BIGINT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Social notifications (friend connect, guild events, etc.)
CREATE TABLE IF NOT EXISTS social_notifications (
    id            BIGSERIAL PRIMARY KEY,
    character_id  BIGINT NOT NULL,
    kind          TEXT NOT NULL,
    payload       JSONB NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_social_notifications_character
    ON social_notifications(character_id);

-- Map presence (who is on which map/channel)
CREATE TABLE IF NOT EXISTS map_presence (
    character_id  BIGINT PRIMARY KEY,
    map_id        SMALLINT NOT NULL,
    channel       SMALLINT NOT NULL,
    name          TEXT NOT NULL,
    model         INTEGER NOT NULL,
    partner_model INTEGER NOT NULL,
    x             INTEGER NOT NULL,
    y             INTEGER NOT NULL,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_map_presence_map_channel
    ON map_presence(map_id, channel);
