CREATE TABLE IF NOT EXISTS warp_jobs (
    scheduled_id BIGSERIAL PRIMARY KEY,
    scheduled_at TIMESTAMPTZ NOT NULL,
    ship_id BIGINT NOT NULL REFERENCES ships(id),
    to_star_x INTEGER NOT NULL,
    to_star_y INTEGER NOT NULL
);
