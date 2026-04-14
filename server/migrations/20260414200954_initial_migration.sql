CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ships (
    id BIGSERIAL PRIMARY KEY,
    owner_id UUID NOT NULL REFERENCES users(id),
    stats JSONB NOT NULL,
    cargo JSONB NOT NULL,
    attack_mode TEXT NOT NULL,
    in_transit BOOLEAN NOT NULL DEFAULT FALSE,
    star_x INTEGER NOT NULL,
    star_y INTEGER NOT NULL,
    jump_ready_at TIMESTAMPTZ NOT NULL,
    health INTEGER NOT NULL,
    docked_at BIGINT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
