CREATE TABLE user_custom_statuses (
    user_id BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    emoji VARCHAR NULL,
    status_text VARCHAR NOT NULL,
    expires_at TIMESTAMP NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX index_user_custom_statuses_on_expires_at
    ON user_custom_statuses (expires_at)
    WHERE expires_at IS NOT NULL;
