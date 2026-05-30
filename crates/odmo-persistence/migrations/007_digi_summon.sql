CREATE TABLE IF NOT EXISTS digi_summon_products (
    id BIGSERIAL PRIMARY KEY,
    product_id INTEGER NOT NULL UNIQUE,
    string_id INTEGER NOT NULL DEFAULT 0,
    draw_count INTEGER NOT NULL DEFAULT 1,
    rank INTEGER NOT NULL DEFAULT 0,
    remaining_daily_limit INTEGER NOT NULL DEFAULT 0,
    icon TEXT NOT NULL DEFAULT '',
    name TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS digi_summon_tickets (
    id BIGSERIAL PRIMARY KEY,
    product_row_id BIGINT NOT NULL REFERENCES digi_summon_products(id) ON DELETE CASCADE,
    item_id INTEGER NOT NULL,
    cost INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_digi_summon_tickets_product
    ON digi_summon_tickets(product_row_id);

CREATE TABLE IF NOT EXISTS digi_summon_rewards (
    id BIGSERIAL PRIMARY KEY,
    product_row_id BIGINT NOT NULL REFERENCES digi_summon_products(id) ON DELETE CASCADE,
    item_list_id INTEGER NOT NULL DEFAULT 0,
    item_id INTEGER NOT NULL,
    grade INTEGER NOT NULL DEFAULT 0,
    amount INTEGER NOT NULL DEFAULT 1,
    weight INTEGER NOT NULL DEFAULT 0,
    reward_group INTEGER NOT NULL DEFAULT 0,
    group_code INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_digi_summon_rewards_product
    ON digi_summon_rewards(product_row_id);
