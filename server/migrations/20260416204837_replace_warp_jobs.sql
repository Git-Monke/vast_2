ALTER TABLE ships ADD COLUMN warp_completed_at TIMESTAMPTZ;
ALTER TABLE ships DROP COLUMN in_transit;
DROP TABLE warp_jobs;
