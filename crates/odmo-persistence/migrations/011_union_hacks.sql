ALTER TABLE characters
    ADD COLUMN IF NOT EXISTS union_hack_slots JSONB NOT NULL DEFAULT '[]';
