CREATE TABLE IF NOT EXISTS extra_evolution_npcs (
    id BIGSERIAL PRIMARY KEY,
    npc_id INTEGER NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS extra_evolution_recipes (
    id BIGSERIAL PRIMARY KEY,
    npc_row_id BIGINT NOT NULL REFERENCES extra_evolution_npcs(id) ON DELETE CASCADE,
    exchange_type SMALLINT NOT NULL,
    object_id INTEGER NOT NULL,
    material_type SMALLINT NOT NULL,
    need_material_value INTEGER NOT NULL DEFAULT 0,
    price BIGINT NOT NULL DEFAULT 0,
    way_type SMALLINT NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_extra_evolution_recipes_npc
    ON extra_evolution_recipes (npc_row_id, exchange_type, object_id);

CREATE TABLE IF NOT EXISTS extra_evolution_materials (
    id BIGSERIAL PRIMARY KEY,
    recipe_row_id BIGINT NOT NULL REFERENCES extra_evolution_recipes(id) ON DELETE CASCADE,
    material_scope SMALLINT NOT NULL,
    material_id INTEGER NOT NULL,
    amount INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_extra_evolution_materials_recipe
    ON extra_evolution_materials (recipe_row_id, material_scope);
