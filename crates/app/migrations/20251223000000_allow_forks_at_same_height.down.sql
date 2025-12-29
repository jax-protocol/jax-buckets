-- Revert to requiring unique height per bucket
-- Note: This will fail if there are forks at the same height

CREATE TABLE bucket_log_old (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bucket_id TEXT NOT NULL,
    name TEXT NOT NULL,
    current_link VARCHAR(255) NOT NULL,
    previous_link VARCHAR(255),
    height INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(bucket_id, height),
    UNIQUE(bucket_id, current_link)
);

INSERT INTO bucket_log_old (id, bucket_id, name, current_link, previous_link, height, created_at)
SELECT id, bucket_id, name, current_link, previous_link, height, created_at FROM bucket_log;

DROP TABLE bucket_log;

ALTER TABLE bucket_log_old RENAME TO bucket_log;

CREATE INDEX idx_bucket_log_bucket_height ON bucket_log(bucket_id, height DESC);
CREATE INDEX idx_bucket_log_bucket_link ON bucket_log(bucket_id, current_link);
