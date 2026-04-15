-- Create Enums
CREATE TYPE ship_attack_mode AS ENUM (
    'Defend',
    'StrikeFirst'
);

CREATE TYPE building_kind AS ENUM (
    'MiningDepot',
    'Warehouse',
    'MilitaryGarrison',
    'SalesDepot',
    'ShipDepot',
    'Radar'
);

-- Alter existing ships table to use the new enum
ALTER TABLE ships ALTER COLUMN attack_mode TYPE ship_attack_mode USING attack_mode::ship_attack_mode;

-- Create buildings table
CREATE TABLE IF NOT EXISTS buildings (
    id BIGSERIAL PRIMARY KEY,
    star_x INTEGER NOT NULL,
    star_y INTEGER NOT NULL,
    planet_index SMALLINT NOT NULL,
    slot_index SMALLINT NOT NULL,
    kind building_kind NOT NULL,
    level INTEGER NOT NULL DEFAULT 1,
    degradation_percent REAL NOT NULL DEFAULT 0.0,
    -- Which vein this depot targets. MaterialKind enum (e.g. 'Iron', 'Helium').
    mining_material TEXT,
    -- Owner empire (User ID), used for garrisons, radars, and sales depots
    owner_id UUID REFERENCES users(id),
    -- MilitaryGarrison only: combat posture. Uses same enum as ships.
    attack_mode ship_attack_mode,
    health INTEGER NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),

    -- Ensure a slot on a planet only has one building
    UNIQUE (star_x, star_y, planet_index, slot_index)
);
