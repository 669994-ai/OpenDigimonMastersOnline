-- Digi Combine and Union Combine catalogs share an identical shape, so both
-- live in one table family keyed by `variant`: 0 = Digi Combine, 1 = Union Combine.

CREATE TABLE IF NOT EXISTS combine_ranks (
    id BIGSERIAL PRIMARY KEY,
    variant SMALLINT NOT NULL,
    ceiling_type SMALLINT NOT NULL,
    weight BIGINT NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_combine_ranks_variant
    ON combine_ranks (variant);

CREATE TABLE IF NOT EXISTS combine_rank_rewards (
    id BIGSERIAL PRIMARY KEY,
    rank_row_id BIGINT NOT NULL REFERENCES combine_ranks(id) ON DELETE CASCADE,
    item_id INTEGER NOT NULL,
    amount INTEGER NOT NULL DEFAULT 1,
    grade SMALLINT NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_combine_rank_rewards_rank
    ON combine_rank_rewards (rank_row_id);

CREATE TABLE IF NOT EXISTS combine_items (
    id BIGSERIAL PRIMARY KEY,
    variant SMALLINT NOT NULL,
    item_id INTEGER NOT NULL,
    group_id INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_combine_items_variant
    ON combine_items (variant, item_id);

CREATE TABLE IF NOT EXISTS combine_groups (
    id BIGSERIAL PRIMARY KEY,
    variant SMALLINT NOT NULL,
    group_id INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_combine_groups_variant
    ON combine_groups (variant, group_id);

CREATE TABLE IF NOT EXISTS combine_group_members (
    id BIGSERIAL PRIMARY KEY,
    group_row_id BIGINT NOT NULL REFERENCES combine_groups(id) ON DELETE CASCADE,
    member_id INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_combine_group_members_group
    ON combine_group_members (group_row_id);

CREATE TABLE IF NOT EXISTS combine_ceils (
    id BIGSERIAL PRIMARY KEY,
    variant SMALLINT NOT NULL,
    ceiling_type SMALLINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_combine_ceils_variant
    ON combine_ceils (variant, ceiling_type);

CREATE TABLE IF NOT EXISTS combine_ceil_entries (
    id BIGSERIAL PRIMARY KEY,
    ceil_row_id BIGINT NOT NULL REFERENCES combine_ceils(id) ON DELETE CASCADE,
    tier SMALLINT NOT NULL,
    value_a SMALLINT NOT NULL DEFAULT 0,
    value_b INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_combine_ceil_entries_ceil
    ON combine_ceil_entries (ceil_row_id);
