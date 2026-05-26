CREATE TABLE IF NOT EXISTS accounts (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR(32) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    email VARCHAR(128) NOT NULL,
    access_level SMALLINT NOT NULL DEFAULT 0,
    secondary_password VARCHAR(255),
    suspension_remaining_seconds INT,
    suspension_reason VARCHAR(255)
);
