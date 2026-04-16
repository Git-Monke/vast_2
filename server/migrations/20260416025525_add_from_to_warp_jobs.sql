-- Add migration script here
ALTER TABLE warp_jobs ADD COLUMN from_star_x INT NOT NULL DEFAULT 0;
ALTER TABLE warp_jobs ADD COLUMN from_star_y INT NOT NULL DEFAULT 0;
ALTER TABLE warp_jobs ALTER COLUMN from_star_x DROP DEFAULT;
ALTER TABLE warp_jobs ALTER COLUMN from_star_y DROP DEFAULT;
