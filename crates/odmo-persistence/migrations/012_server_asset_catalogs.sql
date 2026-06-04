CREATE TABLE IF NOT EXISTS evolution_assets (
    base_type INTEGER PRIMARY KEY,
    payload JSONB NOT NULL
);

CREATE TABLE IF NOT EXISTS item_assets (
    item_id INTEGER PRIMARY KEY,
    payload JSONB NOT NULL
);
