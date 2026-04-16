CREATE TABLE IF NOT EXISTS star_system_stock (
    star_location_id NUMERIC PRIMARY KEY, -- u128 handled as numeric in PG
    star_x INT NOT NULL,
    star_y INT NOT NULL,
    last_settled_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    capacity_kt DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    settled JSONB NOT NULL DEFAULT '[]' -- Vec<Material>
);

CREATE INDEX IF NOT EXISTS idx_star_system_stock_coords ON star_system_stock (star_x, star_y);

CREATE TABLE IF NOT EXISTS player_presence (
    id BIGSERIAL PRIMARY KEY,
    star_x INT NOT NULL,
    star_y INT NOT NULL,
    empire_id UUID NOT NULL,
    UNIQUE(star_x, star_y, empire_id)
);
